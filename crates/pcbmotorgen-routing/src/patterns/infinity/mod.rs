//! The "infinity" diamond braid routing pattern.
//!
//! Faithful Rust port of `docs/reference/pcbBraid` (after Verbeek & Dehez).
//! Produces a two-layer braid: `top_layer_segments` on layer 0, `bottom_layer_segments`
//! on layer 1, and vias (at every peak/valley/edge endpoint) routing 0 → 1.
//!
//! Pattern-owned layer semantics: this pattern is inherently 2-layer and uses
//! vias at crossing points. It is not constrained by the earlier single-layer
//! ranking.
//!
//! ## Magnet-aware period count
//!
//! When the [`RoutingContext`] carries the magnet layout (`magnet_pitch_mm` +
//! `magnet_array_span_mm`) the braid sizes its repeating diamond period to the pole
//! pitch. Complete periods are used after reserving the interleave span, so the
//! distance between corresponding diamond repeats is **exactly** the
//! centre-to-centre pole pitch. The phase/strand via pitch is
//! `pole_pitch / (phases × strands)` on both layer copies; the period count is
//! reduced when necessary to keep that uniform grid inside the routable length.
//! Contexts built without a magnet layout fall back to the packaged `n_periods`
//! default (kept for backward compatibility with plugin probes and unit tests).
//!
//! The geometry is split into three submodules:
//! - [`diamonds`] — the diamond / period geometry engine,
//! - [`peaks_valleys`] — peak / valley / braid-crossing computation,
//! - [`segments`] — the per-layer segment builders.
//!
//! The host-side [`RoutingReport`](crate::RoutingReport) derives the diamond
//! edge angle as `atan(board_width / pole_pitch)` and reports the resulting
//! conductor-band width budget for every active (layer, net) bundle.

mod diamonds;
mod peaks_valleys;
mod segments;

use crate::context::RoutingContext;
use crate::error::{RoutingError, RoutingErrorKind};
use crate::model::{Point, PoleRegion, RouteSegment, RoutingResult, Via};
use crate::pattern::RoutingPattern;

use self::diamonds::{compute_diamonds, compute_endpoints, compute_pole_region_xs};
use self::segments::{compute_bottom_layer_segments, compute_top_layer_segments};

const PHASE_NAMES: &[&str] = &["A", "B", "C", "D", "E", "F"];

/// The bundled infinity-braid pattern.
#[derive(Debug, Clone, Default)]
pub struct InfinityBraidPattern;

impl RoutingPattern for InfinityBraidPattern {
    fn id(&self) -> &str {
        "infinity-braid"
    }

    fn display_name(&self) -> &str {
        "Infinity Braid (pcbBraid)"
    }

    fn description(&self) -> &str {
        "Two-layer overlapping diamond braid after Verbeek & Dehez, with vias at crossing points."
    }

    fn author(&self) -> &str {
        "pcbmotorgen (port of pcbBraid)"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn parameters(&self) -> Vec<crate::pattern::PatternParameter> {
        use crate::pattern::PatternParameter;
        // `n_periods` is intentionally NOT exposed: when a magnet layout is
        // present the period count is derived from the pole pitch so the
        // braid regenerates to match the magnets.
        vec![
            PatternParameter::int("num_strands", "Strands per period", 5.0, 2.0, 99.0)
                .with_description("Number of parallel braided paths in each period."),
        ]
    }

    fn expects_continuous(&self) -> bool {
        // The braid is a deliberate weave of many interleaved strands; it is
        // not a single continuous per-(layer,net) path, so we do not enforce
        // the continuity check.
        false
    }

    fn generate(&self, ctx: &RoutingContext) -> Result<RoutingResult, RoutingError> {
        if ctx.num_layers < 2 {
            return Err(RoutingError::new(
                0,
                "num_layers",
                RoutingErrorKind::BadLayer,
                format!(
                    "the infinity braid requires at least 2 copper layers, got {}",
                    ctx.num_layers
                ),
            ));
        }

        let phases = ctx.phases.max(1) as i64;
        let num_strands = (ctx.param("num_strands", 5.0) as i64).max(2);
        // Map the unrolled braid domain onto the routing area.
        let d_tot = ctx.active_area_length_mm + ctx.padding_mm * 2.0;
        if !d_tot.is_finite() || d_tot <= 0.0 {
            return Err(RoutingError::new(
                0,
                "active_area_length_mm",
                RoutingErrorKind::Generation,
                "active area plus padding must be finite and greater than zero",
            ));
        }
        if !ctx.board_width_mm.is_finite() || ctx.board_width_mm <= 0.0 {
            return Err(RoutingError::new(
                0,
                "board_width_mm",
                RoutingErrorKind::Generation,
                "board width must be finite and greater than zero",
            ));
        }
        let magnet_pitch = match ctx.magnet_pitch_mm {
            Some(pitch) => {
                if !pitch.is_finite() || pitch <= 0.0 {
                    return Err(RoutingError::new(
                        0,
                        "magnet_pitch_mm",
                        RoutingErrorKind::Generation,
                        "pole pitch must be finite and greater than zero",
                    ));
                }
                Some(pitch)
            }
            None => None,
        };
        // Period count: one complete diamond period per pole pitch when the
        // magnet layout is known. The uniform phase/strand grid is reserved
        // before selecting the count so no remainder becomes a wide via gap.
        let (n_periods, d_phase, o) = match magnet_pitch {
            Some(pitch) => {
                // The final right-hand endpoint is one interleave step before
                // the end of the next period. Reserve that step before
                // selecting the number of complete periods; otherwise the
                // leftover remainder creates a visibly wide via gap between
                // the last strand of one period and the first strand of the
                // next period.
                let interleave_step = pitch / (num_strands * phases) as f64;
                let n = ((d_tot + interleave_step) / pitch).floor() as i64 - 1;
                if n < 1 {
                    return Err(RoutingError::new(
                        0,
                        "active_area_length_mm",
                        RoutingErrorKind::Generation,
                        format!(
                            "the routable length ({:.3} mm) cannot fit one complete pole-pitched braid period plus its phase/strand interleave ({:.3} mm required); increase active area or padding",
                            d_tot,
                            2.0 * pitch - interleave_step
                        ),
                    ));
                }
                let exact_period_span = n as f64 * pitch;
                // One pole pitch contains every phase/strand position exactly
                // once. This keeps the final point of one phase to the first
                // point of the next phase equal to the preceding within-phase
                // point spacing on both layer copies.
                let o = -interleave_step;
                (n, exact_period_span, o)
            }
            None => {
                let n = (ctx.param("n_periods", 4.0) as i64).max(1);
                let o = d_tot
                    / ((n + 1) * num_strands * phases - 1) as f64
                    * -1.0;
                let d_phase = d_tot - ((o * num_strands as f64 * -1.0) * phases as f64) - o;
                (n, d_phase, o)
            }
        };
        // Amplitude = half the board width, mapping diamond ±A onto [0, board_width].
        let a = ctx.board_width_mm / 2.0;

        let offset_step = o * num_strands as f64 * -1.0;

        let mut segments: Vec<RouteSegment> = Vec::new();
        let mut vias: Vec<Via> = Vec::new();
        let mut pole_regions: Vec<PoleRegion> = Vec::new();

        for i in 0..phases {
            let phase_idx = i as usize;
            let net = PHASE_NAMES[phase_idx % PHASE_NAMES.len()].to_string();
            let start_offset = offset_step * i as f64;

            let diamonds = compute_diamonds(start_offset, d_phase, n_periods, num_strands, o, a);
            let (points_obj, flatlist) = compute_endpoints(&diamonds);
            let region_xs = compute_pole_region_xs(&diamonds);
            for (pole_index, (start_x, end_x)) in region_xs.into_iter().enumerate() {
                pole_regions.push(PoleRegion {
                    phase: net.clone(),
                    pole_index: pole_index as u32,
                    start: Point::new(start_x, a),
                    end: Point::new(end_x, a),
                });
            }

            for (s, e) in compute_top_layer_segments(&points_obj) {
                segments.push(RouteSegment {
                    start: s,
                    end: e,
                    layer: 0,
                    net: net.clone(),
                    is_active: true,
                });
            }
            for (s, e) in compute_bottom_layer_segments(&points_obj) {
                segments.push(RouteSegment {
                    start: s,
                    end: e,
                    layer: 1,
                    net: net.clone(),
                    is_active: true,
                });
            }
            for p in flatlist {
                vias.push(Via {
                    position: p,
                    from_layer: 0,
                    to_layer: 1,
                    net: net.clone(),
                });
            }
        }

        Ok(RoutingResult {
            segments,
            curves: Vec::new(),
            vias,
            pole_regions,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::Validator;
    use std::collections::HashMap;

    fn ctx() -> RoutingContext {
        let mut params = HashMap::new();
        params.insert("num_strands".to_string(), 5.0);
        params.insert("n_periods".to_string(), 4.0);
        RoutingContext {
            active_area_length_mm: 600.0,
            board_width_mm: 20.0,
            num_layers: 2,
            phases: 3,
            min_trace_mm: 0.1,
            min_space_mm: 0.1,
            padding_mm: 100.0,
            expects_continuous: false,
            params,
            // No magnet layout by default: the tests below opt in explicitly.
            ..RoutingContext::default()
        }
    }

    /// Context with a magnet layout: 10 magnets at 12 mm pole pitch (span 120 mm).
    fn magnet_ctx() -> RoutingContext {
        let mut c = ctx();
        c.magnet_pitch_mm = Some(12.0);
        c.magnet_array_span_mm = Some(120.0);
        c
    }

    #[test]
    fn produces_two_layer_braid_with_vias() {
        let pat = InfinityBraidPattern;
        let r = pat.generate(&ctx()).unwrap();
        assert!(!r.segments.is_empty());
        assert!(!r.vias.is_empty());
        assert!(r.curves.is_empty());
        // Layer 0 and layer 1 both used.
        assert!(r.segments.iter().any(|s| s.layer == 0));
        assert!(r.segments.iter().any(|s| s.layer == 1));
        // Three phases present.
        let nets: std::collections::BTreeSet<&str> =
            r.segments.iter().map(|s| s.net.as_str()).collect();
        assert!(nets.contains("A") && nets.contains("B") && nets.contains("C"));
        // Vias route 0 → 1.
        assert!(r.vias.iter().all(|v| v.from_layer == 0 && v.to_layer == 1));
    }

    #[test]
    fn passes_strict_validation() {
        let pat = InfinityBraidPattern;
        let r = pat.generate(&ctx()).unwrap();
        let v = Validator::validate(&r, &ctx(), pat.expects_continuous());
        assert!(v.is_ok(), "braid failed validation: {:?}", v.err());
    }

    #[test]
    fn requires_two_layers() {
        let mut c = ctx();
        c.num_layers = 1;
        let err = InfinityBraidPattern.generate(&c).unwrap_err();
        assert_eq!(err.kind, RoutingErrorKind::BadLayer);
    }

    #[test]
    fn one_period_per_pole_pitch_when_magnet_layout_present() {
        // 800 mm routing domain contains 65 complete 12 mm periods after the
        // uniform phase/strand interleave grid is reserved. The period is
        // exact and every adjacent left/right via-grid point uses the same
        // pole_pitch / (phases × strands) x spacing.
        let pat = InfinityBraidPattern;
        let base = pat.generate(&magnet_ctx()).unwrap();
        assert!(!base.segments.is_empty());
        assert!(
            Validator::validate(&base, &magnet_ctx(), pat.expects_continuous()).is_ok(),
            "magnet-aligned braid must remain bounded"
        );

        // The diamond period length equals the pole pitch and consecutive
        // period grids are spaced by that same pitch.
        let d_tot = magnet_ctx().active_area_length_mm + magnet_ctx().padding_mm * 2.0;
        let n_expected = ((d_tot + 12.0 / (3.0 * 5.0)) / 12.0).floor() - 1.0;
        let period_length: f64 = 12.0;
        assert!((period_length - 12.0).abs() < 1e-9, "period {period_length} mm");
        assert_eq!(n_expected as usize, 65);
        assert_eq!(base.pole_regions.len(), 65 * magnet_ctx().phases as usize);
        assert_eq!(
            base.pole_regions
                .iter()
                .filter(|region| region.phase == "A")
                .count(),
            65
        );
        for region in base.pole_regions.iter().filter(|region| region.phase == "A") {
            assert!(region.end.x > region.start.x);
            assert_eq!(region.start.y, magnet_ctx().board_width_mm / 2.0);
            assert_eq!(region.end.y, magnet_ctx().board_width_mm / 2.0);
        }
        let phase_a_widths: Vec<f64> = base
            .pole_regions
            .iter()
            .filter(|region| region.phase == "A")
            .map(|region| region.end.x - region.start.x)
            .collect();
        assert!(phase_a_widths
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 1e-12));

        // The shared boundary between adjacent pole regions is the midpoint
        // of point 3's rightmost vertex in one period and point 1's leftmost
        // vertex in the next period. It must not be the point-0/top vertex.
        let context = magnet_ctx();
        let phases = context.phases as i64;
        let strands = context.param("num_strands", 5.0) as i64;
        let pitch = context.magnet_pitch().unwrap();
        let interleave = pitch / (phases * strands) as f64;
        let periods = ((context.active_area_length_mm + context.padding_mm * 2.0
            + interleave)
            / pitch)
            .floor() as i64
            - 1;
        let o = -interleave;
        let a = context.board_width_mm / 2.0;
        let diamonds = compute_diamonds(
            0.0,
            periods as f64 * pitch,
            periods,
            strands,
            o,
            a,
        );
        let expected_boundary = (diamonds[0].last().unwrap()[3].x
            + diamonds[1].first().unwrap()[1].x)
            / 2.0;
        let phase_a = base
            .pole_regions
            .iter()
            .filter(|region| region.phase == "A")
            .collect::<Vec<_>>();
        assert!((phase_a[0].end.x - expected_boundary).abs() < 1e-12);

        // Phase-0 active conductors repeat at the period spacing.
        let phase0: Vec<f64> = base
            .segments
            .iter()
            .filter(|s| s.net == "A" && s.is_active)
            .map(|s| s.start.x)
            .collect();
        assert!(!phase0.is_empty());
        let first = phase0[0];
        let second = phase0.iter().find(|x| **x > first + 1e-6);
        if let Some(second_x) = second {
            // Spacing between neighbouring conductors is a sub-multiple of the
            // period (strands × phases interleave within one period).
            let spacing = second_x - first;
            assert!(spacing > 0.0 && spacing <= period_length + 1e-6);
        }
    }

    #[test]
    fn traces_regenerate_when_pole_pitch_changes() {
        // Changing the magnet pattern (pole pitch) MUST change the generated
        // geometry — this is the "traces follow the layout" invariant.
        let pat = InfinityBraidPattern;
        let pitch12 = pat.generate(&magnet_ctx()).unwrap();
        let mut tighter = magnet_ctx();
        tighter.magnet_pitch_mm = Some(10.0);
        let pitch10 = pat.generate(&tighter).unwrap();
        assert_ne!(pitch12.segments.len(), pitch10.segments.len());

        // And the fallback (no magnet layout) is unchanged from the packaged
        // default so plugin probes and legacy tests keep their behaviour.
        let legacy = pat.generate(&ctx()).unwrap();
        assert_ne!(legacy.segments.len(), pitch12.segments.len());
        let mut with_param = ctx();
        with_param.params.insert("n_periods".to_string(), 4.0);
        let legacy_param = pat.generate(&with_param).unwrap();
        assert_eq!(legacy.segments.len(), legacy_param.segments.len());
    }

    #[test]
    fn report_exposes_exact_pole_pitch_and_phase_band_width_budget() {
        let report = crate::generate_routing_report(&magnet_ctx(), "infinity-braid")
            .expect("reference braid report");
        assert_eq!(report.dimensions.pole_pitch_mm, Some(12.0));
        assert_eq!(report.dimensions.magnet_array_span_mm, Some(120.0));
        assert_eq!(report.dimensions.period_pitch_mm, Some(12.0));
        assert_eq!(report.dimensions.phase_band_pitch_mm, Some(4.0));
        assert_eq!(report.dimensions.period_count, Some(65));
        assert!(!report.dimensions.phase_band_widths.is_empty());
        assert!(report.dimensions.all_phase_bands_fit());
        assert!(report
            .dimensions
            .phase_band_widths
            .iter()
            .all(|band| (band.angle_rad - (20.0_f64 / 12.0).atan()).abs() < 1e-12));
    }

    #[test]
    fn rejects_a_routing_span_shorter_than_one_pole_pitch() {
        let mut short = magnet_ctx();
        short.active_area_length_mm = 5.0;
        short.padding_mm = 0.0;
        let error = InfinityBraidPattern.generate(&short).unwrap_err();
        assert_eq!(error.kind, RoutingErrorKind::Generation);
        assert!(error.message.contains("cannot fit one complete pole-pitched"));
    }

    #[test]
    fn fallback_report_exposes_period_without_claiming_magnet_alignment() {
        let report = crate::generate_routing_report(&ctx(), "infinity-braid")
            .expect("fallback braid report");
        assert!(report.dimensions.pole_pitch_mm.is_none());
        assert_eq!(report.dimensions.period_count, Some(4));
        assert!(report.dimensions.period_pitch_mm.unwrap() > 0.0);
    }

    #[test]
    fn via_x_spacing_is_uniform_across_phase_boundaries_on_both_layers() {
        let context = magnet_ctx();
        let result = InfinityBraidPattern.generate(&context).unwrap();
        let phases = context.phases as usize;
        let strands = context.param("num_strands", 5.0) as usize;
        let vias_per_phase = result.vias.len() / phases;
        let phase_vias: Vec<&[Via]> = result.vias.chunks(vias_per_phase).collect();
        let expected_step = context.magnet_pitch().unwrap() / (phases * strands) as f64;

        // The first `num_strands` vias in each phase are its left-side
        // endpoints. They form the horizontal via grid shared by the top and
        // bottom layer copies. The boundary between phase i and phase i+1
        // must be the same as the final within-phase step.
        for phase in 0..phases {
            let left = &phase_vias[phase][..strands];
            let within_phase = (left[strands - 1].position.x - left[strands - 2].position.x).abs();
            assert!((within_phase - expected_step).abs() < 1e-12);

            if let Some(next) = phase_vias.get(phase + 1) {
                let across_phases =
                    (next[0].position.x - left[strands - 1].position.x).abs();
                assert!((across_phases - within_phase).abs() < 1e-12);
            }
        }

        // Both layer copies use the same via coordinates. Check the explicit
        // 0 → 1 via rows as well as the phase-grid relationship above.
        assert!(result
            .vias
            .iter()
            .all(|via| via.from_layer == 0 && via.to_layer == 1));
    }
}
