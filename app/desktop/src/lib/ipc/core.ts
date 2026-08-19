/**
 * IPC core: Tauri runtime detection, unit helpers, debounce.
 */

// ---------------------------------------------------------------------------
// Unit helpers (mm ↔ m)
// ---------------------------------------------------------------------------

export const mm = (v: number): number => v / 1000.0;
export const to_mm = (v: number): number => v * 1000.0;

// ---------------------------------------------------------------------------
// Tauri runtime detection
// ---------------------------------------------------------------------------

/**
 * `true` when the page is running inside the Tauri shell, `false` for
 * plain `vite dev` (or any browser without the Tauri IPC bridge).
 *
 * The Tauri v2 runtime injects `window.__TAURI_INTERNALS__`; its mere
 * presence is the canonical "we have a backend" signal. We use this
 * instead of a try/catch around `invoke()` so we can distinguish
 * "no backend available" (return mock) from "backend returned an error"
 * (let the error propagate to the UI).
 */
export function isTauriAvailable(): boolean {
  if (typeof window === "undefined") return false;
  return (window as unknown as { __TAURI_INTERNALS__?: unknown })
    .__TAURI_INTERNALS__ !== undefined;
}

// ---------------------------------------------------------------------------
// Debounce helper for throttling rapid consecutive invokes
// ---------------------------------------------------------------------------

type Debounced<T extends (...args: never[]) => void> = ((
  ...args: Parameters<T>
) => void) & { cancel: () => void };

export function debounce<T extends (...args: never[]) => void>(
  fn: T,
  delayMs: number,
): Debounced<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;

  const debounced = ((...args: Parameters<T>) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      fn(...args);
    }, delayMs);
  }) as Debounced<T>;

  debounced.cancel = () => {
    if (timer) clearTimeout(timer);
    timer = undefined;
  };

  return debounced;
}
