<script lang="ts">
  import type { ConfigStore } from "../../../stores/config.svelte";
  import MagnetGradeHelper from "../MagnetGradeHelper.svelte";
  import NumberField from "../../ui/NumberField.svelte";

  let { config }: { config: ConfigStore } = $props();
</script>

<details open class="overflow-hidden rounded-md border border-slate-700 bg-slate-800/30">
  <summary class="cursor-pointer px-3 py-2 marker:text-slate-500 hover:text-emerald-300">
    <span class="flex items-center justify-between gap-2">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-200">Magnets</h3>
    </span>
  </summary>

  <div class="border-t border-slate-700 px-3 pb-3 pt-2.5">
    <div class="grid gap-x-3 gap-y-2.5 sm:grid-cols-2">
      <label class="min-w-0" title="Even number of magnets in the mover array.">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Magnet count</span>
          <NumberField
            id="magnet-count"
            value={config.magnet_count}
            min={2}
            max={64}
            step={2}
            integer
            ariaLabel="Magnet count"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.magnet_count = value)}
          />
        </span>
      </label>

      <label class="min-w-0" title="X Length of one magnet along the travel axis (W_m). The pole fill factor k = W_m / τ_p is derived from it; 0.75 (135° electrical) is the default optimum and 1.00 gives end-to-end magnets with no inter-pole gap.">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">X Length (mm)</span>
          <NumberField
            id="magnet-width"
            value={config.magnet_width_mm}
            min={0.5 * config.pole_pitch_mm}
            max={1.0 * config.pole_pitch_mm}
            step={0.1}
            ariaLabel="X Length (mm), magnet length along the travel axis"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.magnet_width_mm = value)}
          />
        </span>
        <span class="mt-1 block text-[10px] text-slate-500">
          k = W_m/τ_p = {(config.magnet_width_mm / config.pole_pitch_mm).toFixed(2)} ·
          width is the input, k is derived · k = 1.00 → magnets end-to-end.
        </span>
        {#if config.magnet_width_mm / config.pole_pitch_mm > 0.85}
          <span
            class="mt-1 block text-[10px] text-amber-400"
            role="status"
            aria-live="polite"
          >
            Flux leakage between adjacent magnets.
          </span>
        {/if}
      </label>

      <label class="min-w-0" title="Y Width (mm): magnet width across the stator; defines active conductor length.">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Y Width (mm)</span>
          <NumberField
            id="magnet-width"
            value={config.magnet_cross_width_mm}
            min={0.1}
            max={40}
            step={0.1}
            ariaLabel="Y Width (mm), magnet width across the stator"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.magnet_cross_width_mm = value)}
          />
        </span>
      </label>

      <label class="min-w-0" title="Z Thickness (mm): magnetisation-axis thickness. Coreless motors have no iron core, so the recommended minimum is T_m = 0.5 × pole pitch (3.0 mm at the default pitch).">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Z Thickness (mm)</span>
          <NumberField
            id="magnet-height"
            value={config.magnet_height_mm}
            min={0.1}
            max={20}
            step={0.1}
            ariaLabel="Z Thickness (mm), magnetisation-axis thickness"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.magnet_height_mm = value)}
          />
        </span>
        <span class="mt-1 block text-[10px] text-slate-500">
          Recommended: {(config.pole_pitch_mm * 0.5).toFixed(1)} mm (0.5 × pole pitch).
        </span>
      </label>

      <div class="sm:col-span-2">
        <div class="rounded-md border border-slate-700/80 bg-slate-900/40 px-2.5 py-2 text-[10px] leading-relaxed text-slate-400">
          <span class="font-semibold text-slate-300">Auto geometry</span>
          — pole pitch {config.pole_pitch_mm.toFixed(1)} mm (electrical pitch {config.electrical_pitch_mm.toFixed(
            1,
          )} mm) · X Length {config.magnet_width_mm.toFixed(1)} mm · gap {config.magnet_gap_mm.toFixed(
            1,
          )} mm · mover span {config.mover_span_mm.toFixed(1)} mm.
          The X Length IS the input; the fill factor is derived from it
          (k = W_m / τ_p), and the inter-magnet gap follows as
          W_gap = τ_p − W_m (zero at k = 1.00).
        </div>
      </div>

      <div class="sm:col-span-2">
        <MagnetGradeHelper {config} />
      </div>
    </div>
  </div>
</details>