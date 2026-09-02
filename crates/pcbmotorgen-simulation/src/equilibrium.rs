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
//! endpoints as the FIRST and LAST STABLE REST POSITION inside the copper
//! active area — the glossary "Travel Envelope" (see that function's docs).

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

/// Tolerance for lattice snapping [m]: one nanometre — many orders of
/// magnitude above f64 representation noise at metre-scale positions
/// (~1e-16 m) and far below any engineering significance (µm scale).
const LATTICE_SNAP_EPS_M: f64 = 1e-9;

/// Smallest point of the lattice `phase + k·period` (k ∈ ℤ) that is ≥ `x`.
///
/// The adjustment loop guards against float rounding stepping a full period
/// too far when `x` lands (within [`LATTICE_SNAP_EPS_M`]) exactly on the
/// lattice: a quotient that is mathematically an integer but nudged to
/// e.g. 3.000…4 would otherwise `ceil` to 4.
#[must_use]
fn lattice_ceil(x: f64, phase: f64, period: f64) -> f64 {
    let mut point = phase + ((x - phase) / period).ceil() * period;
    while point - period >= x - LATTICE_SNAP_EPS_M {
        point -= period;
    }
    point
}

/// Largest point of the lattice `phase + k·period` (k ∈ ℤ) that is ≤ `x`.
///
/// Mirror-image float guard of [`lattice_ceil`].
#[must_use]
fn lattice_floor(x: f64, phase: f64, period: f64) -> f64 {
    let mut point = phase + ((x - phase) / period).floor() * period;
    while point + period <= x + LATTICE_SNAP_EPS_M {
        point += period;
    }
    point
}

/// Stable-equilibrium travel envelope anchored to the stator copper region.
///
/// Glossary-normative spec (2026-09-02, kata xb16): the Travel Envelope is
/// "the span of valid mover positions between the FIRST and LAST STABLE
/// REST POSITION inside the copper active area". The endpoints are derived
/// in two steps:
///
/// 1. **Span-aware centre clamp** — the array centre must keep the whole
///    mover inside copper: `centre ∈ [copper_start + span/2, copper_end −
///    span/2]` with the glossary "Mover Span" `span = N · τ_p`
///    (τ_p = P_e/2).
/// 2. **Lattice snapping** — endpoints must be STABLE rest positions, i.e.
///    on the track-frame lattice `x ≡ φ_track (mod P_e)` with
///    `φ_track = (copper_region_start + φ) mod P_e` (φ the baseline rest
///    phase, see [`rest_phase_m`]):
///    - `min` = smallest lattice point ≥ `copper_region_start + span/2`,
///    - `max` = largest lattice point ≤ `copper_region_end − span/2`.
///
/// Degenerate behavior (no lattice point between the clamped bounds — the
/// copper region is shorter than the mover span, or too short to admit one
/// full lattice step past the lower bound): `max` is clamped to `min`,
/// where `min` is the nearest lattice point ≥ the lower bound. The envelope
/// therefore never inverts (max ≥ min), but in this degenerate case the
/// array edge may overhang the copper at that single rest position.
///
/// Worked default example (P_e = 12 mm, N = 12, copper region [30, 177] mm
/// in track coords): φ = (P_e/12 + (11/2)·τ_p) mod P_e = 10 mm, so
/// φ_track = (30 + 10) mod 12 = 4 mm; span = 12·6 mm = 72 mm gives a centre
/// range [66, 141] mm; the lattice {…, 64, 76, 88, …} mm snaps the
/// endpoints to **min = 76 mm, max = 136 mm**. Unlike the pre-xb16
/// coil-capture convention, the endpoints DEPEND on N (N = 4 gives
/// 52 → 160 mm on the same copper and period).
///
/// `rest_phase_m` is unchanged: the TRACK-FRAME lattice phase
/// `(copper_region_start + φ) mod P_e`, so the holding-force chart's zero
/// markers stay aligned to the stable rests.
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
    // Span-aware centre clamp: keep the whole array inside the copper.
    let lower = copper_region_start_m + span / 2.0;
    let upper = copper_region_end_m - span / 2.0;
    // Lattice snapping: endpoints must be stable rest positions.
    let min = lattice_ceil(lower, phase_track, period);
    let max = lattice_floor(upper, phase_track, period);
    // Degenerate (no lattice point between the bounds): never inverted.
    let max = if max < min { min } else { max };
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
    // travel_envelope_over_slots — glossary travel envelope (kata xb16)
    //
    // These pins are the PRODUCT REFERENCE for the slider endpoints:
    // lattice-snapped, span-aware first/last stable rest positions
    // (glossary "Travel Envelope", decision 2026-09-02). Every value below
    // was verified numerically against rest_phase_m before pinning. If min
    // or max move, these tests fail — change them only alongside the spec.
    // ------------------------------------------------------------------

    /// Defaults (N = 12, P_e = 12 mm, copper [30, 177] mm in track coords),
    /// per the function doc's worked example: φ = 10 mm → φ_track = 4 mm;
    /// span = 12·6 = 72 mm → centre range [66, 141] mm; lattice
    /// {…, 64, 76, 88, …} → **min = 76 mm, max = 136 mm**. The span-aware
    /// clamp keeps the whole array inside copper: leading edge
    /// 76 − 36 = 40 mm ≥ 30 mm, trailing edge 136 + 36 = 172 mm ≤ 177 mm.
    #[test]
    fn twelve_pole_defaults_pin_76_to_136_mm() {
        let env = travel_envelope_over_slots(12.0 * MM, 12, 30.0 * MM, 177.0 * MM);
        assert!((env.min_position_m - 76.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 136.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 4.0 * MM).abs() < 1e-12);
        assert!((env.electrical_period_m - 12.0 * MM).abs() < 1e-12);
        let span = 72.0 * MM; // N · τ_p
        assert!(env.min_position_m - span / 2.0 >= 30.0 * MM);
        assert!(env.max_position_m + span / 2.0 <= 177.0 * MM);
    }

    /// Endpoints DEPEND on N (the xb16 fix — pre-xb16 they were fixed
    /// coil-capture offsets). N = 4: φ = 1 + (3/2)·6 = 10 mm → φ_track =
    /// (30 + 10) mod 12 = 4 mm (same lattice as N = 12); span = 24 mm →
    /// centre range [42, 165] mm → **min = 52 mm, max = 160 mm**. With
    /// small N the slider now reaches near the track ends.
    #[test]
    fn four_pole_endpoints_depend_on_magnet_count() {
        let env = travel_envelope_over_slots(12.0 * MM, 4, 30.0 * MM, 177.0 * MM);
        assert!((env.min_position_m - 52.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 160.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 4.0 * MM).abs() < 1e-12);
    }

    /// N = 6: φ = 1 + (5/2)·6 = 16 mm → φ_track = (30 + 16) mod 12 =
    /// 10 mm (its own lattice {…, 58, 70, 82, …}); span = 36 mm → centre
    /// range [48, 159] mm → **min = 58 mm, max = 154 mm**. Different
    /// endpoints AND different phase from both N = 4 and N = 12.
    #[test]
    fn six_pole_endpoints_depend_on_magnet_count() {
        let env = travel_envelope_over_slots(12.0 * MM, 6, 30.0 * MM, 177.0 * MM);
        assert!((env.min_position_m - 58.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 154.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 10.0 * MM).abs() < 1e-12);
    }

    /// Endpoints scale with P_e through BOTH the span clamp and the lattice
    /// phase (φ itself depends on P_e: x_peak = P_e/12, τ_p = P_e/2).
    /// N = 12, P_e = 18 mm: x_peak = 1.5 mm, φ = (1.5 + 49.5) mod 18 =
    /// 15 mm → φ_track = (30 + 15) mod 18 = 9 mm; span = 108 mm → centre
    /// range [84, 123] mm; lattice {…, 99, 117, …} → **min = 99 mm,
    /// max = 117 mm**. (Pre-xb16 this pinned the coil-capture 42 → 163.5.)
    #[test]
    fn endpoints_scale_with_electrical_period() {
        let env = travel_envelope_over_slots(18.0 * MM, 12, 30.0 * MM, 177.0 * MM);
        assert!((env.min_position_m - 99.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 117.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 9.0 * MM).abs() < 1e-12);
    }

    /// Degenerate (kata xb16): copper [30, 40] mm is far shorter than the
    /// N = 24 span (144 mm), so the clamped centre range [102, −32] mm
    /// holds no lattice point. min = nearest lattice point ≥ 102 mm on the
    /// φ_track = 4 mm lattice = **112 mm**, and max clamps to min: never
    /// inverted, even though the array necessarily overhangs the copper at
    /// that single rest position (documented degenerate behavior).
    #[test]
    fn narrow_copper_region_clamps_max_to_min() {
        let env = travel_envelope_over_slots(12.0 * MM, 24, 30.0 * MM, 40.0 * MM);
        assert!((env.min_position_m - 112.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - env.min_position_m).abs() < 1e-12);
    }

    /// Degenerate variant (kata xb16): the clamped centre range [66, 68.5]
    /// mm (N = 12, copper [30, 104.5] mm) is NON-empty but narrower than
    /// one lattice step (P_e = 12 mm), so it contains no stable rest
    /// position. max clamps to min = **76 mm**.
    #[test]
    fn no_lattice_point_between_bounds_clamps_max_to_min() {
        let env = travel_envelope_over_slots(12.0 * MM, 12, 30.0 * MM, 104.5 * MM);
        assert!((env.min_position_m - 76.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - env.min_position_m).abs() < 1e-12);
    }

    /// Exact lattice hit on the UPPER bound: with copper length 142 mm ≡
    /// (φ + span/2) (mod P_e), the upper bound lands exactly on the
    /// φ_track = 4 mm lattice and max must equal it BIT-EXACTLY (the float
    /// guard must not step down a period). N = 4: span/2 = 12 mm →
    /// **min = 52 mm, max = 160 mm** (trailing edge at exactly 172 mm);
    /// N = 12: span/2 = 36 mm → **min = 76 mm, max = 136 mm**. The LOWER
    /// bound can never be an exact hit: span/2 − φ ≡ P_e/6 (mod P_e) for
    /// every N, so min is always one sixth period above the lattice.
    #[test]
    fn exact_upper_lattice_hit_is_preserved() {
        // N = 4: upper = 172 − 12 = 160 mm = 4 + 13·12 mm.
        let env = travel_envelope_over_slots(12.0 * MM, 4, 30.0 * MM, 172.0 * MM);
        assert_eq!(env.max_position_m, 172.0 * MM - 12.0 * MM);
        assert!((env.min_position_m - 52.0 * MM).abs() < 1e-12);
        // N = 12: upper = 172 − 36 = 136 mm = 4 + 11·12 mm.
        let env = travel_envelope_over_slots(12.0 * MM, 12, 30.0 * MM, 172.0 * MM);
        assert_eq!(env.max_position_m, 172.0 * MM - 36.0 * MM);
        assert!((env.min_position_m - 76.0 * MM).abs() < 1e-12);
    }

    #[test]
    fn non_positive_period_zeroes_envelope() {
        let env = travel_envelope_over_slots(0.0, 4, 30.0 * MM, 177.0 * MM);
        assert_eq!(env.min_position_m, 0.0);
        assert_eq!(env.max_position_m, 0.0);
    }
}
