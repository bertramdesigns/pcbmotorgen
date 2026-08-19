//! Computed PCB stackup recommendation.

use serde::{Deserialize, Serialize};

use super::SimulationError;

/// Computed PCB stackup recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackupResult {
    pub layer_count: u32,
    pub trace_widths_m: Vec<f64>,
    pub cu_thickness_m: Vec<f64>,
    pub via_drill_m: f64,
    pub via_annular_ring_m: f64,
    pub via_grid_rows: u32,
    pub via_grid_cols: u32,
    pub estimated_force_n: f64,
    pub estimated_dc_resistance_ohm: f64,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl StackupResult {
    /// Validate all fields.
    pub fn validate(&self) -> Result<(), SimulationError> {
        if self.layer_count < 2 || self.layer_count % 2 != 0 {
            return Err(SimulationError(format!(
                "layer_count must be even and ≥ 2, got {}",
                self.layer_count
            )));
        }
        if self.trace_widths_m.len() != self.layer_count as usize {
            return Err(SimulationError(format!(
                "trace_widths_m must have {} entries, got {}",
                self.layer_count,
                self.trace_widths_m.len()
            )));
        }
        if self.cu_thickness_m.len() != self.layer_count as usize {
            return Err(SimulationError(format!(
                "cu_thickness_m must have {} entries, got {}",
                self.layer_count,
                self.cu_thickness_m.len()
            )));
        }
        if self.trace_widths_m.iter().any(|&w| w <= 0.0) {
            return Err(SimulationError("All trace widths must be positive".into()));
        }
        if self.cu_thickness_m.iter().any(|&t| t <= 0.0) {
            return Err(SimulationError("All copper thicknesses must be positive".into()));
        }
        if self.via_drill_m <= 0.0 {
            return Err(SimulationError(format!(
                "via_drill_m must be positive, got {}",
                self.via_drill_m
            )));
        }
        if self.via_annular_ring_m <= 0.0 {
            return Err(SimulationError(format!(
                "via_annular_ring_m must be positive, got {}",
                self.via_annular_ring_m
            )));
        }
        if self.via_grid_rows < 1 {
            return Err(SimulationError(format!(
                "via_grid_rows must be ≥ 1, got {}",
                self.via_grid_rows
            )));
        }
        if self.via_grid_cols < 1 {
            return Err(SimulationError(format!(
                "via_grid_cols must be ≥ 1, got {}",
                self.via_grid_cols
            )));
        }
        Ok(())
    }

    /// (0, layer_count - 1)
    pub fn outer_layer_ids(&self) -> (usize, usize) {
        (0, (self.layer_count - 1) as usize)
    }

    /// 1 .. layer_count-1
    pub fn inner_layer_ids(&self) -> Vec<usize> {
        (1..(self.layer_count - 1) as usize).collect()
    }

    /// Via pad diameter = drill + 2 × annular ring.
    pub fn via_pad_m(&self) -> f64 {
        self.via_drill_m + 2.0 * self.via_annular_ring_m
    }

    /// Total number of vias per end-turn.
    pub fn via_grid_count(&self) -> u32 {
        self.via_grid_rows * self.via_grid_cols
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        let mut lines = vec![
            format!("StackupResult: {} layers", self.layer_count),
            format!("  Estimated force:  {:.3} N", self.estimated_force_n),
            format!(
                "  DC resistance:    {:.3} Ω / phase",
                self.estimated_dc_resistance_ohm
            ),
            format!(
                "  Via grid:         {}×{} ({} vias/end-turn)",
                self.via_grid_rows,
                self.via_grid_cols,
                self.via_grid_count()
            ),
            format!(
                "  Via drill/pad:    {:.2} / {:.2} mm",
                self.via_drill_m * 1e3,
                self.via_pad_m() * 1e3
            ),
            "  Layer trace widths and copper weights:".to_string(),
        ];
        let (outer0, outer1) = self.outer_layer_ids();
        for (i, (&w, &t)) in self
            .trace_widths_m
            .iter()
            .zip(self.cu_thickness_m.iter())
            .enumerate()
        {
            let role = if i == outer0 || i == outer1 {
                "outer"
            } else {
                "inner"
            };
            let oz = t / 35e-6;
            lines.push(format!(
                "    L{:>2} ({}): trace={:.3} mm  Cu={:.0} µm (~{:.1} oz)",
                i + 1,
                role,
                w * 1e3,
                t * 1e6,
                oz
            ));
        }
        if !self.notes.is_empty() {
            lines.push("  Notes:".to_string());
            for note in &self.notes {
                lines.push(format!("    • {}", note));
            }
        }
        lines.join("\n")
    }
}