<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import type { RoutingParamDef } from "../../types";
  import NumberField from "../ui/NumberField.svelte";

  let { config }: { config: ConfigStore } = $props();

  /** Current value for a def, falling back to its declared default. */
  function valueFor(def: RoutingParamDef): number {
    const value = config.routing_params[def.key];
    return typeof value === "number" && Number.isFinite(value) ? value : def.default;
  }

  function onParamInput(def: RoutingParamDef, value: number): void {
    if (!Number.isFinite(value)) return;
    config.setRoutingParam(def.key, value);
  }
</script>

<div class="space-y-2">
  <p class="text-[11px] text-slate-500" role="note">
    Set the parameters exposed by this generator; amplitude and line length are derived.
  </p>

  {#if config.routing_param_defs.length === 0}
    <p class="text-xs italic text-slate-500">No user-editable parameters for this pattern.</p>
  {:else}
    <div class="grid gap-x-3 gap-y-1.5 sm:grid-cols-2" role="list">
      {#each config.routing_param_defs as def (def.key)}
        <label class="min-w-0" title={def.description || def.key}>
          <span class="flex items-center justify-between gap-2">
            <span class="min-w-0 truncate text-xs text-slate-300">
              {def.label}
              <span class="font-mono text-[10px] text-slate-500">({def.key})</span>
            </span>
            <NumberField
              id={`routing-param-${def.key}`}
              value={valueFor(def)}
              step={def.step ?? (def.param_type === "int" ? 1 : 0.1)}
              integer={def.param_type === "int"}
              min={def.min}
              max={def.max}
              ariaLabel={def.label}
              class="w-24 shrink-0 bg-slate-900 px-2 py-1 text-xs font-mono text-emerald-200"
              onCommit={(value) => onParamInput(def, value)}
            />
          </span>
          {#if def.min !== undefined || def.max !== undefined}
            <span class="mt-0.5 block text-[10px] text-slate-500">
              range {def.min ?? "-"} to {def.max ?? "-"}
            </span>
          {/if}
        </label>
      {/each}
    </div>
  {/if}
</div>