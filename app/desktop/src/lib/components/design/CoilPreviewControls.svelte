<script lang="ts">
  /**
   * CoilPreviewControls.svelte — presentation-only zoom/reset button group.
   *
   * No gesture logic lives here: the parent passes the formatted zoom label,
   * the enabled states and the callback props. Placed below BOTH the inline
   * and the expanded canvas frames so both views keep identical controls.
   */
  let {
    zoomLabel,
    canZoomIn,
    canZoomOut,
    onZoomIn,
    onZoomOut,
    onResetZoom,
    onResetView,
  }: {
    zoomLabel: string;
    canZoomIn: boolean;
    canZoomOut: boolean;
    onZoomIn: () => void;
    onZoomOut: () => void;
    onResetZoom: () => void;
    onResetView: () => void;
  } = $props();
</script>

<div class="flex items-center justify-end gap-1" role="group" aria-label="Zoom controls">
  <button
    type="button"
    class="px-2 py-0.5 text-xs rounded bg-slate-800 border border-slate-700 text-slate-300 hover:text-emerald-300 hover:border-emerald-600 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
    aria-label="Zoom out"
    disabled={canZoomOut}
    onclick={onZoomOut}
  >−</button>
  <button
    type="button"
    class="px-2 py-0.5 text-xs font-mono rounded bg-slate-800 border border-slate-700 text-slate-300 hover:text-emerald-300 hover:border-emerald-600 transition-colors"
    aria-label="Reset zoom"
    title="Reset zoom"
    onclick={onResetZoom}
  >{zoomLabel}×</button>
  <button
    type="button"
    class="px-2 py-0.5 text-xs rounded bg-slate-800 border border-slate-700 text-slate-300 hover:text-emerald-300 hover:border-emerald-600 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
    aria-label="Zoom in"
    disabled={canZoomIn}
    onclick={onZoomIn}
  >+</button>
  <button
    type="button"
    class="ml-1 px-2 py-0.5 text-xs rounded bg-slate-800 border border-slate-700 text-slate-300 hover:text-emerald-300 hover:border-emerald-600 transition-colors"
    aria-label="Reset zoom and pan"
    title="Reset zoom and pan"
    onclick={onResetView}
  >Reset View</button>
</div>