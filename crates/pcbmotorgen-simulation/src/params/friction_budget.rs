//! Breakdown of mechanical friction contributors.

use serde::{Deserialize, Serialize};

use super::SAFETY_MARGIN;

/// Breakdown of mechanical friction contributors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrictionBudget {
    pub bearing_friction_n: f64,
    pub cable_drag_n: f64,
    #[serde(default)]
    pub wiper_contact_n: f64,
    /// Conservative friction-budget placeholder; physical detent (cogging)
    /// force is zero for coreless/slotless topologies (glossary). The app
    /// path overrides cogging_n to 0.
    #[serde(default)]
    pub cogging_n: f64,
}

impl FrictionBudget {
    /// Total friction force [N].
    pub fn total_n(&self) -> f64 {
        self.bearing_friction_n + self.cable_drag_n + self.wiper_contact_n + self.cogging_n
    }

    /// Minimum motor force to start motion with 1.3× safety margin [N].
    pub fn minimum_drive_force_n(&self) -> f64 {
        self.total_n() * SAFETY_MARGIN
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "FrictionBudget:\n\
             \x20 Bearing friction: {:.1} mN\n\
             \x20 Cable drag:       {:.1} mN\n\
             \x20 Wiper contact:    {:.1} mN\n\
             \x20 Cogging:          {:.1} mN\n\
             \x20 ─────────────────────────────\n\
             \x20 Total:            {:.1} mN\n\
             \x20 Min drive force:  {:.1} mN  (×{:.1} margin)",
            self.bearing_friction_n * 1e3,
            self.cable_drag_n * 1e3,
            self.wiper_contact_n * 1e3,
            self.cogging_n * 1e3,
            self.total_n() * 1e3,
            self.minimum_drive_force_n() * 1e3,
            SAFETY_MARGIN,
        )
    }
}