import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Mover screenshot tooling (kata 8tc4).
 *
 * Captures the app at the min and max mover-position slider endpoints so
 * the magnet position can be VERIFIED VISUALLY against the copper band —
 * the field-verification loop for the travel-envelope endpoint spec
 * (kata xb16 / 5c7r).
 *
 * Run with: `pnpm shot:mover` (desktop-tauri project, 1280×800).
 * Output: screenshots/mover-at-min.png and screenshots/mover-at-max.png.
 *
 * The browser (vite dev) build runs on the deterministic mock IPC, whose
 * `mockTravelEnvelope` mirrors the Rust `travel_envelope_over_slots` 1:1,
 * so what is captured here is the same endpoint geometry the backend
 * computes. Frontend defaults (N=12, P_e=12 mm, active area 147 mm — the
 * copper active area is the whole track) under the FLUSH endpoint spec
 * (kata 5c7r): span 72 mm → envelope 36 → 111 mm, so the drawn strip
 * spans 0 → 72 mm at min and 75 → 147 mm at max — array edges EXACTLY on
 * the copper bounds at both endpoints, sweep = configured travel (75 mm)
 * exactly. (Note: the geometric fallback now equals the envelope limits,
 * so bounds cannot distinguish envelope arrival — pinned via the readout
 * values instead.)
 */

const OUT_DIR = fileURLToPath(new URL("../screenshots", import.meta.url));
const MOVER_BLOCK = 'div[aria-label="Mover position"]';
const SLIDER = `${MOVER_BLOCK} input[aria-label="Mover position slider (mm)"]`;
const READOUT = `${MOVER_BLOCK} div[aria-live="polite"]`;

test("capture the mover at the min and max travel endpoints", async ({
  page,
}) => {
  await page.goto("/");
  const slider = page.locator(SLIDER).first();
  await slider.waitFor({ state: "visible", timeout: 15_000 });

  // Wait for the debounced preview/envelope stream to settle. Under the
  // flush endpoint spec (kata 5c7r) the geometric fallback equals the
  // envelope limits, so the sweep width (75 mm at defaults) cannot
  // distinguish them — the readout pins below carry the assertion instead.
  const readout = page.locator(READOUT).first();
  await expect(readout).toContainText("/ 75.0 mm", { timeout: 10_000 });

  fs.mkdirSync(OUT_DIR, { recursive: true });

  const setEndpoint = async (which: "min" | "max") => {
    await slider.evaluate((el: HTMLInputElement, w) => {
      el.value = w === "min" ? el.min : el.max;
      el.dispatchEvent(new Event("input", { bubbles: true }));
    }, which);
  };

  const shoot = async (label: "min" | "max", expectedExtent: string) => {
    await setEndpoint(label);
    // The readout prints the drawn strip extent ("Mover extent: X - Y mm")
    // from the SAME bounds the canvas overlay draws — pin it so the number,
    // the picture and the slider can never silently disagree.
    await expect(readout).toContainText(expectedExtent, { timeout: 5000 });
    const text = ((await readout.textContent()) ?? "").replace(/\s+/g, " ").trim();
    // eslint-disable-next-line no-console
    console.log(`[mover-at-${label}] ${text}`);
    // Full page (the reflection + readouts) plus the coil canvas itself —
    // the inline CoilPreview canvas draws the magnet strip over the traces;
    // the app scrolls internally so a plain fullPage shot misses it.
    await page.screenshot({
      path: path.join(OUT_DIR, `mover-at-${label}.png`),
      fullPage: true,
    });
    const canvas = page.locator("canvas:visible").first();
    await canvas.scrollIntoViewIfNeeded();
    await canvas.screenshot({
      path: path.join(OUT_DIR, `mover-at-${label}-canvas.png`),
    });
  };

  // Frontend defaults, flush spec (kata 5c7r): envelope 36 → 111 mm,
  // span 72 mm → strip 0 → 72 mm at min, 75 → 147 mm at max — array
  // edges EXACTLY on the copper bounds, sweep = configured travel.
  await shoot("min", "0.0 - 72.0 mm");
  await shoot("max", "75.0 - 147.0 mm");
});
