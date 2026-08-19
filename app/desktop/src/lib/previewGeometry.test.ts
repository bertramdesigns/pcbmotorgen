/**
 * Unit tests for the preview geometry math (lib/previewGeometry.ts).
 * These pin the schematic contract the canvas viewer renders:
 * footprint-proportional layer explosion, magnet strip placement,
 * one-section filtering, and phase deduplication.
 */

import { describe, expect, it } from "vitest";
import {
  computeFootprintBox,
  computeMagnets,
  computePreviewGeometry,
  computeUniqueLayers,
  computeUniquePhases,
  computeVisibleArcs,
  computeVisibleSegments,
  computePolePitchValue,
  computePolePitchRuler,
  computeSlotWidthRows,
  computeOverlayFitBounds,
  computePoleRegionZones,
  computePoleRegionPhases,
  filterPoleRegionsByPhase,
  resolvePoleRegionPhaseSelection,
  computePoleRegionBounds,
  isValidPoleRegion,
  poleRegionPolarity,
  firstMagnetCenterX,
  medianActiveX,
  formatMetresMm,
  formatMarginMm,
} from "./previewGeometry";
import type { CoilPathDto, PhaseCoilDto, PoleRegionDto, SlotWidthDto } from "./types";

/** Default-config-like magnet layout (10 magnets, 12 mm pitch). */
const CONFIG = { magnet_count: 10, magnet_width_mm: 10, magnet_gap_mm: 2 };

/**
 * 3-phase × 2-layer serpentine fixture: segments span y 0..0.02 (board
 * width), conductors at x = 0, 0.01, 0.02 per phase (offset per phase),
 * plus one end-turn per conductor pair. Mirrors the infinity-braid's
 * "2 populated layers" shape.
 */
function braidLikeFixture(): CoilPathDto {
  const phases: PhaseCoilDto[] = [];
  for (let layer = 0; layer < 2; layer++) {
    for (let p = 0; p < 3; p++) {
      const segs = [];
      const offset = p * 0.003;
      for (let i = 0; i < 4; i++) {
        const x = offset + i * 0.004;
        const yTop = layer % 2 === 0 ? 0 : 0.02;
        segs.push({ start: [x, yTop], end: [x, 0.02 - yTop], is_active: true });
        if (i < 3) {
          segs.push({
            start: [x, 0.02 - yTop],
            end: [x + 0.004, 0.02 - yTop],
            is_active: false,
          });
        }
      }
      phases.push({
        phase_idx: p,
        layer_idx: layer,
        phase_name: "ABC"[p],
        pattern_id: "infinity-braid",
        segments: segs,
        corner_arcs: layer === 0 && p === 0 ? [{ start: [0, 0.02], mid: [0.001, 0.021], end: [0.002, 0.02], is_active: false }] : [],
        via_positions: [],
        total_length_m: 1,
        active_length_m: 1,
        end_turn_length_m: 0,
        active_conductor_count: 4,
        bounding_box: [0, 0, 0.02, 0.02],
        terminal_start: [0, 0],
        terminal_end: [0.02, 0.02],
      });
    }
  }
  return { phases, layer_count: 4 };
}

describe("computeFootprintBox", () => {
  it("unions raw winding bounds without layer offsets", () => {
    const box = computeFootprintBox(braidLikeFixture());
    // Phases 0..2 at offset 0..0.006; conductors at x up to 0.006 + 3 * 0.004
    expect(box.minX).toBeCloseTo(0);
    expect(box.maxX).toBeCloseTo(0.018);
    expect(box.minY).toBeCloseTo(0);
    expect(box.maxY).toBeCloseTo(0.02);
  });

  it("includes corner-arc extremes", () => {
    const box = computeFootprintBox(braidLikeFixture());
    expect(box.maxY).toBeCloseTo(0.02); // arcs' mid (0.021) is INSIDE the layer-1 row, so y-extent is still the braid's
    expect(box.maxX).toBeGreaterThanOrEqual(0.018); // arcs don't extend x beyond segments here
  });

  it("is degenerate-safe with no coils", () => {
    const box = computeFootprintBox(null);
    expect(box.maxX).toBeGreaterThan(box.minX);
    expect(box.maxY).toBeGreaterThan(box.minY);
  });
});

describe("computePreviewGeometry", () => {
  it("overlays layers at their true coordinates (no exploded offset)", () => {
    const g = computePreviewGeometry(braidLikeFixture(), CONFIG);
    // contentBox == footprintBox: layers share the same coordinates.
    expect(g.contentBox.minX).toBe(g.footprintBox.minX);
    expect(g.contentBox.minY).toBe(g.footprintBox.minY);
    expect(g.contentBox.maxX).toBe(g.footprintBox.maxX);
    expect(g.contentBox.maxY).toBe(g.footprintBox.maxY);
    expect(g.contentBox.maxY).toBeCloseTo(0.02);
    expect(g.contentBox.minY).toBeCloseTo(0);
  });

  it("places the magnet strip 1 mm above the tallest content", () => {
    const g = computePreviewGeometry(braidLikeFixture(), CONFIG);
    expect(g.magnetTop).toBeCloseTo(g.contentBox.maxY + 0.001);
    expect(g.magnetSpan).toBeCloseTo(10 * 0.012);
    expect(g.fitBounds.maxY).toBeCloseTo(g.magnetTop + 0.003);
    // fit x must cover both the winding and the magnet span origin
    expect(g.fitBounds.minX).toBeLessThanOrEqual(g.contentBox.minX);
    expect(g.fitBounds.maxX).toBeGreaterThanOrEqual(g.contentBox.maxX);
  });

  it("sizes the board panel 1.5 mm around the exploded content", () => {
    const g = computePreviewGeometry(braidLikeFixture(), CONFIG);
    expect(g.boardRect.x).toBeCloseTo(g.contentBox.minX - 0.0015);
    expect(g.boardRect.y).toBeCloseTo(g.contentBox.minY - 0.0015);
    expect(g.boardRect.w).toBeCloseTo(g.contentBox.maxX - g.contentBox.minX + 0.003);
    expect(g.boardRect.h).toBeCloseTo(g.contentBox.maxY - g.contentBox.minY + 0.003);
  });

  it("reports only the layers actually present, not the configured total", () => {
    const g = computePreviewGeometry(braidLikeFixture(), CONFIG);
    expect(g.renderedLayerCount).toBe(2); // fixture config says layer_count 4
    const empty = computePreviewGeometry(null, CONFIG);
    expect(empty.renderedLayerCount).toBe(0);
  });
});

describe("computeMagnets", () => {
  it("produces count pitch-spaced segments with alternating polarity", () => {
    const mags = computeMagnets(CONFIG);
    expect(mags).toHaveLength(10);
    expect(mags[0]).toEqual({ x: 0, w: 0.01, pole: 1 });
    expect(mags[1]).toEqual({ x: 0.012, w: 0.01, pole: -1 });
    expect(mags[9].x).toBeCloseTo(9 * 0.012);
  });
});

describe("computeUniqueLayers", () => {
  it("lists distinct layer indices ascending", () => {
    const layers = computeUniqueLayers(braidLikeFixture());
    expect(layers.map((l) => l.idx)).toEqual([0, 1]);
  });

  it("returns [] with no coils", () => {
    expect(computeUniqueLayers(null)).toEqual([]);
  });
});

describe("computeUniquePhases", () => {
  it("dedupes per-(phase, layer) coils by phase_idx, sorted ascending", () => {
    const uniq = computeUniquePhases(braidLikeFixture());
    expect(uniq.map((u) => u.idx)).toEqual([0, 1, 2]);
    expect(uniq[0].name).toBe("A");
  });

  it("returns [] with no coils", () => {
    expect(computeUniquePhases(null)).toEqual([]);
  });
});

describe("computeVisibleSegments", () => {
  const coilKey = (ph: PhaseCoilDto) => ph.phase_idx * 1000 + ph.layer_idx;

  it("fast path returns all segments keyed by phase+layer", () => {
    const coils = braidLikeFixture();
    const m = computeVisibleSegments(coils, false, 6);
    expect(m.size).toBe(coils.phases.length);
    for (const ph of coils.phases) {
      expect(m.get(coilKey(ph))).toBe(ph.segments);
    }
  });

  it("one-section keeps the first N active conductors + interior end-turns", () => {
    const coils = braidLikeFixture();
    const m = computeVisibleSegments(coils, true, 2);
    for (const ph of coils.phases) {
      const segs = m.get(coilKey(ph))!;
      const actives = segs.filter((s) => s.is_active);
      // 4 conductors but only 2 kept
      expect(actives.length).toBeLessThanOrEqual(2);
      expect(actives.length).toBeGreaterThan(0);
      // end-turn bridging the two kept actives survives
      expect(segs.some((s) => !s.is_active)).toBe(true);
    }
  });

  it("returns empty map with no coils", () => {
    expect(computeVisibleSegments(null, false, 6).size).toBe(0);
  });
});

describe("computeVisibleArcs", () => {
  it("fast path returns all arcs keyed by phase+layer", () => {
    const coils = braidLikeFixture();
    const m = computeVisibleArcs(coils, false, 6);
    expect(m.size).toBe(coils.phases.length);
    // only layer 0 phase 0 has arcs in the fixture
    const withArcs = [...m.values()].filter((a) => a.length > 0);
    expect(withArcs).toHaveLength(1);
  });

  it("one-section drops arcs outside the kept x-window", () => {
    const coils = braidLikeFixture();
    const m = computeVisibleArcs(coils, true, 1);
    let kept = 0;
    for (const arcs of m.values()) kept += arcs.length;
    // kept window is the first conductor (x ~ 0..0.004); fixture arc sits at
    // x 0..0.002 → inside → still kept.
    expect(kept).toBe(1);
  });
});

// ===========================================================================
// routing_dimensions sidecar overlays
// ===========================================================================

/** Build a minimal SlotWidthDto record (the helpers only read the fields we
 *  exercise here). */
function slot(
  layer: number,
  net: string,
  width: number,
  max: number | null = null,
  margin: number | null = null,
): SlotWidthDto {
  return {
    layer,
    net,
    trace_count: 1,
    trace_width_m: 0.0002,
    trace_spacing_m: 0.0002,
    angle_rad: Math.PI / 2,
    slot_width_m: width,
    max_slot_width_m: max,
    margin_m: margin,
  };
}

describe("computePolePitchValue", () => {
  it("returns null for absent/legacy sidecars and invalid pitches", () => {
    expect(computePolePitchValue(null)).toBeNull();
    expect(computePolePitchValue(undefined)).toBeNull();
    expect(computePolePitchValue({ pole_pitch_m: null, period_pitch_m: null })).toBeNull();
    expect(computePolePitchValue({ pole_pitch_m: 0, period_pitch_m: 0 })).toBeNull();
    expect(computePolePitchValue({ pole_pitch_m: -0.012, period_pitch_m: -0.013 })).toBeNull();
    expect(computePolePitchValue({ pole_pitch_m: Number.NaN, period_pitch_m: Number.NaN })).toBeNull();
    expect(
      computePolePitchValue({ pole_pitch_m: Number.POSITIVE_INFINITY, period_pitch_m: undefined }),
    ).toBeNull();
  });

  it("prefers pole_pitch_m and labels it Pole pitch", () => {
    const v = computePolePitchValue({ pole_pitch_m: 0.012, period_pitch_m: 0.013 });
    expect(v).toEqual({ pitchM: 0.012, source: "pole_pitch_m", label: "Pole pitch" });
  });

  it("falls back to period_pitch_m but clearly labels it Repeat period", () => {
    const v = computePolePitchValue({ pole_pitch_m: null, period_pitch_m: 0.013 });
    expect(v).toEqual({ pitchM: 0.013, source: "period_pitch_m", label: "Repeat period" });
  });
});

describe("firstMagnetCenterX", () => {
  it("returns the centre of the first magnet strip", () => {
    expect(firstMagnetCenterX([{ x: 0, w: 0.01, pole: 1 }])).toBeCloseTo(0.005);
  });

  it("returns null when the array is empty", () => {
    expect(firstMagnetCenterX([])).toBeNull();
  });
});

describe("computePolePitchRuler", () => {
  it("is null when the pitch or the magnet centre is unavailable", () => {
    expect(computePolePitchRuler({ pole_pitch_m: 0.012 }, null, 0.1)).toBeNull();
    expect(computePolePitchRuler(null, 0.005, 0.1)).toBeNull();
  });

  it("aligns the ruler to first centre → first centre + pitch", () => {
    const r = computePolePitchRuler({ pole_pitch_m: 0.012 }, 0.005, 0.1);
    expect(r).not.toBeNull();
    expect(r!.x1).toBeCloseTo(0.005);
    expect(r!.x2).toBeCloseTo(0.017);
    expect(r!.y).toBeCloseTo(0.1);
    expect(r!.label).toBe("Pole pitch");
    expect(r!.source).toBe("pole_pitch_m");
  });
});

describe("computeSlotWidthRows", () => {
  it("returns [] while the sidecar is missing", () => {
    expect(computeSlotWidthRows(braidLikeFixture(), null)).toEqual([]);
    expect(computeSlotWidthRows(null, { slot_widths: [] })).toEqual([]);
  });

  it("matches each record to its (layer, net) phase coil", () => {
    const coils = braidLikeFixture();
    const rows = computeSlotWidthRows(coils, {
      slot_widths: [
        slot(0, "A", 0.002, 0.004, 0.002),
        slot(1, "B", 0.003, 0.0025, -0.0005),
        slot(0, "C", 0.002, null, null),
        slot(0, "D", 0.002, 0.004, 0.002), // no phase "D" in the fixture
      ],
    });
    expect(rows).toHaveLength(3);

    const a = rows.find((r) => r.phaseName === "A")!;
    expect(a.phaseIdx).toBe(0);
    expect(a.layer).toBe(0);
    // median active-conductor x for phase A (offset 0, 4 conductors at 0..0.012)
    expect(a.anchorX).toBeCloseTo(0.006);
    expect(a.slotM).toBeCloseTo(0.002);
    expect(a.maxM).toBeCloseTo(0.004);
    expect(a.marginM).toBeCloseTo(0.002);
    expect(a.status).toBe("ok");

    const b = rows.find((r) => r.phaseName === "B")!;
    expect(b.phaseIdx).toBe(1);
    expect(b.status).toBe("over-budget");

    const c = rows.find((r) => r.phaseName === "C")!;
    expect(c.status).toBe("no-limit");
    expect(c.maxM).toBeNull();
    expect(c.marginM).toBeNull();
  });

  it("skips records with invalid slot widths", () => {
    const coils = braidLikeFixture();
    const rows = computeSlotWidthRows(coils, {
      slot_widths: [
        slot(0, "A", 0),
        slot(0, "A", Number.NaN),
        slot(0, "A", -0.001),
      ],
    });
    expect(rows).toEqual([]);
  });

  it("keeps a finite zero/negative limit visible as an over-budget diagnostic", () => {
    const rows = computeSlotWidthRows(braidLikeFixture(), {
      slot_widths: [slot(0, "A", 0.002, 0, -0.002)],
    });
    expect(rows).toHaveLength(1);
    expect(rows[0].maxM).toBe(0);
    expect(rows[0].marginM).toBe(-0.002);
    expect(rows[0].status).toBe("over-budget");
  });

  it("stacks matched rows symmetrically around the conductor rail", () => {
    const coils = braidLikeFixture();
    const rows = computeSlotWidthRows(coils, {
      slot_widths: [slot(0, "A", 0.002), slot(1, "A", 0.002)],
    });
    expect(rows).toHaveLength(2);
    // fixture median active y is 0.01 for both layers; stride 1 mm by default.
    expect(rows[0].y).toBeCloseTo(0.01 - 0.0005);
    expect(rows[1].y).toBeCloseTo(0.01 + 0.0005);
  });
});

describe("computeOverlayFitBounds", () => {
  const base = { minX: 0, minY: 0, maxX: 0.018, maxY: 0.033 };

  it("keeps the base box when there are no overlays", () => {
    expect(computeOverlayFitBounds(base, null, [])).toEqual(base);
  });

  it("expands the fit for the ruler floating above the magnet strip", () => {
    const ruler = computePolePitchRuler({ pole_pitch_m: 0.012 }, 0.005, 0.034);
    const bounds = computeOverlayFitBounds(base, ruler, []);
    expect(bounds.maxY).toBeGreaterThan(base.maxY); // ruler above the strip
    expect(bounds.maxX).toBeGreaterThanOrEqual(base.maxX);
    expect(bounds.minX).toBeLessThanOrEqual(base.minX);
  });

  it("covers slot-row brackets that reach past the content box", () => {
    const row = {
      phaseIdx: 0,
      layer: 0,
      phaseName: "A",
      anchorX: 0.019,
      y: 0.012,
      slotM: 0.004,
      maxM: null,
      marginM: null,
      status: "no-limit" as const,
    };
    const bounds = computeOverlayFitBounds(base, null, [row]);
    expect(bounds.maxX).toBeGreaterThan(base.maxX);
  });

  it("expands the fit to cover pole-region zones reaching into padding", () => {
    const zones = computePoleRegionZones(poleRegionsFixture());
    const zoneBox = computePoleRegionBounds(zones, 0.018, 0.02)!;
    expect(zoneBox).not.toBeNull();
    expect(zoneBox.minX).toBeLessThan(base.minX); // A[0] at -0.01 (out of content)
    expect(zoneBox.maxX).toBeGreaterThan(base.maxX); // C[last] at 0.014 (out of content)
    const bounds = computeOverlayFitBounds(base, null, [], zones, 0.018, 0.02);
    expect(bounds.minX).toBeLessThan(base.minX);
    expect(bounds.maxX).toBeGreaterThan(base.maxX);
    // Backward compatible: no zones → identical to the 3-arg call.
    expect(computeOverlayFitBounds(base, null, [])).toEqual(base);
  });
});

/** Minimal, valid pole-region sidecar used by the zone tests below. */
function poleRegionsFixture(): { pole_regions: PoleRegionDto[] } {
  return {
    pole_regions: [
      { phase: "A", pole_index: 0, start: [-0.004, 0.001], end: [0.008, 0.001] },
      { phase: "A", pole_index: 1, start: [0.008, 0.001], end: [0.02, 0.001] },
      { phase: "B", pole_index: 0, start: [0.005, 0.01], end: [0.017, 0.01] },
      { phase: "C", pole_index: 0, start: [0.012, 0.01], end: [0.009, 0.01] }, // inverted x
    ],
  };
}

describe("poleRegionPolarity", () => {
  it("alternates +1 even / -1 odd by pole_index", () => {
    expect(poleRegionPolarity(0)).toBe(1);
    expect(poleRegionPolarity(1)).toBe(-1);
    expect(poleRegionPolarity(2)).toBe(1);
    expect(poleRegionPolarity(3)).toBe(-1);
  });
});

describe("isValidPoleRegion", () => {
  const good: PoleRegionDto = { phase: "A", pole_index: 0, start: [0, 0], end: [0.012, 0] };
  it("accepts a well-formed region", () => {
    expect(isValidPoleRegion(good)).toBe(true);
  });
  it("rejects missing / malformed / legacy input", () => {
    expect(isValidPoleRegion(null)).toBe(false);
    expect(isValidPoleRegion(undefined)).toBe(false);
    expect(isValidPoleRegion({} as PoleRegionDto)).toBe(false);
    expect(isValidPoleRegion({ ...good, phase: "" })).toBe(false);
    expect(isValidPoleRegion({ ...good, phase: "   " })).toBe(false);
    expect(isValidPoleRegion({ ...good, pole_index: -1 })).toBe(false);
    expect(isValidPoleRegion({ ...good, pole_index: 1.5 })).toBe(false);
    expect(isValidPoleRegion({ ...good, pole_index: Number.NaN })).toBe(false);
    expect(isValidPoleRegion({ ...good, start: [Number.NaN, 0] })).toBe(false);
    expect(isValidPoleRegion({ ...good, end: [0.012, Number.POSITIVE_INFINITY] })).toBe(false);
    expect(isValidPoleRegion({ ...good, start: [0, 0, 0] } as unknown as PoleRegionDto)).toBe(true); // extra coords tolerated
  });
  it("rejects a zero-width x span", () => {
    expect(isValidPoleRegion({ ...good, end: [0, 0] })).toBe(false);
  });
});

describe("computePoleRegionZones", () => {
  it("returns [] for missing/legacy/malformed dimensions", () => {
    expect(computePoleRegionZones(null)).toEqual([]);
    expect(computePoleRegionZones(undefined)).toEqual([]);
    expect(computePoleRegionZones({ pole_regions: [] })).toEqual([]);
    expect(computePoleRegionZones({ pole_regions: [{ phase: "A", pole_index: 0, start: [0, 0], end: [0, 0] }] })).toEqual([]);
  });
  it("validates, orders boundaries, and assigns alternating polarity", () => {
    const zones = computePoleRegionZones(poleRegionsFixture());
    expect(zones).toHaveLength(4);
    // Inverted C region ordered so x0 <= x1.
    const c = zones.find((z) => z.phase === "C")!;
    expect(c.x0).toBeCloseTo(0.009);
    expect(c.x1).toBeCloseTo(0.012);
    expect(c.polarity).toBe(1);
    // Even pole_index → polarity +1, odd → -1.
    const a0 = zones.find((z) => z.phase === "A" && z.poleIndex === 0)!;
    const a1 = zones.find((z) => z.phase === "A" && z.poleIndex === 1)!;
    expect(a0.polarity).toBe(1);
    expect(a1.polarity).toBe(-1);
  });
  it("sorts deterministically by ascending x", () => {
    const zones = computePoleRegionZones(poleRegionsFixture());
    for (let i = 1; i < zones.length; i++) {
      // Cards may share x0 (A[0] and B[0] both start at -0.004/0.005 is fine);
      // strictly ascending over the sorted array.
      expect(zones[i].x0).toBeGreaterThanOrEqual(zones[i - 1].x0);
    }
  });
});

describe("computePoleRegionPhases", () => {
  it("extracts distinct, sorted phase labels from region data", () => {
    const zones = computePoleRegionZones(poleRegionsFixture());
    expect(computePoleRegionPhases(zones)).toEqual(["A", "B", "C"]);
  });
  it("returns [] when there are no valid zones", () => {
    expect(computePoleRegionPhases([])).toEqual([]);
  });
});

describe("filterPoleRegionsByPhase", () => {
  const zones = computePoleRegionZones(poleRegionsFixture());
  it("returns all zones for null/empty selection", () => {
    expect(filterPoleRegionsByPhase(zones, null)).toHaveLength(4);
    expect(filterPoleRegionsByPhase(zones, "")).toHaveLength(4);
  });
  it("returns only the selected phase's zones", () => {
    const a = filterPoleRegionsByPhase(zones, "A");
    expect(a).toHaveLength(2);
    expect(a.every((z) => z.phase === "A")).toBe(true);
  });
  it("returns [] for an unknown phase", () => {
    expect(filterPoleRegionsByPhase(zones, "Z")).toEqual([]);
  });
  it("returns a fresh copy (never mutates the source)", () => {
    const all = filterPoleRegionsByPhase(zones, "");
    expect(all).not.toBe(zones);
  });
});

describe("resolvePoleRegionPhaseSelection", () => {
  it("keeps a valid selection and always keeps '' (all phases)", () => {
    expect(resolvePoleRegionPhaseSelection("A", ["A", "B", "C"])).toBe("A");
    expect(resolvePoleRegionPhaseSelection("", ["A", "B"])).toBe("");
  });
  it("collapses a stale selection to '' when the label is gone", () => {
    expect(resolvePoleRegionPhaseSelection("C", ["A", "B"])).toBe("");
    expect(resolvePoleRegionPhaseSelection("Z", [])).toBe("");
  });
});

describe("computePoleRegionBounds", () => {
  it("returns null for no zones", () => {
    expect(computePoleRegionBounds([], 0, 1)).toBeNull();
  });
  it("unions x spans and honours the supplied y band", () => {
    const zones = computePoleRegionZones(poleRegionsFixture());
    const b = computePoleRegionBounds(zones, 0.018, 0.02)!;
    expect(b.minX).toBeCloseTo(-0.004);
    expect(b.maxX).toBeCloseTo(0.02);
    expect(b.minY).toBeCloseTo(0.018);
    expect(b.maxY).toBeCloseTo(0.02);
  });
});

describe("formatMetresMm / formatMarginMm", () => {
  it("formats metre lengths as compact mm", () => {
    expect(formatMetresMm(0.012)).toBe("12.0 mm");
    expect(formatMetresMm(0.001777)).toBe("1.78 mm");
    expect(formatMetresMm(0.0002)).toBe("0.200 mm");
    expect(formatMetresMm(Number.NaN)).toBe("—");
  });

  it("formats signed slot margins", () => {
    expect(formatMarginMm(0.002)).toBe("+2.00 mm");
    expect(formatMarginMm(-0.0005)).toBe("-0.500 mm");
  });
});

describe("medianActiveX", () => {
  it("returns the median active conductor x, ignoring end-turns", () => {
    const coils = braidLikeFixture();
    const ph = coils.phases.find((p) => p.phase_idx === 1 && p.layer_idx === 0)!;
    // phase 1 offset 0.003, conductors at 0.003..0.015 → median 0.009
    expect(medianActiveX(ph)).toBeCloseTo(0.009);
  });
});
