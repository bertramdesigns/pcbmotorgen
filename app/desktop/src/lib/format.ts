/**
 * Presentation formatting helpers shared by panels: severity → Tailwind
 * classes, pluralization, and threshold classification.
 */

/** Severity level of a write pre-condition warning. */
export type WarningLevel = "info" | "warning" | "error";

/**
 * Tailwind border/bg/text triple for a pre-condition severity level.
 * Kept as a function (not a lookup) so callers can use it in {#each}.
 */
export function warningClasses(level: WarningLevel): string {
  switch (level) {
    case "error":
      return "border-rose-500/60 bg-rose-500/10 text-rose-200";
    case "warning":
      return "border-amber-500/60 bg-amber-500/10 text-amber-200";
    case "info":
      return "border-sky-500/40 bg-sky-500/10 text-sky-200";
  }
}

/** Uppercase severity label for badges ("ERROR", "WARNING", "INFO"). */
export function warningLabel(level: WarningLevel): string {
  return level.toUpperCase();
}

/** `pluralize(1, "line")` → "1 line"; `pluralize(3, "line")` → "3 lines". */
export function pluralize(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}

/** Status tone for force ripple: <5% ok, <10% warn, else bad; "na" when unknown. */
export function rippleStatus(ripplePct: number | null | undefined): "ok" | "warn" | "bad" | "na" {
  if (ripplePct === null || ripplePct === undefined || !Number.isFinite(ripplePct)) return "na";
  if (ripplePct < 5) return "ok";
  if (ripplePct < 10) return "warn";
  return "bad";
}