<script lang="ts">
  let { loading }: { loading: boolean } = $props();
</script>

<div class="flex items-center gap-2 text-xs" aria-live="polite">
  <span class="text-slate-400">{loading ? "computing" : "ready"}</span>
  {#if loading}
    <span
      class="inline-block h-3 w-3 animate-spin rounded-full border-2 border-amber-400/30 border-t-amber-400"
      aria-hidden="true"
    ></span>
  {:else}
    <span
      class="inline-block h-3 w-3 rounded-full bg-emerald-400"
      aria-hidden="true"
    ></span>
  {/if}
</div>
