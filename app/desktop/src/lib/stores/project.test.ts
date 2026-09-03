import { describe, expect, it } from "vitest";
import { ConfigStore } from "./config.svelte";
import { MotionStore } from "./motion.svelte";
import { ProjectStore } from "./project.svelte";
import type { ProjectState } from "../types";

/**
 * Unit coverage of the frontend HALF of project persistence (kata 0cgm):
 * snapshot mapping, dirty tracking, and the restore mapping. The Rust
 * artifact round-trip lives in `src-tauri/src/ipc/project.rs`.
 */

function makeStores() {
  const config = new ConfigStore();
  const motion = new MotionStore(config);
  const projects = new ProjectStore(config, motion);
  return { config, motion, projects };
}

/** Apply a realistic set of edits across all persisted groups. */
function editDesign(config: ConfigStore, motion: MotionStore): void {
  config.desired_travel_mm = 60;
  config.active_area_width_mm = 25;
  config.strands_per_phase = 1;
  config.magnet_count = 16;
  config.magnet_width_mm = 5.0;
  config.magnet_cross_width_mm = 12;
  config.magnet_height_mm = 3.5;
  config.magnet_grade = "N52";
  config.magnet_remanence_t = 1.43;
  config.air_gap_mm = 0.75;
  config.electrical_pitch_mm = 14;
  config.routing_pattern = "quad-stack";
  config.setRoutingParam("num_strands", 3);
  config.phases = 4;
  config.num_layers = 6;
  config.max_current_a = 2.5;
  config.supply_voltage_v = 12;
  config.target_force_n = 1.2;
  config.peak_force_n = 2.0;
  config.friction_n = 0.1;
  config.carriage_mass_kg = 0.02;
  config.max_accel_m_s2 = 3;
  config.capacitor_bank_uf = 2200;
  config.commutation = "phase_a_only";
  config.n_positions = 100;
  config.meshing = 30;
  config.min_trace_mm = 0.2;
  config.min_space_mm = 0.2;
  config.min_via_drill_mm = 0.3;
  config.min_via_annular_ring_mm = 0.15;
  config.pcb_thickness_mm = 2.0;
  config.max_layers = 8;
  config.drive_frequency_hz = 1000;
  config.max_temperature_rise_c = 30;
  motion.positionMm = 42.5;
}

describe("ProjectStore snapshot mapping", () => {
  it("captures every persisted input group in UI units", () => {
    const { config, motion, projects } = makeStores();
    editDesign(config, motion);
    const state = projects.snapshotIpc();

    expect(state.mover_position_mm).toBe(42.5);
    const c = state.config;
    expect(c.desired_travel_mm).toBe(60);
    expect(c.magnet_grade).toBe("N52");
    expect(c.routing_pattern).toBe("quad-stack");
    expect(c.routing_params.num_strands).toBe(3);
    expect(c.commutation).toBe("phase_a_only");
    expect(c.num_layers).toBe(6);
    expect(c.min_trace_mm).toBe(0.2);
    expect(c.capacitor_bank_uf).toBe(2200);
  });

  it("serializes routing params with sorted keys for a stable snapshot", () => {
    const a = makeStores();
    const b = makeStores();
    // Same two params inserted in opposite orders across two stores.
    a.config.setRoutingParam("aaa", 1);
    a.config.setRoutingParam("zzz", 2);
    b.config.setRoutingParam("zzz", 2);
    b.config.setRoutingParam("aaa", 1);
    expect(a.projects.snapshotJson).toBe(b.projects.snapshotJson);
  });
});

describe("ProjectStore dirty tracking", () => {
  it("is clean until the first baseline (untitled design)", () => {
    const { projects } = makeStores();
    expect(projects.savedSnapshot).toBeNull();
    expect(projects.isDirty).toBe(false);
  });

  it("goes dirty on an input change after a baseline, clean after markClean", () => {
    const { config, projects } = makeStores();
    projects.markClean();
    expect(projects.isDirty).toBe(false);

    config.desired_travel_mm = 80;
    expect(projects.isDirty).toBe(true);

    projects.markClean();
    expect(projects.isDirty).toBe(false);
  });

  it("detects mover-position edits", () => {
    const { motion, projects } = makeStores();
    projects.markClean();
    motion.positionMm = 10;
    expect(projects.isDirty).toBe(true);
  });

  it("reports the file name and untitled label", () => {
    const { projects } = makeStores();
    expect(projects.label).toBe("untitled design");
    projects.currentPath = "/home/user/designs/my-motor.pmproj";
    expect(projects.fileName).toBe("my-motor.pmproj");
    expect(projects.label).toBe("my-motor.pmproj");
  });
});

describe("ProjectStore restore mapping", () => {
  it("round-trips a snapshot into a fresh store set", () => {
    const source = makeStores();
    editDesign(source.config, source.motion);
    const state: ProjectState = source.projects.snapshotIpc();

    const target = makeStores();
    target.projects.applyToState(state);

    // The canonical snapshot of the restored stores equals the snapshot
    // of the source — full state restore, no drift.
    expect(target.projects.snapshotJson).toBe(source.projects.snapshotJson);
    expect(target.motion.positionMm).toBe(42.5);
  });

  it("bumps routing_params_version so scheduling effects re-run", () => {
    const { config, projects } = makeStores();
    const before = config.routing_params_version;
    projects.applyToState({
      config: { ...projects.snapshotIpc().config },
      mover_position_mm: 10,
    });
    expect(config.routing_params_version).toBeGreaterThan(before);
  });

  it("copies routing params (later store mutations do not leak back)", () => {
    const source = makeStores();
    source.config.setRoutingParam("num_strands", 3);
    const state = source.projects.snapshotIpc();

    const target = makeStores();
    target.projects.applyToState(state);
    target.config.setRoutingParam("num_strands", 9);

    expect(source.config.routing_params.num_strands).toBe(3);
    expect(target.config.routing_params.num_strands).toBe(9);
  });
});
