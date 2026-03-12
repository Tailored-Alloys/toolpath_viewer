//! ILT File Loader
//!
//! ILT files are ZIP or gzip-compressed CLI files used in industrial
//! laser sintering and additive manufacturing.
//!
//! A single ILT ZIP may contain multiple CLI files:
//! - `*_vk.cli` → contour vectors
//! - `*_vs.cli` → infill/hatch vectors
//! - `*_param.txt` → parameter file
//!
//! This loader decompresses, parses each CLI file with the appropriate
//! vector-type tagging, and merges them layer-by-layer into one toolpath.

use crate::application::ports::{FileError, FileLoader, FileResult};
use crate::domain::entities::{Toolpath, VectorType};
use crate::infrastructure::file_adapters::CliParser;
use flate2::read::GzDecoder;
use log::{debug, info};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

/// Compression type detected from file magic
enum CompressionType {
    Zip,
    Gzip,
    None,
}

/// Loader for ILT (compressed CLI) files
pub struct IltLoader {
    /// Optional scale override for coordinates
    scale_override: Option<f32>,
}

impl IltLoader {
    /// Create a new ILT loader
    pub fn new() -> Self {
        Self {
            scale_override: None,
        }
    }

    /// Create with a specific coordinate scale
    pub fn with_scale(scale: f32) -> Self {
        Self {
            scale_override: Some(scale),
        }
    }

    /// Detect compression type from file magic bytes
    fn detect_compression(path: &Path) -> FileResult<CompressionType> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut magic = [0u8; 4];
        
        match reader.read_exact(&mut magic) {
            Ok(_) => {
                // ZIP: PK\x03\x04
                if magic[0..2] == [0x50, 0x4B] {
                    Ok(CompressionType::Zip)
                // Gzip: 1F 8B
                } else if magic[0..2] == [0x1f, 0x8b] {
                    Ok(CompressionType::Gzip)
                } else {
                    Ok(CompressionType::None)
                }
            }
            Err(_) => Ok(CompressionType::None),
        }
    }

    /// Load and decompress a ZIP-compressed ILT file
    fn load_zip(&self, path: &Path) -> FileResult<Toolpath> {
        info!("Loading ZIP-compressed ILT file: {:?}", path);
        
        let file = File::open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileError::NotFound(path.display().to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FileError::PermissionDenied(path.display().to_string())
            } else {
                FileError::IoError(e)
            }
        })?;

        let mut archive = ZipArchive::new(BufReader::new(file))
            .map_err(|e| FileError::DecompressionError(format!("Failed to open ZIP: {}", e)))?;
        
        if archive.is_empty() {
            return Err(FileError::DecompressionError("ZIP archive is empty".to_string()));
        }
        
        // Collect CLI file names and classify them
        let mut cli_entries: Vec<(usize, String)> = Vec::new();
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                if name.to_lowercase().ends_with(".cli") {
                    cli_entries.push((i, name));
                }
            }
        }
        
        if cli_entries.is_empty() {
            // Fall back to first file
            cli_entries.push((0, archive.by_index(0)
                .map(|f| f.name().to_string())
                .unwrap_or_default()));
        }
        
        info!("Found {} CLI file(s) in ZIP: {:?}", cli_entries.len(), 
              cli_entries.iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>());
        
        let mut merged_toolpath: Option<Toolpath> = None;
        
        for (file_index, file_name) in &cli_entries {
            // Determine source type from filename
            let lower = file_name.to_lowercase();
            let (source_label, hatch_override) = if lower.contains("_vk") {
                ("vk", Some(VectorType::Contour))
            } else if lower.contains("_vs") {
                ("vs", Some(VectorType::Hatch))
            } else {
                ("unknown", None)
            };
            
            info!("Parsing {} as '{}' (type override: {:?})", file_name, source_label, hatch_override);
            
            let mut zip_file = archive.by_index(*file_index)
                .map_err(|e| FileError::DecompressionError(format!("Failed to read ZIP entry: {}", e)))?;
            
            debug!("Extracting: {} ({} bytes)", zip_file.name(), zip_file.size());
            
            let mut decompressed = Vec::new();
            zip_file.read_to_end(&mut decompressed)
                .map_err(|e| FileError::DecompressionError(format!("Failed to extract: {}", e)))?;

            debug!("Extracted {} bytes", decompressed.len());

            // Parse with appropriate overrides
            let mut parser = CliParser::new();
            if let Some(scale) = self.scale_override {
                parser = CliParser::with_scale(scale);
            }
            if let Some(vtype) = hatch_override {
                parser.set_hatch_type_override(vtype);
            }
            parser.set_source_label(source_label);
            
            let cursor = std::io::Cursor::new(decompressed);
            let toolpath = parser.parse(cursor)?;
            
            let stats = toolpath.slice_stack.stats();
            info!(
                "  {} -> {} layers, {} vectors, {} points",
                source_label, stats.layer_count, stats.total_vectors, stats.total_points
            );
            
            // Merge into combined toolpath
            merged_toolpath = Some(match merged_toolpath {
                None => toolpath,
                Some(existing) => Self::merge_toolpaths(existing, toolpath),
            });
        }
        
        let mut toolpath = merged_toolpath.unwrap_or_default();
        
        // Set source path
        toolpath.slice_stack.source_path = Some(path.display().to_string());
        toolpath.metadata.format = "ILT".to_string();
        
        let stats = toolpath.slice_stack.stats();
        info!(
            "Final merged: {} layers, {} vectors, {} points",
            stats.layer_count, stats.total_vectors, stats.total_points
        );

        Ok(toolpath)
    }

    /// Merge two toolpaths layer-by-layer, matching by Z height
    fn merge_toolpaths(mut base: Toolpath, other: Toolpath) -> Toolpath {
        // Build a map from z_height to layer index in base
        let mut z_to_idx: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
        for (i, layer) in base.slice_stack.layers.iter().enumerate() {
            // Use fixed-point key to avoid float comparison issues (5 decimal places)
            let key = (layer.z_height * 100000.0).round() as i64;
            z_to_idx.insert(key, i);
        }
        
        for other_layer in other.slice_stack.layers {
            let key = (other_layer.z_height * 100000.0).round() as i64;
            if let Some(&base_idx) = z_to_idx.get(&key) {
                // Merge vectors and params into existing layer
                let base_layer = &mut base.slice_stack.layers[base_idx];
                base_layer.vectors.extend(other_layer.vectors);
                base_layer.params.extend(other_layer.params);
            } else {
                // Layer doesn't exist in base—add it
                let idx = base.slice_stack.layers.len();
                let mut new_layer = other_layer;
                new_layer.index = idx;
                z_to_idx.insert(key, idx);
                base.slice_stack.layers.push(new_layer);
            }
        }
        
        // Re-sort by Z height
        base.slice_stack.sort_by_z();
        base
    }

    /// Load and decompress a gzip-compressed ILT file
    fn load_gzip(&self, path: &Path) -> FileResult<Toolpath> {
        info!("Loading gzip-compressed ILT file: {:?}", path);
        
        let file = File::open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileError::NotFound(path.display().to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FileError::PermissionDenied(path.display().to_string())
            } else {
                FileError::IoError(e)
            }
        })?;

        let decoder = GzDecoder::new(BufReader::new(file));
        
        let mut decompressed = Vec::new();
        let mut decoder_reader = BufReader::new(decoder);
        
        decoder_reader.read_to_end(&mut decompressed).map_err(|e| {
            FileError::DecompressionError(format!("Failed to decompress gzip: {}", e))
        })?;

        debug!("Decompressed {} bytes", decompressed.len());

        let parser = self.make_parser();
        let cursor = std::io::Cursor::new(decompressed);
        let mut toolpath = parser.parse(cursor)?;
        
        toolpath.slice_stack.source_path = Some(path.display().to_string());
        toolpath.metadata.format = "ILT".to_string();
        
        let stats = toolpath.slice_stack.stats();
        info!(
            "Loaded {} layers, {} vectors, {} points",
            stats.layer_count, stats.total_vectors, stats.total_points
        );

        Ok(toolpath)
    }

    /// Load an uncompressed CLI file (fallback)
    fn load_uncompressed(&self, path: &Path) -> FileResult<Toolpath> {
        info!("Loading uncompressed CLI file: {:?}", path);
        
        let file = File::open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileError::NotFound(path.display().to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FileError::PermissionDenied(path.display().to_string())
            } else {
                FileError::IoError(e)
            }
        })?;

        let parser = self.make_parser();
        let reader = BufReader::new(file);
        let mut toolpath = parser.parse(reader)?;
        
        toolpath.slice_stack.source_path = Some(path.display().to_string());
        
        Ok(toolpath)
    }

    /// Create a CLI parser with the loader's scale setting
    fn make_parser(&self) -> CliParser {
        match self.scale_override {
            Some(s) => CliParser::with_scale(s),
            None => CliParser::new(),
        }
    }
}

impl Default for IltLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLoader for IltLoader {
    fn load(&self, path: &Path) -> FileResult<Toolpath> {
        // Detect compression type from file magic
        let compression = Self::detect_compression(path)?;
        
        match compression {
            CompressionType::Zip => self.load_zip(path),
            CompressionType::Gzip => self.load_gzip(path),
            CompressionType::None => {
                debug!("File is not compressed, trying plain CLI");
                self.load_uncompressed(path)
            }
        }
    }

    fn supported_extensions(&self) -> &[&str] {
        &["ilt", "cli", "cli.gz"]
    }
}

/// Loader specifically for uncompressed CLI files
pub struct CliLoader;

impl CliLoader {
    /// Create a new CLI loader
    pub fn new() -> Self {
        Self
    }
}

impl Default for CliLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLoader for CliLoader {
    fn load(&self, path: &Path) -> FileResult<Toolpath> {
        let file = File::open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileError::NotFound(path.display().to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FileError::PermissionDenied(path.display().to_string())
            } else {
                FileError::IoError(e)
            }
        })?;

        let parser = CliParser::new();
        let reader = BufReader::new(file);
        let mut toolpath = parser.parse(reader)?;
        toolpath.slice_stack.source_path = Some(path.display().to_string());
        
        Ok(toolpath)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["cli"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let loader = IltLoader::new();
        assert!(loader.can_load(Path::new("test.ilt")));
        assert!(loader.can_load(Path::new("test.ILT")));
        assert!(loader.can_load(Path::new("test.cli")));
        assert!(!loader.can_load(Path::new("test.txt")));
    }
}
