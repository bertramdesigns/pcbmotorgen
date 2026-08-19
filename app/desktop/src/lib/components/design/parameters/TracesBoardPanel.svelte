<script lang="ts">
  import type { ConfigStore } from "../../../stores/config.svelte";
  import NumberField from "../../ui/NumberField.svelte";

  let { config }: { config: ConfigStore } = $props();
</script>

<!-- Plain sub-block of the combined "Topology & Board" section. No mode
     gating: every field is a normal editable input. -->
<div class="border-t border-slate-700/80 pt-2.5">
  <h4 class="mb-2 text-[11px] font-semibold uppercase tracking-wider text-slate-300">
    Traces &amp; Board
  </h4>

  <div class="grid gap-x-3 gap-y-2.5 sm:grid-cols-2">
    <label class="min-w-0" title="Ratio of slot pitch to pole pitch.">
      <span class="mb-1 block text-xs text-slate-300">Spacing ratio</span>
      <select
        bind:value={config.spacing_ratio_label}
        class="w-full rounded-md border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-100 focus:border-emerald-500 focus:outline-none"
      >
        <option value="1:1">1:1 standard</option>
        <option value="4:5">4:5 vernier</option>
        <option value="5:6">5:6 vernier</option>
      </select>
    </label>

    <label class="min-w-0" title="Even number of copper layers used for the winding layout.">
      <span class="flex items-center justify-between gap-2">
        <span class="min-w-0 truncate text-xs text-slate-300">Number of layers</span>
        <NumberField
          id="number-of-layers"
          value={config.num_layers}
          min={2}
          max={12}
          step={2}
          integer
          ariaLabel="Number of layers"
          class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
          onCommit={(value) => (config.num_layers = value)}
        />
      </span>
    </label>

    <label class="min-w-0" title="Extra board length reserved for end-turn routing.">
      <span class="flex items-center justify-between gap-2">
        <span class="min-w-0 truncate text-xs text-slate-300">Routing padding (mm)</span>
        <NumberField
          id="routing-padding"
          value={config.padding_mm}
          min={0}
          max={400}
          step={0.5}
          ariaLabel="Routing padding (mm)"
          class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
          onCommit={(value) => (config.padding_mm = value)}
        />
      </span>
    </label>

    <label class="min-w-0" title="Parallel winding paths per phase.">
      <span class="flex items-center justify-between gap-2">
        <span class="min-w-0 truncate text-xs text-slate-300">Windings per phase</span>
        <NumberField
          id="windings-per-phase"
          value={config.windings_per_phase}
          min={1}
          max={16}
          step={1}
          integer
          ariaLabel="Windings per phase"
          class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
          onCommit={(value) => (config.windings_per_phase = value)}
        />
      </span>
    </label>
  </div>
</div>