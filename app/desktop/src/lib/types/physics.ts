/**
 * Simulation result contracts: force sweep, stackup / power / friction
 * budgets, and the B-field sampling grid.
 */

import type { CommutationMode } from "./enums";

/** Force-vs-position sweep (evaluate_force_sweep). */
export interface ForceSweepResult {
  positions_m: number[];
  force_x_n: number[];
  force_y_n: number[];
  force_z_n: number[];
  per_phase_force_x: number[][];
  commutation: CommutationMode;
  current_a: number;
  mean_thrust_n: number;
  peak_thrust_n: number;
  min_thrust_n: number;
  ripple_pct: number;
  n_positions: number;
}

/** Layer stackup summary (compute_stackup). */
export interface StackupResultDto {
  layer_count: number;
  trace_widths_m: number[];
  cu_thickness_m: number[];
  via_drill_m: number;
  via_annular_ring_m: number;
  via_grid_rows: number;
  via_grid_cols: number;
  estimated_force_n: number;
  estimated_dc_resistance_ohm: number;
  notes: string[];
}

/** Height stack cross-section (compute_height_stack). */
export interface HeightStackResultDto {
  pcb_thickness_m: number;
  cu_protrusion_m: number;
  solder_mask_m: number;
  air_gap_m: number;
  magnet_height_m: number;
  tolerance_m: number;
  total_height_m: number;
}

/** Friction budget breakdown (compute_friction). */
export interface FrictionBudgetDto {
  bearing_friction_n: number;
  cable_drag_n: number;
  wiper_contact_n: number;
  cogging_n: number;
  total_n: number;
  minimum_drive_force_n: number;
}

/** Power / thermal budget (compute_power_budget). */
export interface PowerBudgetDto {
  phase_resistance_ohm: number;
  continuous_power_w: number;
  burst_power_w: number;
  temperature_rise_c: number;
  capacitor_required_uf: number;
  efficiency_pct: number;
}

/** One B-field sample on the X–Z sampling grid (sample_b_field). */
export interface BFieldSampleDto {
  x_m: number;
  z_m: number;
  bx_t: number;
  by_t: number;
  bz_t: number;
  mag_t: number;
}

/** B-field grid (sample_b_field). */
export interface BFieldGridDto {
  samples: BFieldSampleDto[];
  /** [x_min, x_max] [m] */
  x_extent_m: [number, number];
  /** [z_min, z_max] [m] */
  z_extent_m: [number, number];
}