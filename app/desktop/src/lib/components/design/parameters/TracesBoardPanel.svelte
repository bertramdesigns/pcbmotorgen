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
    <label class="min-w-0" title="Parallel strands per phase — number of parallel serpentine paths per phase, stacked across the board width.">
      <span class="flex items-center justify-between gap-2">
        <span class="min-w-0 truncate text-xs text-slate-300">Strands per phase</span>
        <NumberField
          id="strands-per-phase"
          value={config.strands_per_phase}
          min={1}
          max={16}
          step={1}
          integer
          ariaLabel="Strands per phase"
          class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
          onCommit={(value) => (config.strands_per_phase = value)}
        />
      </span>
    </label>
  </div>
</div>