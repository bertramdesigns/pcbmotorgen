<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import RoutingParamsPanel from "./RoutingParamsPanel.svelte";
  import GeneratorUploadPanel from "../plugins/GeneratorUploadPanel.svelte";

  let { config }: { config: ConfigStore } = $props();

  /** Sentinel value for the "load a new generator" dropdown entry. */
  const LOAD_VALUE = "__load_generator__";
  /** Sentinel value for the disabled divider row in the dropdown. */
  const DIVIDER_VALUE = "__divider__";

  let modalOpen = $state(false);

  // Pull the selected pattern's declared user-editable params whenever the
  // pattern changes (also on initial mount). The store reseeds defaults for
  // keys the user hasn't set yet.
  $effect(() => {
    const id = config.routing_pattern;
    void config.loadRoutingParams(id);
  });

  function onPatternChange(event: Event): void {
    const element = event.currentTarget as HTMLSelectElement;
    const value = element.value;
    if (value === LOAD_VALUE) {
      // Don't persist the sentinel — revert the visible selection to the
      // current real pattern and open the modal to load a new generator.
      element.value = config.routing_pattern;
      modalOpen = true;
      return;
    }
    config.routing_pattern = value;
  }
</script>

<div class="space-y-2.5">
  <!-- Title row: the routing-pattern dropdown hangs LEFT beside the heading
       instead of sitting in its own box above it. No nested section — routing
       params live directly in the Topology & Board box. -->
  <div class="flex flex-wrap items-center gap-2">
    <h3
      id="routing-parameters-heading"
      class="text-[11px] font-semibold uppercase tracking-wider text-slate-300"
    >
      Routing parameters
    </h3>

    <select
      id="routing-pattern"
      value={config.routing_pattern}
      onchange={onPatternChange}
      disabled={config.routing_patterns.length === 0}
      aria-label="Routing pattern"
      title="Routing pattern"
      class="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-100 focus:border-emerald-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
    >
      {#if config.routing_patterns.length === 0}
        <option value="__loading__" disabled>Loading patterns…</option>
      {:else}
        {#each config.routing_patterns as pattern (pattern.id)}
          <option value={pattern.id}>{pattern.display_name}</option>
        {/each}
      {/if}
      <option value={DIVIDER_VALUE} disabled>&mdash;&mdash;&mdash;&mdash;&mdash;&mdash;</option>
      <option value={LOAD_VALUE}>&#43; Load new generator&#8230;</option>
    </select>
  </div>

  <RoutingParamsPanel {config} />

  <p class="text-[11px] text-slate-500" role="note">
    Pattern parameters edit the trace layout; generator upload is available here.
  </p>
</div>

{#if modalOpen}
  <GeneratorUploadPanel {config} onClose={() => (modalOpen = false)} />
{/if}