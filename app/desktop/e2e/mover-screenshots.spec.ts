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
 * copper active area is the whole track): envelope 34 → 106 mm, span
 * 72 mm, so the drawn strip spans −2 → 70 mm at min and 70 → 142 mm at
 * max (the ≤ P_e/2 nearest-snap overhang shows as the strip poking past
 * the copper band edge).
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

  // Wait for the travel envelope to arrive (mock IPC, debounced). The
  // geometric fallback bounds [span/2, active − span/2] are already
  // non-degenerate, so range width alone cannot distinguish them; the
  // envelope-installed sweep at the pinned defaults is 72 mm (34 → 106),
  // vs the 75 mm fallback range. The readout prints the sweep width.
  const readout = page.locator(READOUT).first();
  await expect(readout).toContainText("/ 72.0 mm", { timeout: 10_000 });

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

  // Frontend defaults: envelope 34 → 106 mm, span 72 mm → strip −2 → 70 at
  // min, 70 → 142 at max (nearest-snap overhang: kata xb16 pins).
  await shoot("min", "-2.0 - 70.0 mm");
  await shoot("max", "70.0 - 142.0 mm");
});
