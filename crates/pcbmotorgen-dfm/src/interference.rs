//! Design-rule (DRC) interference checks on generated routing geometry.
//!
//! The routing pattern emits only raw geometry (line endpoints + layer + net,
//! via positions + from/to layers). Trace width and via pad size are owned by
//! the DFM crate via [`DesignRules`](crate::rules::DesignRules) (see the crate
//! division, kata 0rgs). This module verifies copper-to-copper clearance and
//! via-pad-to-trace clearance against those rules. These are *diagnostics* —
//! they are reported, not used to silently alter geometry.

use std::collections::BTreeMap;

use crate::rules::DesignRules;
use pcbmotorgen_routing::model::{Point, RouteSegment, RoutingResult};

/// One DRC violation.
#[derive(Debug, Clone)]
pub struct InterferenceViolation {
    pub layer: u32,
    pub net_a: String,
    pub net_b: String,
    pub kind: &'static str,
    pub gap_mm: f64,
    pub message: String,
}

/// Run design-rule (interference) checks on a validated [`RoutingResult`]
/// using the supplied DFM rules.
///
/// Checks:
/// - Same-layer, different-net segment segments closer than
///   `min_trace + min_space` (edge-to-edge clearance).
/// - Via pad (drill + 2×annular) to different-net traces on the via's from/to
///   layers closer than `via_pad_radius + trace_width/2 + min_space`.
///
/// Bounded at 200k pair checks to avoid runaway on pathological inputs.
pub fn check_interference(
    rules: &DesignRules,
    result: &RoutingResult,
) -> Vec<InterferenceViolation> {
    let mut violations = Vec::new();

    let trace_width = rules.min_trace_mm;
    let min_space = rules.min_space_mm;
    let via_pad_radius = rules.via_pad_radius_mm();

    // Copper pitch: center-to-center distance at which two same-layer,
    // different-net traces just touch (edge-to-edge clearance = min_space).
    let copper_pitch = trace_width + min_space;

    // --- Same-layer, different-net segment clearance ---
    // Group segments by layer, then compare different-net pairs.
    let mut by_layer: BTreeMap<u32, Vec<(usize, &RouteSegment)>> = BTreeMap::new();
    for (i, s) in result.segments.iter().enumerate() {
        by_layer.entry(s.layer).or_default().push((i, s));
    }
    let mut pair_checks = 0usize;
    for (layer, segs) in &by_layer {
        for a in 0..segs.len() {
            for b in a + 1..segs.len() {
                if pair_checks > 200_000 {
                    break;
                }
                pair_checks += 1;
                let (ia, sa) = segs[a];
                let (ib, sb) = segs[b];
                if sa.net == sb.net {
                    continue;
                }
                let d = seg_seg_distance(sa, sb);
                if d < copper_pitch * 0.999 {
                    violations.push(InterferenceViolation {
                        layer: *layer,
                        net_a: sa.net.clone(),
                        net_b: sb.net.clone(),
                        kind: "clearance",
                        gap_mm: d,
                        message: format!(
                            "segments {} (net {}) and {} (net {}) on layer {} are {:.3} mm apart — below copper clearance {:.3} mm",
                            ia, sa.net, ib, sb.net, layer, d, min_space
                        ),
                    });
                }
            }
        }
    }

    // --- Via-pad to different-net trace clearance on from/to layers ---
    let via_min_distance = via_pad_radius + trace_width / 2.0 + min_space;
    for (vi, v) in result.vias.iter().enumerate() {
        for (layer, segs) in &by_layer {
            if *layer != v.from_layer && *layer != v.to_layer {
                continue;
            }
            for (_si, s) in segs {
                if s.net == v.net {
                    continue;
                }
                let d = point_seg_distance(v.position, s);
                if d < via_min_distance * 0.999 {
                    violations.push(InterferenceViolation {
                        layer: *layer,
                        net_a: v.net.clone(),
                        net_b: s.net.clone(),
                        kind: "via_clearance",
                        gap_mm: d,
                        message: format!(
                            "via {} (net {}) pad is only {:.3} mm from net {} on layer {} — below required pad clearance {:.3} mm",
                            vi, v.net, s.net, layer, d, via_min_distance
                        ),
                    });
                }
            }
        }
    }

    violations
}

fn point_seg_distance(p: Point, s: &RouteSegment) -> f64 {
    let (px, py) = (p.x, p.y);
    let (ax, ay) = (s.start.x, s.start.y);
    let (bx, by) = (s.end.x, s.end.y);
    let abx = bx - ax;
    let aby = by - ay;
    let denom = abx * abx + aby * aby;
    if denom <= 1e-12 {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = (((px - ax) * abx + (py - ay) * aby) / denom).clamp(0.0, 1.0);
    let cx = ax + t * abx;
    let cy = ay + t * aby;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// Minimum distance between two line segments (2D).
fn seg_seg_distance(a: &RouteSegment, b: &RouteSegment) -> f64 {
    let mut d = f64::INFINITY;
    // Endpoints of a to b, and of b to a.
    for &(p, q) in [
        (a.start, b),
        (a.end, b),
        (b.start, a),
        (b.end, a),
    ]
    .iter()
    {
        d = d.min(point_seg_distance(p, q));
    }
    // Intersection (distance 0) check.
    if segments_intersect(a, b) {
        return 0.0;
    }
    d
}

fn segments_intersect(a: &RouteSegment, b: &RouteSegment) -> bool {
    fn ccw(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> f64 {
        (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
    }
    let (ax1, ay1, ax2, ay2) = (a.start.x, a.start.y, a.end.x, a.end.y);
    let (bx1, by1, bx2, by2) = (b.start.x, b.start.y, b.end.x, b.end.y);
    let d1 = ccw(bx1, by1, bx2, by2, ax1, ay1);
    let d2 = ccw(bx1, by1, bx2, by2, ax2, ay2);
    let d3 = ccw(ax1, ay1, ax2, ay2, bx1, by1);
    let d4 = ccw(ax1, ay1, ax2, ay2, bx2, by2);
    (d1 * d2) < 0.0 && (d3 * d4) < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use pcbmotorgen_routing::model::Via;

    fn rules() -> DesignRules {
        DesignRules {
            min_trace_mm: 0.1,
            min_space_mm: 0.1,
            min_via_drill_mm: 0.2,
            min_via_annular_ring_mm: 0.1,
        }
    }

    fn seg(x1: f64, y1: f64, x2: f64, y2: f64, layer: u32, net: &str) -> RouteSegment {
        RouteSegment {
            start: Point::new(x1, y1),
            end: Point::new(x2, y2),
            layer,
            net: net.to_string(),
            is_active: true,
        }
    }

    fn result(segments: Vec<RouteSegment>, vias: Vec<Via>) -> RoutingResult {
        RoutingResult {
            format_version: pcbmotorgen_routing::model::FORMAT_VERSION,
            segments,
            curves: Vec::new(),
            vias,
            pole_regions: Vec::new(),
            leg_grid: None,
            phase_bands: Vec::new(),
            io_pads: Vec::new(),
            io_traces: Vec::new(),
        }
    }

    #[test]
    fn clear_different_net_segments_pass() {
        // 1 mm apart vertically; copper pitch = 0.2 mm → no violation.
        let r = result(
            vec![
                seg(0.0, 0.0, 10.0, 0.0, 0, "A"),
                seg(0.0, 1.0, 10.0, 1.0, 0, "B"),
            ],
            Vec::new(),
        );
        assert!(check_interference(&rules(), &r).is_empty());
    }

    #[test]
    fn too_close_different_net_segments_violate() {
        // 0.05 mm apart; below copper pitch (0.2) → clearance violation.
        let r = result(
            vec![
                seg(0.0, 0.0, 10.0, 0.0, 0, "A"),
                seg(0.0, 0.05, 10.0, 0.05, 0, "B"),
            ],
            Vec::new(),
        );
        let v = check_interference(&rules(), &r);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, "clearance");
        assert_eq!(v[0].net_a, "A");
        assert_eq!(v[0].net_b, "B");
        assert_eq!(v[0].layer, 0);
    }

    #[test]
    fn same_net_segments_never_violate() {
        let r = result(
            vec![
                seg(0.0, 0.0, 10.0, 0.0, 0, "A"),
                seg(0.0, 0.05, 10.0, 0.05, 0, "A"),
            ],
            Vec::new(),
        );
        assert!(check_interference(&rules(), &r).is_empty());
    }

    #[test]
    fn different_layers_are_independent() {
        let r = result(
            vec![
                seg(0.0, 0.0, 10.0, 0.0, 0, "A"),
                seg(0.0, 0.05, 10.0, 0.05, 1, "B"),
            ],
            Vec::new(),
        );
        assert!(check_interference(&rules(), &r).is_empty());
    }

    #[test]
    fn crossing_segments_report_zero_gap() {
        let r = result(
            vec![
                seg(0.0, 0.0, 10.0, 10.0, 0, "A"),
                seg(0.0, 10.0, 10.0, 0.0, 0, "B"),
            ],
            Vec::new(),
        );
        let v = check_interference(&rules(), &r);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, "clearance");
        assert_eq!(v[0].gap_mm, 0.0);
    }

    #[test]
    fn via_pad_near_different_net_trace_violates() {
        // Via pad radius 0.2, trace half width 0.05, space 0.1 → required
        // pad-to-trace-centre distance 0.35 mm. The via sits 0.1 mm from the
        // B trace centre → violation.
        let r = result(
            vec![seg(0.0, 1.0, 10.0, 1.0, 0, "B")],
            vec![Via {
                position: Point::new(5.0, 0.9),
                from_layer: 0,
                to_layer: 1,
                net: "A".to_string(),
            }],
        );
        let v = check_interference(&rules(), &r);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, "via_clearance");
        assert_eq!(v[0].net_a, "A");
        assert_eq!(v[0].net_b, "B");
    }

    #[test]
    fn via_pad_on_same_net_never_violates() {
        let r = result(
            vec![seg(0.0, 1.0, 10.0, 1.0, 0, "A")],
            vec![Via {
                position: Point::new(5.0, 1.0),
                from_layer: 0,
                to_layer: 1,
                net: "A".to_string(),
            }],
        );
        assert!(check_interference(&rules(), &r).is_empty());
    }

    #[test]
    fn via_checked_only_on_from_to_layers() {
        // Segment on layer 2 only; via spans layers 0↔1 → not compared.
        let r = result(
            vec![seg(0.0, 1.0, 10.0, 1.0, 2, "B")],
            vec![Via {
                position: Point::new(5.0, 0.9),
                from_layer: 0,
                to_layer: 1,
                net: "A".to_string(),
            }],
        );
        assert!(check_interference(&rules(), &r).is_empty());
    }
}
