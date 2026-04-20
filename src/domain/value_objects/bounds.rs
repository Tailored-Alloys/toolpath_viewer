//! Bounds Value Object
//!
//! Axis-aligned bounding box representations.

use crate::domain::value_objects::{Point2D, Point3D};

/// 2D axis-aligned bounding box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds2D {
    pub min: Point2D,
    pub max: Point2D,
}

impl Bounds2D {
    /// Create a new bounding box
    pub fn new(min: Point2D, max: Point2D) -> Self {
        Self { min, max }
    }

    /// Create an empty (invalid) bounding box
    pub fn empty() -> Self {
        Self {
            min: Point2D::new(f32::MAX, f32::MAX),
            max: Point2D::new(f32::MIN, f32::MIN),
        }
    }

    /// Check if bounds are valid (min < max)
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y
    }

    /// Width of the bounding box
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height of the bounding box
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Center point
    pub fn center(&self) -> Point2D {
        Point2D::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }
}

impl Default for Bounds2D {
    fn default() -> Self {
        Self::empty()
    }
}

/// 3D axis-aligned bounding box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds3D {
    pub min: Point3D,
    pub max: Point3D,
}

impl Bounds3D {
    /// Create a new bounding box
    pub fn new(min: Point3D, max: Point3D) -> Self {
        Self { min, max }
    }

    /// Create an empty (invalid) bounding box
    pub fn empty() -> Self {
        Self {
            min: Point3D::new(f32::MAX, f32::MAX, f32::MAX),
            max: Point3D::new(f32::MIN, f32::MIN, f32::MIN),
        }
    }
}

impl Default for Bounds3D {
    fn default() -> Self {
        Self::empty()
    }
}
