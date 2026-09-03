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
//!    name, copper layer count, edge-cut bounding box (via `GetItems` +
//!    client-side geometry) and the net classes in use (via `GetNets` +
//!    `GetNetClassForNets`). Returns a [`BoardDiagnostics`] struct.
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
/// and `available_net_classes` are populated from the live board
/// (kata ze9f). A snapshot is best-effort: if an individual backing query
/// fails (e.g. KiCad does not handle it, or the board is empty), that field
/// degrades to its neutral default — `0.0` for the bounding box, an empty
/// list for the net classes — while the remaining fields stay real.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardDiagnostics {
    /// File name of the open board, e.g. `"board.kicad_pcb"`. Empty if no
    /// board is open.
    pub board_name: String,
    /// Number of copper layers enabled on the board (from
    /// `GetBoardEnabledLayers`).
    pub copper_layer_count: u32,
    /// Bounding box of the board's edge-cut graphics [mm], derived from
    /// `GetItems` (`KOT_PCB_SHAPE`) filtered to the `Edge.Cuts` layer and
    /// computed client-side — the IPC has no `GetBoardBounds` command.
    /// Defaults to `0.0` when the board has no edge-cut graphics or the
    /// query fails — see [`board_x_min_mm`].
    pub board_x_min_mm: f64,
    /// See [`board_x_min_mm`].
    pub board_x_max_mm: f64,
    /// See [`board_x_min_mm`].
    pub board_y_min_mm: f64,
    /// See [`board_x_min_mm`].
    pub board_y_max_mm: f64,
    /// Names of the net classes effectively in use on the board, sorted.
    /// Backed by `GetNets` + `GetNetClassForNets` (the effective/merged
    /// class per net). This is the set of classes **in use** — not the
    /// project's full netclass list, which the IPC cannot list; a class
    /// with no nets assigned does not appear. Composite (implicit) classes
    /// surface their constituent explicit class names.
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
/// Per-field provenance (kata ze9f):
///
/// | Field                | Backing IPC command(s)                             | Degraded default |
/// |----------------------|----------------------------------------------------|------------------|
/// | `board_name`         | document specifier (no IPC)                        | `""`             |
/// | `copper_layer_count` | `GetBoardEnabledLayers`                            | `0`              |
/// | `board_*_min/max_mm` | `GetItems` (`KOT_PCB_SHAPE`) → `Edge.Cuts` filter, | `0.0`            |
/// |                      | client-side exact bbox (no `GetBoardBounds` in IPC)|                  |
/// | `available_net_classes` | `GetNets` + `GetNetClassForNets` (effective     | `[]`             |
/// |                      | per-net classes; no global netclass list in IPC)   |                  |
///
/// The snapshot is **best-effort**: each backing query degrades to its
/// neutral default on failure (unsupported command, empty board, connection
/// hiccup mid-snapshot) without failing the whole call, so the UI always
/// gets a usable snapshot.
///
/// Note on `GetBoardOrigin`: the IPC exposes it, but it returns a single
/// grid/drill origin *point*, not bounds, and no diagnostics field represents
/// one — it is available separately via [`BoardHandle::get_board_origin`]
/// and deliberately not mixed into the edge-cut bounding box.
///
/// Returns `Err` on connection failure before any query is made.
pub fn get_board_diagnostics(
    board: &mut BoardHandle<'_>,
) -> Result<BoardDiagnostics, KiCadError> {
    let board_name = board.name().unwrap_or_default();
    let copper_layer_count = board.get_copper_layer_count().unwrap_or(0);

    // Edge-cut bounding box: real via GetItems + client-side geometry.
    // Degrades to placeholder zeros when the board has no edge-cut graphics
    // or the query fails.
    let bbox = board.get_edge_cut_bbox_mm().ok().flatten();
    let (board_x_min_mm, board_x_max_mm, board_y_min_mm, board_y_max_mm) = match bbox {
        Some(b) => (b.x_min_mm, b.x_max_mm, b.y_min_mm, b.y_max_mm),
        None => (0.0, 0.0, 0.0, 0.0),
    };

    // Net classes: real via GetNets + GetNetClassForNets (effective class
    // per net). Degrades to an empty list when the board has no nets or a
    // query fails. An empty board sends no GetNetClassForNets request.
    let available_net_classes = board
        .get_net_names()
        .ok()
        .and_then(|nets| board.get_effective_net_classes(&nets).ok())
        .unwrap_or_default();

    Ok(BoardDiagnostics {
        board_name,
        copper_layer_count,
        board_x_min_mm,
        board_x_max_mm,
        board_y_min_mm,
        board_y_max_mm,
        available_net_classes,
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