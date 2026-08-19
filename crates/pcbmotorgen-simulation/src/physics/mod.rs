//! Thin adapter over `magba` to insulate upstream code from API breaks.
//!
//! All direct `magba` calls live here. Upstream modules (`magnetic::*`) call
//! these wrappers instead of touching `magba` types directly. If a future
//! magba version changes its API, only this file needs updating.
//!
//! ## magba 0.6.2 API mapping
//!
//! | Magpylib (Python)                              | magba (Rust)                                        |
//! |------------------------------------------------|-----------------------------------------------------|
//! | `magpy.magnet.Cuboid(polarization, position, dimension)` | `CuboidMagnet::new(pos, quat, pol, dim)`     |
//! | `magpy.Collection(*magnets)`                   | `SourceAssembly::new(...)` or `sources!` macro      |
//! | `magpy.getB(collection, observers)`            | `Source::compute_B_batch(&assembly, &points)`      |
//! | `magpy.current.Polyline(current, vertices)`     | `PathCurrent::new(pos, quat, current, vertices)`    |
//! | `magpy.getFT(magnets, polylines)`              | **NATIVE**: Lorentz loop `F = I·Σ(dLᵢ × Bᵢ)`         |

use nalgebra::{Point3, UnitQuaternion, Vector3};
use rayon::prelude::*;

use magba::base::Source;
use magba::collections::SourceAssembly;
use magba::currents::PathCurrent;
use magba::magnets::CuboidMagnet;

// Re-export key magba types so upstream code doesn't need to import magba directly.
pub use magba::collections::SourceAssembly as MagbaSourceAssembly;
pub use magba::currents::PathCurrent as MagbaPathCurrent;
pub use magba::magnets::CuboidMagnet as MagbaCuboidMagnet;

/// Build a `CuboidMagnet` from simple array inputs.
///
/// Wraps `magba::magnets::CuboidMagnet::new` so callers don't need to
/// construct nalgebra points/vectors manually.
///
/// - `pos`: `[x, y, z]` centre position [m]
/// - `orientation`: `UnitQuaternion<f64>` rotation (identity = no rotation)
/// - `polarization`: `[px, py, pz]` polarization vector [T]
/// - `dimensions`: `[dx, dy, dz]` cuboid side lengths [m]
#[inline]
pub fn make_cuboid_magnet(
    pos: [f64; 3],
    orientation: UnitQuaternion<f64>,
    polarization: [f64; 3],
    dimensions: [f64; 3],
) -> CuboidMagnet {
    CuboidMagnet::new(
        Point3::from(pos),
        orientation,
        Vector3::from(polarization),
        Vector3::from(dimensions),
    )
}

/// Build a `PathCurrent` from simple array inputs.
///
/// - `pos`: `[x, y, z]` position [m] (usually origin)
/// - `orientation`: `UnitQuaternion<f64>` (usually identity)
/// - `current`: current in Amperes
/// - `vertices`: `Vec<[f64; 3]>` polyline vertices [m]
#[inline]
pub fn make_path_current(
    pos: [f64; 3],
    orientation: UnitQuaternion<f64>,
    current: f64,
    vertices: Vec<[f64; 3]>,
) -> PathCurrent {
    let verts: Vec<Vector3<f64>> = vertices.into_iter().map(Vector3::from).collect();
    PathCurrent::new(Point3::from(pos), orientation, current, verts)
}

/// Build a `SourceAssembly` from a vector of magnets.
///
/// The assembly position is the origin; each magnet keeps its own global
/// position. This mirrors Python `magpy.Collection(*magnets)`.
#[inline]
pub fn make_source_assembly(magnets: Vec<CuboidMagnet>) -> SourceAssembly {
    SourceAssembly::new(
        Point3::origin(),
        UnitQuaternion::identity(),
        magnets,
    )
}

/// Compute B-field at a single observation point.
///
/// Works for any `Source` implementor (`CuboidMagnet`, `SourceAssembly`, etc.).
#[inline]
pub fn compute_b_at(source: &impl Source<f64>, point: Point3<f64>) -> Vector3<f64> {
    source.compute_B(point)
}

/// Compute B-field at a batch of observation points.
///
/// Uses magba's built-in batch computation (which parallelizes over source
/// nodes via rayon when the `rayon` feature is enabled).
#[inline]
pub fn compute_b_batch(source: &impl Source<f64>, points: &[Point3<f64>]) -> Vec<Vector3<f64>> {
    source.compute_B_batch(points)
}

/// Compute B-field at a batch of observation points with **point-parallel**
/// rayon iteration.
///
/// This is the preferred path when there are many observation points and
/// a single source (or a `SourceAssembly` whose internal node-parallel
/// reduction is less efficient for small node counts). Each observation
/// point is processed on a separate rayon task.
#[inline]
pub fn compute_b_batch_parallel(
    source: &(impl Source<f64> + Sync),
    points: &[Point3<f64>],
) -> Vec<Vector3<f64>> {
    points.par_iter().map(|p| source.compute_B(*p)).collect()
}

/// Convert `[f64; 3]` to a `Point3<f64>`.
#[inline]
pub fn point3(arr: [f64; 3]) -> Point3<f64> {
    Point3::from(arr)
}

/// Convert a slice of `[f64; 3]` to a `Vec<Point3<f64>>`.
#[inline]
pub fn points3(arr: &[[f64; 3]]) -> Vec<Point3<f64>> {
    arr.iter().map(|a| Point3::from(*a)).collect()
}
