<script lang="ts">
  /**
   * TravelDiagram.svelte — two views of the stator / mover assembly.
   *
   *   1. 3/4 isometric view (TravelIsoView): axonometric projection of
   *      the assembly (PCB wireframe + magnet wireframe + optional
   *      back-iron). Z is exaggerated so the thin stackup is visible.
   *      The magnet block sits at the current `moverPosMm` along the
   *      travel axis.
   *
   *   2. Front-on orthographic view (TravelStackupView): Y–Z
   *      cross-section; each layer is a full-width rectangle. The N·S
   *      pole alternation happens along the TRAVEL direction (X), which
   *      is perpendicular to this view, so the magnet block is uniform
   *      and a "N · S" label makes the alternation axis explicit.
   *
   *   `moverPosRaw` is shared state for the mover position (mm along
   *   travel). It is initialized to 0 and can be adjusted by the
   *   visualization control below. The 3/4 view's magnet placement reads
   *   from it via the derived `moverPosMm` (clamped to `config.travel_mm`).
   */
  import type { ConfigStore } from "../../stores/config.svelte";
  import NumberField from "../ui/NumberField.svelte";
  import TravelIsoView from "./TravelIsoView.svelte";
  import TravelStackupView from "./TravelStackupView.svelte";
  import {
    coilSpanMm as coilSpanMmFor,
    clampMoverCenter,
  } from "../../geometry";

  let { config }: { config: ConfigStore } = $props();

  /** Vertical exaggeration factor of the iso view (mirrored in the hint). */
  const Z_EXAG = 10;

  // ====================================================================
  // Shared mover position (mm along travel, CENTER of magnet array)
  // ====================================================================
  // `moverPosRaw` is shared state consumed by the 3/4 view's `isoGeom`
  // (via the clamped `moverPosMm` derived below) AND by the number field
  // below the views. The committed value represents the CENTER of the
  // magnet array — the magnet extends from
  // `(moverPosMm - coilSpan/2)` to `(moverPosMm + coilSpan/2)`. The
  // clamp keeps the magnet fully inside the active area:
  // `[coilSpan/2, active_area_length - coilSpan/2]`.
  let moverPosRaw = $state(0);
  let moverMaxMm = $derived(Math.max(config.active_area_length_mm, 0));
  let coilSpanMm = $derived(coilSpanMmFor(config.magnet_count, config.pole_pitch_mm));
  let moverPosMm = $derived(
    clampMoverCenter(moverPosRaw, coilSpanMm, config.active_area_length_mm),
  );
  // Magnet extent in mm (used by the 3/4 view AND the display row).
  let magnetStartMm = $derived(moverPosMm - coilSpanMm / 2);
  let magnetEndMm = $derived(moverPosMm + coilSpanMm / 2);

  let invalid = $derived(config.travel_mm <= 0);

  function updateMoverPosition(value: number): void {
    if (Number.isFinite(value)) moverPosRaw = value;
  }
</script>

<div class="rounded-lg bg-slate-800/40 border border-slate-700 p-4">
  <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
    <div>
      <h2 class="text-sm font-semibold text-slate-200">Design reflection</h2>
      <p class="text-[11px] text-slate-500">Live stackup and travel geometry</p>
    </div>
    <span class="text-xs text-slate-400" role="status" aria-live="polite">
      L_active = {config.active_area_length_mm.toFixed(1)} mm ·
      coil_span = {config.coil_span_mm.toFixed(1)} mm ·
      <span class={invalid ? 'text-rose-400' : 'text-sky-300'}>L_travel = {config.travel_mm.toFixed(1)} mm</span>
    </span>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-[1fr_180px] gap-3 items-start">
    <!-- ===== 1. 3/4 isometric view ===== -->
    <TravelIsoView {config} {moverPosMm} {magnetStartMm} {magnetEndMm} />

    <!-- ===== 2. Front-on orthographic stackup (Y–Z) ===== -->
    <TravelStackupView {config} />
  </div>

  <!-- ===== 3. Mover position number field + extent display ===== -->
  <!-- The field value is the CENTER of the magnet array (mm). The 3/4
       view's magnet tracks the same value (clamped to keep it on the
       active area). The display row below confirms the actual magnet
       extent in mm. -->
  <div class="mt-3" aria-label="Mover position">
    <div class="flex items-center gap-2">
      <label for="mover-position-value" class="text-xs text-slate-400 whitespace-nowrap">
        Position:
      </label>
      <NumberField
        id="mover-position-value"
        min={0}
        max={moverMaxMm}
        step={0.1}
        value={moverPosRaw}
        ariaLabel="Mover position (mm)"
        class="w-24 rounded px-1.5 py-0.5 text-right font-mono text-emerald-300"
        onCommit={updateMoverPosition}
      />
      <span class="text-xs text-slate-400">mm</span>
    </div>
    <div class="mt-1 text-[10px] text-slate-500" aria-live="polite">
      Position: {moverPosMm.toFixed(1)} mm · Magnet: {magnetStartMm.toFixed(1)} mm - {magnetEndMm.toFixed(1)} mm
    </div>
  </div>

  {#if invalid}
    <p class="mt-3 text-xs text-rose-400">
      Travel is zero or negative — increase Desired Travel (center-to-center) or reduce Magnet Count / Width / Gap.
    </p>
  {:else}
    <p class="mt-3 text-xs text-slate-500">
      3/4 view shows the assembly in axonometric projection (Z exaggerated ×{Z_EXAG}). Front
      view (Y–Z) reflects the real stack height from Design settings; the N·S marker indicates
      the alternation axis (along travel, hidden in this view).
    </p>
  {/if}
</div>