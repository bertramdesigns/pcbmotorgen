//! Continuous and burst power analysis.

use serde::{Deserialize, Serialize};

/// Continuous and burst power analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerBudget {
    pub phase_resistance_ohm: f64,
    pub continuous_power_w: f64,
    pub burst_power_w: f64,
    pub temperature_rise_c: f64,
    pub capacitor_required_uf: f64,
    pub efficiency_pct: f64,
}

impl PowerBudget {
    /// Human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "PowerBudget:\n\
             \x20 Phase resistance:  {:.3} Ω\n\
             \x20 Continuous loss:   {:.0} mW\n\
             \x20 Burst loss:        {:.0} mW\n\
             \x20 Temperature rise:  +{:.1} °C\n\
             \x20 Capacitor needed:  {:.0} µF\n\
             \x20 Efficiency (continuous, at rated speed): {:.1} %",
            self.phase_resistance_ohm,
            self.continuous_power_w * 1e3,
            self.burst_power_w * 1e3,
            self.temperature_rise_c,
            self.capacitor_required_uf,
            self.efficiency_pct,
        )
    }
}