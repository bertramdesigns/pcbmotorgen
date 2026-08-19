/**
 * Pure geometry math for the coil preview (shared by the Canvas viewer and
 * available to any future renderer). No Svelte, no DOM — unit-testable.
 *
 * Layers are drawn OVERLAID at their TRUE coordinates (a top-down view,
 * matching the DXF export): there is no schematic exploded-layer offset.
 * Overlapping layers are distinguished with the per-layer show/hide
 * toggles in the viewer, so each copper layer can be inspected alone.
 *
 * `contentBox` = the raw winding footprint; the magnet strip overlay sits
 * 1 mm above the tallest drawn content.
 */

import { unionBounds, type ViewportBBox } from "./chart";
import type {
  CoilPathDto,
  CoilSegmentDto,
  PhaseCoilDto,
  PoleRegionDto,
  RoutingDimensionsDto,
} from "./types";

/** The subset of the config the preview geometry depends on. */
export interface PreviewConfigLike {
  magnet_count: number;
  magnet_width_mm: number;
  magnet_gap_mm: number;
}

/** Board panel rectangle (world units) drawn behind the winding. */
export interface BoardRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** All layout constants the canvas viewer needs before drawing. */
export interface PreviewGeometry {
  /** Union of raw winding geometry bounds (true coordinates). */
  footprintBox: ViewportBBox;
  /** Content bounds — layers overlaid, so identical to `footprintBox`. */
  contentBox: ViewportBBox;
  /** y of the magnet strip top edge (1 mm above the tallest content). */
  magnetTop: number;
  /** x-extent of the magnet strip (magnet_count × pitch). */
  magnetSpan: number;
  /** Union of content + magnet strip — what the camera must fit. */
  fitBounds: ViewportBBox;
  /** Dark PCB panel rect (contentBox + 1.5 mm margin). */
  boardRect: BoardRect;
  /** Number of DISTINCT layer indices in the data (what actually renders). */
  renderedLayerCount: number;
}

/** One magnet strip segment (x range + pole polarity). */
export interface MagnetStrip {
  x: number;
  w: number;
  pole: number; // +1 N, -1 S
}

const BOARD_MARGIN_M = 0.0015; // 1.5 mm of panel around the winding
const MAGNET_STRIP_CLEARANCE_M = 0.001; // 1 mm above tallest content
const MAGNET_STRIP_HEIGHT_M = 0.003;

/**
 * Raw winding footprint: union of every phase's segments + corner arcs.
 * Layers are overlaid at true coordinates, so this IS the content box.
 */
export function computeFootprintBox(coils: CoilPathDto | null): ViewportBBox {
  const boxes: ViewportBBox[] = [];
  if (coils) {
    for (const ph of coils.phases) {
      let minX = Infinity,
        minY = Infinity,
        maxX = -Infinity,
        maxY = -Infinity;
      for (const s of ph.segments) {
        minX = Math.min(minX, s.start[0], s.end[0]);
        maxX = Math.max(maxX, s.start[0], s.end[0]);
        minY = Math.min(minY, s.start[1], s.end[1]);
        maxY = Math.max(maxY, s.start[1], s.end[1]);
      }
      for (const a of ph.corner_arcs ?? []) {
        minX = Math.min(minX, a.start[0], a.mid[0], a.end[0]);
        maxX = Math.max(maxX, a.start[0], a.mid[0], a.end[0]);
        minY = Math.min(minY, a.start[1], a.mid[1], a.end[1]);
        maxY = Math.max(maxY, a.start[1], a.mid[1], a.end[1]);
      }
      if (minX !== Infinity) boxes.push({ minX, minY, maxX, maxY });
    }
  }
  return unionBounds(...boxes);
}

/** Full geometry derivation for the preview camera + drawing. */
export function computePreviewGeometry(
  coils: CoilPathDto | null,
  config: PreviewConfigLike,
): PreviewGeometry {
  const footprintBox = computeFootprintBox(coils);

  // Layers are overlaid at their true coordinates (top-down schematic,
  // matching the DXF). contentBox == footprintBox.
  const contentBox: ViewportBBox = { ...footprintBox };

  // Magnet strip sits 1 mm above the tallest drawn content.
  const magnetTop = contentBox.maxY + MAGNET_STRIP_CLEARANCE_M;
  const magnets = computeMagnets(config, coils?.routing_dimensions);
  const magnetWidth = config.magnet_width_mm / 1000;
  const magnetGap = Math.max(config.magnet_gap_mm, 0) / 1000;
  const magnetSpan = Math.max(
    config.magnet_count * (magnetWidth + magnetGap),
    0.001,
  );
  // Fit the painted solids themselves. Do not include synthetic half-gap
  // padding at either end: that shifts the apparent first-magnet position
  // when the camera is reset, especially for a sidecar-aligned array.
  const magnetStartX = magnets[0]?.x ?? 0;
  const lastMagnet = magnets[magnets.length - 1];
  const magnetEndX = lastMagnet ? lastMagnet.x + lastMagnet.w : magnetStartX + magnetSpan;

  const fitBounds = unionBounds(contentBox, {
    minX: magnetStartX,
    minY: magnetTop,
    maxX: magnetEndX,
    maxY: magnetTop + MAGNET_STRIP_HEIGHT_M,
  });

  const boardRect: BoardRect = {
    x: contentBox.minX - BOARD_MARGIN_M,
    y: contentBox.minY - BOARD_MARGIN_M,
    w: contentBox.maxX - contentBox.minX + 2 * BOARD_MARGIN_M,
    h: contentBox.maxY - contentBox.minY + 2 * BOARD_MARGIN_M,
  };

  const renderedLayerCount = coils
    ? new Set(coils.phases.map((p) => p.layer_idx)).size
    : 0;

  return {
    footprintBox,
    contentBox,
    magnetTop,
    magnetSpan,
    fitBounds,
    boardRect,
    renderedLayerCount,
  };
}

/**
 * Distinct layers present in the data (the per-(phase, layer) coil list is
 * deduped by `layer_idx`), sorted ascending — drives the layer toggles.
 */
export function computeUniqueLayers(
  coils: CoilPathDto | null,
): { idx: number }[] {
  if (!coils) return [];
  const set = new Set<number>();
  for (const ph of coils.phases) set.add(ph.layer_idx);
  return [...set]
    .sort((a, b) => a - b)
    .map((idx) => ({ idx }));
}

/**
 * First solid-magnet x position for the preview.
 *
 * The three-phase phase sequence is A1 → B1 (neutral) → C1 → A2.  Each
 * N/S magnet pitch cell (solid bar + its trailing gap) must land with its
 * RIGHT edge exactly on the centre of a B-phase slot: that is how the poles
 * stay locked to the slot zones while the neutral B phase sits symmetrically
 * between the two magnets that flank it.  Concretely, the first cell's right
 * edge (magnet0.x + width + gap) equals the B1 zone centre, the second cell's
 * right edge the B2 zone centre, and so on.  Pattern-owned pole regions are
 * the authoritative source for those zone centres.  When a legacy payload has
 * no regions, the fallback places each bar in the centre of its configured
 * pitch cell, splitting the configured gap at each cell boundary rather than
 * leaving it all on one side of a bar.
 */
export function computeMagnetStartX(
  config: PreviewConfigLike,
  dimensions:
    | Partial<Pick<RoutingDimensionsDto, "pole_regions">>
    | null
    | undefined = undefined,
): number {
  const widthM = config.magnet_width_mm / 1000;
  const gapM = Math.max(config.magnet_gap_mm, 0) / 1000;
  const pitchM = widthM + gapM;
  if (!Number.isFinite(widthM) || widthM <= 0 || !Number.isFinite(pitchM) || pitchM <= 0) {
    return 0;
  }

  const zones = computePoleRegionZones(dimensions);
  const phaseOrder: string[] = [];
  for (const zone of zones) {
    if (!phaseOrder.includes(zone.phase)) phaseOrder.push(zone.phase);
  }

  const zoneAt = (phase: string, poleIndex: number): PoleRegionZone | null =>
    zones.find((zone) => zone.phase === phase && zone.poleIndex === poleIndex) ?? null;
  const firstZone = (phase: string, poleIndex: number): PoleRegionZone | null =>
    zoneAt(phase, poleIndex) ?? zones.find((zone) => zone.phase === phase) ?? null;
  const centreX = (zone: PoleRegionZone): number => (zone.x0 + zone.x1) / 2;

  // Anchor the array so the FIRST pitch cell's right edge (solid bar + its
  // trailing gap) lands exactly on the neutral B1 zone centre. The neutral
  // phase is then flanked symmetrically by the N and S bars while every later
  // cell's right edge falls on the next B-zone centre automatically (B zones
  // repeat every pitch). Neither the configured gap nor any edge padding can
  // move the first solid bar away from the pattern-owned coordinate.
  if (phaseOrder.length >= 2) {
    const neutral = zoneAt(phaseOrder[1], 0) ?? firstZone(phaseOrder[1], 0);
    if (neutral) {
      const neutralCentre = centreX(neutral);
      if (Number.isFinite(neutralCentre)) {
        return neutralCentre - pitchM;
      }
    }
  }

  // If B1 is absent from a three-phase sidecar, fall back to the equivalent
  // C1/A2 midpoint rather than guessing from a phase-band or board border.
  // B1's centre sits half a pitch before that midpoint, so the same
  // cell-right-edge rule gives `negativeMagnetCentre - 3/2 * pitch`.
  if (phaseOrder.length === 3) {
    const c1 = zoneAt(phaseOrder[2], 0);
    const a2 = zoneAt(phaseOrder[0], 1);
    if (c1 && a2) {
      const negativeMagnetCentre = (centreX(c1) + centreX(a2)) / 2;
      if (Number.isFinite(negativeMagnetCentre)) {
        return negativeMagnetCentre - pitchM * 1.5;
      }
    }
  }

  // No pattern-owned zones: centre each solid bar in its pitch cell.  This
  // preserves the old origin while splitting the configured gap at the array
  // boundaries for legacy/mock payloads.
  return gapM / 2;
}

/** Magnet array overlay: count × pitch segments, alternating polarity. */
export function computeMagnets(
  config: PreviewConfigLike,
  dimensions:
    | Partial<Pick<RoutingDimensionsDto, "pole_regions">>
    | null
    | undefined = undefined,
): MagnetStrip[] {
  const arr: MagnetStrip[] = [];
  const pitch = (config.magnet_width_mm + Math.max(config.magnet_gap_mm, 0)) / 1000;
  const mw = config.magnet_width_mm / 1000;
  const firstX = computeMagnetStartX(config, dimensions);
  for (let i = 0; i < config.magnet_count; i++) {
    arr.push({ x: firstX + i * pitch, w: mw, pole: i % 2 === 0 ? 1 : -1 });
  }
  return arr;
}

/**
 * Distinct phases present in the data, keyed by `phase_idx` (dedupes the
 * per-(phase, layer) coil list), sorted by index.
 */
export function computeUniquePhases(
  coils: CoilPathDto | null,
): { idx: number; name: string; colorIdx: number }[] {
  if (!coils) return [];
  const byIdx: Record<number, { idx: number; name: string; colorIdx: number }> = {};
  for (const ph of coils.phases) {
    if (!(ph.phase_idx in byIdx)) {
      byIdx[ph.phase_idx] = {
        idx: ph.phase_idx,
        name: ph.phase_name,
        colorIdx: ph.phase_idx,
      };
    }
  }
  return Object.values(byIdx).sort((a, b) => a.idx - b.idx);
}

/**
 * Filter segments to the first N active conductors when "one section"
 * is on (keyed by `phase_idx * 1000 + layer_idx`). End-turns that bridge
 * to a conductor inside the window are kept so the section looks
 * complete; end-turns bridging OUT of the window are dropped. When
 * `oneSection` is off, returns every segment (fast path).
 */
export function computeVisibleSegments(
  coils: CoilPathDto | null,
  oneSection: boolean,
  oneSectionConductorCount: number,
): Map<number, CoilSegmentDto[]> {
  if (!coils) return new Map();
  const m = new Map<number, CoilSegmentDto[]>();
  if (!oneSection) {
    for (const ph of coils.phases) {
      m.set(ph.phase_idx * 1000 + ph.layer_idx, ph.segments);
    }
    return m;
  }
  for (const ph of coils.phases) {
    const key = ph.phase_idx * 1000 + ph.layer_idx;
    const active = ph.segments.filter((s) => s.is_active);
    const keepIdx = new Set<number>();
    for (let i = 0; i < Math.min(oneSectionConductorCount, active.length); i++) {
      keepIdx.add(ph.segments.indexOf(active[i]));
    }
    const keptSegs: CoilSegmentDto[] = [];
    for (let i = 0; i < ph.segments.length; i++) {
      if (ph.segments[i].is_active) {
        if (keepIdx.has(i)) keptSegs.push(ph.segments[i]);
      } else {
        // End-turn: keep when its neighbours' range overlaps the kept
        // actives (cheap index-range proxy for "bridges inside the window").
        const firstKept = Math.min(...keepIdx);
        const lastKept = Math.max(...keepIdx);
        if (i > firstKept && i < lastKept) keptSegs.push(ph.segments[i]);
      }
    }
    m.set(key, keptSegs);
  }
  return m;
}

/**
 * Corner arcs filtered to the same one-section window as the segments
 * (same keep scheme + x-range proxy, keyed like `computeVisibleSegments`).
 */
export function computeVisibleArcs(
  coils: CoilPathDto | null,
  oneSection: boolean,
  oneSectionConductorCount: number,
): Map<number, CoilArcDto[]> {
  if (!coils) return new Map();
  const m = new Map<number, CoilArcDto[]>();
  if (!oneSection) {
    for (const ph of coils.phases) {
      m.set(ph.phase_idx * 1000 + ph.layer_idx, ph.corner_arcs ?? []);
    }
    return m;
  }
  for (const ph of coils.phases) {
    const key = ph.phase_idx * 1000 + ph.layer_idx;
    const keptArcs: CoilArcDto[] = [];
    const active = ph.segments.filter((s) => s.is_active);
    const keepIdx = new Set<number>();
    for (let i = 0; i < Math.min(oneSectionConductorCount, active.length); i++) {
      keepIdx.add(ph.segments.indexOf(active[i]));
    }
    if (keepIdx.size > 0) {
      let minX = Infinity,
        maxX = -Infinity;
      for (const i of keepIdx) {
        const s = ph.segments[i];
        minX = Math.min(minX, s.start[0], s.end[0]);
        maxX = Math.max(maxX, s.start[0], s.end[0]);
      }
      for (const a of ph.corner_arcs ?? []) {
        if (
          (a.start[0] >= minX && a.start[0] <= maxX) ||
          (a.end[0] >= minX && a.end[0] <= maxX)
        ) {
          keptArcs.push(a);
        }
      }
    }
    m.set(key, keptArcs);
  }
  return m;
}

// ===========================================================================
// routing_dimensions sidecar overlays: pole-pitch ruler + slot-width rows.
//
// Everything here is PURE (no canvas). The physics for these values lives in
// `pcbmotorgen-routing`'s `dimensions` module; the frontend only translates
// the sidecar into world-space geometry for the preview canvas. All inputs
// are treated defensively: absent / null / NaN / non-positive metre values
// simply produce "no overlay" instead of junk geometry.
// ===========================================================================

const MARGIN_EPS = 1e-12;

/** True when `v` is a finite, strictly positive metre quantity. */
export function isValidMetres(v: number | null | undefined): v is number {
  return typeof v === "number" && Number.isFinite(v) && v > 0;
}

/** Centre of the first magnet strip segment (world x, in metres). */
export function firstMagnetCenterX(magnets: readonly MagnetStrip[]): number | null {
  const m = magnets[0];
  return m ? m.x + m.w / 2 : null;
}

/** Pitch choice for the dimension ruler, with an honest caption. */
export interface PolePitchValue {
  /** Centre-to-centre adjacent-pole pitch, in metres. */
  pitchM: number;
  /** Which sidecar field supplied the pitch. */
  source: "pole_pitch_m" | "period_pitch_m";
  /**
   * Ruler caption. `pole_pitch_m` is the true periodic pole pitch; a
   * `period_pitch_m` fallback must NEVER be called "Pole pitch" — it is the
   * pattern's repeat period, so it is labelled "Repeat period" instead.
   */
  label: "Pole pitch" | "Repeat period";
}

/**
 * Resolve the pitch to dimension: prefer `pole_pitch_m` (centre-to-centre
 * adjacent North/South poles), fall back to a clearly-labelled
 * `period_pitch_m`. Returns `null` for absent / legacy / invalid values.
 */
export function computePolePitchValue(
  dimensions:
    | Partial<Pick<RoutingDimensionsDto, "pole_pitch_m" | "period_pitch_m">>
    | null
    | undefined,
): PolePitchValue | null {
  if (isValidMetres(dimensions?.pole_pitch_m)) {
    return {
      pitchM: dimensions.pole_pitch_m,
      source: "pole_pitch_m",
      label: "Pole pitch",
    };
  }
  if (isValidMetres(dimensions?.period_pitch_m)) {
    return {
      pitchM: dimensions.period_pitch_m,
      source: "period_pitch_m",
      label: "Repeat period",
    };
  }
  return null;
}

/** World-space geometry for the horizontal double-ended dimension ruler. */
export interface PolePitchRuler {
  source: "pole_pitch_m" | "period_pitch_m";
  label: "Pole pitch" | "Repeat period";
  pitchM: number;
  /** Ruler start: centre of the first magnet (world x, m). */
  x1: number;
  /** Ruler end: centre of the next pole (world x, m). */
  x2: number;
  /** Ruler baseline y (world, m). */
  y: number;
}

/**
 * Build the pole-pitch ruler aligned to `firstCenterX` → `firstCenterX +
 * pitch`. The sidecar has no origin, so the caller supplies the magnet-array
 * anchor (first centre) and the baseline y; this function only sizes the
 * ruler.
 */
export function computePolePitchRuler(
  dimensions:
    | Partial<Pick<RoutingDimensionsDto, "pole_pitch_m" | "period_pitch_m">>
    | null
    | undefined,
  firstCenterX: number | null,
  baselineY: number,
): PolePitchRuler | null {
  const polar = computePolePitchValue(dimensions);
  if (!polar || firstCenterX === null) return null;
  return {
    source: polar.source,
    label: polar.label,
    pitchM: polar.pitchM,
    x1: firstCenterX,
    x2: firstCenterX + polar.pitchM,
    y: baselineY,
  };
}

/** Effective conductor-band width status for a slot-width row. */
export type SlotWidthStatus = "over-budget" | "ok" | "no-limit";

/** One slot-width diagnostic row, anchored to its matched phase geometry. */
export interface SlotWidthRow {
  /** Matched `PhaseCoilDto.phase_idx`. */
  phaseIdx: number;
  /** Sidecar `(layer)` the record was matched on. */
  layer: number;
  /** Sidecar `net`/phase name (matches `PhaseCoilDto.phase_name`). */
  phaseName: string;
  /** Representative x anchor: median active-conductor x (m). */
  anchorX: number;
  /** Representative y/rail: median active-conductor y + per-row bias (m). */
  y: number;
  /** Effective conductor-band width `slot_width_m` (m). */
  slotM: number;
  /** Top-down limit `max_slot_width_m`, when known (m). */
  maxM: number | null;
  /** `maxM - slotM`, when the limit is known (m). */
  marginM: number | null;
  status: SlotWidthStatus;
}

function medianValues(values: number[]): number | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/** Median active-conductor x (start/end) of a phase coil, in metres. */
export function medianActiveX(coil: PhaseCoilDto): number | null {
  const xs: number[] = [];
  for (const seg of coil.segments) {
    if (!seg.is_active) continue;
    xs.push(seg.start[0], seg.end[0]);
  }
  return medianValues(xs);
}

/** Median active-conductor y (start/end) of a phase coil, in metres. */
export function medianActiveY(coil: PhaseCoilDto): number | null {
  const ys: number[] = [];
  for (const seg of coil.segments) {
    if (!seg.is_active) continue;
    ys.push(seg.start[1], seg.end[1]);
  }
  return medianValues(ys);
}

/**
 * Build the slot-width diagnostic rows by matching each `slot_widths[]`
 * record to its `(layer, phase_name)` PhaseCoilDto. Rows are anchored at the
 * median active-segment x and a median-active y "rail"; they are never placed
 * on an invented physical phase-band boundary. Records that cannot be matched
 * or that carry invalid widths are skipped.
 */
export function computeSlotWidthRows(
  coils: CoilPathDto | null,
  dimensions: Pick<RoutingDimensionsDto, "slot_widths"> | null | undefined,
  rowStrideM = 0.001,
): SlotWidthRow[] {
  if (!coils || !dimensions?.slot_widths) return [];
  const byKey = new Map<string, PhaseCoilDto>();
  for (const ph of coils.phases) byKey.set(`${ph.layer_idx}|${ph.phase_name}`, ph);

  const rows: SlotWidthRow[] = [];
  for (const slot of dimensions.slot_widths) {
    if (!isValidMetres(slot.slot_width_m)) continue;
    const phase = byKey.get(`${slot.layer}|${slot.net}`);
    if (!phase) continue;
    const anchorX = medianActiveX(phase);
    const railY = medianActiveY(phase);
    if (anchorX === null || railY === null) continue;

    // Unlike a physical length, the top-down limit may legitimately be zero
    // or negative when the phase-clearance rule leaves no feasible band. Keep
    // finite values so the preview exposes the routing crate's over-budget
    // diagnostic instead of silently turning it into "no limit".
    const maxM =
      typeof slot.max_slot_width_m === "number" && Number.isFinite(slot.max_slot_width_m)
        ? slot.max_slot_width_m
        : null;
    let marginM: number | null = null;
    if (maxM !== null) {
      marginM =
        typeof slot.margin_m === "number" && Number.isFinite(slot.margin_m)
          ? slot.margin_m
          : maxM - slot.slot_width_m;
    }
    rows.push({
      phaseIdx: phase.phase_idx,
      layer: slot.layer,
      phaseName: slot.net,
      anchorX,
      y: railY,
      slotM: slot.slot_width_m,
      maxM,
      marginM,
      status:
        maxM === null
          ? "no-limit"
          : marginM >= -MARGIN_EPS
            ? "ok"
            : "over-budget",
    });
  }

  // Deterministic draw order + a compact vertical stack so adjacent rows do
  // not sit on exactly the same rail.
  rows.sort((a, b) => a.layer - b.layer || a.phaseIdx - b.phaseIdx);
  const n = rows.length;
  rows.forEach((row, i) => {
    row.y += (i - (n - 1) / 2) * rowStrideM;
  });
  return rows;
}

// ===========================================================================
// Pole-region zones (pattern-owned phase/pole boundaries).
//
// Unlike the pole-pitch ruler (whose x-origin is re-anchored onto the magnet
// array), pole regions carry their OWN authoritative x boundaries from the
// routing sidecar (`routing_dimensions.pole_regions`). We never infer their
// extent from the magnet config; we only translate the sidecar's metres into
// world-space vertical zones that fill the board's y extent. Everything here
// is PURE and defensive: missing/legacy/empty/malformed region data yields
// "no zones" instead of junk geometry.
// ===========================================================================

/** Zone polarity: `+1` for even `pole_index`, `-1` for odd (drives the
 *  alternating red/blue fill). Parity is derived from the authoritative
 *  `pole_index`, never from magnet geometry. */
export function poleRegionPolarity(poleIndex: number): 1 | -1 {
  return poleIndex % 2 === 0 ? 1 : -1;
}

function isFiniteNumberPair(v: unknown): v is [number, number] {
  return (
    Array.isArray(v) &&
    v.length >= 2 &&
    typeof v[0] === "number" &&
    typeof v[1] === "number" &&
    Number.isFinite(v[0]) &&
    Number.isFinite(v[1])
  );
}

/**
 * True when a `PoleRegionDto` is usable: a non-empty phase label, a
 * non-negative integer `pole_index`, finite metre `start`/`end` points, and
 * a non-degenerate x span (zero-width regions are dropped).
 */
export function isValidPoleRegion(
  region: PoleRegionDto | null | undefined,
): boolean {
  if (!region) return false;
  if (typeof region.phase !== "string" || region.phase.trim() === "") return false;
  const idx = region.pole_index;
  if (typeof idx !== "number" || !Number.isFinite(idx) || idx < 0 || !Number.isInteger(idx)) {
    return false;
  }
  if (!isFiniteNumberPair(region.start) || !isFiniteNumberPair(region.end)) return false;
  // A zero-width x span paints an invisible/zero-area zone — drop it.
  return region.end[0] !== region.start[0];
}

/** One painted zone: a translucent band between two authoritative x
 *  boundaries (in metres), filling the caller's board y extent. */
export interface PoleRegionZone {
  phase: string;
  poleIndex: number;
  /** Lower x boundary (m); `min(start.x, end.x)`. */
  x0: number;
  /** Upper x boundary (m); `max(start.x, end.x)`. */
  x1: number;
  /** `+1` (even pole) or `-1` (odd pole) — alternating fill. */
  polarity: 1 | -1;
}

/**
 * Translate the sidecar's `pole_regions[]` into sorted, validated zones.
 * Invalid/legacy/missing regions are skipped; boundaries are defensively
 * ordered so `x0 <= x1` regardless of the pattern's start/end orientation.
 * Returns a deterministic (ascending x, then phase, then index) array.
 */
export function computePoleRegionZones(
  dimensions:
    | Partial<Pick<RoutingDimensionsDto, "pole_regions">>
    | null
    | undefined,
): PoleRegionZone[] {
  const regions = dimensions?.pole_regions;
  if (!Array.isArray(regions)) return [];
  const zones: PoleRegionZone[] = [];
  for (const region of regions) {
    if (!isValidPoleRegion(region)) continue;
    zones.push({
      phase: region.phase,
      poleIndex: region.pole_index,
      x0: Math.min(region.start[0], region.end[0]),
      x1: Math.max(region.start[0], region.end[0]),
      polarity: poleRegionPolarity(region.pole_index),
    });
  }
  zones.sort(
    (a, b) => a.x0 - b.x0 || a.phase.localeCompare(b.phase) || a.poleIndex - b.poleIndex,
  );
  return zones;
}

/**
 * Distinct phase labels present in the (validated) zone data, sorted
 * ascending. These come from the region records themselves — the authoritative
 * labels for the phase picker — never from the magnet/trace config.
 */
export function computePoleRegionPhases(
  zones: readonly PoleRegionZone[],
): string[] {
  return [...new Set(zones.map((z) => z.phase))].sort();
}

/**
 * Keep every zone when `phase` is null/empty ("All phases"), otherwise only
 * the zones whose label matches `phase`. Returns a copy so callers never
 * mutate the shared zone list.
 */
export function filterPoleRegionsByPhase(
  zones: readonly PoleRegionZone[],
  phase: string | null,
): PoleRegionZone[] {
  if (phase == null || phase === "") return zones.length === 0 ? [] : zones.slice();
  const result: PoleRegionZone[] = [];
  for (const z of zones) if (z.phase === phase) result.push(z);
  return result;
}

/**
 * Resolve a possibly-stale phase selection against the current labels so the
 * picker stays valid when a regenerated payload ships different phase labels.
 * An empty selection always resolves to itself ("All phases"); a selection
 * not present in `available` collapses back to "".
 */
export function resolvePoleRegionPhaseSelection(
  selected: string,
  available: readonly string[],
): string {
  return selected === "" || available.includes(selected) ? selected : "";
}

/**
 * World-space fit box covering every given zone's x span, bounded to the
 * caller-supplied board y band (`yMin`..`yMax`). Used by the camera fit so
 * reset-view never clips a zone that reaches into routing padding. Returns
 * `null` when there are no zones.
 */
export function computePoleRegionBounds(
  zones: readonly PoleRegionZone[],
  yMin: number,
  yMax: number,
): ViewportBBox | null {
  if (zones.length === 0) return null;
  let minX = Infinity;
  let maxX = -Infinity;
  for (const z of zones) {
    minX = Math.min(minX, z.x0);
    maxX = Math.max(maxX, z.x1);
  }
  if (!Number.isFinite(minX) || !Number.isFinite(maxX)) return null;
  return { minX, minY: yMin, maxX, maxY: yMax };
}

/** World-space box the pole-pitch ruler occupies (camera-fit input). */
export function polePitchRulerBounds(ruler: PolePitchRuler): ViewportBBox {
  const padX = Math.max(ruler.pitchM * 0.1, 0.001);
  return {
    minX: ruler.x1 - padX,
    minY: ruler.y - 0.004,
    maxX: ruler.x2 + padX,
    maxY: ruler.y + 0.004,
  };
}

/** World-space box a slot-width row occupies (camera-fit input). */
export function slotWidthRowBounds(row: SlotWidthRow): ViewportBBox {
  // Include the optional top-down limit too: the canvas draws it as a dashed
  // reference bracket so a generous phase band cannot be clipped at reset.
  const extentM = Math.max(row.slotM, row.maxM ?? 0);
  const half = Math.max(extentM / 2, 0.0005);
  return {
    minX: row.anchorX - half,
    minY: row.y - 0.002,
    maxX: row.anchorX + half,
    maxY: row.y + 0.002,
  };
}

/**
 * Expand the schematic fit box to cover the active overlays, so the camera
 * reset never clips the ruler or slot brackets (rows live inside the content
 * box already, but the ruler floats above the magnet strip). Pole-region
 * zones (optional, `regionZones`) are also included bounded to a caller-
 * supplied board y band so a zone reaching into routing padding is not
 * clipped at reset.
 */
export function computeOverlayFitBounds(
  base: ViewportBBox,
  ruler: PolePitchRuler | null,
  rows: readonly SlotWidthRow[],
  regionZones: readonly PoleRegionZone[] = [],
  regionYMin = 0,
  regionYMax = 0,
): ViewportBBox {
  const boxes: ViewportBBox[] = [base];
  if (ruler) boxes.push(polePitchRulerBounds(ruler));
  for (const row of rows) boxes.push(slotWidthRowBounds(row));
  const zoneBox = computePoleRegionBounds(regionZones, regionYMin, regionYMax);
  if (zoneBox) boxes.push(zoneBox);
  return unionBounds(...boxes);
}

/** Format a metre length as a compact mm string, e.g. "12 mm" / "1.78 mm". */
export function formatMetresMm(v: number): string {
  if (!Number.isFinite(v)) return "—";
  const mmV = Math.abs(v * 1000);
  const digits = mmV >= 100 ? 0 : mmV >= 10 ? 1 : mmV >= 1 ? 2 : 3;
  return `${(v * 1000).toFixed(digits)} mm`;
}

/** Signed mm formatter for slot margins, e.g. "+0.28 mm" / "-0.10 mm". */
export function formatMarginMm(v: number): string {
  const magnitude = formatMetresMm(Math.abs(v));
  return v < 0 ? `-${magnitude}` : `+${magnitude}`;
}
