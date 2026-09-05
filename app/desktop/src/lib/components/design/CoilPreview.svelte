<!--
  ═══════════════════════════════════════════════════════════════════════
  Canvas coil viewer.

  Layers are drawn OVERLAID at their TRUE coordinates (a top-down view
  matching the DXF export). Per-layer show/hide toggles let you inspect 
  each copper layer alone.

  All geometry math lives in `../../previewGeometry.ts` (pure, tested,
  shared) and the canvas paint routines in `./coilPreviewCanvas.ts`
  (`CoilPreviewRenderer` — extracted from this file's former ~400-line
  `drawInto`); this component maps refs/sizing/state onto them.

  Lightbox: the preview expands into a modal overlay via the ⤢ button (or a
  double-click on the inline preview). The expanded view hosts ALL
  interactive presentation toggles — per-phase, per-layer, via, pole-pitch,
  band-width and pole-region visibility, the pole-region phase picker, and
  the one-section hint. Zoom and reset-view controls live
  BELOW the canvas in both views. Both view instances (inline frame + canvas
  and modal frame + canvas) stay mounted while the component exists, but
  only ONE of them is drawn per render tick — the `expanded` rune picks the
  active pair, so both canvases always show the same live schematic. The
  lightbox is a Bits UI Dialog (Dialog.Root/Portal/Overlay/Content): a
  backdrop click, Escape, and the × close button all dismiss it through
  Bits' dismissible/escape layers — no custom keydown/pointerdown handlers
  live here anymore. While expanded, the page behind the lightbox is
  scroll-locked by the custom refcounted helper (document overflow lock +
  backdrop wheel/touchmove guard), which stays the source of truth — Bits'
  built-in scroll lock is therefore disabled (`preventScroll={false}`).
  Assessed in kata 1jfa — Bits' lock is not a replacement: it only writes
  overflow/padding/pointer-events on <body> (never inline on <html>) and
  has no wheel/touchmove preventDefault, so the e2e contract's pinned
  invariants (inline root+body overflow lock, backdrop wheel
  defaultPrevented) fail without this helper (4/6 spec runs red under
  pure-Bits). It must not run alongside either: Bits' delayed body-style
  reset re-applies a stale snapshot and leaves the page scroll-locked
  after close.

  Interaction: one pointer (mouse / pen / lone touch) drags to pan; a
  two-finger pinch or a ctrl+wheel trackpad pinch zooms CONTINUOUSLY
  (clamped 0.5×…10×) with focal-point anchoring. All gesture behavior —
  touch/pointer ownership, pointer-capture cleanup, pan/pinch/focal math —
  lives in lib/utils/coilPreviewGestures.svelte; this file only binds the
  gesture handlers and renders the reactive view state.
  ═══════════════════════════════════════════════════════════════════════
-->
<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import type { MotionStore } from "../../stores/motion.svelte";
  import type { CoilPathDto } from "../../types";
  import {
    clientToVirtual,
    computePreviewGeometry,
    computeMagnets,
    computeMeasureRuler,
    computeUniqueLayers,
    computeUniquePhases,
    computeVisibleSegments,
    computeVisibleArcs,
    computePolePitchRuler,
    computeBandWidthRows,
    computeOverlayFitBounds,
    computeMoverStripBounds,
    computePoleRegionZones,
    computePoleRegionPhases,
    filterPoleRegionsByPhase,
    resolvePoleRegionPhaseSelection,
    virtualToWorld,
    firstMagnetCenterX as magnetCenterX,
    formatMetresMm,
    type Point2D,
    type PreviewConfigLike,
  } from "../../previewGeometry";
  import type { WorldTransform } from "../../chart";
  import { fitWorldToView, unionBounds } from "../../chart";
  import { Dialog } from "bits-ui";
  import { CoilPreviewGestures } from "../../utils/coilPreviewGestures.svelte";
  import CoilPreviewControls from "./CoilPreviewControls.svelte";
  import MoverPositionControls from "./MoverPositionControls.svelte";
  import {
    PHASE_COLORS,
    PREVIEW_H,
    PREVIEW_W,
    CoilPreviewRenderer,
    bandRowInOnePeriod,
  } from "./coilPreviewCanvas";
  import {
    attachBackdropScrollGuard,
    lockPageScroll,
  } from "../../utils/pageScrollLock";

  let {
    config,
    coils,
    motion,
  }: { config: ConfigStore; coils: CoilPathDto | null; motion: MotionStore } =
    $props();

  // Virtual drawing space. The CSS box keeps the same 760:260 aspect ratio as
  // the old SVG viewBox; every overlay (legend, disclaimer) is laid out in
  // these virtual px so it matches the SVG markup exactly. The paint routines
  // live in ./coilPreviewCanvas.ts, which shares these constants.
  const W = PREVIEW_W;
  const H = PREVIEW_H;
  const PAD = 30;

  // -------------------------------------------------------------------
  // Zoom bounds + button steps. The actual zoom state, pan, pinch and
  // scroll-zoom logic live in lib/utils/coilPreviewGestures.svelte — this
  // component only supplies the geometry transform + constants and forwards
  // DOM events to the gesture class. `zoom` is CONTINUOUS (0.5…10): the
  // buttons step through ZOOM_STEPS, while pinches are continuous.
  const MIN_ZOOM = 0.5;
  const MAX_ZOOM = 10;
  const ZOOM_STEPS = [0.5, 1, 1.5, 2, 3, 4, 6, 8, 10] as const;
  let oneSection = $state(false); // start OFF so the full winding is visible

  /** One repeating unit of the winding = max(6, 2 × phases) conductors. */
  let oneSectionConductorCount = $derived(Math.max(6, 2 * config.phases));

  // -------------------------------------------------------------------
  // Geometry + view — ALL math comes from lib/previewGeometry + lib/chart.
  // `config` (ConfigStore) satisfies PreviewConfigLike structurally (it has
  // magnet_count / magnet_width_mm / magnet_gap_mm plus extras, which is
  // fine for structural typing). Magnet placement also consumes the generated
  // routing_dimensions sidecar when available: each pitch cell's right edge
  // (solid bar + trailing gap) is anchored to the pattern's B-phase leg
  // centres so the poles stay locked to the leg zones. Legacy or missing
  // sidecars use computeMagnets' centered pitch-cell fallback.
  // -------------------------------------------------------------------
  let g = $derived(computePreviewGeometry(coils, config as PreviewConfigLike));

  /** World→virtual transform for a zoom level, using the CURRENT geometry
   *  (backed by lib/chart fitWorldToView; injected into the gesture class).
   *  `overlayFitBounds` unions the schematic fit box with any
   *  `routing_dimensions` overlay geometry (pole-pitch ruler, band-width
   *  rows, pole regions) so the camera never clips an annotation. */
  function worldTransformFor(zoom: number): WorldTransform {
    return fitWorldToView(overlayFitBounds, W, H, PAD, zoom);
  }

  // All pan/pinch/zoom state, cursor maths, touch/pointer coordination and
  // pointer-capture cleanup live in the gesture utility; this component only
  // reads its reactive view state and binds its handlers.
  const gestures = new CoilPreviewGestures({
    virtualW: W,
    virtualH: H,
    minZoom: MIN_ZOOM,
    maxZoom: MAX_ZOOM,
    zoomSteps: ZOOM_STEPS,
    getWorldTransform: worldTransformFor,
  });

  let worldTransform = $derived(worldTransformFor(gestures.zoom));
  let magnets = $derived(computeMagnets(config, coils?.routing_dimensions));
  // The mover position from the shared MotionStore places the whole strip in
  // ABSOLUTE track coordinates: bar 0's left edge lands exactly at
  // `motion.stripStartMm` (= position − mover_span/2), matching the design
  // reflection's iso view and readouts. Polarity order and pitch come from
  // the pattern-anchored `computeMagnets` layout.
  let visibleMagnets = $derived.by(() => {
    if (magnets.length === 0) return magnets;
    const restStartM = Math.min(...magnets.map((m) => m.x));
    const targetStartM = motion.stripStartMm / 1000;
    return magnets.map((m) => ({ ...m, x: m.x + targetStartM - restStartM }));
  });
  // Camera-fit extreme: the largest leading-edge shift of either travel end
  // relative to the pattern-anchored rest layout, so reset-view never clips
  // the moved magnets.
  let maxMoverOffsetM = $derived.by(() => {
    const halfSpanM = config.mover_span_mm / 2000;
    const restStartM =
      magnets.length > 0 ? Math.min(...magnets.map((m) => m.x)) : 0;
    const leadAtMin = motion.moverMinMm / 1000 - halfSpanM - restStartM;
    const leadAtMax = motion.moverMaxMm / 1000 - halfSpanM - restStartM;
    return Math.max(0, leadAtMin, leadAtMax);
  });
  let uniquePhases = $derived(computeUniquePhases(coils));
  let uniqueLayers = $derived(computeUniqueLayers(coils));
  let visibleSegments = $derived(
    computeVisibleSegments(coils, oneSection, oneSectionConductorCount),
  );
  let visibleArcs = $derived(
    computeVisibleArcs(coils, oneSection, oneSectionConductorCount),
  );

  // -------------------------------------------------------------------
  // routing_dimensions sidecar overlays: pole-pitch ruler, band-width rows,
  // and pattern-owned pole-region zones.
  // Pure world-space geometry from lib/previewGeometry; resistant to an
  // absent/legacy sidecar (null sidecars → no ruler, no rows). The camera
  // fit always includes them so reset-view never clips an annotation.
  // -------------------------------------------------------------------
  let firstMagnetCentreX = $derived(magnetCenterX(visibleMagnets));
  let poleRuler = $derived(
    computePolePitchRuler(
      coils?.routing_dimensions,
      firstMagnetCentreX,
      g.fitBounds.maxY + 0.0008, // just above the magnet strip
    ),
  );
  let bandRows = $derived(
    computeBandWidthRows(coils, coils?.routing_dimensions),
  );

  // Independent presentation toggles, expanded-preview controls only. Kept
  // even when no data exists (they simply are not bound then).
  let showPolePitch = $state(true);
  let showBandWidths = $state(true);
  let showPoleRegions = $state(true); // default on, like the other overlays
  let poleRegionPhase = $state(""); // "" = all phases

  // Pole-region zones (pattern-owned phase/pole boundaries from the routing
  // sidecar). `poleRegionPhase` is the phase-picker selection; "" = "All
  // phases". Draws only the selected phase's zones. Independent of the
  // per-phase trace visibility toggles.
  let poleRegionZones = $derived(
    computePoleRegionZones(coils?.routing_dimensions),
  );
  let hasPoleRegionData = $derived(poleRegionZones.length > 0);
  let poleRegionPhases = $derived(computePoleRegionPhases(poleRegionZones));
  let visiblePoleRegionZones = $derived(
    filterPoleRegionsByPhase(poleRegionZones, poleRegionPhase),
  );

  /** Guarded sync so the picker stays valid when a regenerated payload ships
   *  different phase labels: if the selected label is gone, collapse back to
   *  "All phases". Reads + writes `poleRegionPhase`; the write only happens
   *  on the invalid branch so the effect terminates (no update cycle). */
  $effect(() => {
    const resolved = resolvePoleRegionPhaseSelection(
      poleRegionPhase,
      poleRegionPhases,
    );
    if (resolved !== poleRegionPhase) poleRegionPhase = resolved;
  });

  let overlayFitBounds = $derived(
    (() => {
      const base = computeOverlayFitBounds(
        computeMoverStripBounds(
          g.fitBounds,
          magnets,
          maxMoverOffsetM,
          g.magnetTop,
          0.003,
        ),
        showPolePitch ? poleRuler : null,
        showBandWidths ? bandRows : [],
        showPoleRegions ? visiblePoleRegionZones : [],
        g.boardRect.y,
        g.boardRect.y + g.boardRect.h,
      );
      return lockedMeasureBounds ? unionBounds(base, lockedMeasureBounds) : base;
    })(),
  );
  let hasPolePitchData = $derived(poleRuler !== null);
  let hasBandWidthData = $derived(bandRows.length > 0);

  // Band rows the renderer paints: phase/layer visibility + the one-section
  // window filter (pure helper lives in ./coilPreviewCanvas.ts).
  let visibleBandRows = $derived(
    showBandWidths
      ? bandRows.filter(
          (row) =>
            isPhaseVisible(row.phaseIdx) &&
            isLayerVisible(row.layer) &&
            bandRowInOnePeriod(row, oneSection, visibleSegments),
        )
      : [],
  );

  // -------------------------------------------------------------------
  // Per-phase visibility. `coils.phases` is one entry per (phase, layer)
  // pair, so we toggle against `phase_idx` via `uniquePhases`. Out-of-bounds
  // indices default to visible.
  // -------------------------------------------------------------------
  let phaseVisibility = $state<boolean[]>([true, true, true, true, true, true]);

  function isPhaseVisible(phaseIdx: number): boolean {
    return phaseVisibility[phaseIdx] !== false;
  }

  // ---------------------------------------------------------------------
  // Per-layer visibility. Layers are overlaid at their true coordinates,
  // so toggling them is the only way to inspect a single copper layer in
  // isolation. Default: all layers visible (empty record → everything on).
  // ---------------------------------------------------------------------
  let layerVisibility = $state<Record<number, boolean>>({});

  function isLayerVisible(layerIdx: number): boolean {
    return layerVisibility[layerIdx] !== false;
  }

  function toggleLayer(layerIdx: number): void {
    layerVisibility = {
      ...layerVisibility,
      [layerIdx]: !isLayerVisible(layerIdx),
    };
  }

  // Inter-layer via visibility. Toggle lives in the expanded lightbox only.
  let showVias = $state(true);

  // -------------------------------------------------------------------
  // Lightbox measure ruler. Two-click dimension tool: click 1 sets the
  // start point, click 2 locks the dimension, click 3 clears it. The
  // reset button (visible while measure mode is on) clears without a
  // click. Taps are distinguished from pan-drags by a movement threshold,
  // so pan/pinch/zoom keep working while measuring. Lightbox only — the
  // overlay draws only while `expanded`.
  // -------------------------------------------------------------------
  const MEASURE_TAP_PX = 6;
  let measureMode = $state(false);
  let measureP1 = $state<Point2D | null>(null);
  let measureP2 = $state<Point2D | null>(null);
  let measureCursor = $state<Point2D | null>(null);

  /** Locked-dimension fit bounds (camera-fit input, lightbox only). */
  let lockedMeasureBounds = $derived(
    expanded && measureMode && measureP1 && measureP2
      ? computeMeasureRuler(measureP1, measureP2).bounds
      : null,
  );

  /** Non-reactive tap bookkeeping: pointer/touch start positions. */
  let measureDowns = new Map<number, { x: number; y: number }>();
  let measureTouches = new Map<number, { x: number; y: number }>();

  function measureTapAt(clientX: number, clientY: number): void {
    const frame = modalFrameRef;
    if (!measureMode || !frame) return;
    const rect = frame.getBoundingClientRect();
    const v = clientToVirtual(clientX, clientY, rect, W, H);
    const w = virtualToWorld(
      v.x,
      v.y,
      worldTransform,
      gestures.panX,
      gestures.panY,
    );
    if (!measureP1) {
      measureP1 = w;
      measureCursor = null;
    } else if (!measureP2) {
      measureP2 = w;
      measureCursor = null;
    } else {
      // Third click clears the locked measurement.
      measureP1 = null;
      measureP2 = null;
      measureCursor = null;
    }
  }

  function onModalPointerDown(e: PointerEvent) {
    if (e.pointerType === "mouse" && e.button !== 0) return;
    // Touch-compat pointers are ignored on browsers with native touches (the
    // touch route below handles taps there) — mirrors the gesture utility.
    if (e.pointerType === "touch" && "ontouchstart" in window) {
      gestures.handlePointerDown(e);
      return;
    }
    measureDowns.set(e.pointerId, { x: e.clientX, y: e.clientY });
    gestures.handlePointerDown(e);
  }

  function onModalPointerMove(e: PointerEvent) {
    if (measureMode && measureP1 && !measureP2 && e.pointerType !== "touch") {
      const frame = modalFrameRef;
      if (frame) {
        const rect = frame.getBoundingClientRect();
        const v = clientToVirtual(e.clientX, e.clientY, rect, W, H);
        measureCursor = virtualToWorld(
          v.x,
          v.y,
          worldTransform,
          gestures.panX,
          gestures.panY,
        );
      }
    }
    gestures.handlePointerMove(e);
  }

  function onModalPointerUp(e: PointerEvent) {
    gestures.handlePointerEnd(e);
    if (e.pointerType === "touch" && "ontouchstart" in window) return;
    const down = measureDowns.get(e.pointerId);
    measureDowns.delete(e.pointerId);
    if (!down) return;
    if (measureDowns.size > 0) return; // a second pointer was part of a pinch
    if (Math.hypot(e.clientX - down.x, e.clientY - down.y) <= MEASURE_TAP_PX) {
      measureTapAt(e.clientX, e.clientY);
    }
  }

  function onModalPointerCancel(e: PointerEvent) {
    measureDowns.delete(e.pointerId);
    gestures.handlePointerEnd(e);
  }

  function onModalTouchStart(e: TouchEvent) {
    for (const t of e.changedTouches) {
      measureTouches.set(t.identifier, { x: t.clientX, y: t.clientY });
    }
    gestures.handleTouchStart(e);
  }

  function onModalTouchMove(e: TouchEvent) {
    if (measureMode && measureP1 && !measureP2) {
      const t = e.touches[0];
      const frame = modalFrameRef;
      if (t && frame) {
        const rect = frame.getBoundingClientRect();
        const v = clientToVirtual(t.clientX, t.clientY, rect, W, H);
        measureCursor = virtualToWorld(
          v.x,
          v.y,
          worldTransform,
          gestures.panX,
          gestures.panY,
        );
      }
    }
    gestures.handleTouchMove(e);
  }

  function onModalTouchEnd(e: TouchEvent) {
    gestures.handleTouchEnd(e);
    for (const t of e.changedTouches) {
      const down = measureTouches.get(t.identifier);
      measureTouches.delete(t.identifier);
      if (!down) continue;
      if (e.touches.length > 0) continue; // other fingers still down (pinch)
      if (Math.hypot(t.clientX - down.x, t.clientY - down.y) <= MEASURE_TAP_PX) {
        measureTapAt(t.clientX, t.clientY);
      }
    }
  }

  function onModalTouchCancel(e: TouchEvent) {
    for (const t of e.changedTouches) measureTouches.delete(t.identifier);
    gestures.handleTouchEnd(e);
  }

  // -------------------------------------------------------------------
  // Frame + backing-store sizing. A ResizeObserver keeps the canvas backing
  // store in lockstep with the rendered CSS box (dpr-capped at 2×); writing
  // `frameSize` re-triggers the draw effect below.
  //
  // Two view instances exist — the inline (collapsed card) pair and the
  // modal (lightbox) pair — but only ONE is drawn per render tick. Each
  // mounted pair gets its own ResizeObserver; every sizing write happens
  // inside a RO callback only (never in the effect body) and observers
  // disconnect on cleanup.
  // -------------------------------------------------------------------
  let frameRef: HTMLDivElement | undefined = $state();
  let canvasRef: HTMLCanvasElement | undefined = $state();
  let modalFrameRef: HTMLDivElement | undefined = $state();
  let modalCanvasRef: HTMLCanvasElement | undefined = $state();
  let backdropRef: HTMLDivElement | null = $state(null);
  let modalPanelRef: HTMLDivElement | null = $state(null);
  let expanded = $state(false);
  let frameSize = $state(0); // frame CSS-pixel width, measured by the RO

  // The pair that is currently visible: the inline canvas while collapsed,
  // the modal canvas while expanded. The draw effect paints only this pair.
  let activeFrame = $derived(expanded ? modalFrameRef : frameRef);
  let activeCanvas = $derived(expanded ? modalCanvasRef : canvasRef);

  $effect(() => {
    const frame = frameRef;
    const canvas = canvasRef;
    if (!frame || !canvas) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    // All sizing happens inside the RO callbacks only (no state writes in the
    // effect body). observe() guarantees an initial delivery before paint, so
    // the backing store + `frameSize` (which re-triggers the draw effect) are
    // always set before the first render is presented.
    const inlineRo = new ResizeObserver(() => {
      canvas.width = Math.round(frame.clientWidth * dpr);
      canvas.height = Math.round(frame.clientHeight * dpr);
      frameSize = frame.clientWidth;
    });
    inlineRo.observe(frame);

    // The modal pair only exists while expanded (bound on open, unbound on
    // close) — observe it only while it is mounted.
    const modalFrame = modalFrameRef;
    const modalCanvas = modalCanvasRef;
    let modalRo: ResizeObserver | undefined;
    if (modalFrame && modalCanvas) {
      modalRo = new ResizeObserver(() => {
        modalCanvas.width = Math.round(modalFrame.clientWidth * dpr);
        modalCanvas.height = Math.round(modalFrame.clientHeight * dpr);
        frameSize = modalFrame.clientWidth;
      });
      modalRo.observe(modalFrame);
    }

    return () => {
      inlineRo.disconnect();
      modalRo?.disconnect();
    };
  });

  // -------------------------------------------------------------------
  // Canvas rendering — the paint routines live in ./coilPreviewCanvas.ts
  // (`CoilPreviewRenderer`, extracted verbatim from the former `drawInto`).
  // This component owns the dual-instance inline/modal draw pattern: the
  // frame/canvas refs, the ResizeObserver sizing above, and the draw effect
  // below, which builds the per-tick `CoilPreviewDrawInput` snapshot and
  // paints the ACTIVE pair only. One renderer instance per component: its
  // revision counter is shared across the inline and modal canvases.
  // -------------------------------------------------------------------
  const renderer = new CoilPreviewRenderer();

  // Redraw whenever any input to the schematic changes. Draws the ACTIVE
  // canvas only — the inline pair while collapsed, the modal pair while
  // expanded. All reads below happen synchronously so Svelte auto-tracks
  // them (including the ones the renderer consumes through the visibility
  // callbacks). Runs even without coils so the "Awaiting coil generation…"
  // placeholder paints on the empty active canvas.
  $effect(() => {
    const frame = activeFrame;
    const canvas = activeCanvas;
    if (!frame || !canvas) return;
    if (frame.clientWidth <= 0) return;
    void worldTransform;
    void gestures.panX;
    void gestures.panY;
    void oneSection;
    void phaseVisibility;
    void layerVisibility;
    void showVias;
    void showPolePitch;
    void showBandWidths;
    void showPoleRegions;
    void poleRegionPhase;
    void visiblePoleRegionZones;
    void visibleSegments;
    void visibleArcs;
    void visibleMagnets;
    void poleRuler;
    void bandRows;
    void overlayFitBounds;
    void g;
    void frameSize;
    void expanded;
    renderer.draw(frame, canvas, {
      coils,
      activeAreaLengthMm: config.active_area_length_mm,
      geometry: g,
      worldTransform,
      panX: gestures.panX,
      panY: gestures.panY,
      visibleMagnets,
      visibleSegments,
      visibleArcs,
      isPhaseVisible: (phaseIdx) => isPhaseVisible(phaseIdx),
      isLayerVisible: (layerIdx) => isLayerVisible(layerIdx),
      poleRuler,
      showPolePitch,
      bandRows: visibleBandRows,
      poleRegions: showPoleRegions ? visiblePoleRegionZones : [],
      measure:
        expanded && measureMode
          ? { p1: measureP1, p2: measureP2, cursor: measureCursor }
          : null,
    });
  });

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

  // -------------------------------------------------------------------
  // Interaction — ALL pan/pinch/scroll-zoom logic, touch↔pointer
  // coordination, pointer-capture cleanup and cursor math lives in
  // lib/utils/coilPreviewGestures.svelte. The component merely forwards the
  // DOM events below and reads the gesture instance's reactive state
  // (gestures.zoom / gestures.panX / gestures.panY / gestures.isPanning).
  // -------------------------------------------------------------------

  /** Double-clicking the inline preview opens the expanded lightbox. */
  function onInlineDoubleClick() {
    expanded = true;
  }

  // ctrl+wheel (macOS trackpad pinches / browser pinch-zoom) must be able to
  // preventDefault page zoom, so the utility handler is attached NON-passively
  // to both the inline and the expanded frames.
  $effect(() => {
    const frame = frameRef;
    if (!frame) return;
    frame.addEventListener("wheel", gestures.handleWheel, { passive: false });
    return () => frame.removeEventListener("wheel", gestures.handleWheel);
  });

  $effect(() => {
    const frame = modalFrameRef;
    if (!frame) return;
    frame.addEventListener("wheel", gestures.handleWheel, { passive: false });
    return () => frame.removeEventListener("wheel", gestures.handleWheel);
  });
</script>

<div class="rounded-lg bg-slate-800/40 border border-slate-700 p-4">
  <!-- Header: title + stats + the expand button only. Zoom/reset controls
       live BELOW the canvas so the header stays collapsed-card-simple. -->
  <div class="flex items-center justify-between mb-2 flex-wrap gap-2">
    <h3 class="text-sm font-semibold text-slate-200">Coil Preview</h3>
    <div class="flex items-center gap-2">
      <span class="text-xs text-slate-400">
        {coils
          ? `${uniquePhases.length} phase${uniquePhases.length === 1 ? "" : "s"} · ${g.renderedLayerCount} layer${g.renderedLayerCount === 1 ? "" : "s"} · ${coils.phases.length} coil group${coils.phases.length === 1 ? "" : "s"}`
          : "no coils yet"}
      </span>
      <button
        type="button"
        class="px-2 py-0.5 text-xs rounded bg-slate-800 border border-slate-700 text-slate-300 hover:text-emerald-300 hover:border-emerald-600 transition-colors"
        aria-label="Expand coil preview"
        aria-expanded={expanded}
        onclick={() => (expanded = true)}>⤢</button
      >
    </div>
  </div>

  <!-- Frame div owns the ARIA label (the canvas is the paint surface; data-*
       attributes on the canvas carry the introspection counters). All pan/
       pinch/scroll-zoom handling is delegated to the gesture utility. -->
  <div
    bind:this={frameRef}
    class="relative w-full touch-none select-none {gestures.isPanning
      ? 'cursor-grabbing'
      : 'cursor-grab'}"
    role="img"
    aria-label="Coil preview"
    style="aspect-ratio: {W} / {H};"
    onpointerdown={gestures.handlePointerDown}
    onpointermove={gestures.handlePointerMove}
    onpointerup={gestures.handlePointerEnd}
    onpointercancel={gestures.handlePointerEnd}
    onlostpointercapture={gestures.handleLostPointerCapture}
    ontouchstart={gestures.handleTouchStart}
    ontouchmove={gestures.handleTouchMove}
    ontouchend={gestures.handleTouchEnd}
    ontouchcancel={gestures.handleTouchEnd}
    ondblclick={onInlineDoubleClick}
  >
    <!-- data-segments / data-vias / data-pole-pitch / data-band-widths /
         data-pole-regions / data-revision are written imperatively by the
         renderer (canvas.dataset in coilPreviewCanvas.ts), NOT bound
         reactively — see the counter comment there. -->
    <canvas bind:this={canvasRef} class="block h-full w-full"></canvas>
  </div>

  <!-- Zoom + reset view, pinned to the bottom-right of the preview. -->
  <div class="mt-2">
    <CoilPreviewControls
      zoomLabel={gestures.zoomLabel}
      canZoomIn={gestures.canZoomIn}
      canZoomOut={gestures.canZoomOut}
      onZoomIn={gestures.zoomIn}
      onZoomOut={gestures.zoomOut}
      onResetZoom={gestures.zoomReset}
      onResetView={gestures.resetView}
    />
  </div>

  {#if coils && coils.phases.length > 0}
    <p class="mt-2 text-xs text-slate-500">
      <span class="text-slate-600"
        >Drag to pan. Pinch, or ctrl+scroll on a trackpad, or use the zoom
        controls below when zoomed in.</span
      >
    </p>
  {/if}

  {#if expanded}
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
                {coils
                  ? `${uniquePhases.length} phase${uniquePhases.length === 1 ? "" : "s"} · ${g.renderedLayerCount} layer${g.renderedLayerCount === 1 ? "" : "s"} · ${coils.phases.length} coil group${coils.phases.length === 1 ? "" : "s"}`
                  : "no coils yet"}
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
            {#if coils && uniquePhases.length > 0}
              <div
                class="flex items-center gap-2 flex-wrap"
                role="group"
                aria-label="Phase visibility"
              >
                {#each uniquePhases as ph (ph.idx)}
                  <label
                    class="flex items-center gap-1 text-xs select-none cursor-pointer"
                    class:text-slate-500={!isPhaseVisible(ph.idx)}
                    class:text-slate-300={isPhaseVisible(ph.idx)}
                  >
                    <input
                      type="checkbox"
                      bind:checked={phaseVisibility[ph.idx]}
                      class="accent-emerald-500"
                      aria-label={"Show phase " + ph.name}
                    />
                    <span
                      class="inline-block w-2.5 h-2.5 rounded-full"
                      style="background-color: {PHASE_COLORS[
                        ph.colorIdx % PHASE_COLORS.length
                      ]}; opacity: {isPhaseVisible(ph.idx) ? 1 : 0.35}"
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
            {#if coils && uniqueLayers.length > 0}
              <div
                class="flex items-center gap-2 flex-wrap"
                role="group"
                aria-label="Layer visibility"
              >
                {#each uniqueLayers as l (l.idx)}
                  <label
                    class="flex items-center gap-1 text-xs select-none cursor-pointer"
                    class:text-slate-500={!isLayerVisible(l.idx)}
                    class:text-slate-300={isLayerVisible(l.idx)}
                  >
                    <input
                      type="checkbox"
                      checked={isLayerVisible(l.idx)}
                      onchange={() => toggleLayer(l.idx)}
                      class="accent-emerald-500"
                      aria-label={"Show layer " + l.idx}
                    />
                    <span
                      class="inline-block w-2.5 h-2.5 rounded-full"
                      style="background-color: #94a3b8; opacity: {isLayerVisible(
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
                bind:checked={showVias}
                class="accent-emerald-500"
                aria-label="Show vias"
              />
              <span
                class="inline-block w-2.5 h-2.5 rounded-full"
                style="background-color: #fbbf24; opacity: {showVias ? 1 : 0.35}"
              ></span>
              <span>Vias</span>
            </label>
            <!-- One-section toggle -->
            <label
              class="flex items-center gap-1.5 text-xs text-slate-300 select-none cursor-pointer"
            >
              <input
                type="checkbox"
                bind:checked={oneSection}
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
                  bind:checked={showPolePitch}
                  class="accent-emerald-500"
                  aria-label="Show pole-pitch dimension ruler"
                />
                <span
                  class="inline-block w-2.5 h-2.5 rounded-full"
                  style="background-color: #a5b4fc; opacity: {showPolePitch
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
                  bind:checked={showBandWidths}
                  class="accent-emerald-500"
                  aria-label="Show band-width diagnostics"
                />
                <span
                  class="inline-block w-2.5 h-2.5 rounded-full"
                  style="background-color: #34d399; opacity: {showBandWidths
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
                    bind:checked={showPoleRegions}
                    class="accent-rose-500"
                    aria-label="Show pole regions"
                  />
                  <span
                    class="inline-block w-2.5 h-2.5 rounded-full"
                    style="background: linear-gradient(90deg, #f87171 50%, #60a5fa 50%); opacity: {showPoleRegions
                      ? 1
                      : 0.35}"
                  ></span>
                  <span>Pole regions</span>
                </label>
                <select
                  aria-label="Pole regions phase"
                  class="rounded border border-slate-700 bg-slate-800 px-1.5 py-0.5 text-xs text-slate-200 disabled:opacity-40 disabled:cursor-not-allowed focus:outline-none focus:border-emerald-600"
                  bind:value={poleRegionPhase}
                  disabled={!showPoleRegions}
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
                class="px-2 py-0.5 text-xs rounded border transition-colors {measureMode
                  ? 'bg-pink-500/15 border-pink-500 text-pink-300'
                  : 'bg-slate-800 border-slate-700 text-slate-300 hover:text-pink-300 hover:border-pink-600'}"
                aria-pressed={measureMode}
                aria-label="Toggle measure tool"
                onclick={() => {
                  measureMode = !measureMode;
                  if (!measureMode) {
                    measureP1 = null;
                    measureP2 = null;
                    measureCursor = null;
                  }
                }}
              >Measure</button>
              {#if measureMode}
                <button
                  type="button"
                  class="px-2 py-0.5 text-xs rounded bg-slate-800 border border-slate-700 text-slate-300 hover:text-rose-300 hover:border-rose-600 transition-colors"
                  aria-label="Clear measurement"
                  onclick={() => {
                    measureP1 = null;
                    measureP2 = null;
                    measureCursor = null;
                  }}
                >Reset</button>
              {/if}
            </div>
            {#if measureMode}
              <span class="text-xs text-slate-400" role="status" aria-live="polite">
                {#if measureP1}
                  {#if measureP2}
                    {formatMetresMm(computeMeasureRuler(measureP1, measureP2).mm / 1000)} — click again to clear
                  {:else}
                    {#if measureCursor}
                      {formatMetresMm(computeMeasureRuler(measureP1, measureCursor).mm / 1000)} — click to lock
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
              : measureMode
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
            pattern-owned).{#if oneSection}
              <span class="text-amber-300"
                >Showing only the first {oneSectionConductorCount} conductors (one
                repeating section).</span
              >{/if} Drag to pan; pinch or use the zoom controls below.
          </p>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  {/if}
</div>
