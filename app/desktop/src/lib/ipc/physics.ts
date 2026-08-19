/**
 * Physics IPC calls with deterministic mock fallback.
 *
 * The UI schedules coil generation independently for the design reflection
 * and gates Simulation-tab calculations on the active tab. Their failure is
 * recoverable — the user just sees stale data. When the Tauri runtime is
 * missing (plain `vite dev`), the corresponding mock is returned instead.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  LinearMotorConfig,
  ConfigDerived,
  CoilPathDto,
  ForceSweepResult,
  StackupResultDto,
  HeightStackResultDto,
  FrictionBudgetDto,
  PowerBudgetDto,
  BFieldGridDto,
} from "../types";
import { isTauriAvailable } from "./core";
import {
  mockConfigDerived,
  mockCoils,
  mockForceSweep,
  mockHeightStack,
  mockPowerBudget,
  mockFriction,
  mockStackup,
  mockBFieldGrid,
} from "./mocks";

export async function computeConfigDerived(
  config: LinearMotorConfig,
): Promise<ConfigDerived> {
  if (!isTauriAvailable()) return mockConfigDerived(config);
  return await invoke<ConfigDerived>("compute_config_derived", { config });
}

export async function generateCoils(
  config: LinearMotorConfig,
): Promise<CoilPathDto> {
  if (!isTauriAvailable()) return mockCoils(config);
  return await invoke<CoilPathDto>("generate_coils", { config });
}

export async function evaluateForceSweep(
  config: LinearMotorConfig,
): Promise<ForceSweepResult> {
  if (!isTauriAvailable()) return mockForceSweep(config);
  return await invoke<ForceSweepResult>("evaluate_force_sweep", { config });
}

export async function computeHeightStack(
  config: LinearMotorConfig,
): Promise<HeightStackResultDto> {
  if (!isTauriAvailable()) return mockHeightStack(config);
  return await invoke<HeightStackResultDto>("compute_height_stack", { config });
}

export async function computePowerBudget(
  config: LinearMotorConfig,
): Promise<PowerBudgetDto> {
  if (!isTauriAvailable()) return mockPowerBudget(config);
  return await invoke<PowerBudgetDto>("compute_power_budget", { config });
}

export async function computeFriction(
  config: LinearMotorConfig,
): Promise<FrictionBudgetDto> {
  if (!isTauriAvailable()) return mockFriction(config);
  return await invoke<FrictionBudgetDto>("compute_friction", { config });
}

export async function computeStackup(
  config: LinearMotorConfig,
): Promise<StackupResultDto> {
  if (!isTauriAvailable()) return mockStackup(config);
  return await invoke<StackupResultDto>("compute_stackup", { config });
}

/**
 * Sample the B-field on an X–Z grid for the active magnet arrangement.
 * Returns a flat row-major array (Z slow axis) of B-vectors + positions.
 *
 * Defaults: 24×12 grid, x = [0, active_area_length_m],
 * z = [0, air_gap + magnet_height + 2 mm] (a 2 mm window above the magnet top).
 */
export async function sampleBField(
  config: LinearMotorConfig,
  n_x: number = 24,
  n_z: number = 12,
  x_extent_m: [number, number] = [0, config.active_area_length_m],
  z_extent_m: [number, number] = [
    0,
    config.air_gap_m + config.magnet_height_m + 2e-3,
  ],
): Promise<BFieldGridDto> {
  if (!isTauriAvailable()) {
    return mockBFieldGrid(config, n_x, n_z, x_extent_m, z_extent_m);
  }
  return await invoke<BFieldGridDto>("sample_b_field", {
    config,
    nX: n_x,
    nZ: n_z,
    xExtentM: x_extent_m,
    zExtentM: z_extent_m,
  });
}
