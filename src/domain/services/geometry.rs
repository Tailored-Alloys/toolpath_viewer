//! Geometry Services
//!
//! Pure geometric calculations and algorithms.

use crate::domain::value_objects::Point2D;

/// Calculate the minimum distance from a point to a line segment
pub fn point_to_segment_distance(
    point: &Point2D,
    seg_start: &Point2D,
    seg_end: &Point2D,
) -> f32 {
    let dx = seg_end.x - seg_start.x;
    let dy = seg_end.y - seg_start.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-12 {
        // Segment is essentially a point
        return point.distance_to(seg_start);
    }

    // Parameter t of the projection onto the line
    let t = ((point.x - seg_start.x) * dx + (point.y - seg_start.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    // Closest point on segment
    let proj = Point2D::new(
        seg_start.x + t * dx,
        seg_start.y + t * dy,
    );

    point.distance_to(&proj)
}

/// Calculate signed area of a polygon (positive = CCW, negative = CW)
pub fn signed_polygon_area(points: &[Point2D]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    let n = points.len();
    
    for i in 0..n {
        let j = (i + 1) % n;
        area += points[i].x * points[j].y;
        area -= points[j].x * points[i].y;
    }

    area / 2.0
}

/// Calculate absolute area of a polygon
pub fn polygon_area(points: &[Point2D]) -> f32 {
    signed_polygon_area(points).abs()
}

/// Check if polygon vertices are in counter-clockwise order
pub fn is_counter_clockwise(points: &[Point2D]) -> bool {
    signed_polygon_area(points) > 0.0
}

/// Calculate centroid of a polygon
pub fn polygon_centroid(points: &[Point2D]) -> Option<Point2D> {
    if points.is_empty() {
        return None;
    }

    let n = points.len() as f32;
    let sum_x: f32 = points.iter().map(|p| p.x).sum();
    let sum_y: f32 = points.iter().map(|p| p.y).sum();

    Some(Point2D::new(sum_x / n, sum_y / n))
}

/// Simplify a polyline using Douglas-Peucker algorithm
pub fn simplify_polyline(points: &[Point2D], epsilon: f32) -> Vec<Point2D> {
    if points.len() < 3 {
        return points.to_vec();
    }

    // Find the point with maximum distance
    let mut max_dist = 0.0f32;
    let mut max_idx = 0;

    let start = &points[0];
    let end = &points[points.len() - 1];

    for (i, p) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let dist = point_to_segment_distance(p, start, end);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    // If max distance is greater than epsilon, recursively simplify
    if max_dist > epsilon {
        let mut result1 = simplify_polyline(&points[..=max_idx], epsilon);
        let result2 = simplify_polyline(&points[max_idx..], epsilon);

        result1.pop(); // Remove duplicate point
        result1.extend(result2);
        result1
    } else {
        vec![*start, *end]
    }
}

/// Calculate the bounding box of a set of points
pub fn calculate_bounds(points: &[Point2D]) -> Option<(Point2D, Point2D)> {
    if points.is_empty() {
        return None;
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    Some((Point2D::new(min_x, min_y), Point2D::new(max_x, max_y)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_to_segment_distance() {
        let p = Point2D::new(0.0, 1.0);
        let s1 = Point2D::new(-1.0, 0.0);
        let s2 = Point2D::new(1.0, 0.0);
        assert!((point_to_segment_distance(&p, &s1, &s2) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_polygon_area() {
        // Unit square
        let points = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(1.0, 0.0),
            Point2D::new(1.0, 1.0),
            Point2D::new(0.0, 1.0),
        ];
        assert!((polygon_area(&points) - 1.0).abs() < 1e-6);
    }
}
