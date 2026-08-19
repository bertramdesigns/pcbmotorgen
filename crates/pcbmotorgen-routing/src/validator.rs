//! Strict-shape validation gate.
//!
//! Every [`RoutingResult`] from any source (bundled, Rust plugin, Python
//! runner) passes through [`Validator::validate`]. It rejects malformed shapes
//! with field-level errors; a result is never silently sanitised.

use crate::context::RoutingContext;
use crate::error::{RoutingError, RoutingErrorKind};
use crate::model::{Point, RoutingResult, FORMAT_VERSION};

/// The single validation gate.
pub struct Validator;

impl Validator {
    /// Validate `result` against `ctx`. Returns `Ok(())` or the first
    /// [`RoutingError`].
    pub fn validate(
        result: &RoutingResult,
        ctx: &RoutingContext,
        expect_continuous: bool,
    ) -> Result<(), RoutingError> {
        if result.format_version != FORMAT_VERSION {
            return Err(RoutingError::new(
                0,
                "format_version",
                RoutingErrorKind::Malformed,
                format!(
                    "routing result format_version {} is unsupported; this crate requires version {} (millimetre geometry)",
                    result.format_version, FORMAT_VERSION
                ),
            ));
        }
        if result.is_empty() {
            return Err(RoutingError::new(
                0,
                "result",
                RoutingErrorKind::Missing,
                "the routing pattern produced no segments, curves, or vias",
            ));
        }

        let x_max = ctx.active_area_length_mm + ctx.padding_mm * 2.0;
        let board_width = ctx.board_width_mm;
        // A tiny epsilon so coordinates sitting exactly on the boundary pass.
        let eps = 1e-6;

        // ---- Segments ----
        for (i, s) in result.segments.iter().enumerate() {
            let idx = i + 1;
            check_point(&s.start, ctx, x_max, board_width, eps, &format!("segments[{i}].start"), idx)?;
            check_point(&s.end, ctx, x_max, board_width, eps, &format!("segments[{i}].end"), idx)?;
            let len = length(s.start, s.end);
            if !len.is_finite() || len <= eps {
                return Err(RoutingError::new(
                    idx,
                    format!("segments[{i}]"),
                    RoutingErrorKind::Degenerate,
                    format!(
                        "zero-length segment {} ↦ {} — a conductor cannot have zero length",
                        fmt_pt(s.start), fmt_pt(s.end)
                    ),
                ));
            }
            check_layer(s.layer, ctx.num_layers, &format!("segments[{i}].layer"), idx)?;
            check_net(&s.net, &format!("segments[{i}].net"), idx)?;
        }

        // ---- Curves ----
        for (i, c) in result.curves.iter().enumerate() {
            let idx = i + 1;
            check_point(&c.start, ctx, x_max, board_width, eps, &format!("curves[{i}].start"), idx)?;
            check_point(&c.mid, ctx, x_max, board_width, eps, &format!("curves[{i}].mid"), idx)?;
            check_point(&c.end, ctx, x_max, board_width, eps, &format!("curves[{i}].end"), idx)?;
            // Degenerate arc: any two of start/mid/end coincide (radius ~ 0).
            if length(c.start, c.mid) <= eps || length(c.mid, c.end) <= eps {
                return Err(RoutingError::new(
                    idx,
                    format!("curves[{i}]"),
                    RoutingErrorKind::Degenerate,
                    "degenerate arc — start, mid, and end must not collapse to a point",
                ));
            }
            check_layer(c.layer, ctx.num_layers, &format!("curves[{i}].layer"), idx)?;
            check_net(&c.net, &format!("curves[{i}].net"), idx)?;
        }

        // ---- Vias ----
        for (i, v) in result.vias.iter().enumerate() {
            let idx = i + 1;
            check_point(&v.position, ctx, x_max, board_width, eps, &format!("vias[{i}].position"), idx)?;
            check_layer(v.from_layer, ctx.num_layers, &format!("vias[{i}].from_layer"), idx)?;
            check_layer(v.to_layer, ctx.num_layers, &format!("vias[{i}].to_layer"), idx)?;
            if v.from_layer == v.to_layer {
                return Err(RoutingError::new(
                    idx,
                    format!("vias[{i}]"),
                    RoutingErrorKind::Malformed,
                    format!(
                        "via routes layer {} to itself — from_layer must differ from to_layer",
                        v.from_layer
                    ),
                ));
            }
            check_net(&v.net, &format!("vias[{i}].net"), idx)?;
        }

        // ---- Pattern-defined pole regions ----
        for (i, region) in result.pole_regions.iter().enumerate() {
            let idx = i + 1;
            check_point(
                &region.start,
                ctx,
                x_max,
                board_width,
                eps,
                &format!("pole_regions[{i}].start"),
                idx,
            )?;
            check_point(
                &region.end,
                ctx,
                x_max,
                board_width,
                eps,
                &format!("pole_regions[{i}].end"),
                idx,
            )?;
            if length(region.start, region.end) <= eps {
                return Err(RoutingError::new(
                    idx,
                    format!("pole_regions[{i}]"),
                    RoutingErrorKind::Degenerate,
                    "zero-length pole region — start and end must differ",
                ));
            }
            check_net(&region.phase, &format!("pole_regions[{i}].phase"), idx)?;
        }

        // ---- Continuity (per layer+net chain) ----
        if expect_continuous {
            validate_continuity(result, ctx, eps)?;
        }

        Ok(())
    }
}

fn check_point(
    p: &Point,
    _ctx: &RoutingContext,
    x_max: f64,
    board_width: f64,
    eps: f64,
    field: &str,
    index: usize,
) -> Result<(), RoutingError> {
    if !p.x.is_finite() || !p.y.is_finite() {
        return Err(RoutingError::new(
            index,
            field,
            RoutingErrorKind::Malformed,
            format!("non-finite coordinate {},{} — NaN/Inf is not a valid position", p.x, p.y),
        ));
    }
    if p.x < -eps || p.x > x_max + eps {
        return Err(RoutingError::new(
            index,
            field,
            RoutingErrorKind::OutOfBounds,
            format!(
                "x = {:.3} mm outside the routing area [0, {:.3} mm] — extend padding_mm or fix the pattern",
                p.x,
                x_max
            ),
        ));
    }
    if p.y < -eps || p.y > board_width + eps {
        return Err(RoutingError::new(
            index,
            field,
            RoutingErrorKind::OutOfBounds,
            format!(
                "y = {:.3} mm outside the board width [0, {:.3} mm]",
                p.y,
                board_width
            ),
        ));
    }
    Ok(())
}

fn check_layer(layer: u32, num_layers: u32, field: &str, index: usize) -> Result<(), RoutingError> {
    if layer >= num_layers {
        return Err(RoutingError::new(
            index,
            field,
            RoutingErrorKind::BadLayer,
            format!(
                "layer = {} but the board has only {} copper layers (0..{})",
                layer,
                num_layers,
                num_layers.saturating_sub(1)
            ),
        ));
    }
    Ok(())
}

fn check_net(net: &str, field: &str, index: usize) -> Result<(), RoutingError> {
    if net.is_empty() {
        return Err(RoutingError::new(
            index,
            field,
            RoutingErrorKind::BadNet,
            "empty net label — every conductor/via must belong to a named phase net",
        ));
    }
    if !net.is_ascii() {
        return Err(RoutingError::new(
            index,
            field,
            RoutingErrorKind::BadNet,
            format!("net label \"{net}\" is not ASCII — use plain phase labels like \"A\", \"B\", \"C\""),
        ));
    }
    Ok(())
}

/// Check that consecutive elements sharing a (layer, net) chain connect
/// end→start. Chains are built from segments and curves in emitted order.
fn validate_continuity(
    result: &RoutingResult,
    ctx: &RoutingContext,
    eps: f64,
) -> Result<(), RoutingError> {
    use std::collections::HashMap;

    struct El {
        start: Point,
        end: Point,
        idx: usize,
    }

    // Chain keyed by (layer, net), preserving emitted order.
    let mut chains: HashMap<(u32, String), Vec<El>> = HashMap::new();

    let elems = result
        .segments
        .iter()
        .map(|s| (s.layer, s.net.clone(), s.start, s.end, 0))
        .chain(
            result
                .curves
                .iter()
                .map(|c| (c.layer, c.net.clone(), c.start, c.end, 0)),
        )
        .enumerate();

    for (k, (layer, net, start, end, _)) in elems {
        chains
            .entry((layer, net))
            .or_default()
            .push(El { start, end, idx: k });
    }

    for ((layer, net), chain) in &chains {
        if chain.len() < 2 {
            continue;
        }
        for w in 0..chain.len() - 1 {
            let a = &chain[w];
            let b = &chain[w + 1];
            let gap = length(a.end, b.start);
            // Allow the discontinuity to be at most a small multiple of the
            // min trace (corner clearance); larger gaps mean stranded copper.
            let tol = (ctx.min_trace_mm * 2.0).max(eps);
            if gap > tol {
                return Err(RoutingError::new(
                    b.idx,
                    format!("chain layer {layer} net {net}"),
                    RoutingErrorKind::Malformed,
                    format!(
                        "discontinuity in the {} net path on layer {}: gap of {:.3} mm between elements — the pattern declares continuous copper but elements do not connect end→start",
                        net,
                        layer,
                        gap
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn length(a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

fn fmt_pt(p: Point) -> String {
    format!("({:.3}, {:.3}) mm", p.x, p.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RouteSegment, Via};
    use std::collections::HashMap;

    fn ctx() -> RoutingContext {
        RoutingContext {
            active_area_length_mm: 100.0,
            board_width_mm: 20.0,
            num_layers: 2,
            phases: 3,
            min_trace_mm: 0.1,
            min_space_mm: 0.1,
            padding_mm: 0.0,
            expects_continuous: false,
            params: HashMap::new(),
            ..RoutingContext::default()
        }
    }

    fn seg(x1: f64, y1: f64, x2: f64, y2: f64, layer: u32, net: &str) -> RouteSegment {
        RouteSegment {
            start: Point::new(x1, y1),
            end: Point::new(x2, y2),
            layer,
            net: net.into(),
            is_active: true,
        }
    }

    fn result_with(segments: Vec<RouteSegment>) -> RoutingResult {
        RoutingResult {
            segments,
            curves: vec![],
            vias: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn accepts_valid_single_segment() {
        let r = result_with(vec![seg(0.0, 0.0, 0.0, 20.0, 0, "A")]);
        assert!(Validator::validate(&r, &ctx(), false).is_ok());
    }

    #[test]
    fn rejects_meter_contract_version() {
        let mut r = result_with(vec![seg(0.0, 0.0, 0.0, 20.0, 0, "A")]);
        r.format_version = 1;
        let error = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(error.field, "format_version");
        assert!(error.message.contains("millimetre"));
    }

    #[test]
    fn rejects_nan() {
        let r = result_with(vec![seg(f64::NAN, 0.0, 0.0, 20.0, 0, "A")]);
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::Malformed);
        assert!(e.field.contains("segments[0].start"));
    }

    #[test]
    fn rejects_out_of_bounds_x() {
        let r = result_with(vec![seg(200.0, 0.0, 0.0, 20.0, 0, "A")]);
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::OutOfBounds);
    }

    #[test]
    fn rejects_out_of_bounds_y() {
        let r = result_with(vec![seg(0.0, 50.0, 0.0, 20.0, 0, "A")]);
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::OutOfBounds);
    }

    #[test]
    fn rejects_bad_layer() {
        let r = result_with(vec![seg(0.0, 0.0, 0.0, 20.0, 5, "A")]);
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::BadLayer);
        assert!(e.message.contains("2 copper layers"));
    }

    #[test]
    fn rejects_degenerate_segment() {
        let r = result_with(vec![seg(0.0, 0.0, 0.0, 0.0, 0, "A")]);
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::Degenerate);
    }

    #[test]
    fn rejects_empty_net() {
        let r = result_with(vec![seg(0.0, 0.0, 0.0, 20.0, 0, "")]);
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::BadNet);
    }

    #[test]
    fn rejects_empty_result() {
        let r = RoutingResult::default();
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::Missing);
    }

    #[test]
    fn rejects_via_same_layer() {
        let r = RoutingResult {
            segments: vec![seg(0.0, 0.0, 0.0, 20.0, 0, "A")],
            curves: vec![],
            vias: vec![Via {
                position: Point::new(0.0, 10.0),
                from_layer: 0,
                to_layer: 0,
                net: "A".into(),
            }],
            ..Default::default()
        };
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::Malformed);
    }

    #[test]
    fn rejects_discontinuous_path_when_expected() {
        // Two segments on the same layer+net with a big gap between them.
        let r = result_with(vec![
            seg(0.0, 0.0, 10.0, 0.0, 0, "A"),
            seg(50.0, 20.0, 60.0, 20.0, 0, "A"),
        ]);
        let e = Validator::validate(&r, &ctx(), true).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::Malformed);
        assert!(e.message.contains("continuity") || e.message.contains("discontinuity"));
    }

    #[test]
    fn accepts_continuous_path_when_expected() {
        let r = result_with(vec![
            seg(0.0, 0.0, 10.0, 0.0, 0, "A"),
            seg(10.0, 0.0, 20.0, 0.0, 0, "A"),
        ]);
        assert!(Validator::validate(&r, &ctx(), true).is_ok());
    }

    #[test]
    fn rejects_empty_net_on_via() {
        let r = RoutingResult {
            segments: vec![seg(0.0, 0.0, 0.0, 20.0, 0, "A")],
            curves: vec![],
            vias: vec![Via {
                position: Point::new(0.0, 10.0),
                from_layer: 0,
                to_layer: 1,
                net: "".into(),
            }],
            ..Default::default()
        };
        let e = Validator::validate(&r, &ctx(), false).unwrap_err();
        assert_eq!(e.kind, RoutingErrorKind::BadNet);
    }
}
