<script lang="ts">
  import type { ConfigStore } from "../../../stores/config.svelte";
  import { BACK_IRON_ARRANGEMENTS, DEFAULT_BACK_IRON_THICKNESS_MM } from "../../../stores/config.svelte";
  import MagnetGradeHelper from "../MagnetGradeHelper.svelte";
  import NumberField from "../../ui/NumberField.svelte";

  let { config }: { config: ConfigStore } = $props();

  let showBackIron = $derived(BACK_IRON_ARRANGEMENTS.has(config.magnet_arrangement));

  /** Auto-default back iron when a BackIron arrangement is first enabled. */
  $effect(() => {
    if (showBackIron && config.back_iron_thickness_mm === 0) {
      config.back_iron_thickness_mm = DEFAULT_BACK_IRON_THICKNESS_MM;
    }
  });
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

      <label class="min-w-0" title="Alternating or Halbach array; append BackIron to add a steel keeper.">
        <span class="mb-1 block text-xs text-slate-300">Arrangement</span>
        <select
          bind:value={config.magnet_arrangement}
          class="w-full rounded-md border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-100 focus:border-emerald-500 focus:outline-none"
        >
          <option value="Alternating">Alternating</option>
          <option value="AlternatingBackIron">Alternating + back iron</option>
          <option value="Halbach">Halbach</option>
          <option value="HalbachBackIron">Halbach + back iron</option>
        </select>
      </label>

      <label class="min-w-0" title="X Length (mm): magnet span along the travel axis; sets pole pitch with the gap.">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">X Length (mm)</span>
          <NumberField
            id="magnet-length"
            value={config.magnet_width_mm}
            min={0.1}
            max={40}
            step={0.1}
            ariaLabel="X Length (mm), magnet span along the travel axis"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.magnet_width_mm = value)}
          />
        </span>
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

      <label class="min-w-0" title="Z Thickness (mm): magnetisation-axis thickness; affects field strength at the PCB.">
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
      </label>

      <label class="min-w-0" title="Gap between adjacent magnets along the travel axis.">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Gap (mm)</span>
          <NumberField
            id="magnet-gap"
            value={config.magnet_gap_mm}
            min={0}
            max={20}
            step={0.1}
            ariaLabel="Magnet gap (mm)"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.magnet_gap_mm = value)}
          />
        </span>
      </label>

      <label class="min-w-0" title="Steel keeper thickness; set to zero for none.">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Back iron (mm)</span>
          <NumberField
            id="back-iron-thickness"
            value={config.back_iron_thickness_mm}
            min={0}
            max={20}
            step={0.1}
            ariaLabel="Back iron thickness (mm)"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.back_iron_thickness_mm = value)}
          />
        </span>
        {#if !showBackIron}
          <span class="mt-0.5 block text-[10px] text-slate-500">Stored until a back-iron arrangement is selected.</span>
        {/if}
      </label>

      <div class="sm:col-span-2">
        <MagnetGradeHelper {config} />
      </div>
    </div>
  </div>
</details>