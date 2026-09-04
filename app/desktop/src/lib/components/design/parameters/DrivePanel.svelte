<script lang="ts">
  import type { ConfigStore } from "../../../stores/config.svelte";
  import NumberField from "../../ui/NumberField.svelte";

  let { config }: { config: ConfigStore } = $props();
</script>

<details open class="overflow-hidden rounded-md border border-slate-700 bg-slate-800/30">
  <summary class="cursor-pointer px-3 py-2 marker:text-slate-500 hover:text-emerald-300">
    <span class="flex items-center justify-between gap-2">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-200">Drive &amp; Force Targets</h3>
    </span>
  </summary>

  <div class="border-t border-slate-700 px-3 pb-3 pt-2.5">
    <div class="grid gap-x-3 gap-y-2 sm:grid-cols-2">
      <label class="min-w-0">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Phase current (A)</span>
          <NumberField
            id="phase-current"
            value={config.max_current_a}
            min={0.01}
            step={0.05}
            ariaLabel="Phase current (A)"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.max_current_a = value)}
          />
        </span>
      </label>

      <label class="min-w-0">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Continuous target (N)</span>
          <NumberField
            id="target-force"
            value={config.target_force_n}
            min={0.001}
            step={0.01}
            ariaLabel="Continuous target force (N)"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.target_force_n = value)}
          />
        </span>
      </label>

      <label class="min-w-0">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Peak target (N)</span>
          <NumberField
            id="peak-force"
            value={config.peak_force_n}
            min={0.001}
            step={0.01}
            ariaLabel="Peak target force (N)"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.peak_force_n = value)}
          />
        </span>
      </label>

      <label class="min-w-0">
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 truncate text-xs text-slate-300">Friction (N)</span>
          <NumberField
            id="friction-estimate"
            value={config.friction_n}
            min={0}
            step={0.005}
            ariaLabel="Friction estimate (N)"
            class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
            onCommit={(value) => (config.friction_n = value)}
          />
        </span>
      </label>
    </div>
  </div>
</details>
