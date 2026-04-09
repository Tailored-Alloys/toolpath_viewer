//! File Collection DTO
//!
//! Manages a collection of loaded files with helpers for multi-file operations.

use std::path::PathBuf;
use crate::domain::entities::{FileEntry, Layer, Toolpath};
use crate::domain::value_objects::Point2D;
use crate::presentation::ParamRanges;

/// A collection of loaded files
#[derive(Debug, Clone, Default)]
pub struct FileCollection {
    /// All loaded file entries
    pub files: Vec<FileEntry>,
    /// Next ID to assign
    next_id: usize,
}

impl FileCollection {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            next_id: 0,
        }
    }

    /// Add a single file and return its assigned ID
    pub fn add_file(&mut self, path: PathBuf, toolpath: Toolpath) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let name = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let color = FileEntry::palette_color(id);
        self.files.push(FileEntry {
            id,
            name,
            path,
            toolpath,
            visible: true,
            color,
        });
        id
    }

    /// Remove a file by ID
    pub fn remove_file(&mut self, id: usize) {
        self.files.retain(|f| f.id != id);
    }

    /// Toggle visibility of a file by ID
    pub fn toggle_visibility(&mut self, id: usize) {
        if let Some(f) = self.files.iter_mut().find(|f| f.id == id) {
            f.visible = !f.visible;
        }
    }

    /// Set a file to invisible (used when closing a tab)
    pub fn toggle_visibility_off(&mut self, id: usize) {
        if let Some(f) = self.files.iter_mut().find(|f| f.id == id) {
            f.visible = false;
        }
    }

    /// Get visible files
    pub fn visible_files(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.iter().filter(|f| f.visible)
    }

    /// Check if any files are loaded
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Number of loaded files
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if any visible files have layers
    pub fn has_visible_layers(&self) -> bool {
        self.visible_files().any(|f| !f.toolpath.is_empty())
    }

    /// Compute a sorted, deduplicated list of Z-heights across all visible files.
    /// Uses fixed-point keys to merge Z-heights within 0.01µm tolerance.
    pub fn merged_z_heights(&self) -> Vec<f32> {
        let mut z_set = std::collections::BTreeSet::new();
        for f in self.visible_files() {
            for layer in &f.toolpath.slice_stack.layers {
                // Use fixed-point key: round to 0.01µm (z in mm * 100_000)
                let key = (layer.z_height * 100_000.0).round() as i64;
                z_set.insert(key);
            }
        }
        z_set.into_iter().map(|k| k as f32 / 100_000.0).collect()
    }

    /// Get the layer closest to a given Z-height for a specific file
    pub fn get_layer_at_z(&self, file_id: usize, z: f32) -> Option<&Layer> {
        let file = self.files.iter().find(|f| f.id == file_id)?;
        file.toolpath.slice_stack.get_layer_by_z(z)
    }

    /// Compute merged bounding box across all visible files
    pub fn merged_bounds(&self) -> Option<(Point2D, Point2D)> {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut found = false;

        for f in self.visible_files() {
            if let Some((lo, hi)) = f.toolpath.slice_stack.bounds() {
                min_x = min_x.min(lo.x);
                min_y = min_y.min(lo.y);
                max_x = max_x.max(hi.x);
                max_y = max_y.max(hi.y);
                found = true;
            }
        }

        if found {
            Some((Point2D::new(min_x, min_y), Point2D::new(max_x, max_y)))
        } else {
            None
        }
    }

    /// Compute merged parameter ranges across all visible files
    pub fn merged_param_ranges(&self) -> ParamRanges {
        let mut pmin = f32::MAX;
        let mut pmax = f32::MIN;
        let mut smin = f32::MAX;
        let mut smax = f32::MIN;
        let mut wmin = f32::MAX;
        let mut wmax = f32::MIN;
        let (mut hp, mut hs, mut hw) = (false, false, false);

        for f in self.visible_files() {
            for layer in &f.toolpath.slice_stack.layers {
                for v in &layer.vectors {
                    if let Some(val) = v.parameters.power {
                        pmin = pmin.min(val);
                        pmax = pmax.max(val);
                        hp = true;
                    }
                    if let Some(val) = v.parameters.speed {
                        smin = smin.min(val);
                        smax = smax.max(val);
                        hs = true;
                    }
                    if let Some(val) = v.parameters.wait_time {
                        wmin = wmin.min(val);
                        wmax = wmax.max(val);
                        hw = true;
                    }
                }
            }
        }

        ParamRanges {
            power: if hp { Some((pmin, pmax)) } else { None },
            speed: if hs { Some((smin, smax)) } else { None },
            wait_time: if hw { Some((wmin, wmax)) } else { None },
        }
    }

    /// Clear all files
    pub fn clear(&mut self) {
        self.files.clear();
    }
}
