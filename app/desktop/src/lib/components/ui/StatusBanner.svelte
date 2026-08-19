<script lang="ts">
  import type { Snippet } from "svelte";

  type Level = "error" | "warning" | "info" | "success";

  let {
    level,
    role,
    ariaLive,
    children,
  }: {
    level: Level;
    role?: "alert" | "status";
    ariaLive?: "assertive" | "polite";
    children?: Snippet;
  } = $props();

  const LEVEL_TONES: Record<
    Level,
    { classes: string; role: "alert" | "status"; ariaLive: "assertive" | "polite" }
  > = {
    error: { classes: "border-rose-500/60 bg-rose-500/10 text-rose-200", role: "alert", ariaLive: "assertive" },
    warning: { classes: "border-amber-500/60 bg-amber-500/10 text-amber-200", role: "status", ariaLive: "polite" },
    info: { classes: "border-sky-500/40 bg-sky-500/10 text-sky-200", role: "status", ariaLive: "polite" },
    success: { classes: "border-emerald-500/60 bg-emerald-500/10 text-emerald-200", role: "status", ariaLive: "polite" },
  };

  let classes = $derived(LEVEL_TONES[level].classes);
  let resolvedRole = $derived(role ?? LEVEL_TONES[level].role);
  let resolvedAriaLive = $derived(ariaLive ?? LEVEL_TONES[level].ariaLive);
</script>

<div class={"rounded-md border px-4 py-2 text-sm " + classes} role={resolvedRole} aria-live={resolvedAriaLive}>
  {@render children?.()}
</div>