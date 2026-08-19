<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import {
    slotPitchMm as slotPitchMmFor,
    restOffsetMm as restOffsetMmFor,
  } from "../../geometry";

  let { config }: { config: ConfigStore } = $props();

  // Vernier slot-pitch / rest-offset formulas live in lib/geometry.
  let slotPitchMm = $derived(
    slotPitchMmFor(config.pole_pitch_mm, config.phases, config.spacing_ratio),
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
      <dt class="truncate text-[10px] text-slate-500" title="Active area length">Active area length</dt>
      <dd class="font-mono text-xs text-sky-200">{config.active_area_length_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Active area width">Active area width</dt>
      <dd class="font-mono text-xs text-sky-200">{config.active_area_width_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Coil span">Coil span</dt>
      <dd class="font-mono text-xs text-sky-200">{config.coil_span_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Pole pitch">Pole pitch</dt>
      <dd class="font-mono text-xs text-sky-200">{config.pole_pitch_mm.toFixed(2)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Slot pitch">Slot pitch</dt>
      <dd class="font-mono text-xs text-sky-200">{slotPitchMm.toFixed(2)} mm</dd>
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
      <dt class="truncate text-[10px] text-slate-500" title="Magnet travel-axis x cross-width">Magnet size</dt>
      <dd class="font-mono text-xs text-sky-200">{config.magnet_width_mm.toFixed(1)} &times; {config.magnet_cross_width_mm.toFixed(1)} mm</dd>
    </div>
    <div class="min-w-0">
      <dt class="truncate text-[10px] text-slate-500" title="Gap between adjacent magnets">Magnet gap</dt>
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
