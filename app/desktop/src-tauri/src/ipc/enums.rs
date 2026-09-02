//! IPC enums whose wire format differs from the core: `CommutationModeIpc`
//! and `PreconditionLevelIpc`.

use serde::{Deserialize, Serialize};

/// FOC commutation strategy. Wire format: `"max_thrust" | "phase_a_only"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommutationModeIpc {
    /// Legacy alias: persisted payloads written before the glossary
    /// alignment used `"max_torque"`.
    #[serde(alias = "max_torque")]
    MaxThrust,
    PhaseAOnly,
}

/// Severity of a [`PreconditionWarningIpc`]. Wire format is **snake_case**
/// (`"info" | "warning" | "error"`) so the UI can colour-code by value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreconditionLevelIpc {
    Info,
    Warning,
    Error,
}