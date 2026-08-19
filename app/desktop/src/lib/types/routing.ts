/**
 * Routing-pattern plugin contract: catalog metadata, user-editable
 * parameter definitions, installed plugins, and DRC interference results.
 */

/** One selectable coil routing pattern offered by the backend. */
export interface RoutingPatternInfo {
  /** Stable machine id, e.g. `"infinity-braid"`. Stored on the config as
   *  `routing_pattern`. */
  id: string;
  /** Human-friendly label shown in the UI, e.g. "Infinity Braid (pcbBraid)". */
  display_name: string;
}

/** Routing-pattern parameter value type (int = whole number, float = real). */
export type ParamType = "int" | "float";

/** One user-editable parameter exposed by a routing-pattern plugin. */
export interface RoutingParamDef {
  key: string;
  label: string;
  description: string;
  param_type: ParamType;
  default: number;
  min?: number;
  max?: number;
  step?: number;
}

/** One installed routing-pattern plugin (from the persistent store). */
export interface InstalledPlugin {
  id: string;
  kind: "native" | "python";
  display_name: string;
  author: string;
  version: string;
  description: string;
}

/** One DRC interference violation reported by the core (clearance checks). */
export interface InterferenceViolation {
  layer: number;
  net_a: string;
  net_b: string;
  /** `"clearance"` | `"via_clearance"` */
  kind: string;
  /** Measured gap [mm]. */
  gap_mm: number;
  message: string;
}