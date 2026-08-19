/**
 * Generic SVG chart math: linear scales, padded ranges, ticks, polylines,
 * and world→view fit transforms.
 *
 * Extracted from ForceSweepPlot and CoilPreview so every chart shares one
 * tested implementation.
 */

export interface LinearScale {
  /** Map a data value to a pixel position. */
  (v: number): number;
  /** Pixel span occupied by one data unit. */
  scale: number;
  /** Data value at pixel position 0. */
  min: number;
  /** Data value at pixel position `spanPx`. */
  max: number;
}

/**
 * Build a linear data→pixel scaler.
 *
 * @param min data value at `startPx`
 * @param max data value at `startPx + spanPx`
 * @param startPx pixel position of `min`
 * @param spanPx pixel length of the axis
 */
export function createLinearScale(
  min: number,
  max: number,
  startPx: number,
  spanPx: number,
): LinearScale {
  const denom = max - min || 1;
  const scale = spanPx / denom;
  const fn = ((v: number) => startPx + (v - min) * scale) as LinearScale;
  fn.scale = scale;
  fn.min = min;
  fn.max = max;
  return fn;
}

/** Pad a [min, max] range by `padFraction` of its width (0.08 ≈ 8%). */
export function paddedRange(
  values: number[],
  padFraction = 0.08,
): [number, number] {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const pad = (max - min) * padFraction || 0.001;
  return [min - pad, max + pad];
}

/** Evenly spaced tick values + pixel positions along a scale. */
export function ticks(
  min: number,
  max: number,
  count: number,
  sx: LinearScale,
): { v: number; pos: number }[] {
  return Array.from({ length: count }, (_, i) => {
    const v = min + ((max - min) * i) / (count - 1);
    return { v, pos: sx(v) };
  });
}

/** Render (xs, ys) pairs as an SVG polyline points string. */
export function polyline(sx: LinearScale, sy: LinearScale, xs: number[], ys: number[]): string {
  return ys.map((y, i) => `${sx(xs[i]).toFixed(1)},${sy(y).toFixed(1)}`).join(" ");
}

// ---------------------------------------------------------------------------
// World → view fit transform (CoilPreview)
// ---------------------------------------------------------------------------

export interface ViewportBBox {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

/**
 * Defensive bbox: take the min/max of the two x-components and the two
 * y-components separately, so the result is correct regardless of whether
 * the bbox is stored as `[min_x, min_y, max_x, max_y]` or
 * `[max_x, max_y, min_x, min_y]`. The naïve "minX = min(bx0), maxX =
 * max(bx1)" form collapses to bboxW ≈ 1e-6 in the inverted case, which
 * then blows the fit-scale up to ~2e8 and squashes the whole winding into
 * a hairline off-screen — i.e. the "blank SVG" symptom.
 */
export function computeBBox(bboxes: [number, number, number, number][]): ViewportBBox {
  if (bboxes.length === 0) {
    return { minX: 0, minY: 0, maxX: 0.001, maxY: 0.001 };
  }
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (const [b0, b1, b2, b3] of bboxes) {
    minX = Math.min(minX, b0, b2);
    minY = Math.min(minY, b1, b3);
    maxX = Math.max(maxX, b0, b2);
    maxY = Math.max(maxY, b1, b3);
  }
  return {
    minX,
    minY,
    maxX: Math.max(maxX, minX + 1e-6),
    maxY: Math.max(maxY, minY + 1e-6),
  };
}

/**
 * Union of several world-space boxes into one.
 */
export function unionBounds(...boxes: ViewportBBox[]): ViewportBBox {
  if (boxes.length === 0) return { minX: 0, minY: 0, maxX: 0.001, maxY: 0.001 };
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const b of boxes) {
    minX = Math.min(minX, b.minX); minY = Math.min(minY, b.minY);
    maxX = Math.max(maxX, b.maxX); maxY = Math.max(maxY, b.maxY);
  }
  return { minX, minY, maxX: Math.max(maxX, minX + 1e-6), maxY: Math.max(maxY, minY + 1e-6) };
}

/**
 * Expand a box by `margin` on all sides (in the box's own units).
 */
export function expandBounds(box: ViewportBBox, margin: number): ViewportBBox {
  return { minX: box.minX - margin, minY: box.minY - margin, maxX: box.maxX + margin, maxY: box.maxY + margin };
}

export interface WorldTransform {
  s: number;
  tx: number;
  ty: number;
}

/**
 * Fit a world bbox into `viewW × viewH` minus `pad` on each side, with
 * `meet` aspect behaviour (smaller of the two scales, bbox never
 * overflows) and optional zoom. The returned transform assumes the group
 * applies `scale(1, −1)` before the translate, so y-up world coordinates
 * render correctly in SVG's y-down screen space:
 *   `transform="translate(tx, ty) scale(1 -1) scale(s)"` with world
 *   coordinates drawn at `translate(−minX, −minY)`-style origin handled
 *   by the caller via bbox.
 */
export function fitWorldToView(
  bbox: ViewportBBox,
  viewW: number,
  viewH: number,
  pad: number,
  zoom = 1,
): WorldTransform {
  const bboxW = bbox.maxX - bbox.minX;
  const bboxH = bbox.maxY - bbox.minY;
  const drawW = viewW - 2 * pad;
  const drawH = viewH - 2 * pad;
  const fitScale = Math.min(drawW / bboxW, drawH / bboxH);
  const s = fitScale * zoom;
  const renderedW = bboxW * s;
  const renderedH = bboxH * s;
  const cx = pad + (drawW - renderedW) / 2;
  const cy = pad + (drawH - renderedH) / 2;
  return {
    s,
    // After scale(1, -1) the y-axis is flipped; ty shifts so the
    // bbox top (world maxY) lands at `cy` and the bbox bottom
    // (world minY) lands at `cy + bboxH*s`.
    tx: cx - bbox.minX * s,
    ty: cy + bbox.maxY * s,
  };
}