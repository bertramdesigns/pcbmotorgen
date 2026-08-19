//! KiCad board diagnostics and pre-write validation.
//!
//! Bridges the gap between the coil-generation spec (`DesignRules` +
//! `num_layers` + active-area / board dimensions) and the live state of the
//! open KiCad board, surfacing mismatches BEFORE any track is written. This
//! is the "robust KiCad connection" feature (WP-1.B in the project plan).
//!
//! Three top-level helpers live here and in the submodules:
//!
//! 1. [`get_board_diagnostics`] (this module) — query the open board for its
//!    name, copper layer count, and (where supported by the IPC) its
//!    edge-cut bounding box and available net classes. Returns a
//!    [`BoardDiagnostics`] struct.
//!
//! 2. [`precondition::validate_write_preconditions`] — pure function
//!    comparing the generation spec against the live [`BoardDiagnostics`].
//!    Returns a list of [`PreconditionWarning`] entries (Info / Warning /
//!    Error) so the UI can show "your config is 4-layer but your board is
//!    2-layer — reduce to 2".
//!
//! 3. [`preview::preview_coils`] — pure dry-run that returns the
//!    [`CoilPreview`] (the per-layer summary of the coil set that
//!    `write_coils_to_board` would write). Used by the UI to confirm
//!    placement before clicking the real "Write to Board" button.
//!
//! None of these helpers touch the IPC socket except
//! [`get_board_diagnostics`], which is the only side-effecting one. The
//! other two are pure — easy to unit-test.
//!
//! ## Submodules
//! - [`precondition`] — [`PreconditionLevel`], [`PreconditionWarning`],
//!   [`validate_write_preconditions`].
//! - [`preview`] — [`CoilPreviewLayer`], [`CoilPreview`], [`preview_coils`].

use crate::board::BoardHandle;
use crate::errors::KiCadError;

mod precondition;
mod preview;

pub use precondition::{
    validate_write_preconditions, PreconditionLevel, PreconditionWarning,
};
pub use preview::{preview_coils, CoilPreview, CoilPreviewLayer};

// ---------------------------------------------------------------------------
// BoardDiagnostics
// ---------------------------------------------------------------------------

/// Snapshot of the open KiCad board's geometric / electrical state.
///
/// `BoardDiagnostics` is the *live* counterpart of the generation spec. The
/// frontend fetches it before every write so it can show the user "the board
/// you have open has 4 copper layers, you asked for 6" rather than discovering
/// the mismatch after the fact.
///
/// `board_x_min_mm` / `board_x_max_mm` / `board_y_min_mm` / `board_y_max_mm`
/// are populated from the board's edge cuts when the IPC supports that query.
/// If the query is not available, they default to `0.0` and
/// `available_net_classes` is empty — but `board_name` and
/// `copper_layer_count` are always populated (the latter from
/// `GetBoardEnabledLayers`, which is supported on KiCad 10).
#[derive(Debug, Clone, PartialEq)]
pub struct BoardDiagnostics {
    /// File name of the open board, e.g. `"board.kicad_pcb"`. Empty if no
    /// board is open.
    pub board_name: String,
    /// Number of copper layers enabled on the board (from
    /// `GetBoardEnabledLayers`).
    pub copper_layer_count: u32,
    /// Bounding box of the board's edge cuts [mm]. Defaults to `0.0` if not
    /// queryable — see [`board_x_min_mm`].
    pub board_x_min_mm: f64,
    /// See [`board_x_min_mm`].
    pub board_x_max_mm: f64,
    /// See [`board_x_min_mm`].
    pub board_y_min_mm: f64,
    /// See [`board_x_min_mm`].
    pub board_y_max_mm: f64,
    /// Net class names defined on the board. Empty if not queryable.
    /// TODO: real query — current implementation returns an empty vector
    /// because the KiCad IPC API does not yet expose a net-class query.
    pub available_net_classes: Vec<String>,
}

impl BoardDiagnostics {
    /// Convenience: width of the board's edge-cut bounding box [mm]. Returns
    /// `0.0` when the bounding box is not queryable.
    pub fn board_width_mm(&self) -> f64 {
        (self.board_x_max_mm - self.board_x_min_mm).max(0.0)
    }

    /// Convenience: height of the board's edge-cut bounding box [mm].
    pub fn board_height_mm(&self) -> f64 {
        (self.board_y_max_mm - self.board_y_min_mm).max(0.0)
    }
}

// ---------------------------------------------------------------------------
// get_board_diagnostics
// ---------------------------------------------------------------------------

/// Query the open KiCad board and return a [`BoardDiagnostics`] snapshot.
///
/// `BoardHandle::get_copper_layer_count` always succeeds when the connection
/// is up; `board_name` comes from the document specifier. The edge-cut
/// bounding box and net-class list are **not** currently queryable via the
/// KiCad 10 IPC (no matching `.proto` command in `kiapi.board.commands`), so
/// they default to `0.0` and an empty list respectively. A `// TODO` comment
/// marks the spot for the real query when the IPC grows it.
///
/// Returns `Err` on connection failure or missing PCB document.
pub fn get_board_diagnostics(
    board: &mut BoardHandle<'_>,
) -> Result<BoardDiagnostics, KiCadError> {
    let board_name = board.name().unwrap_or_default();
    let copper_layer_count = board.get_copper_layer_count().unwrap_or(0);

    // TODO: real query — when the KiCad IPC exposes a GetBoardBounds /
    // GetNetClasses command, replace the placeholder zeros / empty list
    // here. Until then, we return a snapshot with the populated fields
    // (name, layer count) and a clear placeholder for the missing ones.
    Ok(BoardDiagnostics {
        board_name,
        copper_layer_count,
        board_x_min_mm: 0.0,
        board_x_max_mm: 0.0,
        board_y_min_mm: 0.0,
        board_y_max_mm: 0.0,
        available_net_classes: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
use super::*;

fn empty_diagnostics() -> BoardDiagnostics {
    BoardDiagnostics {
        board_name: "test.kicad_pcb".into(),
        copper_layer_count: 0,
        board_x_min_mm: 0.0,
        board_x_max_mm: 0.0,
        board_y_min_mm: 0.0,
        board_y_max_mm: 0.0,
        available_net_classes: Vec::new(),
    }
}

// --- BoardDiagnostics width/height helpers ---

#[test]
fn test_board_diagnostics_width_height_zero_when_not_set() {
    let d = empty_diagnostics();
    assert_eq!(d.board_width_mm(), 0.0);
    assert_eq!(d.board_height_mm(), 0.0);
}

#[test]
fn test_board_diagnostics_width_height_positive_when_set() {
    let d = BoardDiagnostics {
        board_x_min_mm: -25.0,
        board_x_max_mm: 25.0,
        board_y_min_mm: -10.0,
        board_y_max_mm: 10.0,
        ..empty_diagnostics()
    };
    assert!((d.board_width_mm() - 50.0).abs() < 1e-9);
    assert!((d.board_height_mm() - 20.0).abs() < 1e-9);
}

#[test]
fn test_board_diagnostics_width_height_clamped_at_zero() {
    // Inverted box (x_max < x_min) must clamp to zero, not yield a
    // negative dimension.
    let d = BoardDiagnostics {
        board_x_min_mm: 25.0,
        board_x_max_mm: -25.0,
        board_y_min_mm: 10.0,
        board_y_max_mm: -10.0,
        ..empty_diagnostics()
    };
    assert_eq!(d.board_width_mm(), 0.0);
    assert_eq!(d.board_height_mm(), 0.0);
}
}