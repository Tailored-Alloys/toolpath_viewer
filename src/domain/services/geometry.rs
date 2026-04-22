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
}
