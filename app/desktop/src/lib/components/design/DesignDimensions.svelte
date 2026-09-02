<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import {
    phaseBandPitchMm as phaseBandPitchMmFor,
    restOffsetMm as restOffsetMmFor,
  } from "../../geometry";

  let { config, measuredTraceLengthMm = null }: {
    config: ConfigStore;
    /** Trace X-span MEASURED from the returned coil payload (mm); falls back
     *  to the configured routing domain until a payload arrives. */
    measuredTraceLengthMm?: number | null;
  } = $props();

  // Vernier phase-band pitch / rest-offset formulas live in lib/geometry.
  let phaseBandPitchMm = $derived(
    phaseBandPitchMmFor(config.pole_pitch_mm, config.phases, config.spacing_ratio),
  );
  let restOffsetMm = $derived(
    restOffsetMmFor(config.pole_pitch_mm, config.phases, config.spacing_ratio),
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
      <dt class="truncate text-[10px] text-slate-500" title="Vernier-adjusted phase-band pitch ((pole pitch / phases) × spacing ratio). Not the glossary slot pitch τs = L_stator/N_slots, which coincides only for uniform 1-slot-per-pole-per-phase windings.">Phase-band pitch (Vernier)</dt>
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
</section>
