//! SliceStack Entity
//!
//! Represents a complete stack of layers (the entire toolpath file).

use crate::domain::entities::Layer;
use crate::domain::value_objects::Point2D;

/// Statistics about a slice stack
#[derive(Debug, Clone, Default)]
pub struct SliceStackStats {
    /// Total number of layers
    pub layer_count: usize,
    /// Total number of vectors across all layers
    pub total_vectors: usize,
    /// Total number of points across all vectors
    pub total_points: usize,
    /// Minimum Z height
    pub z_min: f32,
    /// Maximum Z height
    pub z_max: f32,
    /// Build height (z_max - z_min)
    pub build_height: f32,
}

/// A stack of layers representing a complete toolpath
#[derive(Debug, Clone)]
pub struct SliceStack {
    /// Name/identifier for this slice stack
    pub name: String,
    /// Source file path
    pub source_path: Option<String>,
    /// All layers in Z-order
    pub layers: Vec<Layer>,
    /// Build platform bounds
    pub platform_bounds: Option<(Point2D, Point2D)>,
    /// Unit of measurement (typically "mm")
    pub units: String,
}

impl SliceStack {
    /// Create a new empty slice stack
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source_path: None,
            layers: Vec::new(),
            platform_bounds: None,
            units: "mm".to_string(),
        }
    }

    /// Create a slice stack with layers
    pub fn with_layers(name: impl Into<String>, layers: Vec<Layer>) -> Self {
        Self {
            name: name.into(),
            source_path: None,
            layers,
            platform_bounds: None,
            units: "mm".to_string(),
        }
    }

    /// Get a layer by index
    pub fn get_layer(&self, index: usize) -> Option<&Layer> {
        self.layers.get(index)
    }

    /// Get the number of layers
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Get all unique Z heights
    pub fn z_heights(&self) -> Vec<f32> {
        self.layers.iter().map(|l| l.z_height).collect()
    }

    /// Get layer by Z height (finds closest)
    pub fn get_layer_by_z(&self, z: f32) -> Option<&Layer> {
        self.layers
            .iter()
            .min_by(|a, b| {
                let da = (a.z_height - z).abs();
                let db = (b.z_height - z).abs();
                da.partial_cmp(&db).unwrap()
            })
    }

    /// Get the bounding box of all layers
    pub fn bounds(&self) -> Option<(Point2D, Point2D)> {
        if self.layers.is_empty() {
            return None;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for layer in &self.layers {
            if let Some((min, max)) = layer.bounds() {
                min_x = min_x.min(min.x);
                min_y = min_y.min(min.y);
                max_x = max_x.max(max.x);
                max_y = max_y.max(max.y);
            }
        }

        if min_x == f32::MAX {
            return None;
        }

        Some((
            Point2D::new(min_x, min_y),
            Point2D::new(max_x, max_y),
        ))
    }

    /// Calculate statistics for this slice stack
    pub fn stats(&self) -> SliceStackStats {
        let layer_count = self.layers.len();
        let total_vectors: usize = self.layers.iter().map(|l| l.vector_count()).sum();
        let total_points: usize = self.layers.iter().map(|l| l.point_count()).sum();

        let (z_min, z_max) = if self.layers.is_empty() {
            (0.0, 0.0)
        } else {
            let z_min = self.layers.iter().map(|l| l.z_height).fold(f32::MAX, f32::min);
            let z_max = self.layers.iter().map(|l| l.z_height).fold(f32::MIN, f32::max);
            (z_min, z_max)
        };

        SliceStackStats {
            layer_count,
            total_vectors,
            total_points,
            z_min,
            z_max,
            build_height: z_max - z_min,
        }
    }

    /// Sort layers by Z height
    pub fn sort_by_z(&mut self) {
        self.layers.sort_by(|a, b| {
            a.z_height.partial_cmp(&b.z_height).unwrap()
        });
        // Update indices after sorting
        for (i, layer) in self.layers.iter_mut().enumerate() {
            layer.index = i;
        }
    }
}

impl Default for SliceStack {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_stack_stats() {
        let layers = vec![
            Layer::new(0, 0.1),
            Layer::new(1, 0.2),
            Layer::new(2, 0.3),
        ];
        let stack = SliceStack::with_layers("Test", layers);

        let stats = stack.stats();
        assert_eq!(stats.layer_count, 3);
        assert!((stats.z_min - 0.1).abs() < 1e-6);
        assert!((stats.z_max - 0.3).abs() < 1e-6);
    }
}
