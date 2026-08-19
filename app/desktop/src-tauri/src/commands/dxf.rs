//! DXF export command (pure `pcbmotorgen-export` R12 ASCII export).

use crate::ipc::*;

// ===========================================================================
// export_coils_dxf — REAL (pcbmotorgen-export pure DXF R12 ASCII export)
// ===========================================================================

/// DXF export result returned to the frontend.
///
/// The frontend writes `dxf_content` to a file selected by the user via
/// `dialog.save()`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DxfExportResult {
    /// Full DXF R12 ASCII content, ready to write to a `.dxf` file.
    pub dxf_content: String,
    /// Human-readable summary for UI feedback.
    pub summary: DxfExportSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DxfExportSummary {
    pub total_lines: u32,
    pub total_arcs: u32,
    pub total_circles: u32,
    pub layer_count: u32,
}

/// Generate coil geometry from the config and return it as a DXF R12 ASCII
/// string suitable for mechanical CAD / CAM import.
///
/// The command builds the same `PhaseCoil` set as `write_coils_to_board`,
/// converts it to DXF via `pcbmotorgen_export::phase_coils_to_dxf`, and returns
/// the complete file content. The frontend is responsible for saving to disk.
#[tauri::command]
pub async fn export_coils_dxf(
    config: LinearMotorConfigIpc,
) -> Result<DxfExportResult, String> {
    let core = config.to_core();
    tauri::async_runtime::spawn_blocking(move || {
        let coils = core.generate_coils_for_board();
        let num_layers = core.num_layers;
        let rules = core.design_rules();
        let active = core.active_area_length_m * 1e3;

        let dxf_content =
            pcbmotorgen_export::phase_coils_to_dxf(&coils, num_layers, &rules, active);

        let total_lines = dxf_content.matches("0\nLINE\n").count() as u32;
        let total_arcs = dxf_content.matches("0\nARC\n").count() as u32;
        let total_circles = dxf_content.matches("0\nCIRCLE\n").count() as u32;

        // Count unique layer names from the DXF LAYER definitions in the
        // TABLES section (each `LAYER\n  2\n<name>` pair).
        let layer_count = dxf_content
            .match_indices("LAYER\n  2\n")
            .count() as u32;

        Ok(DxfExportResult {
            dxf_content,
            summary: DxfExportSummary {
                total_lines,
                total_arcs,
                total_circles,
                layer_count,
            },
        })
    })
    .await
    .map_err(|e| format!("export_coils_dxf worker failed: {e}"))?
}
