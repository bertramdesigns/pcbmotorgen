//! Method-of-images back-iron copies.

use nalgebra::UnitQuaternion;

use super::super::MagnetArray;
use crate::physics;

/// Empirical correction for finite CRS steel permeability (µ_r ≈ 2 000).
/// Reduces the image-method overestimate by ~15%.
pub(crate) const K_IRON: f64 = 0.85;

impl<'a> MagnetArray<'a> {
    /// Build method-of-images copies for back-iron simulation.
    ///
    /// Each real magnet is mirrored about the steel–magnet interface
    /// (top face of back-iron, not top face of the magnets). The
    /// image-plane `z_mirror` therefore sits at
    /// `z = air_gap + magnet_height + back_iron_thickness` — the
    /// back iron's top surface. The image of a magnet at
    /// `(x, y, orig_z)` sits at `(x, y, 2*z_mirror - orig_z)`.
    /// Image magnets have the same polarisation as the originals,
    /// scaled by `K_IRON = 0.85`.
    ///
    /// Bug 3 fix: the previous implementation used
    /// `z_mirror = air_gap + magnet_height` (the magnet's top surface)
    /// and never read `back_iron_thickness_m`. That made the back-iron
    /// field amplification independent of the back-iron thickness, which
    /// is unphysical: a thicker steel return path concentrates more flux
    /// at the air gap. With the fix, a thicker back iron pushes the
    /// image further from the real magnet (closer to the PCB), which
    /// means the image's field at the PCB surface is **stronger** —
    /// the physically correct sign of the effect.
    ///
    /// When `back_iron_thickness_m = 0` the result is identical to the
    /// pre-fix behaviour (no back iron → no shift of the mirror plane).
    pub(crate) fn build_image_magnets(
        &self,
        real_magnets: &[physics::MagbaCuboidMagnet],
    ) -> Vec<physics::MagbaCuboidMagnet> {
        let cfg = self.config;
        // Mirror plane = top face of the back iron = bottom face of
        // the air gap + magnet height + back iron thickness.
        let z_mirror = cfg.air_gap_m + cfg.magnet_dims_m[2] + cfg.back_iron_thickness_m;

        real_magnets
            .iter()
            .map(|mag| {
                let orig_pos = mag.position();
                let z_image = 2.0 * z_mirror - orig_pos.z;
                // Image polarisation: same as original, scaled by K_IRON
                let orig_pol = mag.polarization();
                let scaled_pol = orig_pol * K_IRON;
                let dim = mag.dimensions();
                physics::make_cuboid_magnet(
                    [orig_pos.x, orig_pos.y, z_image],
                    UnitQuaternion::identity(),
                    [scaled_pol.x, scaled_pol.y, scaled_pol.z],
                    [dim.x, dim.y, dim.z],
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{MagnetArrangement, SimulationInput};

    #[test]
    fn test_back_iron_magnet_count() {
        // Bug 2 fix: the default test_config has back_iron_thickness_m = 0,
        // so `AlternatingBackIron` reduces to plain `Alternating` (10
        // magnets, no image). This test sets a non-zero back iron so the
        // image is added: 10 main + 10 images = 20.
        let mut cfg = SimulationInput::default();
        cfg.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
        cfg.back_iron_thickness_m = 2e-3;
        let arr = MagnetArray::new(&cfg);
        let assembly = arr.build_assembly(0.0);
        // 10 main + 10 images = 20
        assert_eq!(assembly.iter().count(), 20);
    }

    #[test]
    fn test_halbach_back_iron_magnet_count() {
        // Bug 2 fix: see test_back_iron_magnet_count — non-zero t is
        // required to add the image.
        let mut cfg = SimulationInput::default();
        cfg.magnet_arrangement = MagnetArrangement::HalbachBackIron;
        cfg.back_iron_thickness_m = 2e-3;
        let arr = MagnetArray::new(&cfg);
        let assembly = arr.build_assembly(0.0);
        // 10 main + 9 interleave + 19 images = 38
        assert_eq!(assembly.iter().count(), 38);
    }
}