//! `MagnetArray` — builds and manages the magnet source assembly for all
//! four `MagnetArrangement` configurations.
//!
//! ## Arrangement overview
//!
//! `Alternating` — Simple alternating ±Z poles.
//!
//! `Halbach` — X-polarised interleave cuboids inserted in the gap between
//! every adjacent pair of Z-polarised main magnets. The interleave magnets
//! concentrate flux on the stator face and self-cancel on the rear face,
//! giving ≈ 1.35–1.55× field boost.
//!
//! `AlternatingBackIron` / `HalbachBackIron` — Steel keeper (back-iron)
//! simulation via the method of images. Mirror-image copy of every magnet
//! placed on the other side of the steel–magnet interface with the same
//! polarisation, scaled by `K_IRON = 0.85` (calibrated for CRS steel µ_r ≈ 2000).
//!
//! ## Module layout
//! - [`arrangements`](self::arrangements) — per-arrangement builder impls

use crate::params::{MagnetArrangement, SimulationInput};
use crate::physics;

mod arrangements;

/// One B-field sample on the X–Z flux-viz grid.
///
/// `x`, `z` are the observer position in SI metres; `bx`, `by`, `bz` are the
/// B-field vector components in Tesla, in the lab frame (Bx = along travel,
/// By = across board, Bz = vertical). Y is fixed at the board centre-line
/// (`board_width_m / 2`) for every sample.
///
/// `BFieldSample2D` is the core equivalent of the IPC `BFieldSampleIpc`
/// (in `app/src-tauri/src/ipc.rs`); the IPC DTO adds a precomputed magnitude
/// `mag_t = sqrt(bx² + by² + bz²)` for the Svelte renderer's convenience.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BFieldSample2D {
    pub x: f64,
    pub z: f64,
    pub bx: f64,
    pub by: f64,
    pub bz: f64,
}

/// Builds and manages the magnet source assembly for all four arrangements.
///
/// All four arrangements share the same Z-polarised alternating base. Halbach
/// adds X-polarised interleave magnets. Back-iron arrangements add method-of-
/// images mirror copies scaled by `K_IRON`.
pub struct MagnetArray<'a> {
    config: &'a SimulationInput,
}

impl<'a> MagnetArray<'a> {
    /// Create a new `MagnetArray` bound to the given config.
    pub fn new(config: &'a SimulationInput) -> Self {
        Self { config }
    }

    /// Build a `SourceAssembly` for the configured arrangement at the given
    /// mover position.
    ///
    /// Dispatches to the appropriate builder based on `config.magnet_arrangement`.
    ///
    /// ## Back-iron gating (Bug: back iron image when thickness = 0)
    ///
    /// The `AlternatingBackIron` and `HalbachBackIron` arrangements must
    /// only add the method-of-images mirror when there is actually a
    /// back-iron keeper to reflect off. When
    /// `config.back_iron_thickness_m = 0`, the image would otherwise be
    /// placed at the back-iron's nominal top face (which is exactly the
    /// magnet's top face) and scaled by `K_IRON = 0.85`, contributing a
    /// non-zero (and physically wrong) "image" field as if a zero-thickness
    /// steel sheet were present.
    ///
    /// The fix gates the `build_image_magnets` call on
    /// `back_iron_thickness_m > 0`. With this guard, a back-iron
    /// arrangement with `back_iron_thickness_m = 0` reduces to the
    /// no-back-iron variant (same `magnet_count`, same B-field), which
    /// is the user-visible expected behaviour.
    pub fn build_assembly(&self, mover_position_m: f64) -> physics::MagbaSourceAssembly {
        let arr = self.config.magnet_arrangement;
        let mut magnets = self.build_alternating(mover_position_m);
        match arr {
            MagnetArrangement::Alternating => {}
            MagnetArrangement::Halbach => {
                magnets.extend(self.build_halbach_interleave(mover_position_m));
            }
            MagnetArrangement::AlternatingBackIron => {
                // Bug 2: skip the image entirely when there is no back iron.
                if self.config.back_iron_thickness_m > 0.0 {
                    let images = self.build_image_magnets(&magnets);
                    magnets.extend(images);
                }
            }
            MagnetArrangement::HalbachBackIron => {
                let interleave = self.build_halbach_interleave(mover_position_m);
                magnets.extend(interleave.clone());
                // Bug 2: skip the image entirely when there is no back iron.
                // When skipped, this arrangement reduces to plain Halbach
                // (same magnet count, same B-field).
                if self.config.back_iron_thickness_m > 0.0 {
                    let all_magnets = magnets.clone();
                    let images = self.build_image_magnets(&all_magnets);
                    magnets.extend(images);
                }
            }
        }
        physics::make_source_assembly(magnets)
    }

    // ------------------------------------------------------------------
    // Geometry accessors
    // ------------------------------------------------------------------

    /// Z position of main magnet centres above PCB [m].
    pub fn magnet_z_center_m(&self) -> f64 {
        self.config.air_gap_m + self.config.magnet_dims_m[2] / 2.0
    }

    /// X positions of all main magnet centres [m].
    pub fn magnet_x_centers_m(&self, mover_position_m: f64) -> Vec<f64> {
        (0..self.config.magnet_count)
            .map(|k| mover_position_m + k as f64 * self.config.magnet_pitch_m)
            .collect()
    }

    /// Z-polarisation of main magnets, shape `(magnet_count, 3)` [T].
    pub fn polarizations_t(&self) -> Vec<[f64; 3]> {
        let br = self.config.magnet_remanence_t;
        (0..self.config.magnet_count)
            .map(|k| {
                let pol_z = br * if k % 2 == 0 { 1.0 } else { -1.0 };
                [0.0, 0.0, pol_z]
            })
            .collect()
    }

    /// X positions of Halbach interleave magnets [m].
    /// Returns empty vec if not Halbach or HalbachBackIron.
    pub fn interleave_x_centers_m(&self, mover_position_m: f64) -> Vec<f64> {
        let arr = self.config.magnet_arrangement;
        if !matches!(arr, MagnetArrangement::Halbach | MagnetArrangement::HalbachBackIron) {
            return Vec::new();
        }
        let cfg = self.config;
        (0..cfg.magnet_count - 1)
            .map(|k| mover_position_m + k as f64 * cfg.magnet_pitch_m + cfg.magnet_pitch_m / 2.0)
            .collect()
    }

    /// Z position of image magnet centres [m] (back-iron arrangements only).
    ///
    /// Mirrors [`Self::build_image_magnets`]: the image plane is the top
    /// face of the back iron, NOT the top face of the magnets, so the
    /// image is further from the magnet (closer to the PCB) when the back
    /// iron is thicker.
    ///
    /// Returns `None` when there is no back iron
    /// (`back_iron_thickness_m = 0`) — in that case no image magnet is
    /// built (see [`Self::build_assembly`]) and a downstream consumer
    /// must not read a stale image position. The previous version
    /// returned a `f64` even when `back_iron_thickness_m = 0`, which made
    /// the API silently report a meaningless value.
    pub fn image_z_center_m(&self) -> Option<f64> {
        let cfg = self.config;
        if cfg.back_iron_thickness_m <= 0.0 {
            return None;
        }
        let z_mirror = cfg.air_gap_m + cfg.magnet_dims_m[2] + cfg.back_iron_thickness_m;
        let z_original = cfg.air_gap_m + cfg.magnet_dims_m[2] / 2.0;
        Some(2.0 * z_mirror - z_original)
    }

    // ------------------------------------------------------------------
    // B-field sampling
    // ------------------------------------------------------------------

    /// Sample B along the board centre-line at the PCB surface.
    ///
    /// - `x_sample`: X positions [m]
    /// - `mover_position_m`: mover position [m]
    /// - `z_observer`: Z of observation plane [m] (default 0 = PCB surface)
    ///
    /// Returns `Vec<[f64; 3]>` of B vectors [T], one per x_sample.
    pub fn bfield_at_pcb_surface(
        &self,
        x_sample: &[f64],
        mover_position_m: f64,
        z_observer: f64,
    ) -> Vec<[f64; 3]> {
        let y_center = self.config.board_width_m / 2.0;
        let observers: Vec<nalgebra::Point3<f64>> = x_sample
            .iter()
            .map(|&x| nalgebra::Point3::new(x, y_center, z_observer))
            .collect();

        let assembly = self.build_assembly(mover_position_m);
        let b_vec = physics::compute_b_batch_parallel(&assembly, &observers);

        b_vec.into_iter().map(|b| [b.x, b.y, b.z]).collect()
    }

    /// Sample the B-field on a 2D X–Z grid at the board centre-line
    /// (`y = board_width_m / 2`).
    ///
    /// This is the WP4 / WP5 flux-viz sampler: a 24×12 arrow grid that the
    /// `FluxDiagram` Svelte component renders. It composes the existing
    /// 1D [`Self::bfield_at_pcb_surface`] at each Z row — no magba plumbing
    /// is duplicated. The 1D sampler routes through
    /// `pcbmotorgen_simulation::physics::compute_b_batch_parallel` (the magba
    /// adapter), and the upstream call to
    /// [`Self::build_assembly`] dispatches on
    /// `MagnetArrangement` so all four variants
    /// (Alternating, AlternatingBackIron, Halbach, HalbachBackIron) are
    /// reflected.
    ///
    /// - `x_sample`: X positions along the travel axis [m]
    /// - `z_sample`: Z rows of the grid [m] (e.g. PCB surface, magnet
    ///   midplane, 2 mm above magnet top)
    /// - `mover_position_m`: mover X position [m]
    ///
    /// Returns one [`BFieldSample2D`] per `(x, z)` pair, **row-major** with
    /// Z as the slow axis: `samples[i_z * n_x + i_x]`. Total length is
    /// `x_sample.len() * z_sample.len()`. B is in Tesla, in the lab frame
    /// (Bx = along travel, By = across board, Bz = vertical).
    pub fn bfield_grid(
        &self,
        x_sample: &[f64],
        z_sample: &[f64],
        mover_position_m: f64,
    ) -> Vec<BFieldSample2D> {
        let n_x = x_sample.len();
        let n_z = z_sample.len();
        let mut samples = Vec::with_capacity(n_x * n_z);

        for &z in z_sample {
            // Reuse the 1D sampler at this Z row — it owns the magba
            // adapter call and the arrangement dispatch.
            let row = self.bfield_at_pcb_surface(x_sample, mover_position_m, z);
            for (i, b) in row.iter().enumerate() {
                samples.push(BFieldSample2D {
                    x: x_sample[i],
                    z,
                    bx: b[0],
                    by: b[1],
                    bz: b[2],
                });
            }
        }
        samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SimulationInput {
        SimulationInput::default()
    }

    #[test]
    fn test_magnet_z_center() {
        let cfg = test_config();
        let arr = MagnetArray::new(&cfg);
        // air_gap + height/2 = 0.0005 + 0.002 = 0.0025
        assert!((arr.magnet_z_center_m() - 0.0025).abs() < 1e-9);
    }

    #[test]
    fn test_magnet_x_centers() {
        let cfg = test_config();
        let arr = MagnetArray::new(&cfg);
        let xs = arr.magnet_x_centers_m(0.0);
        assert_eq!(xs.len(), 10);
        assert!((xs[0] - 0.0).abs() < 1e-9);
        assert!((xs[1] - 0.012).abs() < 1e-9);
        assert!((xs[9] - 0.108).abs() < 1e-9);
    }

    #[test]
    fn test_bfield_at_pcb_surface() {
        let cfg = test_config();
        let arr = MagnetArray::new(&cfg);
        let xs = vec![0.0, 0.006, 0.012, 0.018];
        let b = arr.bfield_at_pcb_surface(&xs, 0.0, 0.0);
        assert_eq!(b.len(), 4);
        // B should be finite and non-trivial
        for bi in &b {
            assert!(bi[0].is_finite());
            assert!(bi[1].is_finite());
            assert!(bi[2].is_finite());
            // |B| should be > 0
            let mag = (bi[0] * bi[0] + bi[1] * bi[1] + bi[2] * bi[2]).sqrt();
            assert!(mag > 1e-6, "B magnitude too small: {}", mag);
        }
        // Bz should alternate sign between magnet centres
        // x=0 is centre of magnet 0 (Z+), x=0.012 is centre of magnet 1 (Z-)
        assert!(b[0][2] > 0.0, "Bz at x=0 should be positive (Z+ pole)");
        assert!(b[1][2] < 0.0, "Bz at x=6mm should be negative (between poles)");
    }

    #[test]
    fn test_image_z_center() {
        // Default config has back_iron_thickness_m = 0.0 → no back iron
        // → accessor must report None (no image to position).
        let cfg = test_config();
        let arr = MagnetArray::new(&cfg);
        assert!(
            arr.image_z_center_m().is_none(),
            "image_z_center_m must return None when back_iron_thickness_m = 0"
        );

        // With a positive back-iron thickness, the accessor returns Some(z).
        // z_mirror = 0.0005 + 0.004 + 0.002 = 0.0065 (with t = 2 mm)
        // z_original = 0.0025
        // z_image = 2*0.0065 - 0.0025 = 0.0105
        let mut cfg = test_config();
        cfg.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
        cfg.back_iron_thickness_m = 2e-3;
        let arr = MagnetArray::new(&cfg);
        let z = arr.image_z_center_m().expect("Some(z) with t > 0");
        assert!(
            (z - 0.0105).abs() < 1e-9,
            "image_z_center_m with 2mm back iron must equal 0.0105, got {}",
            z
        );
    }

    // --- WP4 2D B-field grid sampler ---

    /// 2D sampler returns one sample per (x, z) in row-major order, with
    /// finite, non-trivial magnitudes for all four arrangements.
    #[test]
    fn test_bfield_grid_row_major_and_magnitude() {
        let cfg = test_config();
        let arr = MagnetArray::new(&cfg);
        let xs = vec![0.0, 0.006, 0.012, 0.018];
        let zs = vec![0.0, 0.002, 0.004, 0.006];
        let grid = arr.bfield_grid(&xs, &zs, 0.0);
        assert_eq!(grid.len(), xs.len() * zs.len());
        // Row-major: samples[0] is (xs[0], zs[0]); samples[1] is (xs[1], zs[0])
        assert!((grid[0].x - xs[0]).abs() < 1e-12);
        assert!((grid[0].z - zs[0]).abs() < 1e-12);
        assert!((grid[1].x - xs[1]).abs() < 1e-12);
        assert!((grid[1].z - zs[0]).abs() < 1e-12);
        // Last row, first column = (xs[0], zs[3])
        let last_row_first = xs.len() * (zs.len() - 1);
        assert!((grid[last_row_first].x - xs[0]).abs() < 1e-12);
        assert!((grid[last_row_first].z - zs[3]).abs() < 1e-12);
        // Every sample must be finite with non-zero |B|.
        for s in &grid {
            assert!(s.bx.is_finite());
            assert!(s.by.is_finite());
            assert!(s.bz.is_finite());
            let mag = (s.bx * s.bx + s.by * s.by + s.bz * s.bz).sqrt();
            assert!(mag > 1e-6, "B magnitude too small at ({}, {}): {}", s.x, s.z, mag);
        }
    }

    /// Bz at the PCB surface (z=0) over magnet 0 centre (x=0) should be
    /// positive (Z+ pole facing the observer).
    #[test]
    fn test_bfield_grid_alternating_pcb_surface_polarity() {
        let cfg = test_config();
        let arr = MagnetArray::new(&cfg);
        let xs = vec![0.0, 0.012]; // centre of magnet 0 (Z+), centre of magnet 1 (Z-)
        let zs = vec![0.0];        // PCB surface
        let grid = arr.bfield_grid(&xs, &zs, 0.0);
        assert!(grid[0].bz > 0.0, "Bz at x=0,z=0 should be positive (Z+ pole)");
        assert!(grid[1].bz < 0.0, "Bz at x=12mm,z=0 should be negative (Z- pole)");
    }

    /// All four MagnetArrangement variants must produce a populated grid
    /// (i.e. the bfield_grid call goes through build_assembly and picks
    /// up the interleave / image magnets when applicable).
    #[test]
    fn test_bfield_grid_works_for_all_four_arrangements() {
        let variants = [
            MagnetArrangement::Alternating,
            MagnetArrangement::AlternatingBackIron,
            MagnetArrangement::Halbach,
            MagnetArrangement::HalbachBackIron,
        ];
        for &arr in &variants {
            let mut cfg = test_config();
            cfg.magnet_arrangement = arr;
            let ma = MagnetArray::new(&cfg);
            let xs = vec![0.0, 0.012, 0.024];
            let zs = vec![0.0, 0.0025];
            let grid = ma.bfield_grid(&xs, &zs, 0.0);
            assert_eq!(
                grid.len(),
                xs.len() * zs.len(),
                "arrangement {:?} produced wrong grid length",
                arr
            );
            for s in &grid {
                assert!(s.bx.is_finite() && s.by.is_finite() && s.bz.is_finite());
            }
        }
    }

    /// Back-iron arrangements should produce a measurably stronger
    /// |B| at the PCB surface than the no-back-iron variant
    /// (steel keeper reinforces the field).
    ///
    /// Bug 2: this test sets a non-zero `back_iron_thickness_m` on the
    /// back-iron config so the image is actually added. Pre-fix, the
    /// default test_config (t = 0) would still add the image, so the
    /// assertion passed by accident. Post-fix, `t > 0` is required and
    /// the test continues to verify the physical amplification.
    #[test]
    fn test_bfield_grid_back_iron_amplifies_field() {
        let mut cfg_no = test_config();
        cfg_no.magnet_arrangement = MagnetArrangement::Alternating;
        let mut cfg_bi = test_config();
        cfg_bi.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
        cfg_bi.back_iron_thickness_m = 2e-3; // Bug 2: required to add the image
        let xs = vec![0.006]; // between magnet 0 (Z+) and magnet 1 (Z-): Bz should be near zero, Bx dominates
        let zs = vec![0.0];
        let no = MagnetArray::new(&cfg_no).bfield_grid(&xs, &zs, 0.0);
        let bi = MagnetArray::new(&cfg_bi).bfield_grid(&xs, &zs, 0.0);
        let mag_no = (no[0].bx * no[0].bx + no[0].by * no[0].by + no[0].bz * no[0].bz).sqrt();
        let mag_bi = (bi[0].bx * bi[0].bx + bi[0].by * bi[0].by + bi[0].bz * bi[0].bz).sqrt();
        assert!(
            mag_bi > mag_no,
            "back-iron should amplify |B| at PCB surface: |B_no|={:.4} T, |B_bi|={:.4} T",
            mag_no,
            mag_bi
        );
    }

    /// 1D `bfield_at_pcb_surface` signature is unchanged — `bfield_grid` is
    /// a sibling, not a replacement. This test guards the 1D contract.
    #[test]
    fn test_bfield_at_pcb_surface_1d_signature_intact() {
        let cfg = test_config();
        let arr = MagnetArray::new(&cfg);
        // Original 3-arg call form: (x_sample, mover_position_m, z_observer)
        let b = arr.bfield_at_pcb_surface(&[0.0, 0.006], 0.0, 0.0);
        assert_eq!(b.len(), 2);
        // Returns Vec<[f64; 3]>, not Vec<BFieldSample2D>
        assert_eq!(b[0].len(), 3);
    }

    // --- Bug 3 regression: back-iron thickness must shift the image plane ---

    /// `image_z_center_m` must move further from the real magnet as the
    /// back iron thickens (the mirror plane is the back-iron's top face,
    /// not the magnet's top face). Pre-fix, this was constant regardless
    /// of `back_iron_thickness_m`.
    ///
    /// With Bug 2 fixed, `image_z_center_m()` returns `None` for
    /// `back_iron_thickness_m = 0` (no image to place), so this test
    /// only exercises `t > 0` cases.
    #[test]
    fn test_image_z_center_moves_with_back_iron_thickness() {
        // Compute the image position from a fresh config each time, since
        // `MagnetArray::new` borrows its config and we want to vary the
        // back-iron thickness across iterations.
        // t = 0 → no image at all (Bug 2 fix: the accessor returns None
        // because build_assembly would skip the image). The pre-fix
        // behaviour was to return 0.0065 here, which was a meaningless
        // value because no image was actually placed.
        let mut cfg = test_config();
        cfg.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
        cfg.back_iron_thickness_m = 0.0;
        assert!(
            MagnetArray::new(&cfg).image_z_center_m().is_none(),
            "no-back-iron image_z_center_m must be None (no image is placed)"
        );

        // t = 2 mm → image plane moves up by 2 × 2 mm = 4 mm
        // (mirror is the back iron top, image is reflected).
        // z_mirror = 0.0005 + 0.004 + 0.002 = 0.0065
        // z_original = 0.0025
        // z_image = 2*0.0065 - 0.0025 = 0.0105
        cfg.back_iron_thickness_m = 2e-3;
        let z2 = MagnetArray::new(&cfg)
            .image_z_center_m()
            .expect("Some(z) for t = 2mm");
        assert!(
            (z2 - 0.0105).abs() < 1e-9,
            "2mm back iron must push z_image to 0.0105, got {}",
            z2
        );

        // t = 4 mm → z_mirror = 0.0065 + 0.002 = 0.0085
        // z_image = 2*0.0085 - 0.0025 = 0.0145
        cfg.back_iron_thickness_m = 4e-3;
        let z4 = MagnetArray::new(&cfg)
            .image_z_center_m()
            .expect("Some(z) for t = 4mm");
        assert!(
            (z4 - 0.0145).abs() < 1e-9,
            "4mm back iron must push z_image to 0.0145, got {}",
            z4
        );

        // Monotonicity: thicker back iron → image further from magnet
        // (i.e. closer to the PCB).
        assert!(z4 > z2, "thicker back iron must move image further away (z4={} > z2={})", z4, z2);
    }

    /// B-field at the PCB surface must change monotonically with
    /// `back_iron_thickness_m` for `AlternatingBackIron` (Bug 3 regression).
    ///
    /// Physical interpretation: with the mirror plane placed at the back
    /// iron's top face (the fix), a thicker back iron pushes the mirror
    /// further from the PCB, which moves the image magnet further from
    /// the PCB and weakens the image's contribution at the observer.
    /// The net |B| therefore decreases monotonically with thickness for
    /// `t > 0`. (The pre-fix code did not move the mirror at all, so |B|
    /// was constant in t — that is the bug.)
    ///
    /// Bug 2 also affects this test: with `t = 0` no image is added (so
    /// |B| equals the plain-Alternating baseline), and the field jumps
    /// upward at the first positive `t` (image appears), then
    /// monotonically decreases as the image moves further away. The
    /// test therefore checks (a) `t = 0` matches the no-iron baseline,
    /// (b) the smallest positive `t` gives the largest |B|, and
    /// (c) |B| is monotonically decreasing for `t > 0`.
    ///
    /// Note: the absolute direction of the dependence is an artefact of
    /// the simple method-of-images model. A more sophisticated model would
    /// also vary `K_IRON` with thickness (thicker steel saturates less,
    /// K_IRON closer to 1.0). That is a future enhancement; for this
    /// round the geometric fix is sufficient and the test asserts that
    /// the field is no longer *independent* of t.
    #[test]
    fn test_back_iron_thickness_changes_bfield_for_alternating() {
        // Sample at the centre of magnet 0 (a Z+ pole). The dominant
        // component is Bz; |B| is the natural aggregate.
        let xs = vec![0.0];
        let zs = vec![0.0];
        let thicknesses = [0.0e-3, 0.5e-3, 1.0e-3, 2.0e-3, 4.0e-3];
        let samples: Vec<f64> = thicknesses
            .iter()
            .map(|&t| {
                let mut cfg = test_config();
                cfg.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
                cfg.back_iron_thickness_m = t;
                let ma = MagnetArray::new(&cfg);
                let grid = ma.bfield_grid(&xs, &zs, 0.0);
                let mag = (grid[0].bx * grid[0].bx
                    + grid[0].by * grid[0].by
                    + grid[0].bz * grid[0].bz)
                    .sqrt();
                assert!(
                    mag.is_finite(),
                    "|B| at back_iron_thickness_m={} must be finite, got {}",
                    t, mag
                );
                mag
            })
            .collect();

        // Bug 2: t = 0 → no image, |B| equals the plain-Alternating
        // baseline. We re-measure the baseline to compare exactly (rather
        // than hard-coding the value, which would couple the test to the
        // magnetic constants).
        let mut cfg_alt = test_config();
        cfg_alt.magnet_arrangement = MagnetArrangement::Alternating;
        let baseline = {
            let grid = MagnetArray::new(&cfg_alt).bfield_grid(&xs, &zs, 0.0);
            (grid[0].bx * grid[0].bx + grid[0].by * grid[0].by + grid[0].bz * grid[0].bz).sqrt()
        };
        assert!(
            (samples[0] - baseline).abs() < 1e-12,
            "t = 0 (no image) |B| must equal the plain-Alternating baseline; \
             got samples[0]={:.6e} baseline={:.6e}",
            samples[0], baseline
        );

        // Bug 3 regression: |B| at the smallest positive t must be
        // larger than the baseline (the image is now present and close
        // to the PCB).
        assert!(
            samples[1] > samples[0],
            "first positive t must be larger than the t=0 baseline (image \
             appears): samples[0]={:.4} samples[1]={:.4}",
            samples[0], samples[1]
        );

        // Bug 3 regression: |B| must be monotonically decreasing for
        // t > 0 (image moves further from PCB as t grows).
        for i in 2..samples.len() {
            assert!(
                samples[i] < samples[i - 1],
                "|B| must decrease monotonically with back_iron_thickness_m for t > 0 \
                 (image moves further from PCB as t grows): samples={:?}",
                samples.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>()
            );
        }
        // Sanity: the no-back-iron case has a non-zero baseline.
        assert!(samples[0] > 0.0, "no-back-iron |B| must be non-zero");
    }

    /// Same monotonic-dependence-on-t test for `HalbachBackIron`
    /// (Bug 3 regression). Halbach already concentrates flux; adding a
    /// back iron still moves the image mirror plane with thickness.
    ///
    /// Bug 2 affects this test the same way as
    /// `test_back_iron_thickness_changes_bfield_for_alternating`: `t = 0`
    /// is a no-image baseline equal to the plain-Halbach |B|, and the
    /// image appears at the first positive `t`.
    #[test]
    fn test_back_iron_thickness_changes_bfield_for_halbach() {
        let xs = vec![0.0];
        let zs = vec![0.0];
        let thicknesses = [0.0e-3, 0.5e-3, 1.0e-3, 2.0e-3, 4.0e-3];
        let samples: Vec<f64> = thicknesses
            .iter()
            .map(|&t| {
                let mut cfg = test_config();
                cfg.magnet_arrangement = MagnetArrangement::HalbachBackIron;
                cfg.back_iron_thickness_m = t;
                let ma = MagnetArray::new(&cfg);
                let grid = ma.bfield_grid(&xs, &zs, 0.0);
                let mag = (grid[0].bx * grid[0].bx
                    + grid[0].by * grid[0].by
                    + grid[0].bz * grid[0].bz)
                    .sqrt();
                assert!(
                    mag.is_finite(),
                    "|B| at back_iron_thickness_m={} must be finite, got {}",
                    t, mag
                );
                mag
            })
            .collect();

        // Bug 2: t = 0 must match the plain-Halbach baseline.
        let mut cfg_hal = test_config();
        cfg_hal.magnet_arrangement = MagnetArrangement::Halbach;
        let baseline = {
            let grid = MagnetArray::new(&cfg_hal).bfield_grid(&xs, &zs, 0.0);
            (grid[0].bx * grid[0].bx + grid[0].by * grid[0].by + grid[0].bz * grid[0].bz).sqrt()
        };
        assert!(
            (samples[0] - baseline).abs() < 1e-12,
            "t = 0 (no image) |B| must equal the plain-Halbach baseline; \
             got samples[0]={:.6e} baseline={:.6e}",
            samples[0], baseline
        );
        // Bug 3: monotonic decrease for t > 0.
        for i in 2..samples.len() {
            assert!(
                samples[i] < samples[i - 1],
                "|B| must decrease monotonically with back_iron_thickness_m for t > 0 \
                 on HalbachBackIron: samples={:?}",
                samples.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>()
            );
        }
    }

    // --- Bug 4 regression: Halbach must beat Alternating by a clear margin ---

    /// Halbach must produce a measurably stronger peak |B| at the PCB
    /// surface than Alternating, for the same Br, air_gap, etc. The
    /// theoretical Halbach boost over a single-side flux-concentrating
    /// array is 1.35–1.55×; we assert at least 5% (a deliberately
    /// conservative bound that proves the interleave is contributing).
    ///
    /// Bug 4: pre-fix, the Halbach interleave was the same width as the
    /// gap (~2 mm vs the 10 mm main magnet) and Halbach's boost was
    /// nearly invisible — the test would fail because |B_halbach| ≤
    /// |B_alternating|.
    ///
    /// Sampling is at the centre of the first main magnet (x=0, z=0),
    /// which is the worst case for the Halbach boost (the interleave
    /// between magnets 0 and 1 contributes X-polarised field, which
    /// shows up as |B| through the in-plane component). The boost is
    /// typically larger in the inter-magnet gap, but we sample at the
    /// magnet centre to keep the comparison conservative.
    #[test]
    fn test_halbach_beats_alternating_by_at_least_5pct() {
        let xs = vec![0.0];
        let zs = vec![0.0];
        let cfg_alt = {
            let mut c = test_config();
            c.magnet_arrangement = MagnetArrangement::Alternating;
            c
        };
        let cfg_hal = {
            let mut c = test_config();
            c.magnet_arrangement = MagnetArrangement::Halbach;
            c
        };
        let b_alt = {
            let grid = MagnetArray::new(&cfg_alt).bfield_grid(&xs, &zs, 0.0);
            (grid[0].bx * grid[0].bx + grid[0].by * grid[0].by + grid[0].bz * grid[0].bz).sqrt()
        };
        let b_hal = {
            let grid = MagnetArray::new(&cfg_hal).bfield_grid(&xs, &zs, 0.0);
            (grid[0].bx * grid[0].bx + grid[0].by * grid[0].by + grid[0].bz * grid[0].bz).sqrt()
        };
        let boost = b_hal / b_alt;
        assert!(
            boost > 1.05,
            "Halbach must beat Alternating by at least 5%; got boost={:.3} \
             (|B_alt|={:.4} T, |B_hal|={:.4} T)",
            boost, b_alt, b_hal
        );
    }

    /// Same comparison with a back iron: `HalbachBackIron` must beat
    /// `AlternatingBackIron` by the same 5% margin. Bug 4's most visible
    /// symptom was that these two arrangements produced the *same* field
    /// (the tiny interleave was lost in the K_IRON=0.85 amplification).
    /// With the widened interleave, the Halbach boost is preserved under
    /// back iron.
    #[test]
    fn test_halbach_back_iron_beats_alternating_back_iron_by_at_least_5pct() {
        let xs = vec![0.0];
        let zs = vec![0.0];
        let cfg_alt = {
            let mut c = test_config();
            c.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
            c.back_iron_thickness_m = 2e-3;
            c
        };
        let cfg_hal = {
            let mut c = test_config();
            c.magnet_arrangement = MagnetArrangement::HalbachBackIron;
            c.back_iron_thickness_m = 2e-3;
            c
        };
        let b_alt = {
            let grid = MagnetArray::new(&cfg_alt).bfield_grid(&xs, &zs, 0.0);
            (grid[0].bx * grid[0].bx + grid[0].by * grid[0].by + grid[0].bz * grid[0].bz).sqrt()
        };
        let b_hal = {
            let grid = MagnetArray::new(&cfg_hal).bfield_grid(&xs, &zs, 0.0);
            (grid[0].bx * grid[0].bx + grid[0].by * grid[0].by + grid[0].bz * grid[0].bz).sqrt()
        };
        let boost = b_hal / b_alt;
        assert!(
            boost > 1.05,
            "HalbachBackIron must beat AlternatingBackIron by at least 5%; got \
             boost={:.3} (|B_alt|={:.4} T, |B_hal|={:.4} T)",
            boost, b_alt, b_hal
        );
    }

    // --- Bug 2 regression: back iron must NOT add an image when thickness = 0 ---

    /// `AlternatingBackIron` with `back_iron_thickness_m = 0` must
    /// reduce to the same magnet count as plain `Alternating` — no
    /// method-of-images mirror should be added when there is no back
    /// iron to reflect off.
    ///
    /// Bug 2: pre-fix, `build_assembly` always called
    /// `build_image_magnets` for the back-iron arrangements, regardless
    /// of `back_iron_thickness_m`. With `t = 0` the image was placed at
    /// the back-iron's nominal top face (which coincides with the
    /// magnet's top face) and scaled by `K_IRON = 0.85`, contributing a
    /// non-zero "image" field as if a zero-thickness steel sheet were
    /// present. The user observed this as "Changing from Halbach to
    /// Halbach with steel shows improved force, even when back iron is 0."
    #[test]
    fn test_back_iron_with_zero_thickness_has_no_images() {
        // AlternatingBackIron with t=0 → magnet count == Alternating
        let mut cfg = test_config();
        cfg.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
        cfg.back_iron_thickness_m = 0.0;
        let count_bi0 = MagnetArray::new(&cfg).build_assembly(0.0).iter().count();
        let cfg_alt = test_config(); // default: Alternating
        let count_alt = MagnetArray::new(&cfg_alt).build_assembly(0.0).iter().count();
        assert_eq!(
            count_bi0, count_alt,
            "AlternatingBackIron with back_iron_thickness_m = 0 must have the same \
             magnet count as plain Alternating (no image added). \
             Got AlternatingBackIron={} Alternating={}",
            count_bi0, count_alt
        );
        // Sanity: this matches the default Alternating count (10).
        assert_eq!(count_bi0, 10, "expected 10 magnets, got {}", count_bi0);

        // HalbachBackIron with t=0 → magnet count == Halbach
        let mut cfg = test_config();
        cfg.magnet_arrangement = MagnetArrangement::HalbachBackIron;
        cfg.back_iron_thickness_m = 0.0;
        let count_hbi0 = MagnetArray::new(&cfg).build_assembly(0.0).iter().count();
        let mut cfg_hal = test_config();
        cfg_hal.magnet_arrangement = MagnetArrangement::Halbach;
        let count_hal = MagnetArray::new(&cfg_hal).build_assembly(0.0).iter().count();
        assert_eq!(
            count_hbi0, count_hal,
            "HalbachBackIron with back_iron_thickness_m = 0 must have the same \
             magnet count as plain Halbach (no image added). \
             Got HalbachBackIron={} Halbach={}",
            count_hbi0, count_hal
        );
        // Sanity: 10 main + 9 interleave = 19.
        assert_eq!(count_hbi0, 19, "expected 19 magnets, got {}", count_hbi0);
    }

    /// `AlternatingBackIron` with `back_iron_thickness_m > 0` must
    /// still add the image (this is the desired behaviour the bug fix
    /// must not break). The magnet count is `magnet_count × 2` = 20.
    #[test]
    fn test_back_iron_with_positive_thickness_adds_images() {
        let mut cfg = test_config();
        cfg.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
        cfg.back_iron_thickness_m = 2e-3;
        let count = MagnetArray::new(&cfg).build_assembly(0.0).iter().count();
        assert_eq!(
            count, 20,
            "AlternatingBackIron with t > 0 must add image magnets: expected 20, got {}",
            count
        );

        // HalbachBackIron with t > 0: 10 main + 9 interleave + 19 images = 38.
        let mut cfg = test_config();
        cfg.magnet_arrangement = MagnetArrangement::HalbachBackIron;
        cfg.back_iron_thickness_m = 2e-3;
        let count = MagnetArray::new(&cfg).build_assembly(0.0).iter().count();
        assert_eq!(
            count, 38,
            "HalbachBackIron with t > 0 must add image magnets: expected 38, got {}",
            count
        );
    }

    /// Bug 2 (the user-observed symptom): with `back_iron_thickness_m = 0`,
    /// `Halbach` and `HalbachBackIron` must produce IDENTICAL B-field
    /// samples. Pre-fix, `HalbachBackIron` added a `K_IRON`-scaled
    /// image even with `t = 0`, producing a measurably larger |B| than
    /// plain `Halbach` — the user-visible bug.
    #[test]
    fn test_halbach_back_iron_with_zero_thickness_matches_halbach() {
        let xs = vec![0.0, 0.003, 0.006, 0.009, 0.012];
        let zs = vec![0.0];
        let mut cfg_hal = test_config();
        cfg_hal.magnet_arrangement = MagnetArrangement::Halbach;
        let mut cfg_hbi = test_config();
        cfg_hbi.magnet_arrangement = MagnetArrangement::HalbachBackIron;
        cfg_hbi.back_iron_thickness_m = 0.0; // explicitly zero
        let grid_hal = MagnetArray::new(&cfg_hal).bfield_grid(&xs, &zs, 0.0);
        let grid_hbi = MagnetArray::new(&cfg_hbi).bfield_grid(&xs, &zs, 0.0);
        assert_eq!(grid_hal.len(), grid_hbi.len());
        for (i, (a, b)) in grid_hal.iter().zip(grid_hbi.iter()).enumerate() {
            let mag_a = (a.bx * a.bx + a.by * a.by + a.bz * a.bz).sqrt();
            let mag_b = (b.bx * b.bx + b.by * b.by + b.bz * b.bz).sqrt();
            assert!(
                (mag_a - mag_b).abs() < 1e-12,
                "HalbachBackIron (t=0) |B| at sample {} must equal Halbach |B|; \
                 got {:.6e} vs {:.6e} (diff {:.3e})",
                i, mag_a, mag_b, (mag_a - mag_b).abs()
            );
            // Also check per-component, since |B| equality could mask a
            // sign flip or rotation.
            assert!(
                (a.bx - b.bx).abs() < 1e-12,
                "HalbachBackIron (t=0) bx at sample {} must equal Halbach bx; got {} vs {}",
                i, a.bx, b.bx
            );
            assert!(
                (a.by - b.by).abs() < 1e-12,
                "HalbachBackIron (t=0) by at sample {} must equal Halbach by; got {} vs {}",
                i, a.by, b.by
            );
            assert!(
                (a.bz - b.bz).abs() < 1e-12,
                "HalbachBackIron (t=0) bz at sample {} must equal Halbach bz; got {} vs {}",
                i, a.bz, b.bz
            );
        }
    }

    /// Same identity for the alternating pair: `AlternatingBackIron`
    /// with `back_iron_thickness_m = 0` must produce the same B-field
    /// as plain `Alternating`. Companion to the previous test.
    #[test]
    fn test_alternating_back_iron_with_zero_thickness_matches_alternating() {
        let xs = vec![0.0, 0.003, 0.006, 0.009, 0.012];
        let zs = vec![0.0];
        let cfg_alt = test_config();
        let mut cfg_abi = test_config();
        cfg_abi.magnet_arrangement = MagnetArrangement::AlternatingBackIron;
        cfg_abi.back_iron_thickness_m = 0.0;
        let grid_alt = MagnetArray::new(&cfg_alt).bfield_grid(&xs, &zs, 0.0);
        let grid_abi = MagnetArray::new(&cfg_abi).bfield_grid(&xs, &zs, 0.0);
        for (i, (a, b)) in grid_alt.iter().zip(grid_abi.iter()).enumerate() {
            assert!(
                (a.bx - b.bx).abs() < 1e-12
                    && (a.by - b.by).abs() < 1e-12
                    && (a.bz - b.bz).abs() < 1e-12,
                "AlternatingBackIron (t=0) B at sample {} must equal Alternating B; \
                 got ({:.6e}, {:.6e}, {:.6e}) vs ({:.6e}, {:.6e}, {:.6e})",
                i, a.bx, a.by, a.bz, b.bx, b.by, b.bz
            );
        }
    }

    /// Bug 2 desired behaviour (regression guard for the fix): with
    /// `back_iron_thickness_m > 0`, `HalbachBackIron` must produce a
    /// measurably larger |B| at the PCB surface than `Halbach` (the
    /// steel keeper amplifies the field). The previous test
    /// `test_halbach_back_iron_beats_alternating_back_iron_by_at_least_5pct`
    /// only checks the inter-arrangement gap; this test pins down the
    /// per-arrangement, with-vs-without-back-iron gap.
    #[test]
    fn test_halbach_back_iron_with_positive_thickness_beats_halbach() {
        let xs = vec![0.0];
        let zs = vec![0.0];
        let mut cfg_hal = test_config();
        cfg_hal.magnet_arrangement = MagnetArrangement::Halbach;
        let mut cfg_hbi = test_config();
        cfg_hbi.magnet_arrangement = MagnetArrangement::HalbachBackIron;
        cfg_hbi.back_iron_thickness_m = 2e-3;
        let mag_hal = {
            let grid = MagnetArray::new(&cfg_hal).bfield_grid(&xs, &zs, 0.0);
            (grid[0].bx * grid[0].bx + grid[0].by * grid[0].by + grid[0].bz * grid[0].bz).sqrt()
        };
        let mag_hbi = {
            let grid = MagnetArray::new(&cfg_hbi).bfield_grid(&xs, &zs, 0.0);
            (grid[0].bx * grid[0].bx + grid[0].by * grid[0].by + grid[0].bz * grid[0].bz).sqrt()
        };
        assert!(
            mag_hbi > mag_hal,
            "HalbachBackIron (t=2mm) |B| must be larger than Halbach |B| (steel \
             keeper amplifies the field): |B_hal|={:.4} T, |B_hbi|={:.4} T",
            mag_hal, mag_hbi
        );
        // Also: a non-trivial gap. K_IRON = 0.85, so even a perfect image
        // doubles the field at the mirror and adds ~0.85 of the original
        // at the observer; a 1% threshold is comfortably conservative.
        assert!(
            (mag_hbi - mag_hal) / mag_hal > 0.01,
            "HalbachBackIron (t=2mm) |B| must be at least 1% larger than Halbach; \
             got (hbi - hal) / hal = {:.4}",
            (mag_hbi - mag_hal) / mag_hal
        );
    }
}