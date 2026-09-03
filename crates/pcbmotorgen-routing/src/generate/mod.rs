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
//! - opt-in host-side IO fanout generation (`io_fanout`, kata xa0f): connector
//!   pads + fanout traces appended after pattern generation, before
//!   validation, via the `generate_routing_*_with_io` entry points,
//! - adapting a result into the [`PhaseCoil`] presentation,
//! - pattern metadata / parameter lookup and validation,
//! - native & Python plugin registration.

mod adapters;
mod dispatch;
mod io_fanout;
mod params_api;
mod runtime_registry;

pub use adapters::routing_result_to_phase_coils;
pub use dispatch::{
    generate_coils_from_context, generate_routing_report, generate_routing_report_with_io,
    generate_routing_result, generate_routing_result_with_io, register_native_plugin,
    register_python_runner,
};
pub use io_fanout::{IoFanoutEdge, IoFanoutOptions, generate_io_fanout};
pub use params_api::{pattern_metadata, pattern_parameters, validate_routing_params};
pub use runtime_registry::{
    available_pattern_ids, available_pattern_metadata, bundled_registry, register_runtime_pattern,
    unregister_runtime_pattern,
};
