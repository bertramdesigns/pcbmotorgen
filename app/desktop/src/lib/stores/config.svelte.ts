/**
 * pcbmotorgen — central configuration store (Svelte 5 runes class).
 *
 * Keeps user-facing values in mm (the natural unit for PCB design) and
 * exposes a `toIpc()` builder that converts to the SI LinearMotorConfig
 * expected by the Tauri backend. Derived geometry (pole pitch, coil span,
 * travel) is computed with `$derived` so every consumer updates live.
 */

import type {
  LinearMotorConfig,
  SpacingRatio,
  MagnetArrangement,
  CommutationMode,
  RoutingPatternInfo,
  RoutingParamDef,
} from "../types";
import { getRemanence } from "../types";
import { mm, listRoutingPatterns, routingPatternParameters, loadInstalledPlugins } from "../ipc";

const SPACING_RATIO_MAP: Record<SpacingRatio, number> = {
  "1:1": 1.0,
  "4:5": 4 / 5,
  "5:6": 5 / 6,
};

/** Default back-iron thickness applied when the user enables a BackIron
 *  magnet arrangement for the first time. Only used when the user has
 *  not already configured a non-zero value. Exported so the auto-default
 *  behavior (in MagnetsPanel.svelte) can reference the same value. */
export const DEFAULT_BACK_IRON_THICKNESS_MM = 1.0;

export const BACK_IRON_ARRANGEMENTS: ReadonlySet<MagnetArrangement> = new Set([
  "AlternatingBackIron",
  "HalbachBackIron",
]);

export class ConfigStore {
  // --- Topology ----------------------------------------------------------
  topology = $state<"linear" | "radial">("linear");

  // --- Active area (mm) --------------------------------------------------
  // The travel-axis length of the copper active area is DERIVED from the
  // desired center-to-center travel (set in the overview constraints) plus
  // the mover's coil span. Only the cross-axis width is a free general
  // input.
  desired_travel_mm = $state(75);
  active_area_width_mm = $state(20);

  // --- Round 9: padding + multi-strand (defaults match the Rust core) -
  // These are the new defaults that the production code path uses
  // for the MagneticFader reference design. With
  // `windings_per_phase = 2` and `padding_mm = 30`:
  //
  // - Each phase gets 2 parallel serpentine paths on its assigned
  //   layer (stacked in y, interleaved in x).
  // - The 30 mm padding gives the strands' offset x positions extra
  //   room at the ends of the active area for routing.
  // - The multi-strand design is single-layer per phase, so no
  //   inter-layer connections, no through-vias, no buried vias —
  //   directly addresses the Bug 17 through-via short-circuit
  //   symptom the user reported in Round 8.
  padding_mm = $state(30);
  windings_per_phase = $state(2);

  // --- Magnet array (mm) -------------------------------------------------
  magnet_count = $state(10);
  magnet_width_mm = $state(10);
  magnet_cross_width_mm = $state(10);
  magnet_gap_mm = $state(2);
  magnet_height_mm = $state(4);
  magnet_grade = $state("N44");
  magnet_remanence_t = $state(1.34);
  magnet_arrangement = $state<MagnetArrangement>("Alternating");
  back_iron_thickness_mm = $state(0);
  air_gap_mm = $state(0.5);

  // --- Coil --------------------------------------------------------------
  /** Routing-pattern id sent to the backend (one of `routing_patterns`). */
  routing_pattern = $state<string>("infinity-braid");
  /** Routing patterns offered by the backend (`list_routing_patterns`). */
  routing_patterns = $state<RoutingPatternInfo[]>([]);
  /** User-editable routing-pattern parameter values, keyed by `def.key`
   *  (e.g. `num_strands`, `n_periods`). Empty = pattern defaults. Sent to
   *  the backend as `routing_params`. */
  routing_params = $state<Record<string, number>>({});
  /** Declared user-editable parameter definitions for the currently
   *  selected routing pattern (`routing_pattern_parameters`). */
  routing_param_defs = $state<RoutingParamDef[]>([]);
  /** Monotonic bump counter for `routing_params` edits. `$state` proxies
   *  only track the exact property read, so mutating `routing_params[key]`
   *  in place would NOT re-trigger dependent scheduling `$effect`s. Bumping
   *  this counter after every param write gives the debounced preview and
   *  simulation effects a stable signal to react to. */
  routing_params_version = $state(0);
  phases = $state(3);
  spacing_ratio_label = $state<SpacingRatio>("1:1");
  num_layers = $state(4);

  // --- Drive / electrical ------------------------------------------------
  max_current_a = $state(1.0);
  supply_voltage_v = $state(5.0);

  // --- Force targets / mechanical ---------------------------------------
  target_force_n = $state(0.5);
  peak_force_n = $state(1.0);
  friction_n = $state(0.05);
  carriage_mass_kg = $state(0.015);
  max_accel_m_s2 = $state(2.0);
  capacitor_bank_uf = $state(1000);

  // --- Solver -----------------------------------------------------------
  commutation = $state<CommutationMode>("max_torque");
  n_positions = $state(50);
  meshing = $state(20);

  // --- PCB manufacturing defaults (mm) ---------------------------------
  min_trace_mm = $state(0.127);
  min_space_mm = $state(0.127);
  min_via_drill_mm = $state(0.2);
  min_via_annular_ring_mm = $state(0.1);
  pcb_thickness_mm = $state(1.6);
  max_layers = $state(12);
  drive_frequency_hz = $state(500);
  max_temperature_rise_c = $state(20);

  // ---------------------------------------------------------------------
  // Derived geometry (mm, for the UI)
  // ---------------------------------------------------------------------

  pole_pitch_mm = $derived(this.magnet_width_mm + this.magnet_gap_mm);
  coil_span_mm = $derived(this.magnet_count * this.pole_pitch_mm);
  /** Active-area length along the travel axis: mover span + desired travel. */
  active_area_length_mm = $derived(this.coil_span_mm + this.desired_travel_mm);
  travel_mm = $derived(this.active_area_length_mm - this.coil_span_mm);
  spacing_ratio = $derived(SPACING_RATIO_MAP[this.spacing_ratio_label]);

  /** True when the active area is too short to cover the mover coil span. */
  is_active_area_invalid = $derived(this.active_area_length_mm <= this.coil_span_mm);

  /** Sync remanence from the selected grade unless "Custom". */
  syncGrade(): void {
    if (this.magnet_grade === "Custom") return;
    try {
      this.magnet_remanence_t = getRemanence(this.magnet_grade);
    } catch {
      // unknown grade — leave remanence untouched
    }
  }

  /**
   * Load the routing patterns the backend can generate and populate
   * `routing_patterns`. Call during app init; failures are swallowed so a
   * missing backend leaves an empty list (the selector shows "Loading…"
   * until the list is populated).
   */
  async loadRoutingPatterns(): Promise<void> {
    try {
      this.routing_patterns = await listRoutingPatterns();
    } catch {
      // backend unavailable or errored — keep the current (possibly empty) list
    }
    // Startup re-registration: reload patterns first, then re-register any
    // persisted plugins so the freshly-loaded catalog includes them. Errors
    // are per-plugin strings (empty = all ok); surface as a console warn so a
    // broken persisted plugin doesn't break startup but is still observable.
    try {
      const loadErrors = await loadInstalledPlugins();
      if (loadErrors.length > 0) {
        console.warn(
          `[plugins] ${loadErrors.length} installed plugin(s) failed to load:`,
          loadErrors,
        );
      }
    } catch (e) {
      console.warn("[plugins] loadInstalledPlugins failed:", e);
    }
  }

  /**
   * Load the declared user-editable parameter definitions for a routing
   * pattern and reseed `routing_params` so every declared parameter has a
   * value in the UI.
   *
   * For each declared `def` we only write a value when the user hasn't
   * already set one for *this* pattern (tracked by key). This keeps the
   * user's tweaks across pattern switches while still showing a sensible
   * default for newly-exposed parameters.
   */
  async loadRoutingParams(patternId: string): Promise<void> {
    try {
      this.routing_param_defs = await routingPatternParameters(patternId);
    } catch {
      // backend unavailable or errored — leave current defs in place
      this.routing_param_defs = [];
      return;
    }
    const userSet = new Set(Object.keys(this.routing_params));
    for (const def of this.routing_param_defs) {
      if (!userSet.has(def.key)) {
        this.routing_params[def.key] = def.default;
      }
    }
    this.routing_params_version += 1;
  }

  /**
   * Set one routing-pattern parameter value and bump the version counter so
   * the App.svelte preview/simulation scheduling effects re-run. Use this
   * instead of mutating `routing_params[key]` directly (which won't propagate
   * to effects).
   */
  setRoutingParam(key: string, value: number): void {
    this.routing_params[key] = value;
    this.routing_params_version += 1;
  }

  /** Build the SI LinearMotorConfig for the Tauri backend. */
  toIpc(): LinearMotorConfig {
    return {
      active_area_length_m: mm(this.active_area_length_mm),
      board_width_m: mm(this.active_area_width_mm),
      pcb_thickness_m: mm(this.pcb_thickness_mm),
      // Round 9: padding + multi-strand pass-through to the core.
      padding_m: mm(this.padding_mm),
      windings_per_phase: this.windings_per_phase,

      magnet_count: this.magnet_count,
      magnet_width_m: mm(this.magnet_width_mm),
      magnet_cross_width_m: mm(this.magnet_cross_width_mm),
      magnet_height_m: mm(this.magnet_height_mm),
      magnet_gap_m: mm(this.magnet_gap_mm),
      magnet_pitch_m: mm(this.pole_pitch_mm),

      magnet_remanence_t: this.magnet_remanence_t,
      magnet_grade: this.magnet_grade,
      magnet_arrangement: this.magnet_arrangement,
      back_iron_thickness_m: mm(this.back_iron_thickness_mm),
      air_gap_m: mm(this.air_gap_mm),

      routing_pattern: this.routing_pattern,
      routing_params: this.routing_params,
      phases: this.phases,
      spacing_ratio: this.spacing_ratio,

      max_current_a: this.max_current_a,
      supply_voltage_v: this.supply_voltage_v,

      num_layers: this.num_layers,
      min_trace_m: mm(this.min_trace_mm),
      min_space_m: mm(this.min_space_mm),
      min_via_drill_m: mm(this.min_via_drill_mm),
      min_via_annular_ring_m: mm(this.min_via_annular_ring_mm),
      max_layers: this.max_layers,
      drive_frequency_hz: this.drive_frequency_hz,
      max_temperature_rise_c: this.max_temperature_rise_c,

      target_force_n: this.target_force_n,
      peak_force_n: this.peak_force_n,
      friction_n: this.friction_n,
      carriage_mass_kg: this.carriage_mass_kg,
      max_accel_m_s2: this.max_accel_m_s2,
      capacitor_bank_uf: this.capacitor_bank_uf,

      commutation: this.commutation,
      n_positions: this.n_positions,
      meshing: this.meshing,

      name: null,
    };
  }
}

export const config = new ConfigStore();
