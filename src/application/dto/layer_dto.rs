//! Layer DTO
//!
//! Data transfer objects for layer data.

use crate::domain::entities::{Layer, VectorType};

/// Summary information about a layer
#[derive(Debug, Clone)]
pub struct LayerSummary {
    pub index: usize,
    pub z_height: f32,
    pub vector_count: usize,
    pub point_count: usize,
    pub has_boundaries: bool,
    pub has_contours: bool,
    pub has_hatches: bool,
}

impl From<&Layer> for LayerSummary {
    fn from(layer: &Layer) -> Self {
        Self {
            index: layer.index,
            z_height: layer.z_height,
            vector_count: layer.vector_count(),
            point_count: layer.point_count(),
            has_boundaries: !layer.boundaries().is_empty(),
            has_contours: !layer.contours().is_empty(),
            has_hatches: !layer.hatches().is_empty(),
        }
    }
}

/// Statistics about a toolpath
#[derive(Debug, Clone, Default)]
pub struct ToolpathStats {
    pub layer_count: usize,
    pub total_vectors: usize,
    pub total_points: usize,
    pub z_min: f32,
    pub z_max: f32,
    pub build_height: f32,
    pub bounds_min: Option<(f32, f32)>,
    pub bounds_max: Option<(f32, f32)>,
}
