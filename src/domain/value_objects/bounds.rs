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

    /// Create from corner coordinates
    pub fn from_coords(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min: Point2D::new(min_x, min_y),
            max: Point2D::new(max_x, max_y),
        }
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

    /// Expand to include a point
    pub fn include(&mut self, point: &Point2D) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
    }

    /// Expand to include another bounds
    pub fn union(&self, other: &Bounds2D) -> Bounds2D {
        Bounds2D {
            min: Point2D::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
            ),
            max: Point2D::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
            ),
        }
    }

    /// Check if point is inside bounds
    pub fn contains(&self, point: &Point2D) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Add padding around bounds
    pub fn pad(&self, padding: f32) -> Bounds2D {
        Bounds2D {
            min: Point2D::new(self.min.x - padding, self.min.y - padding),
            max: Point2D::new(self.max.x + padding, self.max.y + padding),
        }
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

    /// Get the 2D (XY) projection of this bounds
    pub fn to_2d(&self) -> Bounds2D {
        Bounds2D {
            min: self.min.to_2d(),
            max: self.max.to_2d(),
        }
    }

    /// Depth (Z extent)
    pub fn depth(&self) -> f32 {
        self.max.z - self.min.z
    }
}

impl Default for Bounds3D {
    fn default() -> Self {
        Self::empty()
    }
}
