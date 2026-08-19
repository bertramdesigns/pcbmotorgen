//! Per-arrangement magnet builders for [`MagnetArray`](super::MagnetArray).
//!
//! Each submodule owns one arrangement family and a `pub(crate)` builder
//! method on `MagnetArray`, which the parent module's `build_assembly`
//! dispatcher calls. (Inherent methods resolve crate-wide, so no re-exports
//! are needed.)

pub(crate) mod alternating;
pub(crate) mod back_iron;
pub(crate) mod halbach;