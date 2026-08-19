//! # pcbmotorgen-routing
//!
//! Routing-pattern plugin contract and DRC authority for PCB stator coil
//! generation. This crate owns everything about trace generation: the pattern
//! plugin interface, generator loading (Rust `cdylib` + Python runners),
//! trace width / via sizing, and overlap (interference) detection.
//!
//! It is a leaf crate — it depends on nothing internal, so it can be developed
//! and tested independently of the physics simulation, the KiCad adapter, and
//! the parent `pcbmotorgen` app.
//!
//! Per the crate division (see `PRODUCT_ARCHITECTURE.md`):
//! - [`model`] — the canonical [`RoutingResult`] (segments, curves, vias),
//!   each element carrying its own `layer` and `net`.
//! - [`coil`] — the [`PhaseCoil`] presentation model (grouped by layer+net)
//!   that the simulation crate and the KiCad writer consume.
//! - [`design`] — [`DesignRules`]: DFM trace width / clearance / via sizing,
//!   the authority downstream consumers read sizes from.
//! - [`dimensions`] — pole pitch, phase-band budget, and bottom-up slot-width
//!   calculations returned in [`RoutingReport`].
//! - [`interference`] — [`check_interference`]: DRC overlap / via-pad
//!   clearance checks against the [`DesignRules`].
//! - [`generate`] — the app-facing facade: registry, plugin loading, and
//!   `RoutingContext` → `RoutingResult`/`PhaseCoil` generation.
//! - [`pattern`] — the [`RoutingPattern`] trait that crate plugins and Python
//!   runners implement.
//! - [`context`] — the flat [`RoutingContext`] snapshot fed to every pattern.
//! - [`validator`] — the single strict-shape gate every result must pass.
//! - [`registry`] — the [`RoutingRegistry`] patterns register into.
//! - [`loaders`] — dynamic loading of patterns (Rust `cdylib` + Python).
//! - [`patterns`] — bundled reference patterns (the `infinity` braid).
//! - [`report`] — the validated geometry plus its design-dimension sidecar.
//!
//! All coordinates and design dimensions are in millimetres; x = travel axis,
//! y = across board width.

pub mod coil;
pub mod context;
pub mod design;
pub mod dimensions;
pub mod error;
pub mod generate;
pub mod interference;
pub mod loaders;
pub mod model;
pub mod pattern;
pub mod patterns;
pub mod report;
pub mod registry;
pub mod validator;

pub use coil::{CoilArc, CoilSegment, PhaseCoil, PHASE_NAMES};
pub use context::RoutingContext;
pub use design::DesignRules;
pub use dimensions::{
    max_slot_width_from_pole_pitch_mm, slot_width_from_trace_geometry_mm, RoutingDimensions,
    SlotWidth,
};
pub use error::RoutingError;
pub use generate::{
    available_pattern_ids, bundled_registry, generate_coils_from_context, generate_routing_report,
    generate_routing_result, pattern_metadata, pattern_parameters, register_native_plugin,
    register_python_runner, register_runtime_pattern, routing_result_to_phase_coils,
    unregister_runtime_pattern, validate_routing_params,
};
pub use interference::{check_interference, InterferenceViolation};
pub use model::{
    Layer, Net, Point, PoleRegion, RouteCurve, RouteSegment, RoutingResult, Via,
};
pub use pattern::{ParamType, PatternParameter, PluginMetadata, RoutingPattern};
pub use report::RoutingReport;
pub use registry::{RoutingErrorKind, RoutingRegistry};
pub use validator::Validator;
