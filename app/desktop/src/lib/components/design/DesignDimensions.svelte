<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import type { RoutingDimensionsDto } from "../../types";
  import {
    phaseBandPitchMm as phaseBandPitchMmFor,
    restOffsetMm as restOffsetMmFor,
  } from "../../geometry";
  import {
    formatMarginMm,
    formatMetresMm,
    isValidMetres,
  } from "../../previewGeometry";

  let {
    config,
    measuredTraceLengthMm = null,
    routingDimensions = null,
  }: {
    config: ConfigStore;
    /** Trace X-span MEASURED from the returned coil payload (mm); falls back
     *  to the configured routing domain until a payload arrives. */
    measuredTraceLengthMm?: number | null;
    /** `routing_dimensions` sidecar of the last coil payload (metres at the
     *  IPC boundary). Null until a payload arrives; the per-slot metrics
     *  inside are additionally pattern-dependent and stay null when the
     *  pattern declares no leg grid. */
    routingDimensions?: RoutingDimensionsDto | null;
  } = $props();

  // Vernier phase-band pitch / rest-offset formulas live in lib/geometry.
  let phaseBandPitchMm = $derived(
    phaseBandPitchMmFor(config.pole_pitch_mm, config.phases, config.spacing_ratio),
  );
  let restOffsetMm = $derived(
    restOffsetMmFor(config.pole_pitch_mm, config.phases, config.spacing_ratio),
  );

  // --- Routing-payload metrics (metres at the IPC boundary) ---------------
  // Per-slot metrics are pattern-declared: an em-dash means the pattern
  // declares no leg grid (or no payload has arrived yet).

  /** Pass-through for finite positive metre quantities; everything else is null. */
  function validMetres(v: number | null | undefined): number | null {
    return isValidMetres(v) ? v : null;
  }

  let slotCount = $derived(
    typeof routingDimensions?.slot_count === "number" && routingDimensions.slot_count > 0
      ? routingDimensions.slot_count
      : null,
  );
  let slotPitchM = $derived(validMetres(routingDimensions?.slot_pitch_m));
  let interleaveStepM = $derived(validMetres(routingDimensions?.interleave_step_m));

  /**
   * Summarise one metric across all `phase_band_widths` records: a single
   * value when every band agrees (the common case), otherwise the
   * `[min, max]` span. Null when no band reports a finite value.
   */
  function bandValueSpan(values: (number | null | undefined)[]): [number, number] | null {
    let min = Infinity;
    let max = -Infinity;
    for (const v of values) {
      if (typeof v !== "number" || !Number.isFinite(v)) continue;
      min = Math.min(min, v);
      max = Math.max(max, v);
    }
    if (!Number.isFinite(min)) return null;
    return max - min <= 1e-12 ? [min, min] : [min, max];
  }

  let slotWidthM = $derived(
    bandValueSpan((routingDimensions?.phase_band_widths ?? []).map((b) => b.slot_width_m)),
  );
  let bandWidthM = $derived(
    bandValueSpan((routingDimensions?.phase_band_widths ?? []).map((b) => b.band_width_m)),
  );
  let bandMarginM = $derived(
    bandValueSpan((routingDimensions?.phase_band_widths ?? []).map((b) => b.margin_m)),
  );

  /** Single value → one length; a genuine span → "min – max". */
  function formatSpanMm(span: [number, number] | null): string {
    if (!span) return "—";
    return span[0] === span[1]
      ? formatMetresMm(span[0])
      : `${formatMetresMm(span[0])} – ${formatMetresMm(span[1])}`;
  }

  /** Signed margins keep their sign on both ends of a span. */
  function formatMarginSpanMm(span: [number, number] | null): string {
    if (!span) return "—";
    return span[0] === span[1]
      ? formatMarginMm(span[0])
      : `${formatMarginMm(span[0])} – ${formatMarginMm(span[1])}`;
  }

  let slotGroupCaption = $derived(
    routingDimensions === null
      ? "waiting for payload"
      : slotCount === null
        ? "no leg grid declared"
        : "one active leg per slot",
  );
</script>

<section
  class="rounded-md border border-slate-700 bg-slate-800/40 px-2.5 py-2"
  aria-labelledby="design-dimensions-heading"
>
  <div class="mb-1.5 flex items-center justify-between gap-2">
    <h2 id="design-dimensions-heading" class="text-[11px] font-semibold uppercase tracking-wider text-slate-300">
      Design dimensions
    </h2>
    <span class="text-[10px] text-slate-500">live outputs</span>
  </div>

  <dl class="grid grid-cols-2 gap-x-3 gap-y-1.5 sm:grid-cols-3">
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Total X extent of the routed PCB traces (first to last segment point), measured from the returned payload. The braid floors whole periods, so this can sit up to one period below the active-area length (the routing domain equals the active area).">PCB trace total (X)</dt>
      <dd class="font-mono text-xs text-emerald-300">
        {(measuredTraceLengthMm ?? config.trace_total_length_mm).toFixed(1)} mm
        {#if measuredTraceLengthMm !== null}
          <span class="text-[9px] text-slate-500">meas.</span>
        {/if}
      </dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Copper active region (mover span + travel)">Active copper region</dt>
      <dd class="font-mono text-xs text-sky-200">{config.active_area_length_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Active area width">Active area width</dt>
      <dd class="font-mono text-xs text-sky-200">{config.active_area_width_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Mover span (magnet array span)">Mover span</dt>
      <dd class="font-mono text-xs text-sky-200">{config.mover_span_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Pole pitch">Pole pitch</dt>
      <dd class="font-mono text-xs text-sky-200">{config.pole_pitch_mm.toFixed(2)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Vernier-adjusted phase-band pitch ((pole pitch / phases) × spacing ratio). Not the glossary slot pitch τs = L_stator/N_slots, which coincides only for uniform 1-slot-per-pole-per-phase windings — see the Slot (per-leg) group below.">Phase-band pitch (Vernier)</dt>
      <dd class="font-mono text-xs text-sky-200">{phaseBandPitchMm.toFixed(2)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Vernier rest offset">Rest offset</dt>
      <dd class="font-mono text-xs text-sky-200">{restOffsetMm.toFixed(2)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Magnet count">Magnet count</dt>
      <dd class="font-mono text-xs text-sky-200">{config.magnet_count}</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="X Length × Y Width">Magnet size</dt>
      <dd class="font-mono text-xs text-sky-200">{config.magnet_width_mm.toFixed(1)} &times; {config.magnet_cross_width_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Automatic inter-magnet gap = pole pitch − magnet X Length">Magnet gap (auto)</dt>
      <dd class="font-mono text-xs text-sky-200">{config.magnet_gap_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Routing pattern">Routing</dt>
      <dd class="truncate font-mono text-xs text-sky-200" title={config.routing_pattern}>{config.routing_pattern}</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Copper layers">Layers</dt>
      <dd class="font-mono text-xs text-sky-200">{config.num_layers}</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Phases">Phases</dt>
      <dd class="font-mono text-xs text-sky-200">{config.phases}</dd>
    </div>
  </dl>

  <!-- Routing-payload metrics. The glossary disambiguation matters here: a
       SLOT houses ONE active leg; a PHASE BAND is the whole coil bundle, so
       the two width columns must never be conflated. Payload-derived values
       are shown like the measured trace (emerald); an em-dash means the
       metric is unavailable. -->
  <div class="mt-2 space-y-2 border-t border-slate-700/60 pt-2">
    <div>
      <div class="mb-1 flex items-baseline justify-between gap-2">
        <h3 class="text-[10px] font-semibold uppercase tracking-wider text-slate-400">Slot (per-leg)</h3>
        <span class="text-[9px] text-slate-500">{slotGroupCaption}</span>
      </div>
      <dl class="grid grid-cols-2 gap-x-3 gap-y-1.5 sm:grid-cols-3">
        <div class="min-w-0">
          <dt class="truncate text-[10px] text-slate-500" title="Total active leg slots declared by the pattern's leg grid. Pattern-dependent: an em-dash means the pattern declares no leg grid.">Slot count</dt>
          <dd class="font-mono text-xs text-emerald-300">{slotCount ?? "—"}</dd>
        </div>
        <div class="min-w-0">
          <dt class="truncate text-[10px] text-slate-500" title="True slot pitch τs = L_stator / N_slots along the stator track. Not the Vernier phase-band pitch above — the two coincide only for uniform 1-slot-per-pole-per-phase windings.">Slot pitch τs</dt>
          <dd class="font-mono text-xs text-emerald-300">{slotPitchM === null ? "—" : formatMetresMm(slotPitchM)}</dd>
        </div>
        <div class="min-w-0">
          <dt class="truncate text-[10px] text-slate-500" title="Effective leg pitch of braided/slotless patterns: pole pitch / (phases × strands). Braided patterns have no physical slots — this is the equivalent leg-pitch model of the interleaved trace layout.">Interleave step</dt>
          <dd class="font-mono text-xs text-emerald-300">{interleaveStepM === null ? "—" : formatMetresMm(interleaveStepM)}</dd>
        </div>
        <div class="min-w-0">
          <dt class="truncate text-[10px] text-slate-500" title="Glossary Slot Width: along-travel width of the track space housing ONE active leg, w_t / sin(theta). NOT the phase-band bundle width below. Summarised across all (layer, net) bands; a range means the bands differ.">Slot width (one leg)</dt>
          <dd class="font-mono text-xs text-emerald-300">{formatSpanMm(slotWidthM)}</dd>
        </div>
      </dl>
    </div>
    <div>
      <div class="mb-1 flex items-baseline justify-between gap-2">
        <h3 class="text-[10px] font-semibold uppercase tracking-wider text-slate-400">Phase band (coil bundle)</h3>
        <span class="text-[9px] text-slate-500">whole bundle, never one slot</span>
      </div>
      <dl class="grid grid-cols-2 gap-x-3 gap-y-1.5 sm:grid-cols-3">
        <div class="min-w-0">
          <dt class="truncate text-[10px] text-slate-500" title="Full conductor-bundle width along the travel axis (band_width_m), summarised across all (layer, net) bands. A phase band houses the whole coil bundle — a slot houses one active leg.">Band width (bundle)</dt>
          <dd class="font-mono text-xs text-emerald-300">{formatSpanMm(bandWidthM)}</dd>
        </div>
        <div class="min-w-0">
          <dt class="truncate text-[10px] text-slate-500" title="max_band_width_m − band_width_m per band; negative means over budget. Null when the pole pitch is unknown.">Band margin</dt>
          <dd class="font-mono text-xs text-emerald-300">{formatMarginSpanMm(bandMarginM)}</dd>
        </div>
      </dl>
    </div>
  </div>
</section>
