<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import { CUSTOM_GRADE } from "../../types";
  import NumberField from "../ui/NumberField.svelte";

  let { config }: { config: ConfigStore } = $props();

  let selected = $derived(config.magnet_grade);

  // Grade info comes from the runtime-loaded backend table when available;
  // the static TS table in types/magnets.ts is the offline fallback.
  let gradeInfo = $derived.by(() => {
    if (selected === CUSTOM_GRADE) return null;
    return config.getMagnetGrade(selected);
  });

  let tempSuffixes = $derived(
    gradeInfo ? Object.entries(gradeInfo.max_temp_c) : [],
  );

  function onGradeChange(event: Event): void {
    const target = event.currentTarget as HTMLSelectElement;
    config.magnet_grade = target.value;
    config.syncGrade();
  }
</script>

<div class="space-y-1.5">
  <label class="flex items-center justify-between gap-2" for="magnet-grade">
    <span class="text-xs text-slate-300">Magnet grade</span>
    <select
      id="magnet-grade"
      class="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-xs text-slate-100 focus:border-emerald-500 focus:outline-none"
      value={selected}
      onchange={onGradeChange}
    >
      {#each config.magnetGradeNames as name (name)}
        <option value={name}>{name}</option>
      {/each}
      <option value={CUSTOM_GRADE}>{CUSTOM_GRADE}</option>
    </select>
  </label>

  {#if gradeInfo}
    <div class="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-[10px] text-slate-500">
      <span>Reference Br {gradeInfo.br_min_t.toFixed(2)}–{gradeInfo.br_max_t.toFixed(2)} T</span>
      <span>typ {gradeInfo.br_typ_t.toFixed(2)} T</span>
      {#each tempSuffixes as [suffix, temp] (suffix)}
        <span>{suffix} {temp}°C</span>
      {/each}
      <span class="text-emerald-400/80">auto Br {config.magnet_remanence_t.toFixed(2)} T</span>
    </div>
  {:else}
    <label
      class="flex items-center justify-between gap-2 text-xs text-slate-400"
      for="custom-magnet-remanence"
    >
      <span>Custom remanence Br (T)</span>
      <NumberField
        id="custom-magnet-remanence"
        step={0.01}
        min={0.0001}
        max={2.5}
        value={config.magnet_remanence_t}
        ariaLabel="Custom magnet remanence (T)"
        class="w-24 shrink-0 px-2 py-1 text-xs font-mono text-emerald-200"
        onCommit={(value) => (config.magnet_remanence_t = value)}
      />
    </label>
  {/if}
</div>