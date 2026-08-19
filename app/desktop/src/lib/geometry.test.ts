import { describe, expect, it } from "vitest";
import {
  hasBackIron,
  coilSpanMm,
  slotPitchMm,
  restOffsetMm,
  clampMoverCenter,
  magnetPoles,
  isoProject,
  isoCenter,
  isoBoxPath,
} from "./geometry";

describe("hasBackIron", () => {
  it("is true only when the arrangement ends with BackIron and thickness > 0", () => {
    expect(hasBackIron("AlternatingBackIron", 1)).toBe(true);
    expect(hasBackIron("HalbachBackIron", 0.5)).toBe(true);
    expect(hasBackIron("HalbachBackIron", 0)).toBe(false);
    expect(hasBackIron("Alternating", 1)).toBe(false);
    expect(hasBackIron("Halbach", 2)).toBe(false);
  });
});

describe("coilSpanMm", () => {
  it("is magnet_count × pole pitch", () => {
    expect(coilSpanMm(10, 12)).toBe(120);
    expect(coilSpanMm(4, 5.5)).toBe(22);
  });
});

describe("slotPitchMm / restOffsetMm", () => {
  it("scales the pole pitch by the vernier ratio", () => {
    // 3 phases, 1:1 ratio → slot pitch = pitch / 3, rest = 0
    expect(slotPitchMm(12, 3, 1)).toBeCloseTo(4);
    expect(restOffsetMm(12, 3, 1)).toBe(0);
    // 5:6 ratio → rest offset = pitch/3 × 1/6
    expect(slotPitchMm(12, 3, 5 / 6)).toBeCloseTo(10 / 3);
    expect(restOffsetMm(12, 3, 5 / 6)).toBeCloseTo(2 / 3);
  });

  it("never returns a negative rest offset", () => {
    expect(restOffsetMm(12, 3, 1.5)).toBe(0);
  });
});

describe("clampMoverCenter", () => {
  it("keeps the mover center inside [span/2, length − span/2]", () => {
    expect(clampMoverCenter(60, 100, 200)).toBe(60); // inside
    expect(clampMoverCenter(0, 100, 200)).toBe(50); // clamped low
    expect(clampMoverCenter(300, 100, 200)).toBe(150); // clamped high
  });
});

describe("magnetPoles", () => {
  it("alternates N (+1) / S (−1) starting with N", () => {
    const bars = magnetPoles(4, 0.012, 0.01);
    expect(bars.map((b) => b.pole)).toEqual([1, -1, 1, -1]);
    bars.forEach((bar, i) => {
      expect(bar.x).toBeCloseTo(i * 0.012);
      expect(bar.w).toBe(0.01);
    });
  });
});

describe("isoProject", () => {
  it("projects a point with the documented coupling factors", () => {
    const [sx, sy] = isoProject(100, 10, 5, 10, 10, 0.5, 5.0);
    // sx = cx + (x + 0.45y)·sxy
    expect(sx).toBeCloseTo(10 + (100 + 4.5) * 0.5);
    // sy = cy + (−z·sz + 0.45y·sxy)
    expect(sy).toBeCloseTo(10 + (-25 + 2.25));
  });
});

describe("isoCenter", () => {
  it("centers the projected hull of the Z-stack bounding box", () => {
    const project = (x: number, y: number, z: number, cx: number, cy: number) =>
      isoProject(x, y, z, cx, cy, 0.5, 5.0);
    const { cx, cy } = isoCenter(
      { length: 100, width: 20, totalHeight: 5 },
      220,
      220,
      project,
    );
    // Centering must keep the projected hull within the canvas and balanced.
    expect(cx).toBeGreaterThan(0);
    expect(cx).toBeLessThan(220);
    expect(cy).toBeGreaterThan(0);
    expect(cy).toBeLessThan(220);
  });
});

describe("isoBoxPath", () => {
  it("emits a path with 12 edges (4 bottom + 4 top + 4 verticals)", () => {
    const project = (x: number, y: number, z: number, cx: number, cy: number) =>
      isoProject(x, y, z, cx, cy, 0.5, 5.0);
    const { d, corners } = isoBoxPath(0, 0, 0, 10, 4, 2, 110, 110, project);
    const moves = d.match(/M /g)?.length ?? 0;
    const lines = d.match(/ L /g)?.length ?? 0;
    expect(moves).toBe(12);
    expect(lines).toBe(12);
    expect(corners).toHaveLength(8);
  });
});