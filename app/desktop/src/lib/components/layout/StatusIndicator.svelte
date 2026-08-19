<script lang="ts">
  let { loading }: { loading: boolean } = $props();
</script>

<div class="flex items-center gap-2 text-xs" aria-live="polite">
  {#if loading}
    <span class="inline-block h-3 w-3 animate-pulse rounded-full bg-amber-400" aria-hidden="true"></span>
    <span class="text-amber-300">computing…</span>
  {:else}
    <span class="inline-block h-3 w-3 rounded-full bg-emerald-400" aria-hidden="true"></span>
    <span class="text-emerald-300">ready</span>
  {/if}
</div>