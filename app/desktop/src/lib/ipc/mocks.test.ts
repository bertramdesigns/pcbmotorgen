/**
 * Shape tests for the deterministic mock backend: every mock must return a
 * structurally valid DTO so the UI can render it offline.
 */

import { describe, expect, it } from "vitest";
import {
  mockConfigDerived,
  mockCoils,
  mockForceSweep,
  mockHeightStack,
  mockPowerBudget,
  mockFriction,
  mockStackup,
  mockBoardDiagnostics,
  mockPreviewCoils,
  mockDxfExportResult,
  mockBFieldGrid,
  mockTravelEnvelope,
  mockMagnetGrades,
} from "./mocks";
import type { LinearMotorConfig } from "../types";
import { computePreviewGeometry, computeMagnets } from "../previewGeometry";

function makeConfig(overrides: Partial<LinearMotorConfig> = {}): LinearMotorConfig {
  return {
    active_area_length_m: 0.195,
    board_width_m: 0.02,
    pcb_thickness_m: 0.0016,
    padding_m: 0.03,
    windings_per_phase: 2,
    magnet_count: 10,
    magnet_width_m: 0.01,
    magnet_cross_width_m: 0.01,
    magnet_height_m: 0.004,
    magnet_gap_m: 0.002,
    magnet_pitch_m: 0.012,
    magnet_remanence_t: 1.34,
    magnet_grade: "N44",
    air_gap_m: 0.0005,
    routing_pattern: "infinity-braid",
    routing_params: {},
    phases: 3,
    spacing_ratio: 1,
    max_current_a: 1.0,
    supply_voltage_v: 5.0,
    num_layers: 4,
    min_trace_m: 0.000127,
    min_space_m: 0.000127,
    min_via_drill_m: 0.0002,
    min_via_annular_ring_m: 0.0001,
    max_layers: 12,
    drive_frequency_hz: 500,
    max_temperature_rise_c: 20,
    target_force_n: 0.5,
    peak_force_n: 1.0,
    friction_n: 0.05,
    carriage_mass_kg: 0.015,
    max_accel_m_s2: 2.0,
    capacitor_bank_uf: 1000,
    commutation: "max_torque",
    n_positions: 50,
    meshing: 20,
    name: null,
    ...overrides,
  };
}

describe("mockConfigDerived", () => {
  it("derives the same geometry the real backend reports", () => {
    const c = makeConfig();
    const d = mockConfigDerived(c);
    expect(d.pole_pitch_m).toBe(c.magnet_pitch_m);
    expect(d.coil_span_m).toBe(c.magnet_count * c.magnet_pitch_m);
    expect(d.travel_m).toBeCloseTo(c.active_area_length_m - c.magnet_count * c.magnet_pitch_m);
    expect(d.min_via_pad_m).toBe(c.min_via_drill_m + 2 * c.min_via_annular_ring_m);
    expect(d.minimum_drive_force_n).toBe(c.friction_n * 1.3);
  });
});

describe("mockMagnetGrades", () => {
  it("mirrors the static TS grade catalog in wire shape", () => {
    const grades = mockMagnetGrades();
    expect(grades.length).toBeGreaterThanOrEqual(6);
    for (const g of grades) {
      expect(typeof g.name).toBe("string");
      expect(g.br_min_t).toBeLessThanOrEqual(g.br_typ_t);
      expect(g.br_typ_t).toBeLessThanOrEqual(g.br_max_t);
      expect(Object.keys(g.max_temp_c).length).toBeGreaterThan(0);
    }
  });
});

describe("mockCoils", () => {
  it("produces one coil per (phase × layer) with active + end-turn segments", () => {
    const c = makeConfig();
    const coils = mockCoils(c);
    // One coil per (phase, layer) pair — the preview must render every
    // layer, mirroring the real backend's multi-layer output.
    expect(coils.phases).toHaveLength(c.phases * c.num_layers);
    expect(coils.layer_count).toBe(c.num_layers);
    const layerIdxMax = Math.max(...coils.phases.map((p) => p.layer_idx));
    expect(layerIdxMax).toBe(c.num_layers - 1);
    const phaseIdxMax = Math.max(...coils.phases.map((p) => p.phase_idx));
    expect(phaseIdxMax).toBe(c.phases - 1);
    for (const ph of coils.phases) {
      expect(ph.phase_idx).toBeGreaterThanOrEqual(0);
      expect(ph.segments.length).toBeGreaterThan(0);
      const actives = ph.segments.filter((s) => s.is_active);
      expect(actives.length).toBeGreaterThan(0);
      expect(ph.active_conductor_count).toBe(actives.length);
    }
  });

  it("emits representative pole regions (one per phase per pole pitch)", () => {
    const c = makeConfig(); // active_area 0.195, padding 0.03, pitch 0.012
    const regions = mockCoils(c).routing_dimensions!.pole_regions;
    expect(regions.length).toBeGreaterThan(0);
    // Mirrors the infinity braid: regions tile the FULL routing domain
    // (active area + 2 × padding), one per phase per pole pitch.
    const regionsPerPhase = Math.floor(
      (c.active_area_length_m + 2 * c.padding_m) / c.magnet_pitch_m,
    );
    expect(regionsPerPhase).toBeGreaterThan(0);
    expect(regions).toHaveLength(regionsPerPhase * c.phases);
    const labels = new Set(regions.map((r) => r.phase));
    expect(labels).toEqual(new Set(["A", "B", "C"]));
    for (const r of regions) {
      expect(["A", "B", "C"]).toContain(r.phase);
      expect(r.pole_index).toBeGreaterThanOrEqual(0);
      for (const coord of [r.start[0], r.start[1], r.end[0], r.end[1]]) {
        expect(Number.isFinite(coord)).toBe(true);
      }
      // metre coordinates with a strictly positive width
      expect(r.end[0] - r.start[0]).toBeGreaterThan(0);
    }
  });

  it("routes mock coils across the FULL domain so the mover stays over conductors for its whole travel", () => {
    // Default-style mover: P_e 12 mm → pole pitch 6 mm, 12 poles, k 0.75.
    const c = makeConfig({
      magnet_count: 12,
      magnet_pitch_m: 0.006,
      magnet_width_m: 0.0045,
      magnet_gap_m: 0.0015,
      active_area_length_m: 0.147,
      padding_m: 0.03,
    });
    const coils = mockCoils(c);
    const g = computePreviewGeometry(coils, {
      magnet_count: 12,
      magnet_width_mm: 4.5,
      magnet_gap_mm: 1.5,
    });
    // Traces (and the board panel) span the full routing domain: the active
    // area plus BOTH end paddings — the mover must have coils beneath it at
    // every travel position, exactly like the real infinity braid.
    expect(g.contentBox.maxX).toBeCloseTo(0.147 + 2 * 0.03);

    const magnets = computeMagnets(
      { magnet_count: 12, magnet_width_mm: 4.5, magnet_gap_mm: 1.5 },
      coils.routing_dimensions,
    );
    const travelM = 0.075;
    const restMin = magnets[0].x;
    const restMax = magnets[magnets.length - 1].x + 4.5 / 1000;
    // At rest AND at the far end of travel the strip stays on the board.
    expect(restMin).toBeGreaterThanOrEqual(0);
    expect(restMax + travelM).toBeLessThanOrEqual(g.contentBox.maxX);
    // The strip covers exactly one travel length of movement.
    expect(restMax - restMin).toBeCloseTo(0.072);
  });
});

describe("mockForceSweep", () => {
  it("returns arrays sized to n_positions with a sensible mean/peak ordering", () => {
    const c = makeConfig({ n_positions: 25 });
    const s = mockForceSweep(c);
    expect(s.positions_m).toHaveLength(25);
    expect(s.force_x_n).toHaveLength(25);
    expect(s.force_z_n).toHaveLength(25);
    expect(s.n_positions).toBe(25);
    expect(s.min_thrust_n).toBeLessThanOrEqual(s.mean_thrust_n);
    expect(s.mean_thrust_n).toBeLessThanOrEqual(s.peak_thrust_n);
    expect(s.per_phase_force_x).toHaveLength(25);
    expect(s.per_phase_force_x[0]).toHaveLength(c.phases);
  });
});

describe("mockFriction", () => {
  it("splits the total into bearing/cable/wiper parts", () => {
    const f = mockFriction(makeConfig({ friction_n: 0.1 }));
    expect(f.total_n).toBe(0.1);
    expect(f.bearing_friction_n + f.cable_drag_n + f.wiper_contact_n).toBeCloseTo(0.1);
    expect(f.minimum_drive_force_n).toBeCloseTo(0.13);
  });
});

describe("mockPowerBudget", () => {
  it("keeps efficiency in the 2–15% band", () => {
    const p = mockPowerBudget(makeConfig());
    expect(p.efficiency_pct).toBeGreaterThanOrEqual(2);
    expect(p.efficiency_pct).toBeLessThanOrEqual(15);
    expect(p.burst_power_w).toBeGreaterThan(p.continuous_power_w);
  });
});

describe("mockHeightStack", () => {
  it("sums the full stack including tolerance", () => {
    const c = makeConfig();
    const h = mockHeightStack(c);
    expect(h.total_height_m).toBeCloseTo(
      h.pcb_thickness_m + h.cu_protrusion_m + h.solder_mask_m + h.air_gap_m +
        h.magnet_height_m + h.tolerance_m,
    );
  });
});

describe("mockStackup", () => {
  it("mirrors the layer count in its arrays", () => {
    const s = mockStackup(makeConfig({ num_layers: 6 }));
    expect(s.layer_count).toBe(6);
    expect(s.trace_widths_m).toHaveLength(6);
    expect(s.cu_thickness_m).toHaveLength(6);
  });
});

describe("mockBoardDiagnostics", () => {
  it("returns a disconnected placeholder board", () => {
    const d = mockBoardDiagnostics();
    expect(d.board_name).toContain("mock");
    expect(d.copper_layer_count).toBeGreaterThan(0);
    expect(d.available_net_classes).toEqual([]);
  });
});

describe("mockPreviewCoils", () => {
  it("tallies tracks as (conductors + end-turns) × phases × layers", () => {
    const c = makeConfig({ magnet_count: 4, phases: 3, num_layers: 2 });
    const p = mockPreviewCoils(c);
    expect(p.num_layers).toBe(2);
    expect(p.layers).toHaveLength(2);
    const segsPerLayer = Math.max(2, 4 * 2) * 2 - 1; // 15
    expect(p.total_tracks).toBe(15 * 3 * 2);
    expect(p.total_vias).toBe(0);
    expect(p.topology).toBe(c.routing_pattern);
  });
});

describe("mockDxfExportResult", () => {
  it("returns a well-formed R12 skeleton", () => {
    const r = mockDxfExportResult(makeConfig());
    expect(r.dxf_content).toContain("SECTION");
    expect(r.dxf_content).toContain("EOF");
    expect(r.summary.total_lines).toBe(0);
  });
});

describe("mockBFieldGrid", () => {
  it("samples n_x × n_z points in row-major Z-slow order", () => {
    const c = makeConfig();
    const g = mockBFieldGrid(c, 24, 12, [0, 0.195], [0, 0.006]);
    expect(g.samples).toHaveLength(24 * 12);
    expect(g.x_extent_m).toEqual([0, 0.195]);
    expect(g.z_extent_m).toEqual([0, 0.006]);
    for (const s of g.samples) {
      expect(s.mag_t).toBeCloseTo(Math.hypot(s.bx_t, s.by_t, s.bz_t), 10);
    }
  });
});

describe("mockTravelEnvelope", () => {
  // PRODUCT REFERENCE PINS — if min or max move, these tests fail.
  //
  // Coil-capture convention: min = padding + (2/3)·P_e, max =
  // (padding + active) − (3/4)·P_e — endpoints scale with P_e, not N.
  // rest_phase_m is the track-frame phase (padding + φ) mod P_e.
  it("defaults (P_e=12 mm, slots [30,177]) spans 38 → 168 mm", () => {
    const env = mockTravelEnvelope(
      makeConfig({ magnet_count: 12, magnet_pitch_m: 0.006, active_area_length_m: 0.147 }),
    );
    expect(env.electrical_period_m).toBeCloseTo(0.012, 12);
    expect(env.min_position_m).toBeCloseTo(0.038, 12);
    expect(env.max_position_m).toBeCloseTo(0.168, 12);
    expect(env.rest_phase_m).toBeCloseTo(0.004, 12);
  });

  it("N=4 shares the capture endpoints (phase marker differs only if φ does)", () => {
    const env = mockTravelEnvelope(
      makeConfig({ magnet_count: 4, magnet_pitch_m: 0.006, active_area_length_m: 0.147 }),
    );
    expect(env.min_position_m).toBeCloseTo(0.038, 12);
    expect(env.max_position_m).toBeCloseTo(0.168, 12);
    expect(env.rest_phase_m).toBeCloseTo(0.004, 12);
  });

  it("N=6 shares the endpoints; track-frame phase (30+4) mod 12 = 10 mm", () => {
    const env = mockTravelEnvelope(
      makeConfig({ magnet_count: 6, magnet_pitch_m: 0.006, active_area_length_m: 0.147 }),
    );
    expect(env.min_position_m).toBeCloseTo(0.038, 12);
    expect(env.max_position_m).toBeCloseTo(0.168, 12);
    expect(env.rest_phase_m).toBeCloseTo(0.010, 12);
  });

  it("clamps max to min when the slot region cannot host the envelope", () => {
    // Slot [30, 40] mm: min 38, max = max(40−9, 38) = 38 → clamped.
    const env = mockTravelEnvelope(
      makeConfig({ magnet_count: 24, magnet_pitch_m: 0.006, active_area_length_m: 0.01 }),
    );
    expect(env.max_position_m).toBeCloseTo(env.min_position_m, 12);
  });
});
