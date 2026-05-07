//! 3MF Laser Toolpath Loader
//!
//! Parses 3MF files containing the Laser Toolpath Extension.
//! A `.3mf` file is an OPC (ZIP) package that may contain one or more
//! `<toolpathresource>` elements, each referencing per-layer XML files
//! with segment geometry (loops, polylines, hatches).
//!
//! Spec reference:
//! <https://github.com/3MFConsortium/spec_lasertoolpath/blob/master/3MF%20Laser%20Toolpath%20Extension.md>
//!
//! Initial scope:
//! - Planar toolpaths only (2.5D)
//! - ASCII XML layer data only (no binary encoding)
//! - Profile parameters (laser power, speed) mapped to VectorParameters

use crate::application::ports::{FileError, FileLoader, FileResult};
use crate::domain::entities::{
    Layer, SliceStack, Toolpath, ToolpathMetadata, Vector, VectorParameters, VectorType,
};
use crate::domain::value_objects::Point2D;
use log::{debug, info, trace, warn};
use quick_xml::events::Event;
use quick_xml::Reader;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

// ── Internal types ───────────────────────────────────────────────────────

/// Laser/process parameters extracted from a toolpath profile.
#[derive(Debug, Clone, Default)]
struct ToolpathProfile {
    uuid: String,
    name: String,
    laser_power: Option<f32>,
    laser_speed: Option<f32>,
    jump_speed: Option<f32>,
    laser_focus: Option<f32>,
    laser_index: Option<i32>,
}

#[derive(Debug, Clone)]
struct LayerRef {
    ztop: f32,
    path: String,
}

#[derive(Debug, Clone)]
struct ToolpathResource {
    id: String,
    uuid: String,
    unitfactor: f32,
    profiles: Vec<ToolpathProfile>,
    layer_refs: Vec<LayerRef>,
    zbottom: f32,
}

/// Segment geometry type parsed from the `type` attribute.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SegmentType {
    Loop,
    Polyline,
    Hatch,
}

/// Layer parsing context — tracks where we are in the XML tree.
enum LayerContext {
    Root,
    Profiles,
    Segments,
    Segment {
        seg_type: SegmentType,
        profile_uuid: Option<String>,
    },
}

// ── Public loader ────────────────────────────────────────────────────────

/// Loader for 3MF files with the Laser Toolpath Extension.
pub struct ThreemfLoader;

impl ThreemfLoader {
    pub fn new() -> Self {
        Self
    }

    /// Load all toolpath resources from a 3MF file.
    /// Each `<toolpathresource>` becomes a separate `(name, Toolpath)`.
    fn load_resources(&self, path: &Path) -> FileResult<Vec<(String, Toolpath)>> {
        let file = File::open(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileError::NotFound(path.display().to_string()),
            std::io::ErrorKind::PermissionDenied => {
                FileError::PermissionDenied(path.display().to_string())
            }
            _ => FileError::IoError(e),
        })?;

        let mut archive = ZipArchive::new(BufReader::new(file)).map_err(|e| {
            FileError::DecompressionError(format!("Failed to open 3MF ZIP: {}", e))
        })?;

        // Read the 3D model file
        let model_xml = Self::read_zip_entry(&mut archive, "3D/3dmodel.model")?;

        // Parse all toolpath resources from the model
        let resources = Self::parse_model(&model_xml)?;

        if resources.is_empty() {
            return Err(FileError::InvalidFormat(
                "No toolpath resources found in 3MF file".to_string(),
            ));
        }

        info!(
            "Found {} toolpath resource(s) in {:?}",
            resources.len(),
            path
        );

        // Process each resource
        let mut results = Vec::with_capacity(resources.len());

        for resource in &resources {
            info!(
                "Resource '{}': unitfactor={}, {} profiles, {} layers",
                resource.uuid,
                resource.unitfactor,
                resource.profiles.len(),
                resource.layer_refs.len(),
            );

            // Phase 1: Extract all layer XML data from ZIP (sequential, ZipArchive is !Send)
            let mut layer_data: Vec<(f32, Vec<u8>)> =
                Vec::with_capacity(resource.layer_refs.len());
            for lr in &resource.layer_refs {
                let zip_path = lr.path.trim_start_matches('/');
                match Self::read_zip_entry_bytes(&mut archive, zip_path) {
                    Ok(data) => {
                        let z_height = lr.ztop * resource.unitfactor;
                        layer_data.push((z_height, data));
                    }
                    Err(e) => {
                        warn!("Failed to read layer '{}': {}", lr.path, e);
                    }
                }
            }

            // Phase 2: Parse layers in parallel
            let unitfactor = resource.unitfactor;
            let profiles_owned: HashMap<String, ToolpathProfile> = resource
                .profiles
                .iter()
                .map(|p| (p.uuid.clone(), p.clone()))
                .collect();

            let layers: Vec<Layer> = layer_data
                .into_par_iter()
                .enumerate()
                .filter_map(|(idx, (z_height, data))| {
                    let xml = match std::str::from_utf8(&data) {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("Invalid UTF-8 in layer {}: {}", idx, e);
                            return None;
                        }
                    };
                    match Self::parse_layer(xml, idx, z_height, unitfactor, &profiles_owned) {
                        Ok(layer) => Some(layer),
                        Err(e) => {
                            warn!("Failed to parse layer {}: {}", idx, e);
                            None
                        }
                    }
                })
                .collect();

            // Sort by Z and fix indices
            let mut layers = layers;
            layers.sort_by(|a, b| a.z_height.partial_cmp(&b.z_height).unwrap());
            for (i, layer) in layers.iter_mut().enumerate() {
                layer.index = i;
            }

            let stats_layers = layers.len();
            let stats_vectors: usize = layers.iter().map(|l| l.vectors.len()).sum();
            let stats_points: usize = layers.iter().map(|l| l.point_count()).sum();

            info!(
                "Resource '{}': {} layers, {} vectors, {} points",
                resource.uuid, stats_layers, stats_vectors, stats_points
            );

            let resource_name = format!("3MF Resource {}", resource.id);

            let mut slice_stack = SliceStack::with_layers(&resource_name, layers);
            slice_stack.source_path = Some(path.display().to_string());

            let mut metadata = ToolpathMetadata::default();
            metadata.format = "3MF".to_string();
            metadata
                .properties
                .insert("resource_uuid".to_string(), resource.uuid.clone());
            metadata
                .properties
                .insert("unitfactor".to_string(), resource.unitfactor.to_string());

            let toolpath = Toolpath::with_metadata(slice_stack, metadata);
            results.push((resource_name, toolpath));
        }

        Ok(results)
    }

    // ── ZIP helpers ──────────────────────────────────────────────────────

    fn read_zip_entry(archive: &mut ZipArchive<BufReader<File>>, name: &str) -> FileResult<String> {
        let bytes = Self::read_zip_entry_bytes(archive, name)?;
        String::from_utf8(bytes).map_err(|e| {
            FileError::InvalidFormat(format!("Invalid UTF-8 in {}: {}", name, e))
        })
    }

    fn read_zip_entry_bytes(
        archive: &mut ZipArchive<BufReader<File>>,
        name: &str,
    ) -> FileResult<Vec<u8>> {
        let mut entry = archive.by_name(name).map_err(|e| {
            FileError::InvalidFormat(format!("Missing entry '{}': {}", name, e))
        })?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(|e| {
            FileError::DecompressionError(format!("Failed to read '{}': {}", name, e))
        })?;
        Ok(buf)
    }

    // ── Model XML parsing ────────────────────────────────────────────────

    /// Parse the 3D model XML to extract toolpath resources.
    /// Handles both `tp:` and `t:` namespace prefixes (both are used in practice).
    fn parse_model(xml: &str) -> FileResult<Vec<ToolpathResource>> {
        let mut reader = Reader::from_str(xml);
        let mut resources = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = e.name().as_ref().to_vec();
                    if local_name_eq(&name, b"toolpathresource") {
                        let resource = Self::parse_toolpath_resource(&mut reader, e)?;
                        resources.push(resource);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(FileError::ParseError {
                        line: 0,
                        message: format!("XML parse error in model: {}", e),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(resources)
    }

    /// Parse a single `<toolpathresource>` element and its children.
    fn parse_toolpath_resource(
        reader: &mut Reader<&[u8]>,
        start: &quick_xml::events::BytesStart,
    ) -> FileResult<ToolpathResource> {
        let mut resource = ToolpathResource {
            id: String::new(),
            uuid: String::new(),
            unitfactor: 1.0,
            profiles: Vec::new(),
            layer_refs: Vec::new(),
            zbottom: 0.0,
        };

        // Read attributes from the start tag
        for attr in start.attributes().flatten() {
            let key = attr.key.as_ref().to_vec();
            let val = String::from_utf8_lossy(&attr.value).to_string();
            match local_name_ref(&key) {
                b"id" => resource.id = val,
                b"uuid" => resource.uuid = val,
                b"unitfactor" => resource.unitfactor = val.parse().unwrap_or(1.0),
                _ => {}
            }
        }

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().as_ref().to_vec();
                    match local_name_ref(&name) {
                        b"toolpathprofile" => {
                            resource.profiles.push(Self::parse_profile(e));
                        }
                        b"toolpathlayers" => {
                            for attr in e.attributes().flatten() {
                                let k = attr.key.as_ref().to_vec();
                                if local_name_ref(&k) == b"zbottom" {
                                    let val = String::from_utf8_lossy(&attr.value);
                                    resource.zbottom = val.parse().unwrap_or(0.0);
                                }
                            }
                        }
                        b"toolpathlayer" => {
                            resource.layer_refs.push(Self::parse_layer_ref(e));
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().as_ref().to_vec();
                    if local_name_eq(&name, b"toolpathresource") {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(FileError::ParseError {
                        line: 0,
                        message: format!("XML error in toolpathresource: {}", e),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(resource)
    }

    /// Parse a `<toolpathprofile>` element's attributes.
    fn parse_profile(e: &quick_xml::events::BytesStart) -> ToolpathProfile {
        let mut profile = ToolpathProfile::default();
        for attr in e.attributes().flatten() {
            let key = attr.key.as_ref().to_vec();
            let val = String::from_utf8_lossy(&attr.value);
            match local_name_ref(&key) {
                b"uuid" => profile.uuid = val.to_string(),
                b"name" => profile.name = val.to_string(),
                b"laserpower" => profile.laser_power = val.parse().ok(),
                b"laserspeed" => profile.laser_speed = val.parse().ok(),
                b"jumpspeed" => profile.jump_speed = val.parse().ok(),
                b"laserfocus" => profile.laser_focus = val.parse().ok(),
                b"laserindex" => profile.laser_index = val.parse().ok(),
                _ => {}
            }
        }
        profile
    }

    /// Parse a `<toolpathlayer>` element's attributes.
    fn parse_layer_ref(e: &quick_xml::events::BytesStart) -> LayerRef {
        let mut ztop = 0.0f32;
        let mut path = String::new();
        for attr in e.attributes().flatten() {
            let key = attr.key.as_ref().to_vec();
            let val = String::from_utf8_lossy(&attr.value);
            match local_name_ref(&key) {
                b"ztop" => ztop = val.parse().unwrap_or(0.0),
                b"path" => path = val.to_string(),
                _ => {}
            }
        }
        LayerRef { ztop, path }
    }

    // ── Layer XML parsing ────────────────────────────────────────────────

    /// Parse a single layer XML file.
    fn parse_layer(
        xml: &str,
        index: usize,
        z_height: f32,
        unitfactor: f32,
        profiles: &HashMap<String, ToolpathProfile>,
    ) -> FileResult<Layer> {
        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();

        let mut local_profiles: HashMap<String, String> = HashMap::new();
        let mut layer = Layer::new(index, z_height);
        let mut vector_id: u64 = 0;
        let mut ctx = LayerContext::Root;
        let mut segment_points: Vec<Point2D> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = e.name().as_ref().to_vec();
                    Self::handle_layer_open_tag(
                        &name, e, false, unitfactor, profiles,
                        &mut ctx, &mut local_profiles, &mut segment_points,
                        &mut layer, &mut vector_id,
                    );
                }
                Ok(Event::Empty(ref e)) => {
                    let name = e.name().as_ref().to_vec();
                    Self::handle_layer_open_tag(
                        &name, e, true, unitfactor, profiles,
                        &mut ctx, &mut local_profiles, &mut segment_points,
                        &mut layer, &mut vector_id,
                    );
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().as_ref().to_vec();
                    let local = local_name_ref(&name);
                    match local {
                        b"segment" => {
                            Self::finalize_segment(
                                &ctx, profiles, &mut segment_points,
                                &mut layer, &mut vector_id,
                            );
                            ctx = LayerContext::Segments;
                        }
                        b"parts" | b"profiles" | b"segments" => ctx = LayerContext::Root,
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(FileError::ParseError {
                        line: 0,
                        message: format!("XML parse error in layer: {}", e),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        trace!(
            "Layer {} (z={}): {} vectors",
            index,
            z_height,
            layer.vectors.len()
        );
        Ok(layer)
    }

    /// Read `z` and `index` attributes from the root `<layer>` element.
    /// Some 3MF files store layer height here instead of on `<toolpathlayer ztop="...">`.
    fn read_layer_root_attrs(
        e: &quick_xml::events::BytesStart,
        unitfactor: f32,
        layer: &mut Layer,
    ) {
        for attr in e.attributes().flatten() {
            let key = attr.key.as_ref().to_vec();
            let val = String::from_utf8_lossy(&attr.value);
            match local_name_ref(&key) {
                b"z" => {
                    if let Ok(z) = val.parse::<f32>() {
                        layer.z_height = z * unitfactor;
                    }
                }
                b"index" => {
                    if let Ok(idx) = val.parse::<usize>() {
                        layer.index = idx;
                    }
                }
                _ => {}
            }
        }
    }

    /// Handle an opening or self-closing tag within a layer XML.
    fn handle_layer_open_tag(
        tag_name: &[u8],
        e: &quick_xml::events::BytesStart,
        _is_empty: bool,
        unitfactor: f32,
        profiles: &HashMap<String, ToolpathProfile>,
        ctx: &mut LayerContext,
        local_profiles: &mut HashMap<String, String>,
        segment_points: &mut Vec<Point2D>,
        layer: &mut Layer,
        vector_id: &mut u64,
    ) {
        let local = local_name_ref(tag_name);
        match local {
            b"layer" => Self::read_layer_root_attrs(e, unitfactor, layer),
            b"parts" => *ctx = LayerContext::Root, // parts section, skip
            b"profiles" => *ctx = LayerContext::Profiles,
            b"segments" => *ctx = LayerContext::Segments,
            b"profile" => {
                if matches!(ctx, LayerContext::Profiles) {
                    let (id, uuid) = Self::parse_local_profile(e);
                    local_profiles.insert(id, uuid);
                }
            }
            b"segment" => {
                let (seg_type, profile_id) = Self::parse_segment_attrs(e);
                let profile_uuid = profile_id
                    .and_then(|pid| local_profiles.get(&pid))
                    .cloned();
                *ctx = LayerContext::Segment {
                    seg_type,
                    profile_uuid,
                };
                segment_points.clear();
            }
            b"point" => {
                if matches!(ctx, LayerContext::Segment { .. }) {
                    let (x, y) = Self::parse_point_attrs(e);
                    segment_points.push(Point2D::new(
                        x as f32 * unitfactor,
                        y as f32 * unitfactor,
                    ));
                }
            }
            b"hatch" => {
                if let LayerContext::Segment {
                    seg_type: SegmentType::Hatch,
                    ref profile_uuid,
                } = ctx
                {
                    let (x1, y1, x2, y2) = Self::parse_hatch_attrs(e);
                    let p1 = Point2D::new(x1 as f32 * unitfactor, y1 as f32 * unitfactor);
                    let p2 = Point2D::new(x2 as f32 * unitfactor, y2 as f32 * unitfactor);
                    *vector_id += 1;
                    let mut vec = Vector::new(VectorType::Hatch, vec![p1, p2]);
                    vec.id = *vector_id;
                    if let Some(uuid) = profile_uuid {
                        if let Some(prof) = profiles.get(uuid.as_str()) {
                            vec.parameters = Self::profile_to_params(prof);
                        }
                    }
                    layer.add_vector(vec);
                }
            }
            _ => {}
        }
    }

    /// Finalize a segment when its closing tag is reached.
    fn finalize_segment(
        ctx: &LayerContext,
        profiles: &HashMap<String, ToolpathProfile>,
        segment_points: &mut Vec<Point2D>,
        layer: &mut Layer,
        vector_id: &mut u64,
    ) {
        if let LayerContext::Segment {
            seg_type,
            ref profile_uuid,
        } = ctx
        {
            let params = profile_uuid
                .as_ref()
                .and_then(|uuid| profiles.get(uuid.as_str()))
                .map(|prof| Self::profile_to_params(prof));

            match seg_type {
                SegmentType::Loop if segment_points.len() >= 3 => {
                    *vector_id += 1;
                    let mut vec = Vector::polygon(
                        VectorType::Contour,
                        std::mem::take(segment_points),
                    );
                    vec.id = *vector_id;
                    if let Some(p) = params {
                        vec.parameters = p;
                    }
                    layer.add_vector(vec);
                }
                SegmentType::Polyline if segment_points.len() >= 2 => {
                    *vector_id += 1;
                    let mut vec = Vector::new(
                        VectorType::Contour,
                        std::mem::take(segment_points),
                    );
                    vec.id = *vector_id;
                    if let Some(p) = params {
                        vec.parameters = p;
                    }
                    layer.add_vector(vec);
                }
                _ => {
                    segment_points.clear();
                }
            }
        }
    }

    /// Parse local `<profile id="..." uuid="..."/>` in a layer.
    fn parse_local_profile(e: &quick_xml::events::BytesStart) -> (String, String) {
        let mut id = String::new();
        let mut uuid = String::new();
        for attr in e.attributes().flatten() {
            let key = attr.key.as_ref().to_vec();
            let val = String::from_utf8_lossy(&attr.value);
            match local_name_ref(&key) {
                b"id" => id = val.to_string(),
                b"uuid" => uuid = val.to_string(),
                _ => {}
            }
        }
        (id, uuid)
    }

    /// Parse `<segment type="..." profileid="...">` attributes.
    fn parse_segment_attrs(e: &quick_xml::events::BytesStart) -> (SegmentType, Option<String>) {
        let mut seg_type = SegmentType::Polyline;
        let mut profile_id = None;
        for attr in e.attributes().flatten() {
            let key = attr.key.as_ref().to_vec();
            let val = String::from_utf8_lossy(&attr.value);
            match local_name_ref(&key) {
                b"type" => {
                    seg_type = match val.as_ref() {
                        "loop" => SegmentType::Loop,
                        "polyline" => SegmentType::Polyline,
                        "hatch" => SegmentType::Hatch,
                        _ => {
                            debug!("Unknown segment type: {}", val);
                            SegmentType::Polyline
                        }
                    };
                }
                b"profileid" => profile_id = Some(val.to_string()),
                _ => {}
            }
        }
        (seg_type, profile_id)
    }

    /// Parse `<point x="..." y="..."/>` attributes.
    fn parse_point_attrs(e: &quick_xml::events::BytesStart) -> (f64, f64) {
        let mut x = 0.0f64;
        let mut y = 0.0f64;
        for attr in e.attributes().flatten() {
            let key = attr.key.as_ref().to_vec();
            let val = String::from_utf8_lossy(&attr.value);
            match local_name_ref(&key) {
                b"x" => x = val.parse().unwrap_or(0.0),
                b"y" => y = val.parse().unwrap_or(0.0),
                _ => {}
            }
        }
        (x, y)
    }

    /// Parse `<hatch x1="..." y1="..." x2="..." y2="..."/>` attributes.
    fn parse_hatch_attrs(e: &quick_xml::events::BytesStart) -> (f64, f64, f64, f64) {
        let mut x1 = 0.0f64;
        let mut y1 = 0.0f64;
        let mut x2 = 0.0f64;
        let mut y2 = 0.0f64;
        for attr in e.attributes().flatten() {
            let key = attr.key.as_ref().to_vec();
            let val = String::from_utf8_lossy(&attr.value);
            match local_name_ref(&key) {
                b"x1" => x1 = val.parse().unwrap_or(0.0),
                b"y1" => y1 = val.parse().unwrap_or(0.0),
                b"x2" => x2 = val.parse().unwrap_or(0.0),
                b"y2" => y2 = val.parse().unwrap_or(0.0),
                _ => {}
            }
        }
        (x1, y1, x2, y2)
    }

    /// Convert a toolpath profile to VectorParameters.
    fn profile_to_params(profile: &ToolpathProfile) -> VectorParameters {
        VectorParameters {
            power: profile.laser_power,
            speed: profile.laser_speed,
            ..Default::default()
        }
    }
}

impl Default for ThreemfLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLoader for ThreemfLoader {
    fn load(&self, path: &Path) -> FileResult<Toolpath> {
        let mut resources = self.load_resources(path)?;
        if resources.is_empty() {
            return Err(FileError::InvalidFormat(
                "No toolpath resources found in 3MF file".to_string(),
            ));
        }
        Ok(resources.remove(0).1)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["3mf"]
    }

    fn load_all(&self, path: &Path) -> FileResult<Vec<(String, Toolpath)>> {
        self.load_resources(path)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Extract the local name from a potentially namespace-prefixed XML name.
/// Returns a slice of the input: `tp:toolpathresource` → `toolpathresource`.
fn local_name_ref(name: &[u8]) -> &[u8] {
    match name.iter().position(|&b| b == b':') {
        Some(pos) => &name[pos + 1..],
        None => name,
    }
}

/// Check if an owned name's local part equals `target`.
fn local_name_eq(name: &[u8], target: &[u8]) -> bool {
    local_name_ref(name) == target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_name() {
        assert_eq!(local_name_ref(b"tp:toolpathresource"), b"toolpathresource");
        assert_eq!(local_name_ref(b"t:toolpathresource"), b"toolpathresource");
        assert_eq!(local_name_ref(b"segment"), b"segment");
        assert_eq!(local_name_ref(b""), b"");
    }

    #[test]
    fn test_parse_model_single_resource() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"
       xmlns:tp="http://schemas.microsoft.com/3dmanufacturing/toolpath/2019/05">
  <resources>
    <tp:toolpathresource id="1" uuid="abc-123" unitfactor="0.001">
      <tp:toolpathprofiles>
        <tp:toolpathprofile uuid="prof-1" name="contour" laserpower="125" laserspeed="500"/>
        <tp:toolpathprofile uuid="prof-2" name="hatch" laserpower="400" laserspeed="600"/>
      </tp:toolpathprofiles>
      <tp:toolpathlayers zbottom="0">
        <tp:toolpathlayer ztop="50" path="/Toolpath/layer1.xml"/>
        <tp:toolpathlayer ztop="100" path="/Toolpath/layer2.xml"/>
      </tp:toolpathlayers>
    </tp:toolpathresource>
  </resources>
</model>"#;

        let resources = ThreemfLoader::parse_model(xml).unwrap();
        assert_eq!(resources.len(), 1);

        let r = &resources[0];
        assert_eq!(r.id, "1");
        assert_eq!(r.uuid, "abc-123");
        assert!((r.unitfactor - 0.001).abs() < 1e-6);
        assert_eq!(r.profiles.len(), 2);
        assert_eq!(r.profiles[0].name, "contour");
        assert_eq!(r.profiles[0].laser_power, Some(125.0));
        assert_eq!(r.profiles[1].name, "hatch");
        assert_eq!(r.profiles[1].laser_speed, Some(600.0));
        assert_eq!(r.layer_refs.len(), 2);
        assert_eq!(r.layer_refs[0].ztop, 50.0);
        assert_eq!(r.layer_refs[1].path, "/Toolpath/layer2.xml");
    }

    #[test]
    fn test_parse_model_t_prefix() {
        // helix.3mf uses `t:` prefix instead of `tp:`
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"
       xmlns:t="http://schemas.microsoft.com/3dmanufacturing/toolpath/2019/05">
  <resources>
    <t:toolpathresource id="1" uuid="efa-456" unitfactor="0.001">
      <t:toolpathprofiles>
        <t:toolpathprofile uuid="prof-a" name="buildstyle_3" laserpower="100" laserspeed="375"/>
      </t:toolpathprofiles>
      <t:toolpathlayers zbottom="0">
        <t:toolpathlayer ztop="80" path="Toolpath/layer80.xml"/>
      </t:toolpathlayers>
    </t:toolpathresource>
  </resources>
</model>"#;

        let resources = ThreemfLoader::parse_model(xml).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].profiles[0].name, "buildstyle_3");
        assert_eq!(resources[0].layer_refs[0].ztop, 80.0);
    }

    #[test]
    fn test_parse_layer_loop_and_hatch() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<layer xmlns="http://schemas.microsoft.com/3dmanufacturing/toolpath/2019/05">
  <parts>
    <part id="4" uuid="part-uuid"/>
  </parts>
  <profiles>
    <profile id="1" uuid="contour-profile"/>
    <profile id="2" uuid="hatch-profile"/>
  </profiles>
  <segments>
    <segment type="loop" profileid="1" partid="4">
      <point x="0" y="0"/>
      <point x="20000" y="0"/>
      <point x="20000" y="30000"/>
      <point x="0" y="30000"/>
    </segment>
    <segment type="hatch" profileid="2" partid="4">
      <hatch x1="100" y1="15" x2="19900" y2="15"/>
      <hatch x1="100" y1="30" x2="19900" y2="30"/>
    </segment>
  </segments>
</layer>"#;

        let mut profiles = HashMap::new();
        profiles.insert(
            "contour-profile".to_string(),
            ToolpathProfile {
                uuid: "contour-profile".to_string(),
                name: "contour".to_string(),
                laser_power: Some(125.0),
                laser_speed: Some(500.0),
                ..Default::default()
            },
        );
        profiles.insert(
            "hatch-profile".to_string(),
            ToolpathProfile {
                uuid: "hatch-profile".to_string(),
                name: "hatch".to_string(),
                laser_power: Some(400.0),
                laser_speed: Some(600.0),
                ..Default::default()
            },
        );

        let layer = ThreemfLoader::parse_layer(xml, 0, 0.05, 0.001, &profiles).unwrap();

        // 1 loop contour + 2 hatch vectors = 3 total
        assert_eq!(layer.vectors.len(), 3);

        // First vector: loop (closed contour)
        let v0 = &layer.vectors[0];
        assert_eq!(v0.vector_type, VectorType::Contour);
        assert!(v0.is_closed);
        // Points scaled by unitfactor: 20000 * 0.001 = 20.0
        assert!((v0.points[1].x - 20.0).abs() < 1e-3);
        assert!((v0.points[2].y - 30.0).abs() < 1e-3);
        assert_eq!(v0.parameters.power, Some(125.0));
        assert_eq!(v0.parameters.speed, Some(500.0));

        // Second and third vectors: hatches
        let v1 = &layer.vectors[1];
        assert_eq!(v1.vector_type, VectorType::Hatch);
        assert!(!v1.is_closed);
        assert_eq!(v1.points.len(), 2);
        // 100 * 0.001 = 0.1
        assert!((v1.points[0].x - 0.1).abs() < 1e-3);
        assert_eq!(v1.parameters.power, Some(400.0));

        // Z height
        assert!((layer.z_height - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_parse_layer_polyline() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<layer xmlns="http://schemas.microsoft.com/3dmanufacturing/toolpath/2019/05">
  <parts><part id="1" uuid="part-uuid"/></parts>
  <profiles><profile id="2" uuid="prof-a"/></profiles>
  <segments>
    <segment type="polyline" profileid="2" partid="1">
      <point x="-145" y="25852"/>
      <point x="117" y="25875"/>
    </segment>
  </segments>
</layer>"#;

        let profiles = HashMap::new();
        let layer = ThreemfLoader::parse_layer(xml, 0, 0.08, 0.001, &profiles).unwrap();

        assert_eq!(layer.vectors.len(), 1);
        let v = &layer.vectors[0];
        assert_eq!(v.vector_type, VectorType::Contour);
        assert!(!v.is_closed);
        assert_eq!(v.points.len(), 2);
        // -145 * 0.001 = -0.145
        assert!((v.points[0].x - (-0.145)).abs() < 1e-4);
    }

    #[test]
    fn test_load_dummy_3mf() {
        let path = Path::new("example/dummy.toolpath.3mf");
        if !path.exists() {
            return; // Skip if test file not available
        }
        let loader = ThreemfLoader::new();
        let results = loader.load_all(path).unwrap();
        assert!(!results.is_empty());

        let (_name, toolpath) = &results[0];
        assert!(toolpath.slice_stack.layers.len() > 0);
        assert_eq!(toolpath.metadata.format, "3MF");

        // Check that layers have vectors
        let total_vectors: usize = toolpath
            .slice_stack
            .layers
            .iter()
            .map(|l| l.vectors.len())
            .sum();
        assert!(total_vectors > 0, "Expected vectors, got 0");

        // Check z-heights are monotonically increasing
        let z_heights: Vec<f32> = toolpath
            .slice_stack
            .layers
            .iter()
            .map(|l| l.z_height)
            .collect();
        for w in z_heights.windows(2) {
            assert!(w[1] >= w[0], "Z heights not monotonic: {} > {}", w[0], w[1]);
        }
    }

    #[test]
    fn test_load_helix_3mf() {
        let path = Path::new("example/helix.3mf");
        if !path.exists() {
            return;
        }
        let loader = ThreemfLoader::new();
        let toolpath = loader.load(path).unwrap();
        assert!(toolpath.slice_stack.layers.len() > 0);
        assert_eq!(toolpath.metadata.format, "3MF");
    }

    #[test]
    fn test_load_may5_3mf_all_layers() {
        let path = Path::new("example/test_may5.3mf");
        if !path.exists() {
            return; // Skip if test file not available
        }
        let loader = ThreemfLoader::new();
        let results = loader.load_all(path).unwrap();
        assert!(!results.is_empty());

        let (_name, toolpath) = &results[0];
        // This file has 124 layers; ensure all are loaded
        assert!(
            toolpath.slice_stack.layers.len() > 100,
            "Expected 100+ layers, got {}",
            toolpath.slice_stack.layers.len()
        );

        // Z-heights should be distinct (read from <layer z="..."> attrs)
        let z_heights: Vec<f32> = toolpath
            .slice_stack
            .layers
            .iter()
            .map(|l| l.z_height)
            .collect();
        for w in z_heights.windows(2) {
            assert!(
                w[1] > w[0],
                "Z heights not strictly increasing: {} >= {}",
                w[0],
                w[1]
            );
        }
    }
}
