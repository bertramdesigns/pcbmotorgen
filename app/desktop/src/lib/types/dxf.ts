/**
 * DXF export contract (export_coils_dxf) — pure R12 ASCII exporter.
 */

/** Summary of the exported DXF file contents. */
export interface DxfExportSummary {
  /** Number of LINE entities (straight trace segments). */
  total_lines: number;
  /** Number of ARC entities (rounded corners / curves). */
  total_arcs: number;
  /** Number of CIRCLE entities (vias). */
  total_circles: number;
  /** Number of DXF layers defined. */
  layer_count: number;
}

/** Result from `export_coils_dxf`: full DXF R12 ASCII content + summary. */
export interface DxfExportResult {
  /** Complete DXF file content, ready to write to a `.dxf` file. */
  dxf_content: string;
  /** Human-readable summary for UI feedback. */
  summary: DxfExportSummary;
}