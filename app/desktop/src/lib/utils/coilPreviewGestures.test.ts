import { describe, it, expect } from "vitest";
import {
  clampZoom,
  formatZoom,
  nextZoomStepUp,
  nextZoomStepDown,
  CoilPreviewGestures,
  type CoilPreviewGesturesOptions,
} from "./coilPreviewGestures.svelte";

const STEPS = [0.5, 1, 1.5, 2, 3, 4, 6, 8, 10];

function makeGestures(
  overrides: Partial<CoilPreviewGesturesOptions> = {},
): CoilPreviewGestures {
  return new CoilPreviewGestures({
    virtualW: 760,
    virtualH: 260,
    minZoom: 0.5,
    maxZoom: 10,
    zoomSteps: STEPS,
    getWorldTransform: (zoom) => ({ s: zoom, tx: 0, ty: 0 }),
    ...overrides,
  });
}

describe("clampZoom", () => {
  it("clamps below the minimum and above the maximum", () => {
    expect(clampZoom(0.1, 0.5, 10)).toBe(0.5);
    expect(clampZoom(42, 0.5, 10)).toBe(10);
  });

  it("passes values inside the range through unchanged", () => {
    expect(clampZoom(1, 0.5, 10)).toBe(1);
    expect(clampZoom(6, 0.5, 10)).toBe(6);
  });
});

describe("formatZoom", () => {
  it("trims trailing zeros but keeps fractional precision", () => {
    expect(formatZoom(1)).toBe("1");
    expect(formatZoom(0.5)).toBe("0.5");
    expect(formatZoom(1.5)).toBe("1.5");
    expect(formatZoom(10)).toBe("10");
  });

  it("keeps continuous (non-step) zooms readable", () => {
    expect(formatZoom(2.333)).toBe("2.333");
    expect(formatZoom(2.71828)).toBe("2.718");
  });
});

describe("zoom button stepping", () => {
  it("climbs to the next step above the current (continuous) zoom", () => {
    expect(nextZoomStepUp(1, STEPS, 10)).toBe(1.5);
    expect(nextZoomStepUp(2.4, STEPS, 10)).toBe(3);
    expect(nextZoomStepUp(10, STEPS, 10)).toBe(10); // already at max
  });

  it("drops to the previous step below the current zoom", () => {
    expect(nextZoomStepDown(1, STEPS, 0.5)).toBe(0.5);
    expect(nextZoomStepDown(2.4, STEPS, 0.5)).toBe(2);
    expect(nextZoomStepDown(10, STEPS, 0.5)).toBe(8);
    expect(nextZoomStepDown(0.4, STEPS, 0.5)).toBe(0.5); // min floor
  });
});

describe("CoilPreviewGestures (instance-level, no DOM needed)", () => {
  it("starts at 1×", () => {
    const g = makeGestures();
    expect(g.zoom).toBe(1);
    expect(g.zoomLabel).toBe("1");
    expect(g.canZoomIn).toBe(false);
    expect(g.canZoomOut).toBe(false);
  });

  it("steps zoom with buttons and disables at the bounds", () => {
    const g = makeGestures();
    g.zoomIn();
    expect(g.zoom).toBe(1.5);
    g.zoomReset();
    expect(g.zoom).toBe(1);
    for (let i = 0; i < 12; i += 1) g.zoomIn();
    expect(g.zoom).toBe(10);
    expect(g.canZoomIn).toBe(true);
    expect(g.canZoomOut).toBe(false);
    for (let i = 0; i < 12; i += 1) g.zoomOut();
    expect(g.zoom).toBe(0.5);
    expect(g.canZoomOut).toBe(true);
  });

  it("resetView clears zoom and pan but keeps bounds", () => {
    const g = makeGestures();
    g.zoom = 7;
    g.panX = 60;
    g.panY = -40;
    g.resetView();
    expect(g.zoom).toBe(1);
    expect(g.panX).toBe(0);
    expect(g.panY).toBe(0);
  });

  it("continuous pinch math stays clamped via the exposed zoom assignment", () => {
    // Direct zoom writes are allowed on the class instance; the clamp is the
    // gesture path's responsibility (handled by updatePinch → clampZoom).
    const g = makeGestures();
    g.zoom = clampZoom(g.zoom * 3, g.minZoom, g.maxZoom);
    expect(g.zoom).toBe(3);
    g.zoom = clampZoom(g.zoom * 4, g.minZoom, g.maxZoom);
    expect(g.zoom).toBe(10); // clamped to max
  });
});