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

/// Lattice point of `phase + k·period` (k ∈ ℤ) closest to `x`.
///
/// Ties (x exactly half a period between two lattice points, or exactly on
/// the lattice) resolve to `prefer_higher` — callers pass `true` for the
/// lower envelope endpoint (inward = larger) and `false` for the upper
/// endpoint (inward = smaller), so an exact tie always keeps the array
/// inside the span-aware clamp. Float-noised quotients are handled by the
/// [`lattice_ceil`]/[`lattice_floor`] guards, which collapse to `x` itself
/// when it already sits on the lattice.
#[must_use]
fn lattice_nearest(x: f64, phase: f64, period: f64, prefer_higher: bool) -> f64 {
    let below = lattice_floor(x, phase, period);
    let above = lattice_ceil(x, phase, period);
    let d_below = x - below;
    let d_above = above - x;
    if (d_above - d_below).abs() <= LATTICE_SNAP_EPS_M {
        if prefer_higher { above } else { below }
    } else if d_above < d_below {
        above
    } else {
        below
    }
}

/// Stable-equilibrium travel envelope anchored to the stator copper region.
///
/// Glossary-normative spec (2026-09-02, kata xb16; nearest-snap revision
/// same day after field verification): the Travel Envelope is "the span of
/// valid mover positions between the FIRST and LAST STABLE REST POSITION
/// inside the copper active area". The endpoints are derived in two steps:
///
/// 1. **Span-aware centre clamp** — the array centre must keep the whole
///    mover inside copper: `centre ∈ [copper_start + span/2, copper_end −
///    span/2]` with the glossary "Mover Span" `span = N · τ_p`
///    (τ_p = P_e/2). By construction this range has the width of the
///    configured free travel (`travel = copper_length − span`, glossary
///    "Travel Envelope") — the range the slider MUST sweep.
/// 2. **Nearest-rest lattice snapping** — endpoints must be STABLE rest
///    positions, i.e. on the track-frame lattice
///    `x ≡ φ_track (mod P_e)` with
///    `φ_track = (copper_region_start + φ) mod P_e` (φ the baseline rest
///    phase, see [`rest_phase_m`]). Each endpoint snaps to the lattice
///    point NEAREST its clamp bound (ties resolve inward, see
///    [`lattice_nearest`]), deviating by at most `P_e/2` per endpoint.
///    Inward snapping (first rest ≥ lower bound, last rest ≤ upper bound)
///    was tried first and rejected: it can cut up to `2·P_e` from the
///    swept range — 36% of the configured travel at the app defaults —
///    leaving the mover unable to reach the copper ends. With nearest
///    snapping the swept range stays within `P_e` of the configured
    ///    travel, and an endpoint may sit up to `P_e/2` OUTSIDE its clamp
    ///    bound: the array edge then overhangs the copper end by at most
    ///    `P_e/2` — the out-hanging magnets see no conductors and simply
    ///    contribute no force (there is no end padding; kata hrd8 removed
    ///    it — the copper active area is the whole track).
    ///
    /// Degenerate behavior (copper region shorter than the mover span, or a
    /// clamped range narrower than one lattice step): `max` is clamped to
    /// `min`, so the envelope never inverts (max ≥ min). The array necessarily
    /// overhangs the copper at that single rest position.
    ///
    /// Worked default example (P_e = 12 mm, N = 12, copper region [0, 147] mm
    /// in track coords): φ = (P_e/12 + (11/2)·τ_p) mod P_e = 10 mm, so
    /// φ_track = (0 + 10) mod 12 = 10 mm; span = 12·6 mm = 72 mm gives a
    /// centre range [36, 111] mm; the lattice {…, 34, 46, 58, …} mm snaps the
    /// endpoints to **min = 34 mm, max = 106 mm** — a 72 mm sweep against the
    /// 75 mm configured travel (the inward snap gave 46 → 106 mm, only 60 mm).
    /// Unlike the pre-xb16 coil-capture convention, the endpoints DEPEND on N
    /// (N = 4 gives 10 → 130 mm on the same copper and period).
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
    // Lattice snapping: endpoints must be stable rest positions — the
    // lattice point NEAREST each clamp bound (ties inward), deviating by
    // at most P_e/2 per endpoint. Inward snapping (first rest ≥ lower,
    // last rest ≤ upper) was rejected in field verification: it cuts up
    // to 2·P_e from the swept range and breaks the travel contract
    // travel = copper_length − span (kata xb16, nearest-snap revision).
    let min = lattice_nearest(lower, phase_track, period, true);
    let max = lattice_nearest(upper, phase_track, period, false);
    // Degenerate (copper shorter than the span, or range narrower than
    // one lattice step): never inverted.
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

    /// Defaults (N = 12, P_e = 12 mm, copper [0, 147] mm in track coords),
    /// per the function doc's worked example: φ = 10 mm → φ_track = 10 mm;
    /// span = 12·6 = 72 mm → centre range [36, 111] mm; lattice
    /// {…, 34, 46, 58, …} → **min = 34 mm, max = 106 mm** (nearest snap:
    /// 36 → 34 is 2 mm away vs 46 at 10; 111 → 106 is 5 mm away vs 118 at
    /// 7). Each endpoint deviates from its clamp bound by ≤ P_e/2 = 6 mm:
    /// leading edge 34 − 36 = −2 mm overhangs the copper start by 2 mm
    /// (out-hanging magnets see no conductors), trailing edge 106 + 36 =
    /// 142 mm ≤ 147 mm. Sweep = 72 mm against the 75 mm configured travel.
    #[test]
    fn twelve_pole_defaults_pin_34_to_106_mm() {
        let env = travel_envelope_over_slots(12.0 * MM, 12, 0.0, 147.0 * MM);
        assert!((env.min_position_m - 34.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 106.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 10.0 * MM).abs() < 1e-12);
        assert!((env.electrical_period_m - 12.0 * MM).abs() < 1e-12);
        // Nearest-snap deviation bound: each endpoint sits within P_e/2 of
        // its span-aware clamp bound (−2 ≥ 0 − 6; 142 ≤ 147 + 6).
        let span = 72.0 * MM; // N · τ_p
        let period = 12.0 * MM;
        assert!(env.min_position_m - span / 2.0 >= 0.0 - period / 2.0);
        assert!(env.max_position_m + span / 2.0 <= 147.0 * MM + period / 2.0);
    }

    /// Travel contract (kata xb16 field-verification fix): the swept range
    /// approximates the configured free travel `copper_length − span`
    /// (glossary "Travel Envelope") within one lattice step P_e. The
    /// rejected inward snap lost 15 mm of 75 mm here.
    #[test]
    fn sweep_approximates_configured_travel() {
        for (n, p_e) in [(4_u32, 12.0), (6, 12.0), (12, 12.0), (12, 18.0), (10, 24.0)] {
            let env = travel_envelope_over_slots(p_e * MM, n, 0.0, 147.0 * MM);
            let travel = 147.0 * MM - f64::from(n) * p_e * MM / 2.0;
            let sweep = env.max_position_m - env.min_position_m;
            assert!(
                (sweep - travel).abs() <= p_e * MM,
                "N={n} P_e={p_e}: sweep {sweep:.6} vs travel {travel:.6}"
            );
        }
    }

    /// Real app defaults regression (kata xb16 field report): N = 10,
    /// P_e = 24 mm (τ_p = 12 mm), copper [0, 195] mm → span = 120 mm,
    /// centre clamp [60, 135] mm, φ = (2 + 4.5·12) mod 24 = 56 mm →
    /// φ_track = (0 + 56) mod 24 = 8 mm → lattice {…, 56, 80, …, 104,
    /// 128, 152, …}. Nearest snap: **min = 56 mm** (4 mm from the bound,
    /// vs 80 at 20 mm), **max = 128 mm** (7 mm vs 17 mm) — a 72 mm sweep
    /// against the 75 mm configured travel. The rejected inward snap gave
    /// [80, 128] = 48 mm, cutting 36% of the user's travel (the reported
    /// "slider bounds far too short" bug).
    #[test]
    fn app_defaults_pin_nearest_snap_56_to_128_mm() {
        let env = travel_envelope_over_slots(24.0 * MM, 10, 0.0, 195.0 * MM);
        assert!((env.min_position_m - 56.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 128.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 8.0 * MM).abs() < 1e-12);
    }

    /// Endpoints DEPEND on N (the xb16 fix — pre-xb16 they were fixed
    /// coil-capture offsets). N = 4: φ = 1 + (3/2)·6 = 10 mm → φ_track =
    /// (0 + 10) mod 12 = 10 mm (same lattice as N = 12); span = 24 mm →
    /// centre range [12, 135] mm → **min = 10 mm, max = 130 mm**. With
    /// small N the slider now reaches near the track ends.
    #[test]
    fn four_pole_endpoints_depend_on_magnet_count() {
        let env = travel_envelope_over_slots(12.0 * MM, 4, 0.0, 147.0 * MM);
        assert!((env.min_position_m - 10.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 130.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 10.0 * MM).abs() < 1e-12);
    }

    /// N = 6: φ = 1 + (5/2)·6 = 16 mm → φ_track = (0 + 16) mod 12 =
    /// 4 mm (its own lattice {…, 16, 28, …}); span = 36 mm → centre
    /// range [18, 129] mm → **min = 16 mm, max = 124 mm**. Different
    /// endpoints AND different phase from both N = 4 and N = 12.
    #[test]
    fn six_pole_endpoints_depend_on_magnet_count() {
        let env = travel_envelope_over_slots(12.0 * MM, 6, 0.0, 147.0 * MM);
        assert!((env.min_position_m - 16.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 124.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 4.0 * MM).abs() < 1e-12);
    }

    /// Endpoints scale with P_e through BOTH the span clamp and the lattice
    /// phase (φ itself depends on P_e: x_peak = P_e/12, τ_p = P_e/2).
    /// N = 12, P_e = 18 mm: x_peak = 1.5 mm, φ = (1.5 + 49.5) mod 18 =
    /// 15 mm → φ_track = (0 + 15) mod 18 = 15 mm; span = 108 mm → centre
    /// range [54, 93] mm; lattice {…, 51, 69, …, 87, 105, …} →
    /// **min = 51 mm, max = 87 mm** (nearest snap; the inward snap gave
    /// 69 → 87, only 18 mm of the 39 mm configured travel).
    #[test]
    fn endpoints_scale_with_electrical_period() {
        let env = travel_envelope_over_slots(18.0 * MM, 12, 0.0, 147.0 * MM);
        assert!((env.min_position_m - 51.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - 87.0 * MM).abs() < 1e-12);
        assert!((env.rest_phase_m - 15.0 * MM).abs() < 1e-12);
    }

    /// Degenerate (kata xb16): copper [0, 10] mm is far shorter than the
    /// N = 24 span (144 mm), so the clamped centre range [72, −34] mm is
    /// inverted. min = nearest lattice point to 72 mm on the φ_track =
    /// 10 mm lattice = **70 mm** (2 mm away, vs 82 mm at 10 mm), and max
    /// clamps to min: never inverted, even though the array necessarily
    /// overhangs the copper at that single rest position (documented
    /// degenerate behavior).
    #[test]
    fn narrow_copper_region_clamps_max_to_min() {
        let env = travel_envelope_over_slots(12.0 * MM, 24, 0.0, 10.0 * MM);
        assert!((env.min_position_m - 70.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - env.min_position_m).abs() < 1e-12);
    }

    /// Degenerate variant (kata xb16): the clamped centre range [36, 38.5]
    /// mm (N = 12, copper [0, 74.5] mm) is NON-empty but narrower than
    /// one lattice step (P_e = 12 mm). Both bounds snap to the same
    /// nearest rest — 34 mm (36 is 2 mm from it, 38.5 is 4.5 mm) — so the
    /// envelope collapses to the single rest position **34 mm**. It sits
    /// 2 mm below the lower clamp bound: the array overhangs the copper
    /// start by 2 mm, bounded by P_e/2.
    #[test]
    fn degenerate_range_snaps_both_bounds_to_one_rest() {
        let env = travel_envelope_over_slots(12.0 * MM, 12, 0.0, 74.5 * MM);
        assert!((env.min_position_m - 34.0 * MM).abs() < 1e-12);
        assert!((env.max_position_m - env.min_position_m).abs() < 1e-12);
    }

    /// Exact lattice hit on the UPPER bound: with copper length 166 mm ≡
    /// (φ + span/2) (mod P_e), the upper bound lands exactly on the
    /// φ_track = 10 mm lattice and max must equal it to within a femtometre
    /// — the float guard must not step to a neighbouring lattice point
    /// (bit-exact equality of the subtraction order is not required; the
    /// lattice point and the clamp bound may differ by 1 ulp).
    /// N = 4: span/2 = 12 mm → **min = 10 mm, max = 154 mm** (trailing
    /// edge at exactly 166 mm); N = 12: span/2 = 36 mm → **min = 34 mm,
    /// max = 130 mm**. The LOWER bound can never be an exact hit:
    /// span/2 − φ ≡ P_e/6 (mod P_e) for every N, so min is always one
    /// sixth period above the lattice.
    #[test]
    fn exact_upper_lattice_hit_is_preserved() {
        // N = 4: upper = 166 − 12 = 154 mm = 10 + 12·12 mm.
        let env = travel_envelope_over_slots(12.0 * MM, 4, 0.0, 166.0 * MM);
        assert!((env.max_position_m - (166.0 * MM - 12.0 * MM)).abs() < 1e-15);
        assert!((env.min_position_m - 10.0 * MM).abs() < 1e-12);
        // N = 12: upper = 166 − 36 = 130 mm = 10 + 10·12 mm.
        let env = travel_envelope_over_slots(12.0 * MM, 12, 0.0, 166.0 * MM);
        assert!((env.max_position_m - (166.0 * MM - 36.0 * MM)).abs() < 1e-15);
        assert!((env.min_position_m - 34.0 * MM).abs() < 1e-12);
    }

    #[test]
    fn non_positive_period_zeroes_envelope() {
        let env = travel_envelope_over_slots(0.0, 4, 0.0, 147.0 * MM);
        assert_eq!(env.min_position_m, 0.0);
        assert_eq!(env.max_position_m, 0.0);
    }
}
