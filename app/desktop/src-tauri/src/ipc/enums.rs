//! IPC enums whose wire format differs from the core: `MagnetArrangementIpc`
//! (PascalCase on the wire), `CommutationModeIpc`, and `PreconditionLevelIpc`.

use serde::{Deserialize, Serialize};

use pcbmotorgen_simulation::params::MagnetArrangement as CoreMagnetArrangement;

/// Permanent magnet arrangement on the carriage.
///
/// Wire format is **PascalCase** to match `types.ts`:
/// `"Alternating" | "AlternatingBackIron" | "Halbach" | "HalbachBackIron"`.
///
/// (The core enum serializes as snake_case, hence this separate IPC enum.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MagnetArrangementIpc {
    Alternating,
    AlternatingBackIron,
    Halbach,
    HalbachBackIron,
}

impl From<MagnetArrangementIpc> for CoreMagnetArrangement {
    fn from(a: MagnetArrangementIpc) -> Self {
        match a {
            MagnetArrangementIpc::Alternating => CoreMagnetArrangement::Alternating,
            MagnetArrangementIpc::AlternatingBackIron => {
                CoreMagnetArrangement::AlternatingBackIron
            }
            MagnetArrangementIpc::Halbach => CoreMagnetArrangement::Halbach,
            MagnetArrangementIpc::HalbachBackIron => CoreMagnetArrangement::HalbachBackIron,
        }
    }
}

impl From<CoreMagnetArrangement> for MagnetArrangementIpc {
    fn from(a: CoreMagnetArrangement) -> Self {
        match a {
            CoreMagnetArrangement::Alternating => MagnetArrangementIpc::Alternating,
            CoreMagnetArrangement::AlternatingBackIron => {
                MagnetArrangementIpc::AlternatingBackIron
            }
            CoreMagnetArrangement::Halbach => MagnetArrangementIpc::Halbach,
            CoreMagnetArrangement::HalbachBackIron => MagnetArrangementIpc::HalbachBackIron,
        }
    }
}

/// FOC commutation strategy. Wire format: `"max_torque" | "phase_a_only"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommutationModeIpc {
    MaxTorque,
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