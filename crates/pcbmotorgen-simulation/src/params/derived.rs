//! Derived-geometry accessors on [`SimulationInput`].

use super::{SAFETY_MARGIN, SimulationInput};

impl SimulationInput {
    // --- Derived geometry ---

    /// Span of the mover magnet array [m]: `magnet_count × magnet_pitch`
    /// (center-of-first-magnet to one pitch beyond the last; the physical
    /// end-to-end span is one inter-magnet gap shorter).
    pub fn magnet_array_span_m(&self) -> f64 {
        self.magnet_count as f64 * self.magnet_pitch_m
    }

    /// Derived center-to-center travel [m]: `active_area_length − magnet array span`.
    pub fn travel_m(&self) -> f64 {
        self.active_area_length_m - self.magnet_array_span_m()
    }

    /// Minimum PCB length required [m] (= active_area_length_m).
    pub fn active_length_m(&self) -> f64 {
        self.active_area_length_m
    }

    /// Magnet pole pitch [m] (= magnet_pitch for alternating arrays).
    pub fn pole_pitch_m(&self) -> f64 {
        self.magnet_pitch_m
    }

    /// Vernier-adjusted phase-band pitch [m]: `(pole_pitch / phases) × spacing_ratio`.
    /// With `spacing_ratio = 1.0` this is the ideal phase-band pitch
    /// `τ_p/phases`; the glossary's slot pitch `τ_s = L_stator/N_slots`
    /// coincides only for uniform 1-slot-per-pole-per-phase windings.
    pub fn phase_band_pitch_m(&self) -> f64 {
        (self.pole_pitch_m() / self.phases as f64) * self.spacing_ratio
    }

    /// Vernier rest offset: phase offset between a coil center and the
    /// nearest pole center [m]. Clamped to `[0, pole_pitch]`.
    pub fn rest_offset_m(&self) -> f64 {
        ((self.pole_pitch_m() / self.phases as f64) * (1.0 - self.spacing_ratio))
            .clamp(0.0, self.pole_pitch_m())
    }

    /// Inter-magnet gap along the travel axis [m]: `magnet_pitch − magnet_width`.
    /// Distinct from the motor air gap `air_gap_m` (stator-to-mover clearance).
    pub fn magnet_gap_m(&self) -> f64 {
        self.magnet_pitch_m - self.magnet_dims_m[0]
    }

    /// Minimum via pad diameter [m] = drill + 2 × annular ring.
    pub fn min_via_pad_m(&self) -> f64 {
        self.min_via_drill_m + 2.0 * self.min_via_annular_ring_m
    }

    /// Peak inertial force [N] = `carriage_mass × max_accel`.
    pub fn acceleration_force_n(&self) -> f64 {
        self.carriage_mass_kg * self.max_accel_m_s2
    }

    /// Minimum motor force to overcome friction with safety margin [N].
    pub fn minimum_drive_force_n(&self) -> f64 {
        self.friction_n * SAFETY_MARGIN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> SimulationInput {
        SimulationInput::default()
    }

    #[test]
    fn test_derived_values() {
        let cfg = default_config();
        assert!((cfg.magnet_array_span_m() - 120e-3).abs() < 1e-12);
        assert!((cfg.travel_m() - 75e-3).abs() < 1e-12);
        assert!((cfg.phase_band_pitch_m() - 4e-3).abs() < 1e-12);
        assert_eq!(cfg.rest_offset_m(), 0.0);
        assert!((cfg.magnet_gap_m() - 2e-3).abs() < 1e-12);
        assert!((cfg.min_via_pad_m() - 0.4e-3).abs() < 1e-12);
        assert!((cfg.acceleration_force_n() - 0.03).abs() < 1e-12);
        assert!((cfg.minimum_drive_force_n() - 0.065).abs() < 1e-12);
    }
}