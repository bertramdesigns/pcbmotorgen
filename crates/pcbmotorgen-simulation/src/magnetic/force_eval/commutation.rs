//! Commutation mode and the phase-current law.

use serde::{Deserialize, Serialize};

use crate::params::SimulationInput;

/// Commutation strategy for phase current selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommutationMode {
    /// Sinusoidal FOC drive maximizing thrust. Per-coil currents follow
    /// `I_p = I_pk·cos(θ_e − p·π·τ_s/τ_p)` where τ_s is the phase-band pitch —
    /// adjacent coils are spaced one phase-band pitch apart, giving a 60°
    /// electrical offset for the default 3-phase 1:1 winding (the classic 120°
    /// balanced law corresponds to τ_s = 2τ_p/3). Legacy pending the FOC
    /// rewrite (TODO: FOC-rewrite-pcb-motor-expert).
    MaxThrust,
    /// Only Phase A energised at full peak current; B and C zero.
    PhaseAOnly,
}

impl Default for CommutationMode {
    fn default() -> Self {
        Self::MaxThrust
    }
}

/// Return the signed phase currents [A] for the given mover position.
///
/// Free function so it can be called from parallel closures without borrowing
/// `self` (which would require `&self` to be `Sync` — it is, but extracting
/// the logic avoids the borrow entirely).
///
/// // TODO: FOC-rewrite-pcb-motor-expert
/// This is the **legacy** FOC law (cos-based with phase-band-pitch offset). The
/// rewrite spec from the `@pcb-motor-expert` agent will replace it with a
/// closed-form version that handles Vernier spacing ratios, phase-loss
/// tolerance, and a 90°-corrected electrical angle. This function remains the
/// live implementation until the rewrite lands (the old `foc_spec` stub was
/// removed in the Round 11 crate restructure).
pub(crate) fn commutation_currents(
    commutation: CommutationMode,
    phase_shift: f64,
    config: &SimulationInput,
    mover_position_m: f64,
    n_phases: usize,
) -> Vec<f64> {
    let i_pk = config.max_current_a;

    if commutation == CommutationMode::PhaseAOnly {
        let mut currents = vec![0.0; n_phases];
        currents[0] = i_pk;
        return currents;
    }

    // MaxThrust: sinusoidal FOC
    // θ_e = 2π · p / (2τ) + phase_shift
    // Per-coil offset = π · phase_band_pitch / pole_pitch (electrical offset
    // between adjacent coils; matches the actual winding geometry in
    // wave_winding.rs).
    //
    // Note: uses `cos` (not `sin`) because the B_z field of an alternating
    // magnet array peaks at the magnet centre (x = p + kτ), not at the
    // pole boundary.  `B_z(x, p) ∝ cos(π(x-p)/τ)`, so the optimal
    // current for max thrust is `I ∝ cos(θ_e)`, not `sin(θ_e)`. The 90°
    // offset between sin and cos is NOT fixed by the self-calibration
    // guard (which only flips 0° ↔ 180°).
    let theta_e =
        2.0 * std::f64::consts::PI * mover_position_m / (2.0 * config.pole_pitch_m())
            + phase_shift;
    let phase_offset = std::f64::consts::PI * config.phase_band_pitch_m() / config.pole_pitch_m();

    (0..n_phases)
        .map(|p| i_pk * (theta_e - p as f64 * phase_offset).cos())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_a_only_commutation() {
        let currents = commutation_currents(
            CommutationMode::PhaseAOnly,
            0.0,
            &SimulationInput::default(),
            0.0,
            3,
        );
        assert!((currents[0] - 1.0).abs() < 1e-9);
        assert!((currents[1] - 0.0).abs() < 1e-9);
        assert!((currents[2] - 0.0).abs() < 1e-9);
    }

    /// Pin the serde wire names (`rename_all = "snake_case"`): the coordinated
    /// rename changed the wire value from "max_torque" to "max_thrust".
    #[test]
    fn test_commutation_wire_names() {
        assert_eq!(
            serde_json::to_string(&CommutationMode::MaxThrust).unwrap(),
            "\"max_thrust\""
        );
        assert_eq!(
            serde_json::to_string(&CommutationMode::PhaseAOnly).unwrap(),
            "\"phase_a_only\""
        );
    }

    #[test]
    fn test_max_thrust_commutation_at_zero() {
        // At p=0 with phase_shift=0: θ_e=0
        // With cos-FOC and phase-band-pitch offset (π/3):
        //   I_A = cos(0)         =  1
        //   I_B = cos(-π/3)      =  0.5
        //   I_C = cos(-2π/3)     = -0.5
        //   sum = 1.0 (NOT zero — coils are 60° apart, not 120°)
        let currents = commutation_currents(
            CommutationMode::MaxThrust,
            0.0,
            &SimulationInput::default(),
            0.0,
            3,
        );
        assert!((currents[0] - 1.0).abs() < 1e-9, "I_A should be ~1, got {}", currents[0]);
        assert!((currents[1] - 0.5).abs() < 1e-6, "I_B = {}, expected 0.5", currents[1]);
        assert!((currents[2] - (-0.5)).abs() < 1e-6, "I_C = {}, expected -0.5", currents[2]);
        // Sum is +1.0 (correct for the cos-FOC with phase-band-pitch offset)
        let sum: f64 = currents.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "3-phase sum should be 1.0, got {sum}");
    }

    #[test]
    fn test_max_thrust_commutation_at_quarter_pitch() {
        // At p=τ/2=0.006 with phase_shift=0: θ_e=π/2
        // With cos-FOC and phase-band-pitch offset (π/3):
        //   I_A = cos(π/2)                =  0
        //   I_B = cos(π/2 - π/3) = cos(π/6)  =  √3/2
        //   I_C = cos(π/2 - 2π/3) = cos(-π/6) =  √3/2
        let currents = commutation_currents(
            CommutationMode::MaxThrust,
            0.0,
            &SimulationInput::default(),
            0.006,
            3,
        );
        assert!(currents[0].abs() < 1e-6, "I_A = {}, expected ~0", currents[0]);
        let s3_2 = 3.0_f64.sqrt() / 2.0;
        assert!((currents[1] - s3_2).abs() < 1e-6, "I_B = {}, expected √3/2", currents[1]);
        assert!((currents[2] - s3_2).abs() < 1e-6, "I_C = {}, expected √3/2", currents[2]);
    }

    /// Verify the FOC phase offset uses phase_band_pitch (not 2π/n_phases).
    ///
    /// At p=0 with phase_shift=0: θ_e=0
    /// For the default config: phase_band_pitch = pole_pitch/3, so the per-coil
    /// offset is π/3 (60°).  The 3 phase currents must be 60° apart in
    /// current-space (NOT 120° — the coils themselves are 60° apart).
    ///
    /// With the corrected offset and `cos` FOC (the actual max-thrust law
    /// for `B_z ∝ cos(π(x-p)/τ)`):
    ///   I_A = cos(0)              =  1
    ///   I_B = cos(-π/3)            =  0.5
    ///   I_C = cos(-2π/3)           = -0.5
    ///   sum = 1.0  (NOT zero — the coils are not 120° apart in current)
    ///
    /// This asymmetry is correct: the FOC drives each coil at the
    /// electrical angle of its *position*, not at a uniform 120° split.
    /// The 3-phase ripple is minimised by the actual coil-to-B-field
    /// alignment.
    #[test]
    fn test_max_thrust_commutation_uses_phase_band_pitch_offset() {
        // Default config: 3 phases, 1:1 spacing → phase_offset = π/3 (60°)
        let currents = commutation_currents(
            CommutationMode::MaxThrust,
            0.0,
            &SimulationInput::default(),
            0.0,
            3,
        );
        // I_A = cos(0) = 1
        assert!((currents[0] - 1.0).abs() < 1e-9, "I_A should be ~1, got {}", currents[0]);
        // I_B = cos(-π/3) = 0.5
        assert!((currents[1] - 0.5).abs() < 1e-6, "I_B = {}, expected 0.5", currents[1]);
        // I_C = cos(-2π/3) = -0.5
        assert!((currents[2] - (-0.5)).abs() < 1e-6, "I_C = {}, expected -0.5", currents[2]);
        // Sum is +1.0 (not zero — this is correct for the phase-band-pitch offset):
        let sum: f64 = currents.iter().sum();
        let expected_sum = 1.0;
        assert!(
            (sum - expected_sum).abs() < 1e-6,
            "3-phase sum should be +1.0 (coils 60° apart, balanced cos FOC), got {sum}"
        );
    }

    /// At p=τ/2=0.006 (i.e. θ_e=π/2):
    /// With the corrected offset (π/3) and cos FOC:
    ///   I_A = cos(π/2)               =  0
    ///   I_B = cos(π/2 - π/3) = cos(π/6)  =  √3/2
    ///   I_C = cos(π/2 - 2π/3) = cos(-π/6) =  √3/2
    #[test]
    fn test_max_thrust_commutation_quarter_pitch_phase_band_offset() {
        let currents = commutation_currents(
            CommutationMode::MaxThrust,
            0.0,
            &SimulationInput::default(),
            0.006,
            3,
        );
        assert!(currents[0].abs() < 1e-6, "I_A should be ~0, got {}", currents[0]);
        let s3_2 = 3.0_f64.sqrt() / 2.0;
        assert!((currents[1] - s3_2).abs() < 1e-6, "I_B = {}, expected √3/2", currents[1]);
        assert!((currents[2] - s3_2).abs() < 1e-6, "I_C = {}, expected √3/2", currents[2]);
    }

    /// Verify the 4:5 Vernier offset: spacing_ratio=0.8 → phase_offset = 0.8·π/3.
    /// At p=0 with phase_shift=0: θ_e=0
    ///   I_A = cos(0)               =  1
    ///   I_B = cos(-0.8π/3)
    ///   I_C = cos(-1.6π/3)
    #[test]
    fn test_max_thrust_commutation_vernier_offset() {
        let cfg = SimulationInput {
            spacing_ratio: 0.8,
            ..SimulationInput::default()
        };
        let currents = commutation_currents(
            CommutationMode::MaxThrust,
            0.0,
            &cfg,
            0.0,
            3,
        );
        let offset = 0.8 * std::f64::consts::PI / 3.0;
        assert!((currents[0] - 1.0).abs() < 1e-9, "I_A should be ~1, got {}", currents[0]);
        assert!(
            (currents[1] - (-offset).cos()).abs() < 1e-6,
            "I_B = {}, expected cos(-0.8π/3) = {}",
            currents[1],
            (-offset).cos()
        );
        assert!(
            (currents[2] - (-2.0 * offset).cos()).abs() < 1e-6,
            "I_C = {}, expected cos(-1.6π/3) = {}",
            currents[2],
            (-2.0 * offset).cos()
        );
    }
}