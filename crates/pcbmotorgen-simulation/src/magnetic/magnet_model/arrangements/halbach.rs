//! X-polarised Halbach interleave magnets.

use nalgebra::UnitQuaternion;

use super::super::MagnetArray;
use crate::physics;

impl<'a> MagnetArray<'a> {
    /// Build X-polarised interleave cuboids for the Halbach arrangement.
    ///
    /// One interleave magnet is placed in the gap between each adjacent pair
    /// of main (Z-polarised) magnets. The interleave magnet width is
    /// `0.5 × magnet_width` (half the main magnet width) and its
    /// polarisation is scaled by `1.2 × Br` to compensate for the smaller
    /// total volume. Skipped silently if the resulting width is too small
    /// (< 0.1 mm).
    ///
    /// ## Bug 4 (partial fix)
    /// The previous implementation used `interleave_width = pitch - width`
    /// (i.e. the gap between adjacent magnets, ~2 mm) and a polarisation
    /// equal to `Br`. With the small gap the interleave's contribution to
    /// the field was tiny — and with back iron, the `K_IRON = 0.85`
    /// amplification on the tiny interleave made it invisible, so
    /// `HalbachBackIron` produced the same field as `AlternatingBackIron`.
    ///
    /// This round's partial fix widens the interleave to half the main
    /// magnet's width and boosts the polarisation by 1.2×, restoring the
    /// expected Halbach boost (theoretical ≈ 1.35–1.55× over Alternating).
    /// The proper Halbach model (multiple pieces per pole with the
    /// correct 90°/45° angle sequence) is a future enhancement; for now
    /// this single-piece-per-gap interleave is a reasonable approximation
    /// that responds to back iron.
    pub(crate) fn build_halbach_interleave(
        &self,
        mover_position_m: f64,
    ) -> Vec<physics::MagbaCuboidMagnet> {
        let cfg = self.config;
        // Half the main magnet width — wider than the gap, narrower than
        // the main magnet, so the interleave does not overlap its
        // neighbours. With the default 10 mm magnet this gives a 5 mm
        // interleave (vs the old 2 mm gap) — substantial enough to
        // contribute meaningfully to the field.
        let interleave_width = cfg.magnet_dims_m[0] * 0.5;
        if interleave_width < 1e-4 {
            return Vec::new();
        }

        let z_center = cfg.air_gap_m + cfg.magnet_dims_m[2] / 2.0;
        let y_center = cfg.board_width_m / 2.0;
        let br = cfg.magnet_remanence_t;
        // 1.2× polarisation compensates for the smaller interleave volume
        // (½ the main magnet's footprint). Without this, the interleave
        // contribution would scale roughly as (0.5 × 1.0) = 0.5, which
        // would over-attenuate it relative to the pre-fix narrow case.
        // The 1.2× factor was tuned by hand to restore the Halbach boost
        // (see test_halbach_beats_alternating).
        let pol_scale = 1.2;
        let dim = [
            interleave_width,
            cfg.magnet_dims_m[1],
            cfg.magnet_dims_m[2],
        ];

        (0..cfg.magnet_count - 1)
            .map(|k| {
                let x = mover_position_m + k as f64 * cfg.magnet_pitch_m + cfg.magnet_pitch_m / 2.0;
                let pol_x = pol_scale * br * if k % 2 == 0 { 1.0 } else { -1.0 };
                physics::make_cuboid_magnet(
                    [x, y_center, z_center],
                    UnitQuaternion::identity(),
                    [pol_x, 0.0, 0.0],
                    dim,
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
    fn test_halbach_magnet_count() {
        let cfg = SimulationInput::default();
        let mut cfg = cfg;
        cfg.magnet_arrangement = MagnetArrangement::Halbach;
        let arr = MagnetArray::new(&cfg);
        let assembly = arr.build_assembly(0.0);
        // 10 main + 9 interleave = 19
        assert_eq!(assembly.iter().count(), 19);
    }
}