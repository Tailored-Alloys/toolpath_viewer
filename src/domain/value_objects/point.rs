//! Point Value Objects
//!
//! 2D and 3D point representations.

/// A 2D point in space
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

impl Point2D {
    /// Create a new 2D point
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Create a zero point
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Calculate distance to another point
    pub fn distance_to(&self, other: &Point2D) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Calculate squared distance (avoids sqrt)
    pub fn distance_squared(&self, other: &Point2D) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Linear interpolation to another point
    pub fn lerp(&self, other: &Point2D, t: f32) -> Point2D {
        Point2D {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    /// Get as array [x, y]
    pub fn to_array(&self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl From<(f32, f32)> for Point2D {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl From<[f32; 2]> for Point2D {
    fn from([x, y]: [f32; 2]) -> Self {
        Self { x, y }
    }
}

/// A 3D point in space
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3D {
    /// Create a new 3D point
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Create a zero point
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }

    /// Calculate distance to another point
    pub fn distance_to(&self, other: &Point3D) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Project to 2D (drop Z)
    pub fn to_2d(&self) -> Point2D {
        Point2D::new(self.x, self.y)
    }

    /// Get as array [x, y, z]
    pub fn to_array(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl From<(f32, f32, f32)> for Point3D {
    fn from((x, y, z): (f32, f32, f32)) -> Self {
        Self { x, y, z }
    }
}

impl From<[f32; 3]> for Point3D {
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self { x, y, z }
    }
}

impl From<Point2D> for Point3D {
    fn from(p: Point2D) -> Self {
        Self { x: p.x, y: p.y, z: 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point2d_distance() {
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(3.0, 4.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_point2d_lerp() {
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(10.0, 10.0);
        let mid = a.lerp(&b, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-6);
        assert!((mid.y - 5.0).abs() < 1e-6);
    }
}
