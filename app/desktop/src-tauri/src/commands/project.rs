//! Project persistence commands (kata 0cgm): save the user's working state
//! to a versioned `.pmproj` artifact and load it back.
//!
//! The frontend resolves the path with the native dialog plugin and hands
//! it over — the same convention as `register_routing_plugin` — while ALL
//! serialization, versioning, file I/O, and validation happen here / in
//! [`crate::ipc::project`]. Both handlers are thin wrappers over the pure
//! logic in [`crate::ipc::project`], executed on the blocking thread pool
//! so the main thread never waits on disk.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::ipc::*;

// ===========================================================================
// save_project — REAL (versioned envelope + atomic write)
// ===========================================================================

/// Save the user's working state as a versioned `.pmproj` artifact.
///
/// The artifact is written atomically (temp file + rename) so a failed or
/// interrupted save never truncates the previous file. Saving does NOT
/// require the design to pass validation — work-in-progress is saveable;
/// the load path reports validation findings instead of blocking.
#[tauri::command]
pub async fn save_project(
    path: String,
    project: ProjectStateIpc,
) -> Result<SaveProjectResultIpc, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let saved_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let envelope = ProjectFileEnvelope::build(&project, saved_at_unix_ms);
        let json = serde_json::to_string_pretty(&envelope)
            .map_err(|e| format!("failed to serialize project: {e}"))?;
        write_project_atomic(std::path::Path::new(&path), &json)?;
        Ok(SaveProjectResultIpc {
            path,
            format_version: PROJECT_FORMAT_VERSION,
            saved_at_unix_ms,
        })
    })
    .await
    .map_err(|e| format!("save_project worker failed: {e}"))?
}

// ===========================================================================
// load_project — REAL (parse + migrate + validate)
// ===========================================================================

/// Load a `.pmproj` artifact and return the state to restore plus the
/// load-time design validation findings.
///
/// Artifact-level failures (file missing, corrupt JSON, missing version,
/// newer format version, malformed state) are REJECTED with a specific
/// human-readable error and the frontend leaves the in-progress work
/// untouched. Design-level findings (an invalid but parseable design) are
/// returned in `validation` and do NOT block the restore.
#[tauri::command]
pub async fn load_project(path: String) -> Result<LoadProjectResultIpc, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let json = std::fs::read_to_string(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                format!("could not open \"{path}\": file not found")
            }
            _ => format!("could not read \"{path}\": {e}"),
        })?;
        let (project, source_version) = parse_project_file(&json)?;
        let validation = design_validation(&project);
        Ok(LoadProjectResultIpc {
            project,
            source_format_version: source_version,
            format_version: PROJECT_FORMAT_VERSION,
            validation,
        })
    })
    .await
    .map_err(|e| format!("load_project worker failed: {e}"))?
}
