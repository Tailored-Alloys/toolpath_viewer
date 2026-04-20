//! Layer Entity
//!
//! Represents a single layer in a toolpath containing multiple vectors.

use crate::domain::entities::{Vector, VectorType};
use crate::domain::value_objects::Point2D;

/// Per-layer laser/process parameters
#[derive(Debug, Clone, Default)]
pub struct LayerParameters {
    /// Laser power value (raw from file)
    pub power: Option<f32>,
    /// Laser speed in mm/s (raw from file)
    pub speed: Option<f32>,
    /// Wait times: maps hatch vector index -> wait time in microseconds
    pub wait_times: Vec<(u32, u32)>,
}

/// A layer in a toolpath at a specific Z height
#[derive(Debug, Clone)]
pub struct Layer {
    /// Layer index (0-based)
    pub index: usize,
    /// Z height of this layer in mm
    pub z_height: f32,
    /// Layer thickness in mm
    pub thickness: f32,
    /// All vectors in this layer
    pub vectors: Vec<Vector>,
    /// Part ID this layer belongs to
    pub part_id: Option<String>,
    /// Per-source parameters (e.g., "vk" contour params, "vs" infill params)
    pub params: Vec<(String, LayerParameters)>,
}

impl Layer {
    /// Create a new empty layer
    pub fn new(index: usize, z_height: f32) -> Self {
        Self {
            index,
            z_height,
            thickness: 0.0,
            vectors: Vec::new(),
            part_id: None,
            params: Vec::new(),
        }
    }

    /// Add a vector to this layer
    pub fn add_vector(&mut self, vector: Vector) {
        self.vectors.push(vector);
    }

    /// Get the bounding box of all vectors in this layer
    pub fn bounds(&self) -> Option<(Point2D, Point2D)> {
        if self.vectors.is_empty() {
            return None;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for vector in &self.vectors {
            if let Some((min, max)) = vector.bounds() {
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

    /// Get the total number of points in all vectors
    pub fn point_count(&self) -> usize {
        self.vectors.iter().map(|v| v.points.len()).sum()
    }

    /// Get total number of vectors
    pub fn vector_count(&self) -> usize {
        self.vectors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_bounds() {
        let mut layer = Layer::new(0, 0.1);
        layer.add_vector(Vector::new(
            VectorType::Contour,
            vec![
                Point2D::new(0.0, 0.0),
                Point2D::new(10.0, 5.0),
            ],
        ));
        layer.add_vector(Vector::new(
            VectorType::Hatch,
            vec![
                Point2D::new(-5.0, 2.0),
                Point2D::new(5.0, 8.0),
            ],
        ));

        let (min, max) = layer.bounds().unwrap();
        assert_eq!(min.x, -5.0);
        assert_eq!(min.y, 0.0);
        assert_eq!(max.x, 10.0);
        assert_eq!(max.y, 8.0);
    }
}
