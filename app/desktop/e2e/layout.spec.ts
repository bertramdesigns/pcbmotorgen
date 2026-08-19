import { test, expect } from "@playwright/test";
import {
  DESIGN_SCROLL,
  designPanel,
  leftColumnFirstCard,
  ASIDE,
  box,
} from "./helpers";

test.describe("Design tab layout geometry", () => {
  test("left column first card aligns with the right column content padding", async ({
    page,
  }) => {
    await page.goto("/");
    const panel = await box(page, designPanel(page));
    const card = await box(page, leftColumnFirstCard(page));
    expect(panel).not.toBeNull();
    expect(card).not.toBeNull();
    // The right column's content begins 16px (p-4) below the panel edge;
    // the aside must start at exactly the same visual line.
    expect(Math.abs(card!.y - (panel!.y + 16))).toBeLessThanOrEqual(1);
  }, { tag: ["@visual", "@desktop"] });

  test("design-tab scrollbar sits flush with the right window edge", async ({
    page,
  }) => {
    await page.goto("/");
    const scroll = page.locator(DESIGN_SCROLL);
    await expect(scroll).toBeVisible();
    await expect(scroll).toHaveCSS("overflow-y", "auto");
    const b = await box(page, scroll);
    expect(b).not.toBeNull();
    const innerWidth = await page.evaluate(() => window.innerWidth);
    // The scrolling container must extend to the window's right edge so the
    // scrollbar no longer floats 16-32px inside the layout.
    expect(innerWidth - (b!.x + b!.w)).toBeLessThanOrEqual(1);
  }, { tag: ["@visual", "@desktop"] });

  test("traces preview is visible in the Design-tab left column", async ({
    page,
  }) => {
    await page.goto("/");
    const aside = page.locator(ASIDE);
    const coilHeading = aside.locator("text=Coil Preview");
    await expect(coilHeading).toBeVisible();
    // It renders the canvas viewer, not a blank "awaiting generation" state.
    const preview = aside.locator("[aria-label='Coil preview']");
    await expect(preview).toBeVisible();
    await expect(
      preview.locator("canvas[data-revision]"),
    ).toBeVisible();
  }, { tag: ["@visual", "@desktop"] });

  test("page height is locked and the footer is always visible", async ({
    page,
  }) => {
    await page.goto("/");
    // Attempt to scroll the window: it must not move (the root scroller is
    // locked at the desktop layout; only the inner columns scroll).
    await page.evaluate(() => window.scrollTo(0, 400));
    await page.waitForTimeout(150);
    expect(await page.evaluate(() => window.scrollY)).toBe(0);

    const footer = page.locator("footer");
    await expect(footer).toBeVisible();
    const fb = await footer.boundingBox();
    expect(fb).not.toBeNull();
    expect(fb!.y).toBeGreaterThanOrEqual(0);
    const innerHeight = await page.evaluate(() => window.innerHeight);
    expect(fb!.y + fb!.height).toBeLessThanOrEqual(innerHeight + 1);
  }, { tag: ["@visual", "@desktop"] });

  test("left and right columns scroll independently", async ({ page }) => {
    await page.goto("/");
    const aside = page.locator(ASIDE);
    const right = page.locator(DESIGN_SCROLL);
    // Both columns are their own scroll containers at the lg layout.
    await expect(aside).toHaveCSS("overflow-y", "auto");
    await expect(right).toHaveCSS("overflow-y", "auto");

    const overflow = await page.evaluate(() => {
      const left = document.querySelector("aside[aria-label='Persistent design reflection']")!;
      const settings = document.querySelector("#design-settings-scroll")!;
      return {
        left: left.scrollHeight > left.clientHeight,
        settings: settings.scrollHeight > settings.clientHeight,
      };
    });
    // The simplified Design tab can fit completely at the wide viewport. In
    // that case there is no scroll position to assert; the desktop-tauri
    // viewport still exercises the independent-scroll behavior below.
    test.skip(
      !overflow.left || !overflow.settings,
      "this viewport has no overflowing design column",
    );

    // Scrolling inside the left column must not move the right column.
    await aside.evaluate((el) => {
      el.scrollTop = 200;
    });
    await page.waitForTimeout(120);
    const afterLeft = await page.evaluate(() => ({
      left: document.querySelector("aside[aria-label='Persistent design reflection']")!.scrollTop,
      right: document.querySelector("#design-settings-scroll")!.scrollTop,
      windowY: window.scrollY,
    }));
    expect(afterLeft.left).toBeGreaterThan(0);
    expect(afterLeft.right).toBe(0);
    expect(afterLeft.windowY).toBe(0);

    // And scrolling the right column must not move the left column.
    await right.evaluate((el) => {
      el.scrollTop = 200;
    });
    await page.waitForTimeout(120);
    const afterRight = await page.evaluate(() => ({
      left: document.querySelector("aside[aria-label='Persistent design reflection']")!.scrollTop,
      right: document.querySelector("#design-settings-scroll")!.scrollTop,
    }));
    expect(afterRight.right).toBeGreaterThan(0);
    expect(afterRight.left).toBe(200);
  }, { tag: ["@visual", "@desktop"] });
});
