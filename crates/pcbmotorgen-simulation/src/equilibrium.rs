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
//! endpoints as COIL-CAPTURE positions anchored to the stator copper region
//! (see that function's docs).

/// Stable-equilibrium travel envelope of the mover array centre (metres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TravelEnvelope {
    /// First stable rest position of the array centre [m].
    pub min_position_m: f64,
    /// Last stable rest position of the array centre [m] (≥ min).
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

/// Stable-equilibrium travel envelope anchored to the stator copper region.
///
/// Product reference convention (2026-08-21): the endpoints are the
/// COIL-CAPTURE positions — the first/last spot where the first/last coil
/// carries enough charge to capture the first/last magnet pole. This is a
/// commutation-phase offset that scales with the ELECTRICAL PERIOD only
/// (independent of N):
///
/// - **min = copper_region_start + (2/3)·P_e** — first coil captures the
///   first pole at electrical phase 240° (Phase A@120°, Phase B@0°,
///   Phase C@240°). Defaults (copper_region_start = 30 mm, P_e = 12 mm):
///   30 + 8 = **38 mm**.
/// - **max = copper_region_end − (3/4)·P_e** — last coil captured
///   symmetrically at the 270° complementary state. Defaults
///   (copper_region_end = 177 mm): 177 − 9 = **168 mm**.
///
/// `rest_phase_m` is the TRACK-FRAME lattice phase
/// `(copper_region_start + φ) mod P_e` (φ the baseline rest phase), still
/// N-dependent, so the holding-force chart's zero markers stay aligned to
/// the stable rests. A copper region narrower than the envelope clamps max
/// to min.
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
            electrical_period_m: electrical_period_m,
        };
    };
    let phi = rest_phase_m(period, magnet_count);
    let min = copper_region_start_m + (2.0 / 3.0) * period;
    let max = (copper_region_end_m - (3.0 / 4.0) * period).max(min);
    let phase_track = (copper_region_start_m + phi) % period;
    TravelEnvelope {
        min_position_m: min,
        max_position_m: max,
        rest_phase_m: phase_track,
        electrical_period_m: period,
    }
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
    // travel_envelope_over_slots — coil-capture envelope
    //
    // These pins are the PRODUCT REFERENCE for the slider endpoints. If min
    // or max move, these tests fail — change them only alongside the spec.
    // ------------------------------------------------------------------

    /// Defaults (P_e = 12 mm, copper region [30, 177] mm in track coords):
    /// min = 30 + (2/3)·12 = **38 mm** (240° first-coil capture), max =
    /// 177 − (3/4)·12 = **168 mm** (270° last-coil capture). Track-frame
    /// lattice phase = (30 + 10) mod 12 = **4 mm**.
    #[test]
    fn twelve_pole_defaults_pins_38_to_168_mm() {
        let env = travel_envelope_over_slots(12.0 * MM, 12, 30.0 * MM, 177.0 * MM);
        assert!((env.min_position_m - 38.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 168.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 4.0 * MM).abs() < 1e-12);
        assert!((env.electrical_period_m - 12.0 * MM).abs() < 1e-12);
    }

    /// Capture offsets depend on P_e only, not N — N=4 shares the endpoints
    /// (its φ and therefore its rest-phase marker differ).}
    #[test]
    fn four_pole_shares_the_capture_endpoints() {
        let env = travel_envelope_over_slots(12.0 * MM, 4, 30.0 * MM, 177.0 * MM);
        assert!((env.min_position_m - 38.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 168.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 4.0 * MM).abs() < 1e-12);
    }

    /// N=6 also shares the endpoints; its φ = 4 mm → track-frame phase
    /// (30 + 4) mod 12 = 10 mm.
    #[test]
    fn six_pole_shares_endpoints_but_has_its_own_phase() {
        let env = travel_envelope_over_slots(12.0 * MM, 6, 30.0 * MM, 177.0 * MM);
        assert!((env.min_position_m - 38.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 168.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 10.0 * MM).abs() < 1e-12);
    }

    /// Endpoints scale with P_e: a wider electrical period pushes min up
    /// and max down by the capture fractions.
    #[test]
    fn endpoints_scale_with_electrical_period() {
        let env = travel_envelope_over_slots(18.0 * MM, 12, 30.0 * MM, 177.0 * MM);
        assert!((env.min_position_m - (30.0 + 2.0 / 3.0 * 18.0) * MM).abs() < 1e-12);
        assert!((env.max_position_m - (177.0 - 3.0 / 4.0 * 18.0) * MM).abs() < 1e-12);
    }

    /// Copper region narrower than the envelope clamps max to min.
    #[test]
    fn narrow_copper_region_clamps_max_to_min() {
        // Copper region [30, 40] mm: min = 38, max = max(40−9, 38) = 38 → clamped.
        let env = travel_envelope_over_slots(12.0 * MM, 24, 30.0 * MM, 40.0 * MM);
        assert!((env.max_position_m - env.min_position_m).abs() < 1e-12);
        assert!((env.min_position_m - 38.0 * MM).abs() < 1e-12);
    }

    #[test]
    fn non_positive_period_zeroes_envelope() {
        let env = travel_envelope_over_slots(0.0, 4, 30.0 * MM, 177.0 * MM);
        assert_eq!(env.min_position_m, 0.0);
        assert_eq!(env.max_position_m, 0.0);
    }
}
