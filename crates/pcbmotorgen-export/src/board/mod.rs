//! High-level handle to an open KiCad board document.
//!
//! [`BoardHandle`] wraps a borrowed [`KiCadClient`] and a target
//! [`DocumentSpecifier`] (the open board). It provides convenience methods for
//! querying board properties and writing coil geometry atomically.
//!
//! ## Submodules
//! - [`write`] — coil-writing orchestration under `BoardHandle` (real write
//!   + dry-run).
//! - [`tally`] — pure per-item failure tallying for write results.

use crate::errors::KiCadError;
use crate::proto::common::types::document_specifier::Identifier;
use crate::proto::common::types::DocumentSpecifier;
use crate::KiCadClient;

mod tally;
mod write;

// Type URLs for board-level queries.
const GET_BOARD_ENABLED_LAYERS_TYPE_URL: &str =
    "type.googleapis.com/kiapi.board.commands.GetBoardEnabledLayers";
const BOARD_ENABLED_LAYERS_RESPONSE_TYPE_URL: &str =
    "type.googleapis.com/kiapi.board.commands.BoardEnabledLayersResponse";

/// KiCad `ItemStatusCode::ISC_OK` (from the `.proto` enum). Per-item
/// `ItemStatus.code == ISC_OK` is the only success indicator — the outer
/// `ItemRequestStatus` reports the *request* status, not the per-item
/// outcomes.
const ITEM_STATUS_OK: i32 = 1;

/// Maximum number of per-item failure messages to surface to the caller.
///
/// Set high enough that, for any realistic failure count, every individual
/// rejection message fits in the IPC response. The previous value of 10
/// silently dropped 89 of the user's 99 failures — the user only saw the
/// first 10 strings and had no way to know what the other 89 were
/// (grouped by error code, message shape, etc.). 1000 is effectively
/// unbounded for any real KiCad write (a typical coil set has a few
/// hundred items at most; even a worst-case 50k-item write would
/// only hit the cap if *every* item failed, in which case the cap is
/// the right behaviour to keep the IPC payload bounded).
const MAX_FAILURES_TO_REPORT: usize = 1000;

/// Result of a [`BoardHandle::write_coils`] call.
///
/// `items_attempted` is the number of items we sent to KiCad;
/// `items_created` is the number KiCad actually accepted (i.e. returned
/// `ISC_OK` in their `ItemStatus`). The two can differ if KiCad rejects
/// individual items (e.g. invalid data, missing layer).
///
/// `failures` contains the first [`MAX_FAILURES_TO_REPORT`] rejection
/// messages verbatim. The total failure count is always recoverable as
/// `items_attempted - items_created`, even if some were truncated.
///
/// `failure_summary` is a compact, **code-grouped** summary of all
/// rejections (not just the surfaced ones): each entry is `(code, count)`
/// where `code` is the `ItemStatus.code` value KiCad returned (e.g. 7 for
/// `ISC_INVALID_DATA`, 2 for `ISC_INVALID_TYPE`) and `count` is the
/// number of items rejected with that code. This is the most useful
/// diagnostic for the UI: instead of listing 99 individual messages that
/// all say the same thing, the UI can render
/// `"99× code=7 (no overlapping layers with the board)"` and the user
/// immediately sees the root cause. Sorted by `(code, count)` descending
/// so the most common failure appears first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteCoilsResult {
    pub items_attempted: u32,
    pub items_created: u32,
    pub failures: Vec<String>,
    /// `(ItemStatus.code, count)` pairs, one entry per distinct
    /// rejection code seen. Sorted by count descending (most-frequent
    /// failure first); ties broken by `code` ascending. Empty when
    /// `items_created == items_attempted`.
    pub failure_summary: Vec<(i32, u32)>,
}

/// High-level handle to the open board document.
pub struct BoardHandle<'a> {
    client: &'a mut KiCadClient,
    document: DocumentSpecifier,
}

impl<'a> BoardHandle<'a> {
    /// Create a handle bound to the given document.
    pub fn new(client: &'a mut KiCadClient, document: DocumentSpecifier) -> Self {
        Self { client, document }
    }

    /// Returns a reference to the underlying document specifier.
    pub fn document(&self) -> &DocumentSpecifier {
        &self.document
    }

    /// Get the board name (filename), e.g. `"board.kicad_pcb"`.
    pub fn name(&self) -> Result<String, KiCadError> {
        match &self.document.identifier {
            Some(Identifier::BoardFilename(name)) => Ok(name.clone()),
            _ => Err(KiCadError::Protocol(
                "document is not a PCB (no board_filename identifier)".to_string(),
            )),
        }
    }

    /// Get the number of copper layers in the board.
    ///
    /// Sends a `GetBoardEnabledLayers` command and reads
    /// `BoardEnabledLayersResponse.copper_layer_count`.
    pub fn get_copper_layer_count(&mut self) -> Result<u32, KiCadError> {
        use crate::proto::board::commands::{
            BoardEnabledLayersResponse, GetBoardEnabledLayers,
        };

        let cmd = GetBoardEnabledLayers {
            board: Some(self.document.clone()),
        };
        let resp: BoardEnabledLayersResponse = self
            .client
            .send::<GetBoardEnabledLayers, BoardEnabledLayersResponse>(
                GET_BOARD_ENABLED_LAYERS_TYPE_URL,
                &cmd,
            )?;
        Ok(resp.copper_layer_count)
    }
}

// Keep the response type URL around for documentation/forward-compat.
#[allow(dead_code)]
const _RESPONSE_TYPE_URLS: &[&str] = &[BOARD_ENABLED_LAYERS_RESPONSE_TYPE_URL];

// ---------------------------------------------------------------------------
// Tests (query methods)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::common::types::DocumentType;

    #[test]
    fn test_board_handle_query_methods() {
        let mut client = KiCadClient::with_transport(
            Box::new(crate::MockTransport::new(Vec::new())),
            Some("test"),
            2000,
        );
        let doc = DocumentSpecifier {
            r#type: DocumentType::DoctypePcb as i32,
            identifier: Some(Identifier::BoardFilename("motor.kicad_pcb".to_string())),
            project: None,
        };
        let board = BoardHandle::new(&mut client, doc);
        assert_eq!(board.name().expect("name"), "motor.kicad_pcb");
        assert!(matches!(
            board.document().identifier,
            Some(Identifier::BoardFilename(_))
        ));
    }

    #[test]
    fn test_board_handle_name_errors_for_non_pcb() {
        let mut client = KiCadClient::with_transport(
            Box::new(crate::MockTransport::new(Vec::new())),
            Some("test"),
            2000,
        );
        let doc = DocumentSpecifier {
            r#type: DocumentType::DoctypeSchematic as i32,
            identifier: None,
            project: None,
        };
        let board = BoardHandle::new(&mut client, doc);
        assert!(board.name().is_err());
    }
}