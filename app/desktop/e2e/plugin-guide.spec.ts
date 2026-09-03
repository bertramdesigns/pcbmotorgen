import { test, expect } from "@playwright/test";

/**
 * In-app plugin authoring guide smoke test (kata bprp).
 *
 * The guide modal must be reachable from the generator upload panel
 * ("How to write a plugin?" entry point) and its content must come from the
 * docs BUNDLED AT BUILD TIME from crates/pcbmotorgen-routing/docs (via Vite
 * `?raw` imports) — the assertions below check for verbatim doc headings so
 * a forked/empty placeholder body would fail.
 *
 * Stacked-modal discipline (kata xy31 scroll-lock precedent): the guide
 * opens ABOVE the upload dialog; Escape closes ONLY the guide while the
 * page scroll lock stays engaged (refcounted), and the second Escape closes
 * the upload dialog and releases the lock.
 */

const PATTERN_SELECT = 'select[aria-label="Routing pattern"]';
const UPLOAD_DIALOG = "[role='dialog'][aria-label='Load new generator']";
const HELP_BUTTON = "button[aria-haspopup='dialog']:has-text('How to write a plugin?')";
const GUIDE_DIALOG = "[role='dialog'][aria-label='How to write a routing plugin']";

test.describe("plugin authoring guide @interaction @desktop", () => {
  test("opens from the upload panel, renders the bundled crate docs, and closes independently", async ({
    page,
  }) => {
    await page.goto("/");

    // Open the "Load new generator" modal from the routing-pattern dropdown.
    await page.selectOption(PATTERN_SELECT, "__load_generator__");
    await expect(page.locator(UPLOAD_DIALOG)).toBeVisible();

    // The entry point inside the upload panel opens the stacked guide.
    await page.locator(HELP_BUTTON).click();
    const guide = page.locator(GUIDE_DIALOG);
    await expect(guide).toBeVisible();

    // Verbatim headings from routing-pattern-authoring.md — proves the body
    // is the bundled doc, not a stub (and that no runtime fetch is needed).
    await expect(guide).toContainText("The contract in one paragraph");
    await expect(guide).toContainText("Authoring a Rust cdylib plugin");

    // Landed features must be documented (we8r layer ranges + multiple_of,
    // hzs2 phase bands, htcq IO elements) — no "coming soon" placeholders.
    await expect(guide).toContainText("layers_multiple_of");
    await expect(guide).toContainText("phase_bands");
    await expect(guide).toContainText("io_pads");

    // The worked example tab bundles the reference runner verbatim.
    await page.getByRole("tab", { name: "Worked example" }).click();
    await expect(guide).toContainText('"id": "example-runner"');

    // Escape closes ONLY the guide; the upload dialog stays open and the
    // refcounted scroll lock stays engaged while it does.
    await page.keyboard.press("Escape");
    await expect(guide).toBeHidden();
    await expect(page.locator(UPLOAD_DIALOG)).toBeVisible();
    expect(
      await page.evaluate(() => document.documentElement.style.overflow),
    ).toBe("hidden");

    // Second Escape closes the upload dialog and releases the lock.
    await page.keyboard.press("Escape");
    await expect(page.locator(UPLOAD_DIALOG)).toBeHidden();
    expect(
      await page.evaluate(() => document.documentElement.style.overflow),
    ).toBe("");
  });
});
