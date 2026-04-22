//! Vector Entity
//!
//! Represents a 2D/3D vector path in a toolpath layer.

use crate::domain::value_objects::Point2D;

/// Type of vector in a toolpath
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VectorType {
    /// Part boundary outline
    Boundary,
    /// Manufacturing contour (shell/perimeter)
    Contour,
    /// Base contour (first layer)
    BaseContour,
    /// Depth contour for variable depth regions
    DepthContour,
    /// Coinciding contour (overlapping regions)
    CoincidingContour,
    /// Infill/hatch lines
    Hatch,
    /// Support structure
    Support,
    /// Travel move (non-printing)
    Travel,
}

impl Default for VectorType {
    fn default() -> Self {
        VectorType::Contour
    }
}

/// A single polyline/path in a toolpath layer
#[derive(Debug, Clone)]
pub struct Vector {
    /// Unique identifier
    pub id: u64,
    /// Type of this vector
    pub vector_type: VectorType,
    /// 2D points making up the path
    pub points: Vec<Point2D>,
    /// Whether the path is closed (polygon vs polyline)
    pub is_closed: bool,
    /// Associated parameters (power, speed, etc.)
    pub parameters: VectorParameters,
    /// Ring index for contours (0 = outer, 1+ = inner)
    pub ring_index: Option<u32>,
    /// Part ID this vector belongs to
    pub part_id: Option<String>,
}

impl Vector {
    /// Create a new vector with the given points
    pub fn new(vector_type: VectorType, points: Vec<Point2D>) -> Self {
        let is_closed = if points.len() >= 3 {
            let first = &points[0];
            let last = &points[points.len() - 1];
            (first.x - last.x).abs() < 1e-6 && (first.y - last.y).abs() < 1e-6
        } else {
            false
        };

        Self {
            id: 0,
            vector_type,
            points,
            is_closed,
            parameters: VectorParameters::default(),
            ring_index: None,
            part_id: None,
        }
    }

    /// Create a closed polygon
    pub fn polygon(vector_type: VectorType, mut points: Vec<Point2D>) -> Self {
        // Ensure closed by appending first point if needed
        if points.len() >= 3 {
            let first = points[0];
            let last = points[points.len() - 1];
            if (first.x - last.x).abs() > 1e-6 || (first.y - last.y).abs() > 1e-6 {
                points.push(first);
            }
        }
        
        Self {
            id: 0,
            vector_type,
            points,
            is_closed: true,
            parameters: VectorParameters::default(),
            ring_index: None,
            part_id: None,
        }
    }

    /// Get the bounding box of this vector
    pub fn bounds(&self) -> Option<(Point2D, Point2D)> {
        if self.points.is_empty() {
            return None;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for p in &self.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        Some((
            Point2D::new(min_x, min_y),
            Point2D::new(max_x, max_y),
        ))
    }

    /// Calculate the total length of this vector
    pub fn length(&self) -> f32 {
        if self.points.len() < 2 {
            return 0.0;
        }

        self.points
            .windows(2)
            .map(|w| w[0].distance_to(&w[1]))
            .sum()
    }
}

/// Processing parameters for a vector
#[derive(Debug, Clone, Default)]
pub struct VectorParameters {
    /// Laser/tool power (0.0 - 1.0)
    pub power: Option<f32>,
    /// Processing speed in mm/s
    pub speed: Option<f32>,
    /// Wait time at end of vector in ms
    pub wait_time: Option<f32>,
    /// Overhang angle in radians
    pub overhang_angle: Option<f32>,
    /// Depth for variable depth processing
    pub depth: Option<f32>,
    /// Surface hit angle in degrees
    pub hit_angle: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_length() {
        let v = Vector::new(
            VectorType::Contour,
            vec![
                Point2D::new(0.0, 0.0),
                Point2D::new(3.0, 0.0),
                Point2D::new(3.0, 4.0),
            ],
        );
        assert!((v.length() - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_polygon_closure() {
        let v = Vector::polygon(
            VectorType::Boundary,
            vec![
                Point2D::new(0.0, 0.0),
                Point2D::new(1.0, 0.0),
                Point2D::new(1.0, 1.0),
            ],
        );
        assert!(v.is_closed);
        assert_eq!(v.points.len(), 4);
    }
}
