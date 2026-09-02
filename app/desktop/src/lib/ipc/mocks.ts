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
  PhaseBandWidthDto,
  ForceSweepResult,
  StackupResultDto,
  HeightStackResultDto,
  FrictionBudgetDto,
  PowerBudgetDto,
  BoardDiagnostics,
  PreconditionWarning,
  CoilPreview,
  KicadConnection,
  KicadWriteResult,
  KicadPingResult,
  BFieldGridDto,
  DxfExportResult,
  TravelEnvelopeDto,
  MagnetGrade,
} from "../types";
import { MAGNET_GRADES } from "../types/magnets";

// ---------------------------------------------------------------------------
// Named mock physics constants.
//
// These mirror the real backend's approximated physics so `vite dev` output
// stays plausibly close to prod. Each value is a shorthand the real solver
// computes from first principles — naming them keeps the mock's intent (and
// the fact that it is an approximation) explicit.
// ---------------------------------------------------------------------------

/** Force baseline coefficient: ~0.4 × Br × I × layers × (N/10) [N]. */
const FORCE_BASELINE_K = 0.4;
/** Friction safety multiplier applied to derive minimum-drive force. */
const FRICTION_SAFETY_K = 1.3;
/** Normal-force pull-in relative to thrust (≈1.5–1.7× ). */
const NORMAL_FORCE_K = 1.6;
/** Ripple fraction of the baseline thrust. */
const FORCE_RIPPLE_PCT = 0.08;
/** Arc/annular height for the decorative preview arcs (m). */
const PREVIEW_ARC_HEIGHT_M = 0.0008;
/** Copper protrusion per side above the substrate (m) — 1oz ≈ 35 µm. */
const COPPER_PROTRUSION_1OZ_M = 35e-6;
/** Solder-mask coating thickness (m). */
const SOLDER_MASK_M = 20e-6;
/** Assembly tolerance allowance in the height stack (m). */
const TOLERANCE_M = 0.1e-3;
/** Nominal trace width for the mock stackup/spec (m). */
const NOMINAL_TRACE_W_M = 0.2e-3;
/** Copper foil thickness for inner layers in the mock stackup (m). */
const INNER_CU_THICKNESS_M = 70e-6;
/** Copper resistivity (Ω·m). */
const RHO = 1.72e-8;

/**
 * Mock magnet-grade table. Projects the static TS mirror of the Rust
 * `pcbmotorgen_simulation::magnet_grades::MAGNET_GRADES` table into the same
 * `MagnetGrade[]` wire shape `get_magnet_grades` returns, so `vite dev` output
 * matches prod.
 */
export function mockMagnetGrades(): MagnetGrade[] {
  return Object.values(MAGNET_GRADES).map((g) => ({ ...g }));
}

export function mockConfigDerived(c: LinearMotorConfig): ConfigDerived {
  const pole_pitch_m = c.magnet_pitch_m;
  const magnet_array_span_m = c.magnet_count * c.magnet_pitch_m;
  const travel_m = c.active_area_length_m - magnet_array_span_m;
  const phase_band_pitch_m = (pole_pitch_m / c.phases) * c.spacing_ratio;
  return {
    pole_pitch_m,
    magnet_array_span_m,
    travel_m,
    phase_band_pitch_m,
    magnet_gap_m: c.magnet_gap_m,
    min_via_pad_m: c.min_via_drill_m + 2 * c.min_via_annular_ring_m,
    acceleration_force_n: c.carriage_mass_kg * c.max_accel_m_s2,
    minimum_drive_force_n: c.friction_n * FRICTION_SAFETY_K,
    active_length_m: c.active_area_length_m,
  };
}

/**
 * Mock travel envelope — mirrors the Rust `travel_envelope_over_slots`
 * (lattice-snapped, span-aware spec, kata xb16).
 *
 * Endpoints are the first/last STABLE REST POSITIONS of the array CENTRE
 * inside the copper active area (glossary "Travel Envelope"), derived in
 * two steps:
 *   1. Span-aware centre clamp — the centre must keep the whole mover
 *      inside copper: centre ∈ [copper_start + span/2, copper_end − span/2]
 *      with the glossary "Mover Span" span = N·τ_p (τ_p = P_e/2). The
 *      range WIDENS as N shrinks.
 *   2. Lattice snapping — both endpoints snap onto the track-frame rest
 *      lattice `x ≡ φ_track (mod P_e)` with φ_track =
 *      (copper_start + φ) mod P_e and φ = (P_e/12 + ((N−1)/2)·τ_p)
 *      mod P_e (φ is N-dependent). Defaults (N=12, τ_p=6 mm → P_e=12 mm,
 *      copper [30,177] mm): the clamp is [66, 141] mm and φ_track = 4 mm,
 *      so min = **76 = 4 + 6·12 mm** and max = **136 = 4 + 11·12 mm** —
 *      the pinned values ARE lattice points (pre-xb16 coil-capture values
 *      were 38/168 mm). N=4 widens the clamp to [42, 165] mm → 52/160 mm.
 * `rest_phase_m` is unchanged: the TRACK-FRAME lattice phase, so the
 * holding-force chart zeros stay aligned to the stable rests. When no
 * lattice point exists between the clamped bounds (copper shorter than the
 * mover span, or too short to admit one full lattice step past the lower
 * bound), max clamps to min — the envelope never inverts, though the
 * array may overhang the copper at that single rest position.
 */
export function mockTravelEnvelope(c: LinearMotorConfig): TravelEnvelopeDto {
  const P_e = 2 * c.magnet_pitch_m; // electrical period
  if (!(P_e > 0)) {
    return {
      min_position_m: 0,
      max_position_m: 0,
      rest_phase_m: 0,
      electrical_period_m: P_e,
    };
  }
  const tau_p = P_e / 2; // pole pitch
  let phi =
    (Math.PI / 6) * (P_e / (2 * Math.PI)) + ((c.magnet_count - 1) / 2) * tau_p;
  phi %= P_e;
  if (phi < 0) phi += P_e;
  const copperRegionStart = c.padding_m;
  const copperRegionEnd = c.padding_m + c.active_area_length_m;
  // Track-frame rest phase: every stable rest centre ≡ φ_track (mod P_e).
  const phaseTrack = (((copperRegionStart + phi) % P_e) + P_e) % P_e;
  // Span-aware centre clamp: keep the whole array inside the copper.
  const span = c.magnet_count * tau_p;
  const lower = copperRegionStart + span / 2;
  const upper = copperRegionEnd - span / 2;
  // Lattice snapping with the Rust implementation's float guards: a
  // mathematically integral quotient nudged to e.g. 3.000…4 must not
  // step a full period too far.
  const LATTICE_SNAP_EPS_M = 1e-9;
  let min = phaseTrack + Math.ceil((lower - phaseTrack) / P_e) * P_e;
  while (min - P_e >= lower - LATTICE_SNAP_EPS_M) min -= P_e;
  let max = phaseTrack + Math.floor((upper - phaseTrack) / P_e) * P_e;
  while (max + P_e <= upper + LATTICE_SNAP_EPS_M) max += P_e;
  // Degenerate (no lattice point between the bounds): never inverted.
  return {
    min_position_m: min,
    max_position_m: Math.max(max, min),
    rest_phase_m: phaseTrack,
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
  // paddings), NOT just the mover span. The mover must have conductors
  // beneath it for every position along its travel, exactly like the real
  // backend.
  const moverSpan = c.magnet_count * c.magnet_pitch_m;
  const domain = c.active_area_length_m + 2 * c.padding_m;
  const width = c.board_width_m;
  const nLayers = Math.max(1, c.num_layers);
  const nConductors = Math.max(2, c.magnet_count * 2);
  const pitchX = domain / (nConductors - 1);
  const pole_pitch_m = c.magnet_pitch_m;
  // Match the canonical phase-band-pitch formula: (pole_pitch / phases) ×
  // spacing_ratio (Vernier ratio). The routing sidecar reports the ideal
  // phase-band pitch the same way — the earlier copy here omitted the
  // Vernier factor.
  const phase_band_pitch_m =
    (pole_pitch_m / Math.max(1, c.phases)) * (c.spacing_ratio || 1);
  const phase_clearance_m = c.min_space_m;
  const max_phase_band_width_m = phase_band_pitch_m - phase_clearance_m;
  const trace_count = Math.max(
    1,
    Math.round(c.routing_params.num_strands ?? c.strands_per_phase ?? 1),
  );
  const band_width_m =
    trace_count * c.min_trace_m + (trace_count - 1) * c.min_space_m;
  const phase_band_widths: PhaseBandWidthDto[] = [];
  // Pattern-owned pole-region bands (the routing sidecar's authoritative
  // phase/pole boundaries). The mock mirrors the infinity braid: one region
  // per phase per pole pitch, each phase interleaved by a phase-band-pitch
  // offset so the alternating red/blue zones overlap the interleaved wave
  // bands. Coordinates are metres (the Tauri adapter converts routing mm →
  // SI).
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
      { start: [pitchX * 0.5, width], mid: [pitchX, width + PREVIEW_ARC_HEIGHT_M], end: [pitchX * 1.5, width], is_active: false },
      { start: [domain - pitchX * 1.5, width], mid: [domain - pitchX, width + PREVIEW_ARC_HEIGHT_M], end: [domain - pitchX * 0.5, width], is_active: false },
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
      phase_band_widths.push({
        layer,
        net,
        trace_count,
        trace_width_m: c.min_trace_m,
        trace_spacing_m: c.min_space_m,
        angle_rad: Math.PI / 2,
        band_width_m,
        max_band_width_m: max_phase_band_width_m,
        margin_m: max_phase_band_width_m - band_width_m,
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
    magnet_array_span_m: moverSpan,
    pole_pitch_m,
    period_pitch_m: c.routing_pattern === "infinity-braid" ? pole_pitch_m : null,
    period_count:
      c.routing_pattern === "infinity-braid"
        ? Math.max(1, Math.floor((c.active_area_length_m + 2 * c.padding_m) / pole_pitch_m))
        : null,
    phase_band_pitch_m,
    phase_clearance_m,
    max_phase_band_width_m,
    phase_band_widths,
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
  const baseline = FORCE_BASELINE_K * br * ipeak * c.num_layers * (c.magnet_count / 10);
  const ripple = baseline * FORCE_RIPPLE_PCT;
  const force_x = positions.map(
    (x) => baseline + ripple * Math.sin((x / Math.max(c.magnet_pitch_m, 1e-6)) * 2 * Math.PI * c.phases),
  );
  const force_y = positions.map((_, idx) => 0.01 * Math.sin(idx));
  const force_z = positions.map(() => baseline * NORMAL_FORCE_K); // pull-in ~ 1.5–1.7× thrust
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
    cu_protrusion_m: COPPER_PROTRUSION_1OZ_M * (c.num_layers >= 6 ? 2 : 1),
    solder_mask_m: SOLDER_MASK_M,
    air_gap_m: c.air_gap_m,
    magnet_height_m: c.magnet_height_m,
    tolerance_m: TOLERANCE_M,
    total_height_m:
      c.pcb_thickness_m + COPPER_PROTRUSION_1OZ_M + SOLDER_MASK_M + c.air_gap_m + c.magnet_height_m + TOLERANCE_M,
  };
}

export function mockPowerBudget(c: LinearMotorConfig): PowerBudgetDto {
  // Crude I²R estimate based on coil length.
  const coilLen = c.active_area_length_m * c.board_width_m * c.num_layers * 2;
  const traceArea = COPPER_PROTRUSION_1OZ_M * NOMINAL_TRACE_W_M; // 1oz, 0.2mm trace
  const r = (RHO * coilLen) / traceArea;
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
    minimum_drive_force_n: total * FRICTION_SAFETY_K,
  };
}

export function mockStackup(c: LinearMotorConfig): StackupResultDto {
  const lc = c.num_layers;
  const traceW = Array.from({ length: lc }, (_, i) =>
    NOMINAL_TRACE_W_M * (1 + Math.abs(i - (lc - 1) / 2) * 0.05),
  );
  const cuT = Array.from({ length: lc }, (_, i) =>
    i === 0 || i === lc - 1 ? COPPER_PROTRUSION_1OZ_M : INNER_CU_THICKNESS_M,
  );
  return {
    layer_count: lc,
    trace_widths_m: traceW,
    cu_thickness_m: cuT,
    via_drill_m: c.min_via_drill_m,
    via_annular_ring_m: c.min_via_annular_ring_m,
    via_grid_rows: 2,
    via_grid_cols: 4,
    estimated_force_n: FORCE_BASELINE_K * c.magnet_remanence_t * c.max_current_a * lc,
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

/** Mock KiCad connection state — no board is open in dev mode. */
export function mockKicadConnection(): KicadConnection {
  return { connected: false, board_name: "(not connected)", copper_layers: 0 };
}

/** Mock KiCad ping — the backend (and KiCad) are absent in dev mode. */
export function mockKicadPing(): KicadPingResult {
  return { ok: false, version: "" };
}

/** Mock write result — surfacing that no backend exists. */
export function mockKicadWrite(): KicadWriteResult {
  return {
    items_attempted: 0,
    items_created: 0,
    failures: ["Backend not available — open the Tauri shell to write to KiCad"],
    failure_summary: [],
    commit_id: "",
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
    pattern_id: config.routing_pattern,
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
 * visualising the alternating-pole field in the absence of a backend.
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
  };
}
