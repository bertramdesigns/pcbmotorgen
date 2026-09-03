//! Mover equilibrium positions under a fixed balanced excitation.
//!
//! Implements the project's mover-equilibrium spec: the Clarke transform of
//! the baseline phase currents (I_A = +I, I_B = 0, I_C = −I) yields an
//! electrical angle θe = π/6 (30°), which maps to a spatial N-pole field
//! peak at `x_peak = θe·P_e/(2π) = P_e/12` inside each electrical cycle of
//! length `P_e`. Alternating mover poles (pitch τ_p = P_e/2) lock onto the
//! successive field peaks, so the array CENTRE rests at
//!
//!   φ = (x_peak + ((N−1)/2)·τ_p) mod P_e
//!
//! and every stable rest position satisfies x ≡ φ (mod P_e).
//!
//! The travel envelope ([`travel_envelope_over_slots`]) exposes the slider
//! endpoints as the span-aware FLUSH limits of the copper active area —
//! the array edges sit exactly on the copper bounds at the endpoints
//! (kata 5c7r; see that function's docs). The stable rests themselves are
//! reported via `rest_phase_m` and marked on the holding-force chart.
//!
//! ## Declared phase bands (kata hzs2)
//!
//! When the routing contract declares phase-band geometry, the equilibrium
//! helpers in this module can anchor the copper region to the declared band
//! extents ([`copper_region_from_phase_bands`] /
//! [`travel_envelope_from_phase_bands`]) instead of the caller's analytic
//! copper arguments; `rest_phase_m` itself is pure lattice math and needs no
//! bands.

use crate::params::PhaseBandPosition;

/// Travel envelope of the mover array centre (metres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TravelEnvelope {
    /// Smallest travel limit of the array centre [m]: leading array edge
    /// flush with the copper start.
    pub min_position_m: f64,
    /// Largest travel limit of the array centre [m] (≥ min): trailing
    /// array edge flush with the copper end.
    pub max_position_m: f64,
    /// Rest phase φ: every stable rest centre ≡ φ (mod electrical_period_m).
    pub rest_phase_m: f64,
    /// Electrical period P_e = 2 × pole pitch (one full 360° electrical
    /// cycle) [m].
    pub electrical_period_m: f64,
}

/// Electrical angle of the baseline excitation (I_A = +I, I_B = 0, I_C = −I)
/// via the Clarke transform, normalized to [0, 2π):
///
///   I_α = I_A − 0.5·I_B − 0.5·I_C = 1.5·I
///   I_β = (√3/2)·(I_B − I_C)     = (√3/2)·I
///   θe  = atan2(I_β, I_α)        = atan2(√3/2, 3/2) = π/6
///
/// The baseline is balanced by construction (I_A + I_B + I_C = 0), so the
/// zero-current singularity (undefined holding state) cannot occur here.
/// Returns radians in [0, 2π).
#[must_use]
pub fn baseline_electrical_angle() -> f64 {
    std::f64::consts::FRAC_PI_6
}

/// Spatial offset of the N-pole field peak within one electrical cycle for
/// the baseline excitation: `x_peak = θe·P_e/(2π)` [m].
#[must_use]
pub fn baseline_field_peak_m(electrical_period_m: f64) -> f64 {
    baseline_electrical_angle() * electrical_period_m / (2.0 * std::f64::consts::PI)
}

/// Rest phase φ of the mover array centre [m]: stable rest centres satisfy
/// `x ≡ φ (mod electrical_period_m)`.
///
/// `electrical_period_m` is P_e = 2 × pole pitch (one full 360° electrical
/// cycle); `magnet_count` is N ≥ 1.
#[must_use]
pub fn rest_phase_m(electrical_period_m: f64, magnet_count: u32) -> f64 {
    if !(electrical_period_m > 0.0) {
        return 0.0;
    }
    let tau_p = electrical_period_m / 2.0;
    let x_peak = baseline_field_peak_m(electrical_period_m);
    let phi = x_peak + ((f64::from(magnet_count) - 1.0) / 2.0) * tau_p;
    let mut phase = phi % electrical_period_m;
    if phase < 0.0 {
        phase += electrical_period_m;
    }
    phase
}

/// Travel envelope of the mover array centre, anchored to the stator copper
/// region.
///
/// Glossary-normative spec (2026-09-02, kata 5c7r): the Travel Envelope is
/// the SPAN-AWARE FLUSH CLAMP of the copper active area —
///
///   `centre ∈ [copper_start + span/2, copper_end − span/2]`
///
/// with the glossary "Mover Span" `span = N · τ_p` (τ_p = P_e/2). At the
/// lower limit the array's leading edge sits exactly on the copper start;
/// at the upper limit the trailing edge sits exactly on the copper end.
/// The endpoints are MECHANICAL LIMITS, not stable rest positions: the
/// swept range equals the configured free travel EXACTLY
/// (`travel = copper_length − span`), and the mover may hold position
/// between rests (a closed-loop drive compensates the non-zero
/// fixed-excitation force there). Supersedes the earlier rest-snapped
/// revisions (kata xb16), which lost travel or overhung the copper.
///
/// Degenerate behavior (copper region shorter than the mover span): the
/// clamped range inverts, so `max` is clamped to `min` — the envelope
/// never inverts, and the array necessarily overhangs the copper at that
/// single position.
///
/// `rest_phase_m` is unchanged: the track-frame lattice phase
/// `(copper_region_start + φ) mod P_e` (φ the baseline rest phase, see
/// [`rest_phase_m`]), so the holding-force chart's zero markers stay
/// aligned to the stable rests — which remain on the `x ≡ φ (mod P_e)`
/// lattice even though the slider endpoints generally do not.
#[must_use]
pub fn travel_envelope_over_slots(
    electrical_period_m: f64,
    magnet_count: u32,
    copper_region_start_m: f64,
    copper_region_end_m: f64,
) -> TravelEnvelope {
    let period = if electrical_period_m > 0.0 {
        electrical_period_m
    } else {
        return TravelEnvelope {
            min_position_m: 0.0,
            max_position_m: 0.0,
            rest_phase_m: 0.0,
            electrical_period_m,
        };
    };
    let phi = rest_phase_m(period, magnet_count);
    let phase_track = (copper_region_start_m + phi) % period;
    // Glossary "Mover Span": N · τ_p (centre-of-first-magnet to one pitch
    // past the last magnet).
    let span = f64::from(magnet_count) * period / 2.0;
    // Span-aware flush clamp: the array edges sit exactly on the copper
    // bounds at the endpoints (kata 5c7r — no lattice snapping).
    let min = copper_region_start_m + span / 2.0;
    let max = copper_region_end_m - span / 2.0;
    // Degenerate (copper shorter than the span): never inverted.
    let max = if max < min { min } else { max };
    TravelEnvelope {
        min_position_m: min,
        max_position_m: max,
        rest_phase_m: phase_track,
        electrical_period_m: period,
    }
}

/// Copper region bounds from declared phase bands (kata hzs2) [m]: the union
/// of the bands' along-travel extents. Returns `None` when no bands are
/// declared — callers then keep their analytic copper region.
#[must_use]
pub fn copper_region_from_phase_bands(bands: &[PhaseBandPosition]) -> Option<(f64, f64)> {
    if bands.is_empty() {
        return None;
    }
    let start = bands
        .iter()
        .map(|band| band.start_m)
        .fold(f64::INFINITY, f64::min);
    let end = bands
        .iter()
        .map(|band| band.end_m)
        .fold(f64::NEG_INFINITY, f64::max);
    Some((start, end))
}

/// Travel envelope anchored to the declared phase-band copper region (kata
/// hzs2) [m].
///
/// The copper region is the union of the declared bands' extents (see
/// [`copper_region_from_phase_bands`]); everything else matches
/// [`travel_envelope_over_slots`]. Returns `None` when no bands are declared
/// — the caller falls back to the analytic copper arguments.
#[must_use]
pub fn travel_envelope_from_phase_bands(
    electrical_period_m: f64,
    magnet_count: u32,
    bands: &[PhaseBandPosition],
) -> Option<TravelEnvelope> {
    let (start, end) = copper_region_from_phase_bands(bands)?;
    Some(travel_envelope_over_slots(
        electrical_period_m,
        magnet_count,
        start,
        end,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = 0.001;

    #[test]
    fn baseline_angle_is_thirty_degrees() {
        let theta = baseline_electrical_angle();
        assert!((theta - std::f64::consts::FRAC_PI_6).abs() < 1e-15);
        // Clarke cross-check with I = 1: α = 1.5, β = √3/2.
        let (i_a, i_b, i_c) = (1.0_f64, 0.0_f64, -1.0_f64);
        let alpha = i_a - 0.5 * i_b - 0.5 * i_c;
        let beta = (3.0_f64).sqrt() / 2.0 * (i_b - i_c);
        let theta_raw = beta.atan2(alpha);
        let theta_norm = if theta_raw < 0.0 {
            theta_raw + 2.0 * std::f64::consts::PI
        } else {
            theta_raw
        };
        assert!((theta_norm - theta).abs() < 1e-12);
    }

    #[test]
    fn field_peak_is_one_twelfth_of_period() {
        assert!((baseline_field_peak_m(12.0 * MM) - 1.0 * MM).abs() < 1e-12);
    }

    // ------------------------------------------------------------------
    // travel_envelope_over_slots — glossary travel envelope (kata 5c7r)
    //
    // These pins are the PRODUCT REFERENCE for the slider endpoints:
    // flush, span-aware travel limits — the array edges sit exactly on
    // the copper bounds (glossary "Travel Envelope", decision 2026-09-02,
    // supersedes the xb16 rest-snapped revisions). Every value below was
    // verified numerically before pinning. If min or max move, these
    // tests fail — change them only alongside the spec.
    // ------------------------------------------------------------------

    /// Defaults (N = 12, P_e = 12 mm, copper [0, 147] mm in track coords):
    /// span = 12·6 = 72 mm → flush clamp **min = 36 mm, max = 111 mm** —
    /// leading edge 36 − 36 = 0 mm exactly on the copper start, trailing
    /// edge 111 + 36 = 147 mm exactly on the copper end. Sweep = 75 mm =
    /// the configured travel EXACTLY. The endpoints are limits, not rest
    /// positions: rest_phase_m = 10 mm still reports where the stable
    /// rests (force-chart zeros) live.
    #[test]
    fn twelve_pole_defaults_pin_flush_36_to_111_mm() {
        let env = travel_envelope_over_slots(12.0 * MM, 12, 0.0, 147.0 * MM);
        assert!((env.min_position_m - 36.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 111.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 10.0 * MM).abs() < 1e-12);
        assert!((env.electrical_period_m - 12.0 * MM).abs() < 1e-12);
        // Flush contract: array edges exactly on the copper bounds.
        let span = 72.0 * MM; // N · τ_p
        assert!((env.min_position_m - span / 2.0).abs() < 1e-12);
        assert!((env.max_position_m + span / 2.0 - 147.0 * MM).abs() < 1e-12);
    }

    /// Travel contract (kata 5c7r): the swept range equals the configured
    /// free travel `copper_length − span` EXACTLY — no lattice snapping,
    /// no endpoint bias.
    #[test]
    fn sweep_equals_configured_travel_exactly() {
        for (n, p_e) in [(4_u32, 12.0), (6, 12.0), (12, 12.0), (12, 18.0), (10, 24.0)] {
            let env = travel_envelope_over_slots(p_e * MM, n, 0.0, 147.0 * MM);
            let travel = 147.0 * MM - f64::from(n) * p_e * MM / 2.0;
            let sweep = env.max_position_m - env.min_position_m;
            assert!(
                (sweep - travel).abs() < 1e-12,
                "N={n} P_e={p_e}: sweep {sweep:.6} vs travel {travel:.6}"
            );
        }
    }

    /// Real app defaults regression: N = 10, P_e = 24 mm (τ_p = 12 mm),
    /// copper [0, 195] mm → span = 120 mm → flush clamp **[60, 135] mm** —
    /// strip 0 → 120 mm at min, 75 → 195 mm at max, sweep = 75 mm = the
    /// configured travel exactly (kata hrd8 geometry; the reported
    /// "short on max / too far on min" bias is gone). rest_phase 8 mm.
    #[test]
    fn app_defaults_pin_flush_60_to_135_mm() {
        let env = travel_envelope_over_slots(24.0 * MM, 10, 0.0, 195.0 * MM);
        assert!((env.min_position_m - 60.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 135.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 8.0 * MM).abs() < 1e-12);
    }

    /// Endpoints DEPEND on N. N = 4: span = 24 mm → flush clamp
    /// **[12, 135] mm** on the copper [0, 147] mm. With small N the
    /// slider reaches near the track ends.
    #[test]
    fn four_pole_endpoints_depend_on_magnet_count() {
        let env = travel_envelope_over_slots(12.0 * MM, 4, 0.0, 147.0 * MM);
        assert!((env.min_position_m - 12.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 135.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 10.0 * MM).abs() < 1e-12);
    }

    /// N = 6: span = 36 mm → flush clamp **[18, 129] mm**; its rest phase
    /// differs from both N = 4 and N = 12 (φ = 16 mm → 4 mm mod 12).
    #[test]
    fn six_pole_endpoints_depend_on_magnet_count() {
        let env = travel_envelope_over_slots(12.0 * MM, 6, 0.0, 147.0 * MM);
        assert!((env.min_position_m - 18.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 129.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 4.0 * MM).abs() < 1e-12);
    }

    /// Endpoints scale with P_e through the span (τ_p = P_e/2).
    /// N = 12, P_e = 18 mm: span = 108 mm → flush clamp **[54, 93] mm**;
    /// rest phase 15 mm.
    #[test]
    fn endpoints_scale_with_electrical_period() {
        let env = travel_envelope_over_slots(18.0 * MM, 12, 0.0, 147.0 * MM);
        assert!((env.min_position_m - 54.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 93.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 15.0 * MM).abs() < 1e-12);
    }

    /// Degenerate: copper [0, 10] mm is far shorter than the N = 24 span
    /// (144 mm), so the flush clamp [72, −34] mm inverts → max clamps to
    /// min: **[72, 72] mm**, never inverted (the array necessarily
    /// overhangs the copper at that single position — documented
    /// degenerate behavior).
    #[test]
    fn narrow_copper_region_clamps_max_to_min() {
        let env = travel_envelope_over_slots(12.0 * MM, 24, 0.0, 10.0 * MM);
        assert!((env.min_position_m - 72.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - env.min_position_m).abs() < 1e-12);
    }

    /// Degenerate variant: the clamp [36, 38.5] mm (N = 12, copper
    /// [0, 74.5] mm) is non-empty but narrower than one electrical
    /// period. With no lattice snapping the range is kept as-is:
    /// **[36, 38.5] mm** (the endpoints are limits, not rests — no
    /// reason to collapse them).
    #[test]
    fn degenerate_narrow_range_is_kept_as_is() {
        let env = travel_envelope_over_slots(12.0 * MM, 12, 0.0, 74.5 * MM);
        assert!((env.min_position_m - 36.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 38.5 * MM).abs() < 1e-12);
    }

    #[test]
    fn non_positive_period_zeroes_envelope() {
        let env = travel_envelope_over_slots(0.0, 4, 0.0, 147.0 * MM);
        assert_eq!(env.min_position_m, 0.0);
        assert_eq!(env.max_position_m, 0.0);
    }
    // ------------------------------------------------------------------
    // Declared phase bands (kata hzs2)
    // ------------------------------------------------------------------

    use crate::params::PhaseBandPosition;

    fn band(phase: &str, centerline_m: f64, start_m: f64, end_m: f64) -> PhaseBandPosition {
        PhaseBandPosition {
            phase: phase.to_string(),
            centerline_m,
            start_m,
            end_m,
        }
    }

    /// The declared bands' union extent anchors the copper region; the
    /// envelope then matches `travel_envelope_over_slots` on those bounds.
    /// Real-app defaults: N = 10, P_e = 24 mm, declared bands spanning the
    /// full 195 mm copper → flush clamp [60, 135] mm (mirrors the
    /// `app_defaults_pin_flush_60_to_135_mm` pin above).
    #[test]
    fn declared_bands_anchor_the_travel_envelope() {
        let bands = vec![
            band("A", 0.002, 0.0, 0.004),
            band("B", 0.006, 0.004, 0.008),
            band("C", 0.010, 0.008, 0.195),
        ];
        let region = copper_region_from_phase_bands(&bands).expect("declared region");
        assert!((region.0 - 0.0).abs() < 1e-15);
        assert!((region.1 - 0.195).abs() < 1e-15);

        let env = travel_envelope_from_phase_bands(24.0 * MM, 10, &bands)
            .expect("envelope from declared bands");
        let direct = travel_envelope_over_slots(24.0 * MM, 10, region.0, region.1);
        assert_eq!(env, direct);
        // Flush contract still holds on the declared region.
        assert!((env.min_position_m - 0.060).abs() < 1e-12);
        assert!((env.max_position_m - 0.135).abs() < 1e-12);
    }

    /// Extents are unioned across all declared bands (including overlapping
    /// layer copies), and empty input falls back to `None`.
    #[test]
    fn copper_region_unions_bands_and_falls_back_to_none() {
        let bands = vec![
            band("A", 0.002, 0.001, 0.005),
            band("A", 0.002, 0.0, 0.004),
            band("B", 0.006, 0.004, 0.009),
        ];
        let region = copper_region_from_phase_bands(&bands).expect("declared region");
        assert!((region.0 - 0.0).abs() < 1e-15);
        assert!((region.1 - 0.009).abs() < 1e-15);

        assert!(copper_region_from_phase_bands(&[]).is_none());
        assert!(travel_envelope_from_phase_bands(24.0 * MM, 10, &[]).is_none());
    }
}
