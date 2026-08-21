/**
 * Deterministic, physics-flavoured mock implementations of the backend
 * commands. Used only when the Tauri runtime is unavailable (plain
 * `vite dev` in a browser) so the dashboard stays interactive.
 *
 * These are pure functions over `LinearMotorConfig` — no IPC imports.
 */

import type {
  LinearMotorConfig,
  ConfigDerived,
  CoilArcDto,
  CoilPathDto,
  CoilSegmentDto,
  PhaseCoilDto,
  PoleRegionDto,
  RoutingDimensionsDto,
  SlotWidthDto,
  ForceSweepResult,
  StackupResultDto,
  HeightStackResultDto,
  FrictionBudgetDto,
  PowerBudgetDto,
  BoardDiagnostics,
  PreconditionWarning,
  CoilPreview,
  BFieldGridDto,
  DxfExportResult,
  TravelEnvelopeDto,
} from "../types";

export function mockConfigDerived(c: LinearMotorConfig): ConfigDerived {
  const pole_pitch_m = c.magnet_pitch_m;
  const coil_span_m = c.magnet_count * c.magnet_pitch_m;
  const travel_m = c.active_area_length_m - coil_span_m;
  const slot_pitch_m = (pole_pitch_m / c.phases) * c.spacing_ratio;
  return {
    pole_pitch_m,
    coil_span_m,
    travel_m,
    slot_pitch_m,
    magnet_gap_m: c.magnet_gap_m,
    min_via_pad_m: c.min_via_drill_m + 2 * c.min_via_annular_ring_m,
    acceleration_force_n: c.carriage_mass_kg * c.max_accel_m_s2,
    minimum_drive_force_n: c.friction_n * 1.3,
    active_length_m: c.active_area_length_m,
  };
}

/**
 * Mock travel envelope — mirrors the Rust `travel_envelope_over_slots`.
 *
 * Product reference convention: endpoints are COIL-CAPTURE positions that
 * scale with the electrical period P_e = 2·pitch (independent of magnet
 * count):
 *   min = padding + (2/3)·P_e    (defaults slot_start 30: 30 + 8 = 38 mm)
 *   max = (padding + active) − (3/4)·P_e   (defaults 177 − 9 = 168 mm)
 * These are the first/last spots where the first/last coil carries enough
 * charge to capture the first/last pole (leading 240°, trailing 270°).
 * `rest_phase_m` is the TRACK-FRAME phase `(padding + φ) mod P_e` so the
 * holding-force chart zeros align to the stable rests; it still depends on
 * N. A slot region narrower than the envelope clamps max to min.
 */
export function mockTravelEnvelope(c: LinearMotorConfig): TravelEnvelopeDto {
  const P_e = 2 * c.magnet_pitch_m; // electrical period
  const tau_p = P_e / 2; // pole pitch
  let phi =
    (Math.PI / 6) * (P_e / (2 * Math.PI)) + ((c.magnet_count - 1) / 2) * tau_p;
  phi %= P_e;
  if (phi < 0) phi += P_e;
  const slotStart = c.padding_m;
  const slotEnd = c.padding_m + c.active_area_length_m;
  const min = slotStart + (2 / 3) * P_e;
  return {
    min_position_m: min,
    max_position_m: Math.max(slotEnd - (3 / 4) * P_e, min),
    rest_phase_m: ((slotStart + phi) % P_e + P_e) % P_e,
    electrical_period_m: P_e,
  };
}

export function mockCoils(c: LinearMotorConfig): CoilPathDto {
  // Build a serpentine per (phase × layer) pair so the dev-mode preview
  // mirrors the real backend's multi-layer output (the real infinity-braid
  // exhausts 2 layers — top braid + mirrored bottom braid). Without this,
  // the preview in browser dev mode only ever rendered layer 0 while the
  // header advertised `num_layers` — i.e. "missing layer segments".
  const phases: PhaseCoilDto[] = [];
  // The mock mirrors the real infinity braid: traces (and their pattern-made
  // pole regions) are routed across the FULL domain (active area + both end
  // paddings), NOT just the coil span. The mover must have conductors
  // beneath it for every position along its travel, exactly like the real
  // backend.
  const coilSpan = c.magnet_count * c.magnet_pitch_m;
  const domain = c.active_area_length_m + 2 * c.padding_m;
  const width = c.board_width_m;
  const nLayers = Math.max(1, c.num_layers);
  const nConductors = Math.max(2, c.magnet_count * 2);
  const pitchX = domain / (nConductors - 1);
  const pole_pitch_m = c.magnet_pitch_m;
  const slot_pitch_m = pole_pitch_m / Math.max(1, c.phases);
  const phase_clearance_m = c.min_space_m;
  const max_slot_width_m = slot_pitch_m - phase_clearance_m;
  const trace_count = Math.max(
    1,
    Math.round(c.routing_params.num_strands ?? c.windings_per_phase ?? 1),
  );
  const slot_width_m =
    trace_count * c.min_trace_m + (trace_count - 1) * c.min_space_m;
  const slot_widths: SlotWidthDto[] = [];
  // Pattern-owned pole-region bands (the routing sidecar's authoritative
  // phase/pole boundaries). The mock mirrors the infinity braid: one region
  // per phase per pole pitch, each phase interleaved by a slot-pitch offset
  // so the alternating red/blue zones overlap the interleaved wave bands.
  // Coordinates are metres (the Tauri adapter converts routing mm → SI).
  const pole_regions: PoleRegionDto[] = [];
  {
    const regionsPerPhase = Math.max(
      1,
      Math.floor(domain / Math.max(pole_pitch_m, 1e-6)),
    );
    for (let p = 0; p < c.phases; p++) {
      const net = "ABC"[p] ?? String(p);
      const offset = (p * pole_pitch_m) / Math.max(1, c.phases);
      for (let i = 0; i < regionsPerPhase; i++) {
        const startX = offset + i * pole_pitch_m;
        const endX = Math.min(domain, offset + (i + 1) * pole_pitch_m);
        if (endX <= startX) continue;
        pole_regions.push({
          phase: net,
          pole_index: i,
          start: [startX, width / 2],
          end: [endX, width / 2],
        });
      }
    }
  }

  const corner_arcs: CoilArcDto[] = [];
  const via_positions: [number, number][] = [];
  if (nConductors >= 2 && width > 0) {
    // Two decorative corner arcs on the top edge + two via centers, purely
    // to exercise the arc/via render path in the preview (not connected).
    corner_arcs.push(
      { start: [pitchX * 0.5, width], mid: [pitchX, width + 0.0008], end: [pitchX * 1.5, width], is_active: false },
      { start: [domain - pitchX * 1.5, width], mid: [domain - pitchX, width + 0.0008], end: [domain - pitchX * 0.5, width], is_active: false },
    );
    via_positions.push([pitchX, width / 2], [domain - pitchX, width / 2]);
  }

  for (let layer = 0; layer < nLayers; layer++) {
    // Alternate serpentine orientation per layer (mirror) — the real braid's
    // top/bottom copies — so overlapping layers stay distinguishable.
    const flip = layer % 2 === 1;
    const segs: CoilSegmentDto[] = [];
    for (let i = 0; i < nConductors; i++) {
      const x = i * pitchX;
      const yTop = (i % 2 === 0) !== flip ? 0 : width;
      const yBot = (i % 2 === 0) !== flip ? width : 0;
      // active (vertical) conductor
      segs.push({ start: [x, yTop], end: [x, yBot], is_active: true });
      if (i < nConductors - 1) {
        // end-turn (horizontal) to next conductor
        segs.push({ start: [x, yBot], end: [x + pitchX, yBot], is_active: false });
      }
    }
    for (let p = 0; p < c.phases; p++) {
      const net = "ABC"[p] ?? String(p);
      slot_widths.push({
        layer,
        net,
        trace_count,
        trace_width_m: c.min_trace_m,
        trace_spacing_m: c.min_space_m,
        angle_rad: Math.PI / 2,
        slot_width_m,
        max_slot_width_m,
        margin_m: max_slot_width_m - slot_width_m,
      });
      phases.push({
        phase_idx: p,
        layer_idx: layer,
        phase_name: net,
        pattern_id: c.routing_pattern,
        segments: segs,
        corner_arcs,
        via_positions,
        total_length_m: segs.reduce((s, sg) => s + Math.hypot(sg.end[0] - sg.start[0], sg.end[1] - sg.start[1]), 0),
        active_length_m: segs.filter((s) => s.is_active).reduce((s, sg) => s + Math.hypot(sg.end[0] - sg.start[0], sg.end[1] - sg.start[1]), 0),
        end_turn_length_m: segs.filter((s) => !s.is_active).reduce((s, sg) => s + Math.hypot(sg.end[0] - sg.start[0], sg.end[1] - sg.start[1]), 0),
        active_conductor_count: segs.filter((s) => s.is_active).length,
        bounding_box: [0, 0, domain, width] as [number, number, number, number],
        terminal_start: [0, 0] as [number, number],
        terminal_end: [domain, width] as [number, number],
      });
    }
  }
  const routing_dimensions: RoutingDimensionsDto = {
    active_area_length_m: c.active_area_length_m,
    total_routing_length_m: c.active_area_length_m + 2 * c.padding_m,
    board_width_m: c.board_width_m,
    phases: Math.max(1, c.phases),
    magnet_array_span_m: coilSpan,
    pole_pitch_m,
    period_pitch_m: c.routing_pattern === "infinity-braid" ? pole_pitch_m : null,
    period_count:
      c.routing_pattern === "infinity-braid"
        ? Math.max(1, Math.floor((c.active_area_length_m + 2 * c.padding_m) / pole_pitch_m))
        : null,
    slot_pitch_m,
    phase_clearance_m,
    max_slot_width_m,
    slot_widths,
    pole_regions,
  };
  return { phases, layer_count: c.num_layers, routing_dimensions };
}

export function mockForceSweep(c: LinearMotorConfig): ForceSweepResult {
  const n = c.n_positions;
  const travel = Math.max(0, c.active_area_length_m - c.magnet_count * c.magnet_pitch_m);
  const positions = Array.from({ length: n }, (_, i) => (travel * i) / (n - 1));
  // Sinusoidal-ish force with ripple + a normal-force baseline.
  const br = c.magnet_remanence_t;
  const ipeak = c.max_current_a;
  const baseline = 0.4 * br * ipeak * c.num_layers * (c.magnet_count / 10);
  const ripple = baseline * 0.08;
  const force_x = positions.map(
    (x) => baseline + ripple * Math.sin((x / Math.max(c.magnet_pitch_m, 1e-6)) * 2 * Math.PI * c.phases),
  );
  const force_y = positions.map((_, idx) => 0.01 * Math.sin(idx));
  const force_z = positions.map(() => baseline * 1.6); // pull-in ~ 1.5–1.7× thrust
  const mean = force_x.reduce((a, b) => a + b, 0) / n;
  const peak = Math.max(...force_x);
  const min = Math.min(...force_x);
  const ripplePct = Math.abs(mean) < 1e-12 ? 0 : ((peak - min) / Math.abs(mean)) * 100;
  return {
    positions_m: positions,
    force_x_n: force_x,
    force_y_n: force_y,
    force_z_n: force_z,
    per_phase_force_x: force_x.map((f) => [f / c.phases, f / c.phases, f / c.phases]),
    commutation: c.commutation,
    current_a: ipeak,
    mean_thrust_n: mean,
    peak_thrust_n: peak,
    min_thrust_n: min,
    ripple_pct: ripplePct,
    n_positions: n,
  };
}

export function mockHeightStack(c: LinearMotorConfig): HeightStackResultDto {
  return {
    pcb_thickness_m: c.pcb_thickness_m,
    cu_protrusion_m: 35e-6 * (c.num_layers >= 6 ? 2 : 1),
    solder_mask_m: 20e-6,
    air_gap_m: c.air_gap_m,
    magnet_height_m: c.magnet_height_m,
    back_iron_thickness_m: c.back_iron_thickness_m,
    tolerance_m: 0.1e-3,
    total_height_m:
      c.pcb_thickness_m + 35e-6 + 20e-6 + c.air_gap_m + c.magnet_height_m + c.back_iron_thickness_m + 0.1e-3,
  };
}

export function mockPowerBudget(c: LinearMotorConfig): PowerBudgetDto {
  // Crude I²R estimate based on coil length.
  const coilLen = c.active_area_length_m * c.board_width_m * c.num_layers * 2;
  const rho = 1.72e-8; // Cu resistivity
  const traceArea = 35e-6 * 0.2e-3; // 1oz, 0.2mm trace
  const r = (rho * coilLen) / traceArea;
  const cont = c.max_current_a ** 2 * r * c.phases;
  const burst = (c.max_current_a * 1.5) ** 2 * r * c.phases;
  return {
    phase_resistance_ohm: r,
    continuous_power_w: cont,
    burst_power_w: burst,
    temperature_rise_c: Math.min(c.max_temperature_rise_c, cont * 4),
    capacitor_required_uf: c.capacitor_bank_uf,
    efficiency_pct: Math.max(2, Math.min(15, (0.25 * 0.1) / (c.supply_voltage_v * c.max_current_a) * 100)),
  };
}

export function mockFriction(c: LinearMotorConfig): FrictionBudgetDto {
  const total = c.friction_n;
  return {
    bearing_friction_n: total * 0.5,
    cable_drag_n: total * 0.3,
    wiper_contact_n: total * 0.2,
    cogging_n: 0,
    total_n: total,
    minimum_drive_force_n: total * 1.3,
  };
}

export function mockStackup(c: LinearMotorConfig): StackupResultDto {
  const lc = c.num_layers;
  const traceW = Array.from({ length: lc }, (_, i) =>
    0.2e-3 * (1 + Math.abs(i - (lc - 1) / 2) * 0.05),
  );
  const cuT = Array.from({ length: lc }, (_, i) =>
    i === 0 || i === lc - 1 ? 35e-6 : 70e-6,
  );
  return {
    layer_count: lc,
    trace_widths_m: traceW,
    cu_thickness_m: cuT,
    via_drill_m: c.min_via_drill_m,
    via_annular_ring_m: c.min_via_annular_ring_m,
    via_grid_rows: 2,
    via_grid_cols: 4,
    estimated_force_n: 0.4 * c.magnet_remanence_t * c.max_current_a * lc,
    estimated_dc_resistance_ohm: 1.2,
    notes: ["Mock stackup — backend not connected"],
  };
}

// ---------------------------------------------------------------------------
// Board diagnostics / preconditions / preview mocks (frontend-only dev)
// ---------------------------------------------------------------------------

/**
 * Mock board snapshot for `vite dev` (no Tauri). All edge-cut bounds
 * and net classes are 0 / empty (matching the real backend's
 * not-yet-queryable placeholders), and `copper_layer_count` mirrors
 * the user's `num_layers` so the validate/preview flows have
 * something realistic to check against.
 */
export function mockBoardDiagnostics(): BoardDiagnostics {
  return {
    board_name: "(mock board — backend not connected)",
    copper_layer_count: 4,
    board_x_min_mm: 0.0,
    board_x_max_mm: 0.0,
    board_y_min_mm: 0.0,
    board_y_max_mm: 0.0,
    available_net_classes: [],
  };
}

/**
 * Mock precondition check. In dev mode we don't have a real board to
 * compare against, so we return an empty list — the UI shows the green
 * "all clear" state. The real Rust validator runs the rule set from
 * `pcbmotorgen_export::validate_write_preconditions`.
 */
export function mockValidatePreconditions(
  _config: LinearMotorConfig,
  _diagnostics: BoardDiagnostics,
): PreconditionWarning[] {
  return [];
}

/**
 * Mock coil preview. Builds a per-layer tally that mirrors the shape
 * the Rust side would produce, so the UI's preview card has something
 * to render in dev mode.
 */
export function mockPreviewCoils(config: LinearMotorConfig): CoilPreview {
  const numLayers = Math.max(1, config.num_layers);
  // Heuristic segment count: ~2 active conductors per magnet per phase,
  // plus one end-turn per conductor pair. Matches the mockCoils() shape
  // closely enough for the "X tracks, Y vias" summary to look real.
  const segsPerLayer =
    Math.max(2, config.magnet_count * 2) * 2 - 1; // conductors + end-turns
  const totalTracks = segsPerLayer * config.phases * numLayers;
  const layers = Array.from({ length: numLayers }, (_, i) => ({
    layer_idx: i,
    phase_count: config.phases,
    segment_count: segsPerLayer * config.phases,
    via_count: 0,
  }));
  return {
    num_layers: numLayers,
    topology: config.routing_pattern,
    layers,
    total_tracks: totalTracks,
    total_vias: 0,
  };
}

/**
 * Mock DXF export for frontend-only dev. Returns a minimal valid DXF
 * file header with a note that the backend is not connected.
 */
export function mockDxfExportResult(config: LinearMotorConfig): DxfExportResult {
  const header = [
    "0",
    "SECTION",
    "2",
    "HEADER",
    "9",
    "$INSUNITS",
    "70",
    "4",
    "0",
    "ENDSEC",
    "0",
    "SECTION",
    "2",
    "ENTITIES",
    // A simple note line indicating mock mode.
    "0",
    "TEXT",
    "8",
    "Notes",
    "10",
    "0.0",
    "20",
    "0.0",
    "40",
    "3.0",
    "1",
    `(Mock DXF — backend not connected. Config: ${config.routing_pattern}, ${config.num_layers} layers)`,
    "0",
    "ENDSEC",
    "0",
    "EOF",
    "",
  ].join("\n");
  return {
    dxf_content: header,
    summary: {
      total_lines: 0,
      total_arcs: 0,
      total_circles: 0,
      layer_count: 1,
    },
  };
}

/**
 * Mock B-field grid: sinusoidal Bz, magnitude ∝ Br. Sufficient for
 * visualising arrangement-dependent asymmetry in the absence of a backend.
 */
export function mockBFieldGrid(
  c: LinearMotorConfig,
  n_x: number,
  n_z: number,
  x_extent_m: [number, number],
  z_extent_m: [number, number],
): BFieldGridDto {
  const xs = Array.from({ length: n_x }, (_, i) =>
    x_extent_m[0] + (x_extent_m[1] - x_extent_m[0]) * (i / Math.max(n_x - 1, 1)),
  );
  const zs = Array.from({ length: n_z }, (_, i) =>
    z_extent_m[0] + (z_extent_m[1] - z_extent_m[0]) * (i / Math.max(n_z - 1, 1)),
  );
  const samples: BFieldGridDto["samples"] = [];
  for (const z of zs) {
    for (const x of xs) {
      const br = c.magnet_remanence_t;
      const k = (2 * Math.PI) / Math.max(c.magnet_pitch_m, 1e-6);
      const bz = 0.4 * br * Math.sin(k * x) * Math.exp(-z / 0.003);
      const bx = 0.05 * br * Math.cos(k * x) * Math.exp(-z / 0.003);
      const by = 0.0;
      const mag = Math.sqrt(bx * bx + by * by + bz * bz);
      samples.push({ x_m: x, z_m: z, bx_t: bx, by_t: by, bz_t: bz, mag_t: mag });
    }
  }
  return {
    samples,
    x_extent_m,
    z_extent_m,
    arrangement: c.magnet_arrangement,
  };
}
