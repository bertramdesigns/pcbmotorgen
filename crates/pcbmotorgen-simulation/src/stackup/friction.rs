//! Friction estimator.

use crate::params::{BearingType, FrictionBudget, SimulationInput};

/// Friction coefficient (µ) for each bearing type.
pub fn mu_bearing(bt: BearingType) -> f64 {
    match bt {
        BearingType::PlasticChannel => 0.25,
        BearingType::PtfeLined => 0.12,
        BearingType::BallBearing => 0.003,
    }
}

/// Empirical FFC drag per conductor [N].
const FFC_DRAG_PER_CONDUCTOR_N: f64 = 0.020;
/// Wiper contact spring force [N].
const WIPER_CONTACT_N: f64 = 0.055;

/// Estimate mover mechanical friction.
#[derive(Debug, Clone)]
pub struct FrictionEstimator {
    pub bearing_type: BearingType,
    pub ffc_conductor_count: u32,
    pub has_wiper_contact: bool,
    pub normal_force_n: f64,
    pub cogging_n: f64,
}

impl Default for FrictionEstimator {
    fn default() -> Self {
        Self {
            bearing_type: BearingType::PtfeLined,
            ffc_conductor_count: 26,
            has_wiper_contact: false,
            normal_force_n: 0.0,
            cogging_n: 0.0,
        }
    }
}

impl FrictionEstimator {
    /// Compute a FrictionBudget.
    pub fn estimate(&self) -> FrictionBudget {
        let bearing_n = mu_bearing(self.bearing_type) * self.normal_force_n;
        let cable_n = FFC_DRAG_PER_CONDUCTOR_N * self.ffc_conductor_count as f64;
        let wiper_n = if self.has_wiper_contact { WIPER_CONTACT_N } else { 0.0 };
        FrictionBudget {
            bearing_friction_n: bearing_n,
            cable_drag_n: cable_n,
            wiper_contact_n: wiper_n,
            cogging_n: self.cogging_n,
        }
    }

    /// Estimate friction using `config.friction_n` as the total, split proportionally.
    pub fn estimate_for_config(&self, config: &SimulationInput) -> FrictionBudget {
        let total = config.friction_n;
        if total <= 0.0 {
            return FrictionBudget {
                bearing_friction_n: 0.0,
                cable_drag_n: 0.0,
                wiper_contact_n: 0.0,
                cogging_n: 0.0,
            };
        }

        let bearing_fraction = match self.bearing_type {
            BearingType::PlasticChannel => 0.70,
            BearingType::PtfeLined => 0.55,
            BearingType::BallBearing => 0.25,
        };
        let cable_fraction = 0.20;
        let wiper_fraction = if self.has_wiper_contact { 0.10 } else { 0.0 };
        // No cogging allocation: physical detent (cogging) force is zero for
        // coreless/slotless topologies (glossary), the only topology this
        // crate models. Callers that need a nonzero detent component can set
        // `FrictionEstimator::cogging_n` and use `estimate()` instead.
        let total_fraction = bearing_fraction + cable_fraction + wiper_fraction;
        let sf = total / total_fraction;

        FrictionBudget {
            bearing_friction_n: bearing_fraction * sf,
            cable_drag_n: cable_fraction * sf,
            wiper_contact_n: wiper_fraction * sf,
            cogging_n: 0.0,
        }
    }

    /// Construct an estimator with a default zero normal force (no back-iron
    /// pull-in — the magnetic keeper was removed from the product scope).
    pub fn from_config(
        bearing_type: BearingType,
        ffc_conductor_count: u32,
        has_wiper_contact: bool,
    ) -> Self {
        Self {
            bearing_type,
            ffc_conductor_count,
            has_wiper_contact,
            normal_force_n: 0.0,
            cogging_n: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::mm;

    fn default_config() -> SimulationInput {
        SimulationInput {
            active_area_length_m: mm(195.0),
            magnet_dims_m: [mm(10.0), mm(10.0), mm(4.0)],
            magnet_count: 10,
            magnet_pitch_m: mm(12.0),
            phases: 3,
            board_width_m: mm(20.0),
            air_gap_m: mm(0.5),
            ..SimulationInput::default()
        }
    }

    #[test]
    fn test_mu_bearing_values() {
        assert_eq!(mu_bearing(BearingType::PlasticChannel), 0.25);
        assert_eq!(mu_bearing(BearingType::PtfeLined), 0.12);
        assert_eq!(mu_bearing(BearingType::BallBearing), 0.003);
    }

    #[test]
    fn test_estimate_ball_bearing() {
        let est = FrictionEstimator {
            bearing_type: BearingType::BallBearing,
            ffc_conductor_count: 26,
            has_wiper_contact: false,
            normal_force_n: 0.0,
            cogging_n: 0.0,
        };
        let fb = est.estimate();
        // Cable drag = 26 * 0.020 = 0.52 N
        assert!((fb.cable_drag_n - 0.52).abs() < 1e-12);
        assert!((fb.bearing_friction_n - 0.0).abs() < 1e-12);
    }

    #[test]
    fn test_estimate_with_wiper() {
        let est = FrictionEstimator {
            bearing_type: BearingType::PtfeLined,
            ffc_conductor_count: 0,
            has_wiper_contact: true,
            normal_force_n: 0.0,
            cogging_n: 0.0,
        };
        let fb = est.estimate();
        assert!((fb.wiper_contact_n - 0.055).abs() < 1e-12);
    }

    #[test]
    fn test_estimate_total() {
        let est = FrictionEstimator {
            bearing_type: BearingType::PlasticChannel,
            ffc_conductor_count: 26,
            has_wiper_contact: true,
            normal_force_n: 10.0,
            cogging_n: 0.0,
        };
        let fb = est.estimate();
        // bearing = 0.25 * 10 = 2.5, cable = 0.52, wiper = 0.055, cogging = 0
        let expected = 2.5 + 0.52 + 0.055;
        assert!((fb.total_n() - expected).abs() < 1e-12);
    }

    #[test]
    fn test_estimate_for_config_zero() {
        let mut cfg = default_config();
        cfg.friction_n = 0.0;
        let est = FrictionEstimator::default();
        let fb = est.estimate_for_config(&cfg);
        assert!((fb.total_n() - 0.0).abs() < 1e-12);
    }

    /// kata zt1r: the 5% cogging allocation is removed — the split covers
    /// bearing + cable + wiper exactly and cogging is always zero
    /// (detent force is zero for coreless topologies, glossary).
    #[test]
    fn test_estimate_for_config_no_cogging_allocation() {
        let mut cfg = default_config();
        cfg.friction_n = 1.0;
        let est = FrictionEstimator::default(); // PTFE-lined, no wiper
        let fb = est.estimate_for_config(&cfg);
        assert!((fb.cogging_n - 0.0).abs() < 1e-12);
        // PTFE: 0.55 bearing / 0.20 cable split over the 0.75 total fraction.
        assert!((fb.bearing_friction_n - 0.55 / 0.75).abs() < 1e-12);
        assert!((fb.cable_drag_n - 0.20 / 0.75).abs() < 1e-12);
        assert!((fb.total_n() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_from_config_no_attractive_normal_force() {
        let est = FrictionEstimator::from_config(BearingType::BallBearing, 26, false);
        assert!((est.normal_force_n - 0.0).abs() < 1e-12);
    }
}
