/**
 * pageScrollLock.ts
 * ============================================================================
 * Scroll discipline for modal overlays (Kata xy31 — coil preview modal
 * leaked wheel scrolling into the page behind it).
 *
 * While a modal is open, EVERY scroll path into the page behind it must be
 * dead. Two layers cover all of them:
 *
 *   1. `lockPageScroll(page)` — refcounted `overflow: hidden` on <html> and
 *      <body>. Kills document scrolling below `lg` (where the columns stack
 *      and the page itself scrolls); above `lg` the stylesheet already hides
 *      the root scroller, so the inline write is redundant but harmless.
 *      Each first lock snapshots the previous inline values and the LAST
 *      release restores them, so stacked modals compose in any open/close
 *      order.
 *
 *   2. `attachBackdropScrollGuard(backdrop, scrollable)` — NON-passive
 *      wheel + touchmove listeners on the overlay itself. Events landing on
 *      the dimmed backdrop (anywhere outside the modal's own scrollable
 *      panel) are preventDefault()-ed, so the browser's scroll-chaining can
 *      never reach a container behind the modal. Events inside `scrollable`
 *      are left to the browser so the panel scrolls itself — pair this with
 *      `overscroll-behavior: contain` (Tailwind `overscroll-contain`) on the
 *      panel so its own scrolling never chains past the modal either.
 *
 * The decision (`blocksBackdropScroll`) is pure and unit-tested; the DOM
 * plumbing is a thin wrapper. Listeners are attached non-passively because
 * preventDefault() is ignored inside passive wheel/touchmove listeners.
 *
 * Bits UI interplay (assessed in kata 1jfa — keep, do not replace with
 * Bits' built-in lock): every Dialog in this app (CoilPreview lightbox,
 * PluginAuthoringGuideModal, GeneratorUploadPanel) sets
 * `preventScroll={false}` on Dialog.Content so Bits' BodyScrollLock
 * (node_modules/bits-ui/dist/internal/body-scroll-lock.svelte.js) stays
 * out of the way. Bits writes overflow/padding/pointer-events on <body>
 * only — never inline on <html> — and ships no wheel/touchmove
 * preventDefault (its touchmove guard is iOS + documentElement-target
 * only), so it cannot satisfy the xy31 e2e contract
 * (e2e/modal-scroll-lock.spec.ts pins inline root+body overflow AND
 * backdrop wheel defaultPrevented). It must also not run BESIDE this
 * helper: Bits snapshots the <body> style attribute at lock time and
 * restores it after a 24ms delay, so its reset re-applies this lock's own
 * overflow:hidden after close — measured stuck lock (spec line 190).
 */

/** Minimal structural view of `document` (real documents satisfy this). */
export interface PageLike {
  documentElement: { style: { overflow: string } };
  body: { style: { overflow: string } };
}

let lockCount = 0;
let savedOverflow: { root: string; body: string } | null = null;

/**
 * Lock document scrolling (`overflow: hidden` on <html> and <body>).
 * Refcounted: nested/stacked locks compose and the page unlocks only when
 * the LAST lock is released. Returns an idempotent release callback that
 * restores the inline overflow values captured when the first lock was
 * taken.
 */
export function lockPageScroll(page: PageLike): () => void {
  if (lockCount === 0) {
    savedOverflow = {
      root: page.documentElement.style.overflow,
      body: page.body.style.overflow,
    };
    page.documentElement.style.overflow = "hidden";
    page.body.style.overflow = "hidden";
  }
  lockCount += 1;

  let released = false;
  return () => {
    if (released) return;
    released = true;
    lockCount = Math.max(0, lockCount - 1);
    if (lockCount === 0 && savedOverflow) {
      page.documentElement.style.overflow = savedOverflow.root;
      page.body.style.overflow = savedOverflow.body;
      savedOverflow = null;
    }
  };
}

/**
 * Decide whether a wheel/touchmove event that landed on `target` must be
 * blocked at the backdrop. Block by default (target outside the modal's
 * scrollable panel, or no panel known yet); allow only inside `scrollable`.
 * Pure so the semantics are unit-testable without a DOM. The panel param is
 * structurally typed (`contains`) so real Elements and test fakes both fit.
 */
export function blocksBackdropScroll(
  target: EventTarget | null,
  scrollable: { contains(node: unknown): boolean } | null | undefined,
): boolean {
  if (!scrollable || target === null) return true;
  return !scrollable.contains(target);
}

/**
 * Attach non-passive wheel + touchmove guards to a modal backdrop that
 * preventDefault() every event landing outside `scrollable` (the modal's
 * scrollable panel, when it has one; pass `null` to block the whole
 * overlay — right for dialogs with no scrollable region). Returns the
 * detach callback.
 */
export function attachBackdropScrollGuard(
  backdrop: HTMLElement,
  scrollable: HTMLElement | null | undefined,
): () => void {
  const onWheel = (e: WheelEvent) => {
    if (blocksBackdropScroll(e.target, scrollable)) e.preventDefault();
  };
  const onTouchMove = (e: TouchEvent) => {
    if (blocksBackdropScroll(e.target, scrollable)) e.preventDefault();
  };
  backdrop.addEventListener("wheel", onWheel, { passive: false });
  backdrop.addEventListener("touchmove", onTouchMove, { passive: false });
  return () => {
    backdrop.removeEventListener("wheel", onWheel);
    backdrop.removeEventListener("touchmove", onTouchMove);
  };
}
