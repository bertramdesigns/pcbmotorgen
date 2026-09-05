<!--
  ═══════════════════════════════════════════════════════════════════════
  CoilPreviewLightbox — the expanded coil preview modal.

  Hosts the Bits UI Dialog (Dialog.Root/Portal/Overlay/Content) with the
  expanded frame + canvas pair, ALL interactive presentation toggles —
  per-phase, per-layer, via, pole-pitch, band-width and pole-region
  visibility, the pole-region phase picker, and the one-section hint — and
  the measure-ruler toolbar. Zoom/reset controls live BELOW the canvas
  (identical to the inline card), and the mover slider joins the strip it
  moves via the shared MotionStore.

  The schematic state itself is shared: `view` (CoilPreviewViewState),
  `gestures` (CoilPreviewGestures) and `measure` (CoilPreviewMeasure) are
  instances owned by CoilPreview and passed down, so both view instances
  (inline frame + canvas and modal frame + canvas) always show the same
  live schematic. Only ONE pair is drawn per render tick — the `expanded`
  rune in the parent picks the active pair. The modal frame + canvas refs
  bind back up (`bind:modalFrameRef`/`bind:modalCanvasRef`) because the
  parent's ResizeObserver sizing + draw effect and the gesture wheel
  listener drive both pairs.

  Dismissal: a backdrop click, Escape, and the × close button all dismiss
  it through Bits' dismissible/escape layers — no custom keydown/
  pointerdown handlers live here. While expanded, the page behind the
  lightbox is scroll-locked by the custom refcounted helper (document
  overflow lock + backdrop wheel/touchmove guard), which stays the source
  of truth — Bits' built-in scroll lock is therefore disabled
  (`preventScroll={false}`).
  Assessed in kata 1jfa — Bits' lock is not a replacement: it only writes
  overflow/padding/pointer-events on <body> (never inline on <html>) and
  has no wheel/touchmove preventDefault, so the e2e contract's pinned
  invariants (inline root+body overflow lock, backdrop wheel
  defaultPrevented) fail without this helper (4/6 spec runs red under
  pure-Bits). It must not run alongside either: Bits' delayed body-style
  reset re-applies a stale snapshot and leaves the page scroll-locked
  after close.
  ═══════════════════════════════════════════════════════════════════════
-->
<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import type { MotionStore } from "../../stores/motion.svelte";
  import { Dialog } from "bits-ui";
  import {
    computeMeasureRuler,
    formatMetresMm,
  } from "../../previewGeometry";
  import type { CoilPreviewGestures } from "../../utils/coilPreviewGestures.svelte";
  import type { CoilPreviewMeasure } from "../../utils/coilPreviewMeasure.svelte";
  import {
    attachBackdropScrollGuard,
    lockPageScroll,
  } from "../../utils/pageScrollLock";
  import CoilPreviewControls from "./CoilPreviewControls.svelte";
  import MoverPositionControls from "./MoverPositionControls.svelte";
  import { PHASE_COLORS, PREVIEW_H, PREVIEW_W } from "./coilPreviewCanvas";
  import type { CoilPreviewViewState } from "./coilPreviewViewState.svelte";

  let {
    expanded = $bindable(false),
    config,
    motion,
    view,
    gestures,
    measure,
    stats,
    oneSectionConductorCount,
    uniquePhases,
    uniqueLayers,
    hasPolePitchData,
    hasBandWidthData,
    hasPoleRegionData,
    poleRegionPhases,
    modalFrameRef = $bindable(),
    modalCanvasRef = $bindable(),
  }: {
    /** Dialog open state, bound to the parent's `expanded` rune. */
    expanded?: boolean;
    config: ConfigStore;
    motion: MotionStore;
    view: CoilPreviewViewState;
    gestures: CoilPreviewGestures;
    measure: CoilPreviewMeasure;
    /** Header stat line ("N phases · N layers · N coil groups"), precomputed. */
    stats: string;
    oneSectionConductorCount: number;
    uniquePhases: { idx: number; name: string; colorIdx: number }[];
    uniqueLayers: { idx: number }[];
    hasPolePitchData: boolean;
    hasBandWidthData: boolean;
    hasPoleRegionData: boolean;
    poleRegionPhases: string[];
    /** Expanded frame + canvas pair, driven by the parent's draw machinery. */
    modalFrameRef?: HTMLDivElement;
    modalCanvasRef?: HTMLCanvasElement;
  } = $props();

  // Virtual drawing space — same constants as the inline card (the CSS box
  // keeps the same 760:260 aspect ratio as the old SVG viewBox).
  const W = PREVIEW_W;
  const H = PREVIEW_H;

  let backdropRef: HTMLDivElement | null = $state(null);
  let modalPanelRef: HTMLDivElement | null = $state(null);

  // Modal pointer/touch handlers compose the measure tool with the gesture
  // utility (order preserved from the pre-extraction implementation: taps
  // bookkeep before/after the gesture pass exactly as it did inline).
  function onModalPointerDown(e: PointerEvent) {
    measure.handlePointerDown(e);
    gestures.handlePointerDown(e);
  }

  function onModalPointerMove(e: PointerEvent) {
    measure.handlePointerMove(e);
    gestures.handlePointerMove(e);
  }

  function onModalPointerUp(e: PointerEvent) {
    gestures.handlePointerEnd(e);
    measure.handlePointerUp(e);
  }

  function onModalPointerCancel(e: PointerEvent) {
    measure.handlePointerCancel(e);
    gestures.handlePointerEnd(e);
  }

  function onModalTouchStart(e: TouchEvent) {
    measure.handleTouchStart(e);
    gestures.handleTouchStart(e);
  }

  function onModalTouchMove(e: TouchEvent) {
    measure.handleTouchMove(e);
    gestures.handleTouchMove(e);
  }

  function onModalTouchEnd(e: TouchEvent) {
    gestures.handleTouchEnd(e);
    measure.handleTouchEnd(e);
  }

  function onModalTouchCancel(e: TouchEvent) {
    measure.handleTouchCancel(e);
    gestures.handleTouchEnd(e);
  }

  // Scroll lock for the lightbox (Kata xy31): while the modal is open the
  // page behind it must never scroll. Two layers:
  //   1. refcounted overflow:hidden on <html>/<body> — kills document
  //      scrolling below lg (above lg the stylesheet already hides the root
  //      scroller);
  //   2. non-passive wheel/touchmove guards on the backdrop that
  //      preventDefault() every event landing OUTSIDE the modal's scrollable
  //      panel, so browser scroll chaining can never reach a container
  //      behind the overlay. The panel itself carries `overscroll-contain`
  //      so its own scrolling never chains past the modal.
  //
  // Bits' own lock must stay OFF (`preventScroll={false}` below): enabled
  // alongside this helper it double-writes <body> style, and its 24ms
  // delayed reset restores a snapshot taken after this lock had already
  // written — the page stays overflow:hidden after close (kata 1jfa).
  $effect(() => {
    if (!expanded) return;
    const backdrop = backdropRef;
    if (!backdrop) return;
    const detachGuard = attachBackdropScrollGuard(backdrop, modalPanelRef);
    const unlock = lockPageScroll(document);
    return () => {
      detachGuard();
      unlock();
    };
  });
</script>

<Dialog.Root bind:open={expanded}>
  <Dialog.Portal>
    <Dialog.Overlay
      bind:ref={backdropRef}
      class="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm"
    />
    <Dialog.Content
      bind:ref={modalPanelRef}
      preventScroll={false}
      aria-label="Coil Preview — expanded"
      class="fixed left-1/2 top-1/2 z-50 flex max-h-[calc(100vh-2rem)] w-[calc(100%-2rem)] max-w-5xl -translate-x-1/2 -translate-y-1/2 flex-col gap-3 overflow-y-auto overscroll-contain rounded-lg border border-slate-700 bg-slate-900 p-4 shadow-2xl"
    >
      <!-- Header row -->
      <div class="flex items-center justify-between flex-wrap gap-2">
        <Dialog.Title level={3} class="text-sm font-semibold text-slate-200">
          Coil Preview — expanded
        </Dialog.Title>
        <div class="flex items-center gap-3 flex-wrap">
          <span class="text-xs text-slate-400">
            {stats}
          </span>
          <Dialog.Close
            class="px-2 py-0.5 text-xs rounded bg-slate-800 border border-slate-700 text-slate-300 hover:text-emerald-300 hover:border-emerald-600 transition-colors"
            aria-label="Close coil preview"
            >×</Dialog.Close
          >
        </div>
      </div>

      <!-- Visibility controls only — zoom/reset live below the canvas. -->
      <div class="flex items-center gap-3 flex-wrap">
        <!-- Phase visibility toggles (per phase). A coloured dot + label
             for each phase, with a checkbox to show/hide that phase's
             traces. The label and dot dim when the phase is hidden. -->
        {#if uniquePhases.length > 0}
          <div
            class="flex items-center gap-2 flex-wrap"
            role="group"
            aria-label="Phase visibility"
          >
            {#each uniquePhases as ph (ph.idx)}
              <label
                class="flex items-center gap-1 text-xs select-none cursor-pointer"
                class:text-slate-500={!view.isPhaseVisible(ph.idx)}
                class:text-slate-300={view.isPhaseVisible(ph.idx)}
              >
                <input
                  type="checkbox"
                  bind:checked={view.phaseVisibility[ph.idx]}
                  class="accent-emerald-500"
                  aria-label={"Show phase " + ph.name}
                />
                <span
                  class="inline-block w-2.5 h-2.5 rounded-full"
                  style="background-color: {PHASE_COLORS[
                    ph.colorIdx % PHASE_COLORS.length
                  ]}; opacity: {view.isPhaseVisible(ph.idx) ? 1 : 0.35}"
                ></span>
                <span>Phase {ph.name}</span>
              </label>
            {/each}
          </div>
        {/if}
        <!-- Layer visibility toggles (per layer). A grey dot + label for
             each copper layer, with a checkbox to show/hide that layer's
             traces. Layers are overlaid at true coordinates, so toggling is
             how you inspect a single layer in isolation. -->
        {#if uniqueLayers.length > 0}
          <div
            class="flex items-center gap-2 flex-wrap"
            role="group"
            aria-label="Layer visibility"
          >
            {#each uniqueLayers as l (l.idx)}
              <label
                class="flex items-center gap-1 text-xs select-none cursor-pointer"
                class:text-slate-500={!view.isLayerVisible(l.idx)}
                class:text-slate-300={view.isLayerVisible(l.idx)}
              >
                <input
                  type="checkbox"
                  checked={view.isLayerVisible(l.idx)}
                  onchange={() => view.toggleLayer(l.idx)}
                  class="accent-emerald-500"
                  aria-label={"Show layer " + l.idx}
                />
                <span
                  class="inline-block w-2.5 h-2.5 rounded-full"
                  style="background-color: #94a3b8; opacity: {view.isLayerVisible(
                    l.idx,
                  )
                    ? 1
                    : 0.35}"
                ></span>
                <span>Layer {l.idx}</span>
              </label>
            {/each}
          </div>
        {/if}
        <!-- Via visibility toggle -->
        <label
          class="flex items-center gap-1.5 text-xs text-slate-300 select-none cursor-pointer"
        >
          <input
            type="checkbox"
            bind:checked={view.showVias}
            class="accent-emerald-500"
            aria-label="Show vias"
          />
          <span
            class="inline-block w-2.5 h-2.5 rounded-full"
            style="background-color: #fbbf24; opacity: {view.showVias ? 1 : 0.35}"
          ></span>
          <span>Vias</span>
        </label>
        <!-- One-section toggle -->
        <label
          class="flex items-center gap-1.5 text-xs text-slate-300 select-none cursor-pointer"
        >
          <input
            type="checkbox"
            bind:checked={view.oneSection}
            class="accent-emerald-500"
            aria-label="Show only one repeating section of the pattern"
          />
          <span>one electrical period</span>
        </label>
        <!-- Pole-pitch ruler toggle (only when the sidecar ships a pitch). -->
        {#if hasPolePitchData}
          <label
            class="flex items-center gap-1.5 text-xs text-slate-300 select-none cursor-pointer"
          >
            <input
              type="checkbox"
              bind:checked={view.showPolePitch}
              class="accent-emerald-500"
              aria-label="Show pole-pitch dimension ruler"
            />
            <span
              class="inline-block w-2.5 h-2.5 rounded-full"
              style="background-color: #a5b4fc; opacity: {view.showPolePitch
                ? 1
                : 0.35}"
            ></span>
            <span>Pole pitch</span>
          </label>
        {/if}
        <!-- Band-width diagnostics toggle (only visible with matched rows). -->
        {#if hasBandWidthData}
          <label
            class="flex items-center gap-1.5 text-xs text-slate-300 select-none cursor-pointer"
          >
            <input
              type="checkbox"
              bind:checked={view.showBandWidths}
              class="accent-emerald-500"
              aria-label="Show band-width diagnostics"
            />
            <span
              class="inline-block w-2.5 h-2.5 rounded-full"
              style="background-color: #34d399; opacity: {view.showBandWidths
                ? 1
                : 0.35}"
            ></span>
            <span>Conductor band widths</span>
          </label>
        {/if}
        <!-- Pole-region zones toggle + phase picker (only when the routing
             sidecar ships valid region data). The checkbox is independent
             of per-phase trace visibility; the select picks which phase's
             zones to draw. -->
        {#if hasPoleRegionData}
          <div
            class="flex items-center gap-2"
            role="group"
            aria-label="Pole regions overlay"
          >
            <label
              class="flex items-center gap-1.5 text-xs text-slate-300 select-none cursor-pointer"
            >
              <input
                type="checkbox"
                bind:checked={view.showPoleRegions}
                class="accent-rose-500"
                aria-label="Show pole regions"
              />
              <span
                class="inline-block w-2.5 h-2.5 rounded-full"
                style="background: linear-gradient(90deg, #f87171 50%, #60a5fa 50%); opacity: {view.showPoleRegions
                  ? 1
                  : 0.35}"
              ></span>
              <span>Pole regions</span>
            </label>
            <select
              aria-label="Pole regions phase"
              class="rounded border border-slate-700 bg-slate-800 px-1.5 py-0.5 text-xs text-slate-200 disabled:opacity-40 disabled:cursor-not-allowed focus:outline-none focus:border-emerald-600"
              bind:value={view.poleRegionPhase}
              disabled={!view.showPoleRegions}
            >
              <option value="">All phases</option>
              {#each poleRegionPhases as phase (phase)}
                <option value={phase}>{phase}</option>
              {/each}
            </select>
          </div>
        {/if}
      </div>

      <!-- Measure ruler toolbar (lightbox only): mode toggle, reset (shown
           only while measuring) and a live status prompt. -->
      <div class="flex items-center gap-3 flex-wrap">
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="px-2 py-0.5 text-xs rounded border transition-colors {measure.mode
              ? 'bg-pink-500/15 border-pink-500 text-pink-300'
              : 'bg-slate-800 border-slate-700 text-slate-300 hover:text-pink-300 hover:border-pink-600'}"
            aria-pressed={measure.mode}
            aria-label="Toggle measure tool"
            onclick={() => measure.toggleMode()}
          >Measure</button>
          {#if measure.mode}
            <button
              type="button"
              class="px-2 py-0.5 text-xs rounded bg-slate-800 border border-slate-700 text-slate-300 hover:text-rose-300 hover:border-rose-600 transition-colors"
              aria-label="Clear measurement"
              onclick={() => measure.clear()}
            >Reset</button>
          {/if}
        </div>
        {#if measure.mode}
          <span class="text-xs text-slate-400" role="status" aria-live="polite">
            {#if measure.p1}
              {#if measure.p2}
                {formatMetresMm(computeMeasureRuler(measure.p1, measure.p2).mm / 1000)} — click again to clear
              {:else}
                {#if measure.cursor}
                  {formatMetresMm(computeMeasureRuler(measure.p1, measure.cursor).mm / 1000)} — click to lock
                {:else}
                  click to lock the dimension
                {/if}
              {/if}
            {:else}
              click to set the start point
            {/if}
          </span>
        {/if}
      </div>

      <!-- Modal frame div owns the ARIA label (the canvas is the paint
           target; data-* attributes carry the introspection counters,
           mirroring the inline pair). All gestures delegate to the same
           utility instance. -->
      <div
        bind:this={modalFrameRef}
        class="relative w-full touch-none select-none {gestures.isPanning
          ? 'cursor-grabbing'
          : measure.mode
            ? 'cursor-crosshair'
            : 'cursor-grab'}"
        role="img"
        aria-label="Coil preview — expanded"
        style="aspect-ratio: {W} / {H};"
        onpointerdown={onModalPointerDown}
        onpointermove={onModalPointerMove}
        onpointerup={onModalPointerUp}
        onpointercancel={onModalPointerCancel}
        onlostpointercapture={gestures.handleLostPointerCapture}
        ontouchstart={onModalTouchStart}
        ontouchmove={onModalTouchMove}
        ontouchend={onModalTouchEnd}
        ontouchcancel={onModalTouchCancel}
      >
        <canvas bind:this={modalCanvasRef} class="block h-full w-full"
        ></canvas>
      </div>

      <!-- Zoom + reset view below the expanded canvas, same layout as the
           inline preview. -->
      <CoilPreviewControls
        zoomLabel={gestures.zoomLabel}
        canZoomIn={gestures.canZoomIn}
        canZoomOut={gestures.canZoomOut}
        onZoomIn={gestures.zoomIn}
        onZoomOut={gestures.zoomOut}
        onResetZoom={gestures.zoomReset}
        onResetView={gestures.resetView}
      />

      <!-- Mover position: same controls as the Design reflection on the
           shared MotionStore, so the slider lives next to the strip it
           moves inside the lightbox. -->
      <MoverPositionControls {config} {motion} />

      <!-- Modal note -->
      <p class="text-xs text-slate-500">
        Solid lines = active legs. Dashed = end-turns. Magnet poles
        overlay along the top edge. Layers are overlaid at true coordinates;
        use the layer toggles to inspect each copper layer. Pole-pitch,
        band-width and pole-region annotations come from the
        routing-dimensions sidecar (pole-region x boundaries are
        pattern-owned).{#if view.oneSection}
          <span class="text-amber-300"
            >Showing only the first {oneSectionConductorCount} conductors (one
            repeating section).</span
          >{/if} Drag to pan; pinch or use the zoom controls below.
      </p>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
