//! Explicit vertical stack from PCB bottom to magnet top.

use serde::{Deserialize, Serialize};

/// Explicit vertical stack from PCB bottom to magnet top.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightStackResult {
    pub pcb_thickness_m: f64,
    pub cu_protrusion_m: f64,
    pub solder_mask_m: f64,
    pub air_gap_m: f64,
    pub magnet_height_m: f64,
    pub tolerance_m: f64,
}

impl HeightStackResult {
    /// Total stack height from PCB bottom to magnet top [m].
    pub fn total_height_m(&self) -> f64 {
        self.pcb_thickness_m
            + self.cu_protrusion_m
            + self.solder_mask_m
            + self.air_gap_m
            + self.magnet_height_m
            + self.tolerance_m
    }

    /// True if the total stack fits within `budget_m`.
    pub fn fits_in_budget(&self, budget_m: f64) -> bool {
        self.total_height_m() <= budget_m
    }

    /// Remaining height headroom [m] (negative = over budget).
    pub fn headroom_m(&self, budget_m: f64) -> f64 {
        budget_m - self.total_height_m()
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        let mut lines = vec![
            "HeightStackResult:".to_string(),
            format!("  PCB substrate:    {:.2} mm", self.pcb_thickness_m * 1e3),
            format!("  Cu protrusion:    {:.0} µm", self.cu_protrusion_m * 1e6),
            format!("  Solder mask:      {:.0} µm", self.solder_mask_m * 1e6),
            format!("  Air gap:          {:.2} mm", self.air_gap_m * 1e3),
            format!("  Magnet height:    {:.2} mm", self.magnet_height_m * 1e3),
        ];
        lines.push(format!("  Tolerance:        {:.2} mm", self.tolerance_m * 1e3));
        lines.push("  ─────────────────────────────".to_string());
        lines.push(format!(
            "  Total height:     {:.2} mm",
            self.total_height_m() * 1e3
        ));
        lines.join("\n")
    }
}