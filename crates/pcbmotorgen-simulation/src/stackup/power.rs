//! Power / thermal estimator.

use crate::params::{PowerBudget, SimulationInput, StackupResult};
use crate::units::{cu_resistance_per_length, oz_to_m, RHO_CU};

/// PCB thermal resistance [°C/W] — 20 mm × 200 mm board, natural convection.
const R_THERMAL_C_PER_W: f64 = 15.0;
/// Burst duration for capacitor sizing [s].
const T_BURST_S: f64 = 0.1;
/// Max acceptable voltage droop during burst (fraction of supply).
const DROOP_FRACTION: f64 = 0.10;
/// Approximate rated velocity for efficiency [m/s].
const V_RATED_M_S: f64 = 0.10;

/// Estimate drive-circuit power budget.
#[derive(Debug, Clone)]
pub struct PowerEstimator {
    /// How many copper layers carry each phase.
    pub layers_per_phase: Option<u32>,
}

impl Default for PowerEstimator {
    fn default() -> Self {
        Self { layers_per_phase: None }
    }
}

impl PowerEstimator {
    /// Compute a PowerBudget.
    pub fn estimate(
        &self,
        config: &SimulationInput,
        stackup: Option<&StackupResult>,
    ) -> PowerBudget {
        let (trace_width_m, cu_thickness_m, lpp) = self.trace_params(config, stackup);

        let single_layer_length_m = Self::single_layer_trace_length_m(config);
        let total_length_m = single_layer_length_m * lpp as f64;

        let r_per_m = cu_resistance_per_length(trace_width_m, cu_thickness_m, RHO_CU);
        let r_phase = r_per_m * total_length_m;

        // CONTINUOUS (thermal) chain — RMS-referenced.
        //
        // Per the glossary ("Thermal Resistance & Power Dissipation"):
        // P_loss = I_rms² · R_phase with I_rms = I_peak/√2 for sinusoidal
        // drive. `max_current_a` is the PEAK phase current, so continuous
        // loss (and the ΔT it feeds) uses the RMS value. (The old
        // peak-referenced model overestimated continuous loss and ΔT by 2×.)
        let i_rms = config.max_current_a / std::f64::consts::SQRT_2;
        let p_cont = config.phases as f64 * i_rms * i_rms * r_phase;

        // BURST (capacitor sizing) chain — PEAK-referenced.
        //
        // The burst is a short transient (~T_BURST_S) at full instantaneous
        // current, so it must use the PEAK current (not I_rms); this keeps
        // the burst loss and the bulk-capacitor sizing physically correct.
        // `i_burst` derives from `config.max_current_a` (peak), scaled by the
        // burst/continuous force ratio.
        let i_burst = if config.target_force_n > 0.0 {
            config.max_current_a * (config.peak_force_n / config.target_force_n)
        } else {
            config.max_current_a
        };
        let p_burst = config.phases as f64 * i_burst * i_burst * r_phase;

        let delta_t = p_cont * R_THERMAL_C_PER_W;

        let delta_v = config.supply_voltage_v * DROOP_FRACTION;
        let c_uf = if delta_v > 0.0 {
            ((i_burst * config.phases as f64 * T_BURST_S / delta_v) * 1e6).max(0.0)
        } else {
            0.0
        };

        let p_mech = config.target_force_n * V_RATED_M_S;
        // Efficiency chain at the continuous (rated) operating point uses
        // the RMS current — same sinusoidal-average reference as p_cont.
        let p_elec = config.supply_voltage_v * i_rms;
        let efficiency = if p_elec > 0.0 {
            (p_mech / p_elec * 100.0).min(100.0)
        } else {
            0.0
        };

        PowerBudget {
            phase_resistance_ohm: r_phase,
            continuous_power_w: p_cont,
            burst_power_w: p_burst,
            temperature_rise_c: delta_t,
            capacitor_required_uf: c_uf,
            efficiency_pct: efficiency,
        }
    }

    fn trace_params(
        &self,
        config: &SimulationInput,
        stackup: Option<&StackupResult>,
    ) -> (f64, f64, u32) {
        let lpp = if let Some(l) = self.layers_per_phase {
            l
        } else if let Some(s) = stackup {
            ((s.layer_count / config.phases) as u32).max(1)
        } else {
            2
        };

        if let Some(s) = stackup {
            let idx = (1).min(s.trace_widths_m.len() - 1);
            let tw = s.trace_widths_m[idx];
            let ct = s.cu_thickness_m[idx];
            (tw, ct, lpp)
        } else {
            (config.min_trace_m * 2.0, oz_to_m(2.0), lpp)
        }
    }

    /// Approximate total trace length per phase per layer [m].
    ///
    /// The old overloaded geometry generator is gone (geometry now lives in
    /// the sibling `pcbmotorgen-routing` crate). This is a conservative
    /// analytical estimate: each conductor spans 2× the board width (out and
    /// back serpentine) and there are roughly `magnet_count` conductors per
    /// path.
    fn single_layer_trace_length_m(config: &SimulationInput) -> f64 {
        // 2 board-widths per conductor, ~magnet_count*2 conductors.
        let n_conductors = (config.magnet_count * 2) as f64;
        2.0 * config.board_width_m * n_conductors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::StackupResult;
    use crate::units::{mm, oz_to_m};

    fn default_config() -> SimulationInput {
        SimulationInput {
            active_area_length_m: mm(195.0),
            magnet_dims_m: [mm(10.0), mm(10.0), mm(4.0)],
            magnet_count: 10,
            magnet_pitch_m: mm(12.0),
            phases: 3,
            target_force_n: 0.5,
            peak_force_n: 1.0,
            max_current_a: 1.0,
            supply_voltage_v: 5.0,
            min_trace_m: mm(0.127),
            min_space_m: mm(0.127),
            board_width_m: mm(20.0),
            air_gap_m: mm(0.5),
            ..SimulationInput::default()
        }
    }

    fn four_layer_stackup() -> StackupResult {
        StackupResult {
            layer_count: 4,
            trace_widths_m: vec![mm(0.15), mm(0.25), mm(0.25), mm(0.15)],
            cu_thickness_m: vec![oz_to_m(1.0), oz_to_m(2.0), oz_to_m(2.0), oz_to_m(1.0)],
            via_drill_m: mm(0.2),
            via_annular_ring_m: mm(0.1),
            via_grid_rows: 2,
            via_grid_cols: 3,
            estimated_force_n: 0.42,
            estimated_dc_resistance_ohm: 3.1,
            notes: vec![],
        }
    }

    #[test]
    fn test_estimate_without_stackup() {
        let cfg = default_config();
        let est = PowerEstimator::default();
        let pb = est.estimate(&cfg, None);
        assert!(pb.phase_resistance_ohm > 0.0);
        assert!(pb.continuous_power_w > 0.0);
        assert!(pb.burst_power_w >= pb.continuous_power_w);
        assert!(pb.temperature_rise_c > 0.0);
        assert!(pb.efficiency_pct >= 0.0 && pb.efficiency_pct <= 100.0);
    }

    #[test]
    fn test_estimate_with_stackup() {
        let cfg = default_config();
        let s = four_layer_stackup();
        let est = PowerEstimator::default();
        let pb = est.estimate(&cfg, Some(&s));
        assert!(pb.phase_resistance_ohm > 0.0);
    }

    #[test]
    fn test_burst_higher_than_continuous() {
        let cfg = default_config();
        let est = PowerEstimator::default();
        let pb = est.estimate(&cfg, None);
        // peak_force = 1.0, target = 0.5 → I_burst = 2×I_peak → P_burst > P_cont
        assert!(pb.burst_power_w > pb.continuous_power_w);
    }

    /// Quantitative vector for the RMS continuous-power model (kata e23n).
    ///
    /// With `default_config()` (max_current_a = 1.0 A PEAK, phases = 3):
    /// - continuous loss is RMS-referenced: P_cont = phases·(I_peak/√2)²·R,
    ///   i.e. exactly HALF of the old peak-referenced phases·1²·R.
    /// - burst loss stays PEAK-referenced: i_burst = 1.0·(1.0/0.5) = 2 A peak,
    ///   so P_burst = 3·4·R while P_cont = 3·0.5·R → ratio 8 (not 4).
    /// - ΔT = P_cont · R_thermal (15 °C/W).
    #[test]
    fn test_rms_continuous_power_quantitative() {
        let cfg = default_config();
        let est = PowerEstimator::default();
        let pb = est.estimate(&cfg, None);
        let eps = 1e-12;

        let r = pb.phase_resistance_ohm;
        let i_rms = cfg.max_current_a / std::f64::consts::SQRT_2;

        // P_cont = phases · I_rms² · R_phase — exactly half the peak-referenced value.
        let expected_cont = cfg.phases as f64 * i_rms * i_rms * r;
        assert!(
            (pb.continuous_power_w - expected_cont).abs() < eps,
            "continuous_power_w {} != expected {expected_cont}",
            pb.continuous_power_w
        );
        let peak_referenced = cfg.phases as f64 * 1.0 * 1.0 * r;
        assert!(
            (pb.continuous_power_w * 2.0 - peak_referenced).abs() < eps,
            "P_cont must be exactly half of the peak-referenced value"
        );

        // P_burst stays peak-referenced: i_burst = I_peak·(peak_force/target_force).
        let i_burst = cfg.max_current_a * (cfg.peak_force_n / cfg.target_force_n);
        let expected_burst = cfg.phases as f64 * i_burst * i_burst * r;
        assert!(
            (pb.burst_power_w - expected_burst).abs() < eps,
            "burst_power_w {} != expected {expected_burst}",
            pb.burst_power_w
        );

        // Burst/continuous ratio reflects the peak reference: (2²)/( (1/√2)² ) = 8, not 4.
        let ratio = pb.burst_power_w / pb.continuous_power_w;
        assert!(
            (ratio - 8.0).abs() < 1e-9,
            "burst/continuous ratio {ratio} must be 8 (peak-referenced burst), not 4"
        );

        // ΔT = P_cont · R_thermal_c_per_w.
        assert!(
            (pb.temperature_rise_c - pb.continuous_power_w * 15.0).abs() < eps,
            "temperature_rise_c {} != P_cont·15.0 = {}",
            pb.temperature_rise_c,
            pb.continuous_power_w * 15.0
        );
    }

    #[test]
    fn test_capacitor_positive() {
        let cfg = default_config();
        let est = PowerEstimator::default();
        let pb = est.estimate(&cfg, None);
        assert!(pb.capacitor_required_uf > 0.0);
    }

    #[test]
    fn test_custom_layers_per_phase() {
        let cfg = default_config();
        let est = PowerEstimator { layers_per_phase: Some(1) };
        let pb_default = PowerEstimator::default().estimate(&cfg, None);
        let pb_custom = est.estimate(&cfg, None);
        // With 1 layer per phase vs 2, resistance should be 2× and power 2×
        assert!(pb_custom.phase_resistance_ohm < pb_default.phase_resistance_ohm);
    }
}
