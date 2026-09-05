<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import { Select } from "bits-ui";
  import { formatLayerRange } from "../../layerConstraints";
  import Separator from "../ui/Separator.svelte";
  import RoutingParamsPanel from "./RoutingParamsPanel.svelte";
  import GeneratorUploadPanel from "../plugins/GeneratorUploadPanel.svelte";

  let { config }: { config: ConfigStore } = $props();

  /** Sentinel value for the "load a new generator" dropdown entry. */
  const LOAD_VALUE = "__load_generator__";

  let modalOpen = $state(false);

  /** Pattern-declared layer range, shown beside the layer selector. */
  const layerRangeLabel = $derived(formatLayerRange(config.patternLayerRange));

  // Pull the selected pattern's declared user-editable params whenever the
  // pattern changes (also on initial mount). The store reseeds defaults for
  // keys the user hasn't set yet and re-constrains the layer count against
  // the pattern's declared range.
  $effect(() => {
    const id = config.routing_pattern;
    void config.loadRoutingParams(id);
  });

  // The Select's value is FUNCTION-BOUND to the store: the getter maps the
  // loading state to "" (so the placeholder shows) and the setter rejects
  // the load-generator sentinel — the sentinel is an ACTION, not a value,
  // so the store is never written with it and the binding re-reads the
  // getter, restoring the active pattern in the trigger.
  function patternBindingGet(): string {
    return config.routing_patterns.length === 0 ? "" : config.routing_pattern;
  }

  function patternBindingSet(v: string): void {
    if (v === LOAD_VALUE) {
      modalOpen = true;
      return;
    }
    config.routing_pattern = v;
  }

  // `items` powers label lookup in Select.Value while the portal content is
  // unmounted (so the trigger shows display names, never raw ids) and
  // native-style typeahead on the closed trigger.
  const patternItems = $derived(
    config.routing_patterns.length === 0
      ? [{ value: LOAD_VALUE, label: "+ Load new generator…" }]
      : [
          ...config.routing_patterns.map((p) => ({
            value: p.id,
            label: p.display_name,
          })),
          { value: LOAD_VALUE, label: "+ Load new generator…" },
        ],
  );

  const layerValue = $derived(String(config.num_layers));
  const layerItems = $derived(
    config.layerOptions.map((n) => ({ value: String(n), label: String(n) })),
  );

  function onLayerValueChange(v: string): void {
    const value = Number(v);
    // The selector only offers valid counts (even, >= 2, within the board
    // stackup and the pattern's range); guard anyway so nothing but a whole
    // number reaches the store.
    if (Number.isInteger(value) && value >= 2) {
      config.num_layers = value;
    }
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

    <Select.Root
      type="single"
      bind:value={patternBindingGet, patternBindingSet}
      disabled={config.routing_patterns.length === 0}
      items={patternItems}
    >
      <Select.Trigger
        id="routing-pattern"
        aria-label="Routing pattern"
        class="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-100 focus:border-emerald-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60 flex items-center justify-between gap-1 text-left"
      >
        <Select.Value placeholder="Loading patterns…" />
        <svg
          viewBox="0 0 12 12"
          class="h-3 w-3 shrink-0 text-slate-500"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          aria-hidden="true"
        >
          <path d="M2.5 4.5 6 8l3.5-3.5" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </Select.Trigger>
      <Select.Portal>
        <Select.Content
          class="z-50 max-h-72 min-w-[var(--bits-select-anchor-width)] overflow-y-auto rounded-md border border-slate-700 bg-slate-800 py-1 shadow-lg shadow-black/40 focus:outline-none"
        >
          {#each config.routing_patterns as pattern (pattern.id)}
            <Select.Item
              value={pattern.id}
              label={pattern.display_name}
              class="flex cursor-pointer items-center justify-between gap-2 px-2.5 py-1.5 text-xs text-slate-100 outline-none data-[selected]:bg-slate-700 data-[highlighted]:bg-slate-700/60 data-[highlighted]:text-emerald-200 data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50"
            >
              {pattern.display_name}
            </Select.Item>
          {/each}
          {#if config.routing_patterns.length > 0}
            <!-- Design-system divider (kata tn66): decorative, purely
                 visual inside the listbox — neither focusable nor
                 selectable. -->
            <Separator class="my-1" />
          {/if}
          <Select.Item
            value={LOAD_VALUE}
            label="+ Load new generator…"
            class="flex cursor-pointer items-center justify-between gap-2 px-2.5 py-1.5 text-xs text-emerald-300 outline-none data-[selected]:bg-slate-700 data-[highlighted]:bg-slate-700/60 data-[highlighted]:text-emerald-200 data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50"
          >
            + Load new generator…
          </Select.Item>
        </Select.Content>
      </Select.Portal>
    </Select.Root>
  </div>

  <!-- Copper-layer count: options are the even ladder (>= 2, <= max_layers)
       intersected with the active pattern's declared range; the caption shows
       that range and updates on pattern switch. The selector can only OFFER
       valid values — the Rust config validation and the routing crate's
       generate-time layer check remain the authorities. -->
  <div class="flex flex-wrap items-center gap-2">
    <label
      for="num-layers"
      class="text-[11px] font-semibold uppercase tracking-wider text-slate-300"
    >
      Copper layers
    </label>
    <Select.Root
      type="single"
      value={layerValue}
      onValueChange={onLayerValueChange}
      disabled={config.layerOptions.length <= 1}
      items={layerItems}
    >
      <Select.Trigger
        id="num-layers"
        aria-label="Copper layer count"
        class="rounded-md border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-100 focus:border-emerald-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60 flex items-center gap-1 text-left"
      >
        <Select.Value />
        <svg
          viewBox="0 0 12 12"
          class="h-3 w-3 shrink-0 text-slate-500"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          aria-hidden="true"
        >
          <path d="M2.5 4.5 6 8l3.5-3.5" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </Select.Trigger>
      <Select.Portal>
        <Select.Content
          class="z-50 max-h-72 min-w-[var(--bits-select-anchor-width)] overflow-y-auto rounded-md border border-slate-700 bg-slate-800 py-1 shadow-lg shadow-black/40 focus:outline-none"
        >
          {#each config.layerOptions as n (n)}
            <Select.Item
              value={String(n)}
              label={String(n)}
              class="flex cursor-pointer items-center justify-between gap-2 px-2.5 py-1.5 text-xs text-slate-100 outline-none data-[selected]:bg-slate-700 data-[highlighted]:bg-slate-700/60 data-[highlighted]:text-emerald-200 data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50"
            >
              {n}
            </Select.Item>
          {/each}
        </Select.Content>
      </Select.Portal>
    </Select.Root>
    <span class="min-w-0 flex-1 text-[10px] text-slate-500" role="note">
      {layerRangeLabel}
    </span>
  </div>

  <RoutingParamsPanel {config} />
</div>

{#if modalOpen}
  <GeneratorUploadPanel {config} onClose={() => (modalOpen = false)} />
{/if}
