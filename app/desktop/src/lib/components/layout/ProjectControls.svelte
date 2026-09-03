<script lang="ts">
  import type { ProjectStore } from "../../stores/project.svelte";

  let { projects }: { projects: ProjectStore } = $props();
</script>

<!--
  Project file affordances (kata 0cgm): Open / Save / Save As plus the
  dirty-state indicator. The store owns the flows; this component only
  renders state and dispatches clicks.
-->
<div class="flex items-center gap-3" aria-label="Project file">
  <span
    class="flex max-w-[260px] items-center gap-1.5 text-sm text-slate-300"
    title={projects.currentPath ?? "Not saved yet — use Save As"}
  >
    <span class="truncate">{projects.label}</span>
    {#if projects.isDirty}
      <span
        class="text-amber-400"
        title="Unsaved changes"
        aria-label="Unsaved changes"
      >●</span>
    {/if}
  </span>

  <div class="flex gap-1" role="group" aria-label="Project actions">
    <button
      type="button"
      class="rounded-md px-3 py-1.5 text-sm text-slate-300 transition hover:bg-slate-800 hover:text-slate-100 disabled:cursor-not-allowed disabled:opacity-50"
      disabled={projects.busy}
      onclick={() => void projects.open()}
    >
      Open
    </button>
    <button
      type="button"
      class="rounded-md px-3 py-1.5 text-sm text-slate-300 transition hover:bg-slate-800 hover:text-slate-100 disabled:cursor-not-allowed disabled:opacity-50"
      disabled={projects.busy}
      onclick={() => void projects.save(false)}
    >
      Save
    </button>
    <button
      type="button"
      class="rounded-md px-3 py-1.5 text-sm text-slate-300 transition hover:bg-slate-800 hover:text-slate-100 disabled:cursor-not-allowed disabled:opacity-50"
      disabled={projects.busy}
      onclick={() => void projects.save(true)}
    >
      Save As
    </button>
  </div>
</div>
