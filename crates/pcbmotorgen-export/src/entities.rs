//! LINE, ARC and CIRCLE entity emitters.
//!
//! Segments become LINE entities, curves become ARC entities (with a LINE
//! fallback when the three defining points are collinear), and vias become
//! CIRCLE entities on the `Via` layer. All emitters append `"code\nvalue"`
//! pairs to the shared fragment buffer.

use crate::groups::{dxf_group, dxf_group_f64};
use crate::helpers::{circle_from_three_points, normalise_angle_deg, rad_to_deg, routing_mm};

/// Emit a LINE entity from two endpoints, in millimetres.
pub(crate) fn write_line(out: &mut Vec<String>, layer: &str, x1: f64, y1: f64, x2: f64, y2: f64) {
    dxf_group(out, 0, "LINE");
    dxf_group(out, 8, layer);
    dxf_group_f64(out, 10, x1);
    dxf_group_f64(out, 20, y1);
    dxf_group_f64(out, 11, x2);
    dxf_group_f64(out, 21, y2);
}

/// Emit an ARC entity from three points (start / mid / end), in millimetres.
///
/// The three points define a circle; the arc runs from the start angle to
/// the end angle about the computed centre. If the points are collinear
/// (degenerate arc), a LINE from start → end is emitted instead.
pub(crate) fn write_arc(
    out: &mut Vec<String>,
    layer: &str,
    p_start: (f64, f64),
    p_mid: (f64, f64),
    p_end: (f64, f64),
) {
    if let Some(((cx, cy), radius)) = circle_from_three_points(p_start, p_mid, p_end) {
        let cx_mm = routing_mm(cx);
        let cy_mm = routing_mm(cy);
        let r_mm = routing_mm(radius);

        let start_angle = rad_to_deg((p_start.1 - cy).atan2(p_start.0 - cx));
        let end_angle = rad_to_deg((p_end.1 - cy).atan2(p_end.0 - cx));

        dxf_group(out, 0, "ARC");
        dxf_group(out, 8, layer);
        dxf_group_f64(out, 10, cx_mm);
        dxf_group_f64(out, 20, cy_mm);
        dxf_group_f64(out, 40, r_mm);
        dxf_group_f64(out, 50, normalise_angle_deg(start_angle));
        dxf_group_f64(out, 51, normalise_angle_deg(end_angle));
    }
    // If the three points are collinear (degenerate), fall back to a LINE
    // from start → end.
    else {
        write_line(
            out,
            layer,
            routing_mm(p_start.0),
            routing_mm(p_start.1),
            routing_mm(p_end.0),
            routing_mm(p_end.1),
        );
    }
}

/// Emit a CIRCLE entity (via pad) on the `Via` layer, in millimetres.
pub(crate) fn write_circle(out: &mut Vec<String>, cx: f64, cy: f64, radius: f64) {
    dxf_group(out, 0, "CIRCLE");
    dxf_group(out, 8, "Via");
    dxf_group_f64(out, 10, cx);
    dxf_group_f64(out, 20, cy);
    dxf_group_f64(out, 40, radius);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing_result_to_dxf;
    use pcbmotorgen_routing::{DesignRules, Point, RouteCurve, RouteSegment, RoutingResult, Via};

    fn sample_rules() -> DesignRules {
        DesignRules {
            min_trace_mm: 0.1,
            min_space_mm: 0.1,
            min_via_drill_mm: 0.2,
            min_via_annular_ring_mm: 0.1,
        }
    }

    #[test]
    fn test_dxf_contains_line_for_segment() {
        let result = RoutingResult {
            segments: vec![
                RouteSegment {
                    start: Point::new(0.0, 0.0),
                    end: Point::new(0.0, 20.0),
                    layer: 0,
                    net: "A".into(),
                    is_active: true,
                },
                RouteSegment {
                    start: Point::new(0.0, 20.0),
                    end: Point::new(10.0, 20.0),
                    layer: 0,
                    net: "A".into(),
                    is_active: false,
                },
            ],
            curves: vec![],
            vias: vec![],
            ..Default::default()
        };
        let dxf = routing_result_to_dxf(&result, &sample_rules(), 48.0, true);
        let line_count = dxf.matches("0\nLINE\n").count();
        assert_eq!(line_count, 2, "expected 2 LINE entities for 2 segments");
    }

    #[test]
    fn test_dxf_contains_circle_for_via() {
        let result = RoutingResult {
            segments: vec![],
            curves: vec![],
            vias: vec![Via {
                position: Point::new(1.0, 2.0),
                from_layer: 0,
                to_layer: 1,
                net: "A".into(),
            }],
            ..Default::default()
        };
        let dxf = routing_result_to_dxf(&result, &sample_rules(), 0.048, true);
        assert!(dxf.contains("0\nCIRCLE"), "missing CIRCLE entity for via");
    }

    #[test]
    fn test_line_coordinates_in_mm() {
        // Segment (0,0) → (0, 20) mm.
        let result = RoutingResult {
            segments: vec![RouteSegment {
                start: Point::new(0.0, 0.0),
                end: Point::new(0.0, 20.0),
                layer: 0,
                net: "A".into(),
                is_active: true,
            }],
            curves: vec![],
            vias: vec![],
            ..Default::default()
        };
        let dxf = routing_result_to_dxf(&result, &sample_rules(), 0.0, false);

        // DXF group codes in the ENTITIES section:
        // 10 = x1, 20 = y1, 11 = x2, 21 = y2
        assert!(
            dxf.contains("10\n0.000000"),
            "x1 should be 0 mm; DXF:\n{}",
            dxf
        );
        assert!(dxf.contains("20\n0.000000"), "y1 should be 0 mm");
        assert!(dxf.contains("11\n0.000000"), "x2 should be 0 mm");
        assert!(dxf.contains("21\n20.000000"), "y2 should be 20 mm");
    }

    #[test]
    fn test_centre_x_offset() {
        // Active area = 100 mm. Segment (50, 0) → (50, 20) mm.
        // With centring: x_offset = 50 mm → x_mm = 50 - 50 = 0
        let result = RoutingResult {
            segments: vec![RouteSegment {
                start: Point::new(50.0, 0.0),
                end: Point::new(50.0, 20.0),
                layer: 0,
                net: "A".into(),
                is_active: true,
            }],
            curves: vec![],
            vias: vec![],
            ..Default::default()
        };
        let dxf = routing_result_to_dxf(&result, &sample_rules(), 100.0, true);
        // x1 = 50 - 50 = 0
        assert!(dxf.contains("10\n0.000000"), "centred x1 should be 0 mm");

        // Without centring: x1 = 50
        let dxf_nc = routing_result_to_dxf(&result, &sample_rules(), 100.0, false);
        assert!(
            dxf_nc.contains("10\n50.000000"),
            "non-centred x1 should be 50 mm"
        );
    }

    #[test]
    fn test_curve_becomes_arc_in_dxf() {
        // 90° arc centred at (0,0), radius 10 mm, from (10,0) to (0,10).
        let r2 = 10.0 / (2.0_f64).sqrt();
        let result = RoutingResult {
            segments: vec![],
            curves: vec![RouteCurve {
                start: Point::new(10.0, 0.0),
                mid: Point::new(r2, r2),
                end: Point::new(0.0, 10.0),
                layer: 0,
                net: "A".into(),
                is_active: false,
            }],
            vias: vec![],
            ..Default::default()
        };
        let dxf = routing_result_to_dxf(&result, &sample_rules(), 0.0, false);
        // After the near-zero clamp in dxf_group_f64, the arc centre (0,0)
        // should format as "0.000000" not "-0.000000".
        assert!(dxf.contains("0\nARC"), "RouteCurve must become a DXF ARC");
        assert!(dxf.contains("10\n0.000000"), "arc centre x = 0 mm");
        assert!(dxf.contains("20\n0.000000"), "arc centre y = 0 mm");
        assert!(dxf.contains("40\n10.000000"), "arc radius = 10 mm");
    }

    #[test]
    fn test_degenerate_curve_falls_back_to_line() {
        // Collinear "curve" — three points on a straight line.
        let result = RoutingResult {
            segments: vec![],
            curves: vec![RouteCurve {
                start: Point::new(0.0, 0.0),
                mid: Point::new(5.0, 0.0),
                end: Point::new(10.0, 0.0),
                layer: 0,
                net: "A".into(),
                is_active: false,
            }],
            vias: vec![],
            ..Default::default()
        };
        let dxf = routing_result_to_dxf(&result, &sample_rules(), 0.0, false);
        assert!(!dxf.contains("ARC"), "degenerate curve must not emit ARC");
        assert!(
            dxf.contains("LINE"),
            "degenerate curve must fall back to LINE"
        );
    }

    #[test]
    fn test_via_radius_respects_design_rules() {
        // drill = 0.2 mm, annular = 0.1 mm → pad diameter = 0.4 mm → radius = 0.2 mm
        let result = RoutingResult {
            segments: vec![],
            curves: vec![],
            vias: vec![Via {
                position: Point::new(0.0, 0.0),
                from_layer: 0,
                to_layer: 1,
                net: "A".into(),
            }],
            ..Default::default()
        };
        let dxf = routing_result_to_dxf(&result, &sample_rules(), 0.0, false);
        assert!(
            dxf.contains("40\n0.200000"),
            "via circle radius should be 0.2 mm"
        );
    }
}
