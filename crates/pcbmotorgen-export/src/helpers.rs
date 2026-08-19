//! Coordinate and geometry helpers.
//!
//! Unit helpers (millimetres, radians → degrees) plus the
//! three-point circle construction backing DXF ARC entities.

use std::f64::consts::PI;

/// Return a routing coordinate already expressed in millimetres.
#[inline]
pub(crate) fn routing_mm(mm: f64) -> f64 {
    mm
}

/// Radians → degrees.
#[inline]
pub(crate) fn rad_to_deg(rad: f64) -> f64 {
    rad * 180.0 / PI
}

/// Normalise an angle in degrees to [0, 360).
pub(crate) fn normalise_angle_deg(mut deg: f64) -> f64 {
    deg %= 360.0;
    if deg < 0.0 {
        deg += 360.0;
    }
    deg
}

/// Three-point circle (start, mid, end) → (cx, cy, radius).
///
/// Returns `None` when the three points are collinear (degenerate arc).
pub(crate) fn circle_from_three_points(
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
) -> Option<((f64, f64), f64)> {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    let (x3, y3) = p3;

    // Signed area of the triangle (→ 0 when collinear).
    let d = 2.0 * (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2));
    if d.abs() < 1e-15 {
        return None;
    }

    let x1sq = x1 * x1 + y1 * y1;
    let x2sq = x2 * x2 + y2 * y2;
    let x3sq = x3 * x3 + y3 * y3;

    let cx = (x1sq * (y2 - y3) + x2sq * (y3 - y1) + x3sq * (y1 - y2)) / d;
    let cy = (x1sq * (x3 - x2) + x2sq * (x1 - x3) + x3sq * (x2 - x1)) / d;
    let r = ((x1 - cx).powi(2) + (y1 - cy).powi(2)).sqrt();

    Some(((cx, cy), r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_from_three_points() {
        // A simple 90° arc: (10,0) → radius 10, centre (0,0).
        // Midpoint at exactly 45°: (10·cos 45°, 10·sin 45°) = (r/√2, r/√2).
        let r2 = 10.0 / (2.0_f64).sqrt(); // = 7.071067811865475…
        let p1 = (10.0, 0.0);
        let p2 = (r2, r2);
        let p3 = (0.0, 10.0);

        let result = circle_from_three_points(p1, p2, p3);
        assert!(result.is_some(), "should find a valid circle");

        let ((cx, cy), r) = result.unwrap();
        assert!((cx).abs() < 1e-12, "cx should be 0, got {cx}");
        assert!((cy).abs() < 1e-12, "cy should be 0, got {cy}");
        assert!((r - 10.0).abs() < 1e-12, "r should be 10, got {r}");
    }
}
