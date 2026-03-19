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

/// Result of a nearest-vector hit-test query.
pub struct NearestVectorHit {
    /// Index of the vector in the layer's `vectors` list.
    pub vector_index: usize,
    /// Minimum distance from the query point to the vector's segments (world units).
    pub distance: f32,
}

/// Find the nearest vector in a layer to a given world-space point.
///
/// Only segments belonging to vectors whose indices are in `visible_indices`
/// are considered.  Returns `None` when the layer has no qualifying vectors.
pub fn find_nearest_vector(
    point: &Point2D,
    layer: &crate::domain::entities::Layer,
    visible_indices: &[usize],
) -> Option<NearestVectorHit> {
    let mut best: Option<NearestVectorHit> = None;

    for &idx in visible_indices {
        let vector = match layer.vectors.get(idx) {
            Some(v) => v,
            None => continue,
        };
        if vector.points.len() < 2 {
            continue;
        }
        // Quick bounding-box rejection
        if let Some((bb_min, bb_max)) = vector.bounds() {
            let margin = best.as_ref().map_or(f32::MAX, |b| b.distance);
            if point.x < bb_min.x - margin
                || point.x > bb_max.x + margin
                || point.y < bb_min.y - margin
                || point.y > bb_max.y + margin
            {
                continue;
            }
        }
        for seg in vector.points.windows(2) {
            let d = point_to_segment_distance(point, &seg[0], &seg[1]);
            let dominated = best.as_ref().map_or(false, |b| d >= b.distance);
            if !dominated {
                best = Some(NearestVectorHit {
                    vector_index: idx,
                    distance: d,
                });
            }
        }
    }
    best
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
