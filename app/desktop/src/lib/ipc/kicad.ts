/**
 * KiCad IPC calls (board write via KiCad 10 IPC socket).
 *
 * These are user-facing IPC calls (the "Write to Board" / "Connect to
 * KiCad" buttons). Real Tauri errors MUST propagate to the UI so the
 * "0 of 0 written" bug (and its siblings) can't be silently hidden
 * behind a synthetic zero. The mock fallback is only used when the Tauri
 * runtime itself is absent (`vite dev` without the Tauri shell).
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  LinearMotorConfig,
  KicadConnection,
  KicadWriteResult,
  KicadPingResult,
  BoardDiagnostics,
  PreconditionWarning,
  CoilPreview,
} from "../types";
import { isTauriAvailable } from "./core";
import {
  mockBoardDiagnostics,
  mockValidatePreconditions,
  mockPreviewCoils,
} from "./mocks";

export async function connectKicad(): Promise<KicadConnection> {
  if (!isTauriAvailable()) {
    return { connected: false, board_name: "(not connected)", copper_layers: 0 };
  }
  return await invoke<KicadConnection>("connect_kicad");
}

/**
 * Generate coils from the config and write them to the open KiCad board.
 *
 * Pass `dryRun: true` to count the items without sending a commit
 * (`commit_id === "(dry run - no commit)"`, `items_created === 0`). The
 * Rust side still establishes a KiCad connection in dry-run mode — it
 * just skips the `Commit` / `create_items` IPC. Use [`previewCoils`]
 * for a no-IPC dry run (useful when KiCad is not open).
 *
 * **No try/catch here.** A real Tauri error (e.g. "no board open",
 * "connection refused") propagates to the caller — that's the fix for
 * the historical "0 of 0 written" bug.
 */
export async function writeCoilsToBoard(
  config: LinearMotorConfig,
  dryRun: boolean = false,
): Promise<KicadWriteResult> {
  if (!isTauriAvailable()) {
    return {
      items_attempted: 0,
      items_created: 0,
      failures: ["Backend not available — open the Tauri shell to write to KiCad"],
      failure_summary: [],
      commit_id: "",
    };
  }
  return await invoke<KicadWriteResult>("write_coils_to_board", {
    config,
    dryRun,
  });
}

export async function pingKicad(): Promise<KicadPingResult> {
  if (!isTauriAvailable()) return { ok: false, version: "" };
  return await invoke<KicadPingResult>("ping_kicad");
}

// ---------------------------------------------------------------------------
// Board diagnostics + preconditions + preview — WP-KiCad
// ---------------------------------------------------------------------------

/**
 * Live snapshot of the open KiCad board (name, layer count, edge-cut
 * bounding box, net classes). Connects to KiCad each call — cache in the
 * UI if you need it more than once per write.
 */
export async function getBoardDiagnostics(): Promise<BoardDiagnostics> {
  if (!isTauriAvailable()) return mockBoardDiagnostics();
  return await invoke<BoardDiagnostics>("get_board_diagnostics");
}

/**
 * Compare the user's `config` against the live `diagnostics` and return
 * a list of pre-condition warnings (info / warning / error). Pure on
 * the Rust side — no IPC. Errors propagate.
 */
export async function validateWritePreconditions(
  config: LinearMotorConfig,
  diagnostics: BoardDiagnostics,
): Promise<PreconditionWarning[]> {
  if (!isTauriAvailable()) return mockValidatePreconditions(config, diagnostics);
  return await invoke<PreconditionWarning[]>("validate_write_preconditions", {
    config,
    diagnostics,
  });
}

/**
 * Dry-run coil preview: builds the same PhaseCoil set the writer would
 * produce, and returns a per-layer tally (phase count, track count, via
 * count). No KiCad roundtrip — safe to call without KiCad running.
 */
export async function previewCoils(
  config: LinearMotorConfig,
): Promise<CoilPreview> {
  if (!isTauriAvailable()) return mockPreviewCoils(config);
  return await invoke<CoilPreview>("preview_coils", { config });
}