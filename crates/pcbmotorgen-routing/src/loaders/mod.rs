//! Dynamic pattern loaders.
//!
//! Patterns can be loaded from two sources, both of which must produce the
//! canonical [`RoutingResult`] shape and are validated identically by the
//! [`Validator`](crate::validator::Validator):
//!
//! - [`native`] — a Rust `cdylib` exposing a documented C ABI.
//! - [`python`] — a Python runner that consumes the flattened context as JSON
//!   on stdin and emits strict `RoutingResult` JSON on stdout.
//!
//! See `docs/adr/0009-routing-pattern-plugin-interface.md` §4.

pub mod native;
pub mod python;
