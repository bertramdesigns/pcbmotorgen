/**
 * Generated coil geometry (generate_coils).
 */

export interface CoilSegmentDto {
  start: [number, number]; // (x, y) [m]
  end: [number, number]; // (x, y) [m]
  is_active: boolean;
}

/** One rounded corner / curve: quadratic Bézier control points (x, y) [m]. */
export interface CoilArcDto {
  start: [number, number];
  mid: [number, number];
  end: [number, number];
  /** Active force-producing curve vs end-turn connector. */
  is_active?: boolean;
}

export interface PhaseCoilDto {
  phase_idx: number;
  layer_idx: number;
  phase_name: string;
  /** Routing-pattern id this coil was generated with. */
  pattern_id: string;
  segments: CoilSegmentDto[];
  total_length_m: number;
  active_length_m: number;
  end_turn_length_m: number;
  active_conductor_count: number;
  bounding_box: [number, number, number, number]; // min_x, min_y, max_x, max_y
  terminal_start: [number, number];
  terminal_end: [number, number];
  /** Rounded corners / curves (absent for straight-line-only patterns). */
  corner_arcs?: CoilArcDto[];
  /** Center positions of inter-layer vias, (x, y) [m]. */
  via_positions?: [number, number][];
}

/** Effective active-conductor band width for one (layer, net) group. */
export interface SlotWidthDto {
  layer: number;
  net: string;
  trace_count: number;
  trace_width_m: number;
  trace_spacing_m: number;
  /** Angle from the travel axis, in radians. */
  angle_rad: number;
  /** Bottom-up effective slot width along the travel axis. */
  slot_width_m: number;
  /** Top-down maximum allowed width, when pole pitch is known. */
  max_slot_width_m: number | null;
  /** max_slot_width_m - slot_width_m, when pole pitch is known. */
  margin_m: number | null;
}

/** Pole-pitch and slot-width handoff returned with generated geometry. */
export interface RoutingDimensionsDto {
  active_area_length_m: number;
  total_routing_length_m: number;
  board_width_m: number;
  phases: number;
  /** Full mover magnet-array span, when supplied by the config. */
  magnet_array_span_m: number | null;
  /** Centre-to-centre distance between adjacent north/south poles. */
  pole_pitch_m: number | null;
  period_pitch_m: number | null;
  period_count: number | null;
  /** Ideal phase-band pitch: pole_pitch_m / phases. */
  slot_pitch_m: number | null;
  phase_clearance_m: number;
  max_slot_width_m: number | null;
  slot_widths: SlotWidthDto[];
  /** Pattern-owned pole-pitch boundaries, in metres, used by preview zones. */
  pole_regions: PoleRegionDto[];
}

export interface PoleRegionDto {
  phase: string;
  pole_index: number;
  /** Start boundary in metres, `[x, y]`. */
  start: [number, number];
  /** End boundary in metres, `[x, y]`. */
  end: [number, number];
}

export interface CoilPathDto {
  phases: PhaseCoilDto[];
  layer_count: number;
  /** Additive design-dimension sidecar for magnet-pattern calculations. */
  routing_dimensions?: RoutingDimensionsDto;
}
