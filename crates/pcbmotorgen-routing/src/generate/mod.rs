//! App-facing facade over the routing-pattern registry and generators.
//!
//! This is the part of the old `pcbmotorgen` `geometry/routing.rs` facade that
//! is independent of the parent's `LinearMotorConfig`. It operates purely on
//! [`RoutingContext`] and [`coil`](crate::coil) types, so the routing crate is
//! fully decoupled from the parent and the physics/simulation crates.
//!
//! Responsibilities:
//! - the runtime pattern registry (bundled + runtime-loaded plugins) and the
//!   `available_pattern_ids` / register / unregister API,
//! - generating a validated [`RoutingResult`] for a context,
//! - adapting a result into the [`PhaseCoil`] presentation,
//! - pattern metadata / parameter lookup and validation,
//! - native & Python plugin registration.

mod adapters;
mod dispatch;
mod params_api;
mod runtime_registry;

pub use adapters::routing_result_to_phase_coils;
pub use dispatch::{
    generate_coils_from_context, generate_routing_report, generate_routing_result,
    register_native_plugin, register_python_runner,
};
pub use params_api::{pattern_metadata, pattern_parameters, validate_routing_params};
pub use runtime_registry::{
    available_pattern_ids, bundled_registry, register_runtime_pattern, unregister_runtime_pattern,
};
