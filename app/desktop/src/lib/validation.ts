/**
 * Cross-field design validation rules.
 *
 * These checks explain conflicts between fields without changing the last
 * committed input value. NumberField handles single-field type/range
 * blocking; cross-field auto-repair is intentionally deferred to a later
 * constraint solver iteration.
 *
 * Extracted from ValidationWarning.svelte so the rule engine is unit
 * testable and reusable. `DesignConfigInput` is a structural snapshot of
 * the fields the rules read — `ConfigStore` satisfies it directly.
 */

export type FindingLevel = "error" | "warning";

export interface Finding {
  id: string;
  level: FindingLevel;
  message: string;
}

/** Structural view of the config fields the validation rules consume. */
export interface DesignConfigInput {
  travel_mm: number;
  coil_span_mm: number;
  magnet_count: number;
  magnet_gap_mm: number;
  magnet_cross_width_mm: number;
  active_area_width_mm: number;
  num_layers: number;
  max_layers: number;
  routing_pattern: string;
  windings_per_phase: number;
  min_trace_mm: number;
  min_space_mm: number;
  peak_force_n: number;
  target_force_n: number;
}

/** Run all cross-field checks against `config`. */
export function validateDesign(config: DesignConfigInput): Finding[] {
  const next: Finding[] = [];
  const travel = config.travel_mm;

  if (!Number.isFinite(travel) || travel <= 0) {
    next.push({
      id: "travel-negative",
      level: "error",
      message:
        `Desired center-to-center travel must be positive ` +
        `(the mover array spans ${config.coil_span_mm.toFixed(1)} mm). ` +
        `Current travel = ${Number.isFinite(travel) ? travel.toFixed(1) : "invalid"} mm.`,
    });
  }

  if (config.magnet_count < 2 || config.magnet_count % 2 !== 0) {
    next.push({
      id: "magnet-count",
      level: "error",
      message: `Magnet count must be an even number ≥ 2 (got ${config.magnet_count}).`,
    });
  }

  if (config.magnet_gap_mm < 0) {
    next.push({
      id: "magnet-gap",
      level: "error",
      message: `Magnet gap must be ≥ 0 mm (got ${config.magnet_gap_mm}).`,
    });
  }

  if (config.magnet_cross_width_mm > config.active_area_width_mm) {
    next.push({
      id: "magnet-cross-width",
      level: "warning",
      message:
        `Magnet width across the stator (${config.magnet_cross_width_mm.toFixed(1)} mm) ` +
        `exceeds the active trace width (${config.active_area_width_mm.toFixed(1)} mm). ` +
        "The force-producing area will not be fully coupled.",
    });
  }

  if (config.num_layers < 2 || config.num_layers % 2 !== 0) {
    next.push({
      id: "layer-count",
      level: "error",
      message: `Layer count must be an even number ≥ 2 (got ${config.num_layers}).`,
    });
  } else if (config.num_layers > config.max_layers) {
    next.push({
      id: "layer-limit",
      level: "error",
      message:
        `Layer count (${config.num_layers}) exceeds the configured maximum ` +
        `of ${config.max_layers}.`,
    });
  }

  if (config.routing_pattern === "infinity-braid" && config.num_layers < 2) {
    next.push({
      id: "infinity-layer-requirement",
      level: "error",
      message: "Infinity Braid requires at least two copper layers.",
    });
  }

  const strandHeight = config.active_area_width_mm / Math.max(config.windings_per_phase, 1);
  const minimumStrandHeight = config.min_trace_mm + config.min_space_mm;
  if (config.windings_per_phase > 1 && strandHeight < minimumStrandHeight) {
    next.push({
      id: "strand-clearance",
      level: "warning",
      message:
        `The active width gives each strand ${strandHeight.toFixed(2)} mm, ` +
        `but trace plus clearance requires ${minimumStrandHeight.toFixed(2)} mm. ` +
        "Reduce strands or increase active width.",
    });
  }

  if (config.peak_force_n < config.target_force_n) {
    next.push({
      id: "force-target-order",
      level: "warning",
      message: `Peak force should be at least the continuous target (${config.target_force_n.toFixed(2)} N).`,
    });
  }

  return next;
}

/** True when any rule produced an error-severity finding. */
export function hasErrors(findings: Finding[]): boolean {
  return findings.some((finding) => finding.level === "error");
}