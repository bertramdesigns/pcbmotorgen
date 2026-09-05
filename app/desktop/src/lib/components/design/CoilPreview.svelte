<!--
  ═══════════════════════════════════════════════════════════════════════
  Canvas coil viewer.

  Layers are drawn OVERLAID at their TRUE coordinates (a top-down view
  matching the DXF export). Per-layer show/hide toggles let you inspect 
  each copper layer alone.

  All geometry math lives in `../../previewGeometry.ts` (pure, tested,
  shared); this component only maps it onto a canvas.

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
  built-in scroll lock is therefore disabled (`preventScroll={false}`),
  so the page scroll lock behaves exactly as the e2e suite asserts.

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
    formatMarginMm,
    type Point2D,
    type PreviewConfigLike,
    type PolePitchRuler,
    type PoleRegionZone,
    type BandWidthRow,
  } from "../../previewGeometry";
  import type { WorldTransform } from "../../chart";
  import { fitWorldToView, unionBounds } from "../../chart";
  import { Dialog } from "bits-ui";
  import { CoilPreviewGestures } from "../../utils/coilPreviewGestures.svelte";
  import CoilPreviewControls from "./CoilPreviewControls.svelte";
  import MoverPositionControls from "./MoverPositionControls.svelte";
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
  // these virtual px so it matches the SVG markup exactly.
  const W = 760;
  const H = 260;
  const PAD = 30;

  const PHASE_COLORS = ["#10b981", "#3b82f6", "#f59e0b", "#ec4899", "#8b5cf6"];

  // Pole-region zone fills. Alternating red/blue by `pole_index` parity,
  // translucent so the winding traces stay readable above them.
  const POLE_REGION_EVEN_FILL = "rgba(248, 113, 113, 0.18)"; // red-400
  const POLE_REGION_ODD_FILL = "rgba(96, 165, 250, 0.18)"; // blue-400

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

  /** One-section filter for band rows: show only rows anchored inside the
   *  currently kept (first-N-active) conductor x-window for their phase. */
  function bandRowInOnePeriod(row: BandWidthRow): boolean {
    if (!oneSection) return true;
    const segs = visibleSegments.get(row.phaseIdx * 1000 + row.layer);
    if (!segs || segs.length === 0) return false;
    let minX = Infinity;
    let maxX = -Infinity;
    for (const seg of segs) {
      if (!seg.is_active) continue;
      minX = Math.min(minX, seg.start[0], seg.end[0]);
      maxX = Math.max(maxX, seg.start[0], seg.end[0]);
    }
    if (!Number.isFinite(minX)) return false;
    return row.anchorX >= minX && row.anchorX <= maxX;
  }

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
  // Canvas rendering.
  //
  // Draw ops happen in two coordinate spaces:
  //  1. world metres (y-up) under `scale(s, -s)` for the schematic itself;
  //     screen-constant strokes are `screenPx / s` wide so they don't
  //     fatten when zoomed (the canvas equivalent of non-scaling-stroke).
  //  2. virtual 760×260 csspx under `setTransform(k, …)` for the legend and
  //     disclaimer overlays that must not move with the world.
  // -------------------------------------------------------------------
  // Introspection counters. These are NOT `$state`: drawInto() runs inside
  // an `$effect`, and writing reactive state from an effect reschedules the
  // effect (Svelte retries the flush ~1000×), i.e. a full repaint storm on
  // load. Plain locals + imperative `canvas.dataset` writes give e2e/debug
  // hooks with zero reactivity feedback. `drawRevision` is a single shared
  // counter across the inline and modal canvases; `drawnSegments`/`drawnVias`
  // are per-draw locals written to whatever canvas was passed in.
  let drawRevision = 0;

  function drawInto(frame: HTMLDivElement, canvas: HTMLCanvasElement) {
    if (frame.clientWidth <= 0) return;
    drawRevision += 1;
    let drawnSegments = 0;
    let drawnVias = 0;
    let drawnBandWidths = 0;
    let drawnPoleRegions = 0;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const k = (frame.clientWidth / W) * dpr; // virtual px → device px
    ctx.setTransform(k, 0, 0, k, 0, 0);
    ctx.clearRect(0, 0, W, H);
    // Page panel behind the (same-coloured) board rect.
    ctx.fillStyle = "#0f172a";
    ctx.fillRect(0, 0, W, H);

    if (!coils) {
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.font = "14px sans-serif";
      ctx.fillStyle = "#64748b";
      ctx.fillText("Awaiting coil generation…", W / 2, H / 2);
      drawnSegments = 0;
      drawnVias = 0;
      canvas.dataset.segments = "0";
      canvas.dataset.vias = "0";
      canvas.dataset.polePitch = "0";
      canvas.dataset.bandWidths = "0";
      canvas.dataset.poleRegions = "0";
      canvas.dataset.revision = String(drawRevision);
      return;
    }

    // World metres, y-up (scale(s, -s) flips y).
    const wt = worldTransform;
    const s = wt.s;
    // World geometry uses a y-up transform. Canvas text would be upside down
    // under `scale(s, -s)`, so dimension captions are painted in the virtual
    // screen space after converting their world anchor explicitly.
    const worldToVirtual = (x: number, y: number): [number, number] => [
      wt.tx + gestures.panX + x * s,
      wt.ty + gestures.panY - y * s,
    ];
    const drawWorldCaption = (
      text: string,
      x: number,
      y: number,
      color: string,
      align: CanvasTextAlign = "center",
    ): void => {
      const [vx, vy] = worldToVirtual(x, y);
      ctx.save();
      ctx.setTransform(k, 0, 0, k, 0, 0);
      ctx.globalAlpha = 0.95;
      ctx.fillStyle = color;
      ctx.font = "9px sans-serif";
      ctx.textAlign = align;
      ctx.textBaseline = "alphabetic";
      ctx.fillText(text, vx, vy);
      ctx.restore();
    };
    ctx.translate(wt.tx + gestures.panX, wt.ty + gestures.panY);
    ctx.scale(s, -s);

    let segments = 0;
    let vias = 0;

    // Dark PCB panel: fill + slate stroke, 1 screen px.
    ctx.fillStyle = "#0f172a";
    ctx.strokeStyle = "#334155";
    ctx.lineWidth = 1 / s;
    ctx.setLineDash([]);
    ctx.beginPath();
    if (typeof ctx.roundRect === "function") {
      ctx.roundRect(
        g.boardRect.x,
        g.boardRect.y,
        g.boardRect.w,
        g.boardRect.h,
        0.0015,
      );
    } else {
      // roundRect is not available in every engine — fall back to a plain rect.
      ctx.rect(g.boardRect.x, g.boardRect.y, g.boardRect.w, g.boardRect.h);
    }
    ctx.fill();
    ctx.stroke();

    // Active-copper band (always on): the copper active area — the full
    // routing domain the traces route across (the braid's end turns are
    // part of the pattern). The mover travels over this whole span.
    {
      const ax = 0;
      const aw = config.active_area_length_mm / 1000;
      if (aw > 0 && ax + aw <= g.boardRect.x + g.boardRect.w) {
        const ay = g.boardRect.y;
        const ah = g.boardRect.h;
        ctx.fillStyle = "rgba(52, 211, 153, 0.07)";
        ctx.fillRect(ax, ay, aw, ah);
        ctx.strokeStyle = "rgba(52, 211, 153, 0.55)";
        ctx.lineWidth = 1 / s;
        ctx.setLineDash([0.0012, 0.0012]);
        ctx.strokeRect(ax, ay, aw, ah);
        ctx.setLineDash([]);
      }
    }

    // Pole-region zones (pattern-owned phase/pole boundaries). Painted here,
    // BEHIND the traces, so the translucent alternating red/blue bands tint
    // the board without hiding the winding. Bands fill the board's y extent
    // and the sidecar's authoritative x boundaries — never the magnet config.
    if (showPoleRegions && visiblePoleRegionZones.length > 0) {
      const zoneY = g.boardRect.y;
      const zoneH = g.boardRect.h;
      for (const zone of visiblePoleRegionZones) {
        ctx.fillStyle =
          zone.polarity > 0 ? POLE_REGION_EVEN_FILL : POLE_REGION_ODD_FILL;
        ctx.fillRect(zone.x0, zoneY, zone.x1 - zone.x0, zoneH);
        drawnPoleRegions += 1;
      }
    }

    // Per-phase schematic layers, overlaid at their true coordinates.
    for (const ph of coils.phases) {
      if (!isPhaseVisible(ph.phase_idx) || !isLayerVisible(ph.layer_idx))
        continue;
      const layerOpacity = Math.max(0.35, 1 - ph.layer_idx * 0.15);
      const color = PHASE_COLORS[ph.phase_idx % PHASE_COLORS.length];
      const segs =
        visibleSegments.get(ph.phase_idx * 1000 + ph.layer_idx) ?? ph.segments;
      const arcs =
        visibleArcs.get(ph.phase_idx * 1000 + ph.layer_idx) ??
        ph.corner_arcs ??
        [];

      // a. Active conductors — thick, solid.
      ctx.strokeStyle = color;
      ctx.lineWidth = 2.4 / s;
      ctx.setLineDash([]);
      ctx.globalAlpha = layerOpacity;
      ctx.beginPath();
      for (const seg of segs) {
        if (!seg.is_active) continue;
        ctx.moveTo(seg.start[0], seg.start[1]);
        ctx.lineTo(seg.end[0], seg.end[1]);
        segments += 1;
      }
      ctx.stroke();

      // b. End-turns — thin, dashed.
      ctx.strokeStyle = color;
      ctx.lineWidth = 1 / s;
      ctx.setLineDash([3 / s, 2 / s]);
      ctx.globalAlpha = layerOpacity * 0.6;
      ctx.beginPath();
      for (const seg of segs) {
        if (seg.is_active) continue;
        ctx.moveTo(seg.start[0], seg.start[1]);
        ctx.lineTo(seg.end[0], seg.end[1]);
        segments += 1;
      }
      ctx.stroke();
      ctx.setLineDash([]);

      // c. Inter-layer vias — amber fill, dark ring. Hidden while `showVias`
      //    is off (the vias counter stays 0 → data-vias reports 0).
      if (showVias) {
        ctx.fillStyle = "#fbbf24";
        ctx.strokeStyle = "#0f172a";
        ctx.lineWidth = 0.5 / s;
        ctx.globalAlpha = layerOpacity;
        for (const [vx, vy] of ph.via_positions ?? []) {
          ctx.beginPath();
          ctx.arc(vx, vy, 0.00035, 0, Math.PI * 2);
          ctx.fill();
          ctx.stroke();
          vias += 1;
        }
        ctx.setLineDash([]);
      }

      // d. Corner arcs — quadratic Béziers, dashed unless active.
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.6 / s;
      for (const arc of arcs) {
        ctx.setLineDash(arc.is_active ? [] : [3 / s, 2 / s]);
        ctx.globalAlpha = layerOpacity * (arc.is_active ? 1 : 0.6);
        ctx.beginPath();
        ctx.moveTo(arc.start[0], arc.start[1]);
        ctx.quadraticCurveTo(arc.mid[0], arc.mid[1], arc.end[0], arc.end[1]);
        ctx.stroke();
      }
    }

    // Magnet array overlay along the top edge of the fitted region. The bars
    // track the shared mover position (shifted by `motion.offsetFromRestMm`),
    // so the strip slides over the fixed stator zones as the slider moves.
    for (const mag of visibleMagnets) {
      ctx.globalAlpha = 0.7;
      ctx.fillStyle = mag.pole > 0 ? "#f97316" : "#3b82f6";
      // Paint the configured solid width exactly. A fixed 0.5 mm inset here
      // adds an unmodelled gap and moves the visible pole centre away from the
      // leg-zone anchor.
      ctx.fillRect(mag.x, g.magnetTop, Math.max(mag.w, 0.0005), 0.003);
    }
    ctx.globalAlpha = 1;
    ctx.setLineDash([]);

    // -----------------------------------------------------------------
    // routing_dimensions overlays — still in world metres, under the same
    // transform, so they pan/zoom WITH the schematic. Dimension strokes are
    // screen-constant; captions are converted to upright virtual-screen text.
    // -----------------------------------------------------------------
    // a. Pole-pitch / repeat-period dimension ruler (sidecar, no own
    //    origin → aligned to the first magnet centre + one pitch).
    if (showPolePitch && poleRuler) {
      const r = poleRuler;
      const tick = 0.0012;
      const labelY = r.y + tick + 0.0004;
      ctx.globalAlpha = 0.95;
      ctx.setLineDash([]);
      ctx.strokeStyle = "#a5b4fc"; // indigo-300: ruler line
      ctx.fillStyle = "#cbd5e1";
      ctx.lineWidth = 1.4 / s;
      // Double-ended baseline between the two magnet centres.
      ctx.beginPath();
      ctx.moveTo(r.x1, r.y);
      ctx.lineTo(r.x2, r.y);
      // End ticks (+y side; the magnet strip sits below the ruler).
      ctx.moveTo(r.x1, r.y);
      ctx.lineTo(r.x1, r.y + tick);
      ctx.moveTo(r.x2, r.y);
      ctx.lineTo(r.x2, r.y + tick);
      ctx.stroke();
      // Centre markers at each end.
      ctx.fillStyle = "#a5b4fc";
      for (const x of [r.x1, r.x2]) {
        ctx.beginPath();
        ctx.arc(x, r.y, 0.00035, 0, Math.PI * 2);
        ctx.fill();
      }
      // Caption (mm), honestly labelled. Paint it upright in virtual space;
      // the world group is y-flipped for the schematic geometry.
      drawWorldCaption(
        `${r.label} ${formatMetresMm(r.pitchM)}`,
        (r.x1 + r.x2) / 2,
        labelY,
        "#cbd5e1",
      );
    }

    // b. Band-width diagnostics — one bracket per matched (layer, net).
    if (showBandWidths) {
      const endTick = 0.0005;
      const labelRaise = endTick + 0.0005;
      for (const row of bandRows) {
        // Honor phase/layer visibility and the one-section window.
        if (!isPhaseVisible(row.phaseIdx) || !isLayerVisible(row.layer))
          continue;
        if (!bandRowInOnePeriod(row)) continue;
        const halfM = Math.max(row.bandM / 2, 0.0004);
        // Negative margin → red over-budget; ok margin → green; no top-down
        // limit known → amber/grey.
        const strokeColor =
          row.status === "over-budget"
            ? "#f87171"
            : row.status === "ok"
              ? "#34d399"
              : "#fbbf24";
        const maxHalfM =
          row.maxM !== null && row.maxM > 0
            ? Math.max(row.maxM / 2, 0.0004)
            : null;
        ctx.globalAlpha = 0.9;
        // Optional top-down maximum: a dashed neutral reference bracket.
        if (maxHalfM !== null) {
          ctx.setLineDash([3 / s, 2 / s]);
          ctx.strokeStyle = "#94a3b8";
          ctx.lineWidth = 0.8 / s;
          ctx.beginPath();
          ctx.moveTo(row.anchorX - maxHalfM, row.y);
          ctx.lineTo(row.anchorX + maxHalfM, row.y);
          ctx.stroke();
        }

        ctx.setLineDash([]);
        ctx.strokeStyle = strokeColor;
        ctx.fillStyle = strokeColor;
        ctx.lineWidth = 1.2 / s;
        // Effective-width bracket + short perpendicular end ticks.
        ctx.beginPath();
        ctx.moveTo(row.anchorX - halfM, row.y);
        ctx.lineTo(row.anchorX + halfM, row.y);
        ctx.moveTo(row.anchorX - halfM, row.y);
        ctx.lineTo(row.anchorX - halfM, row.y + endTick);
        ctx.moveTo(row.anchorX + halfM, row.y);
        ctx.lineTo(row.anchorX + halfM, row.y + endTick);
        ctx.stroke();

        // Compact one-line label above the bracket:
        //   L0 A 2.00 mm / max 4.00 mm · Δ +2.00 mm
        const limitText =
          row.maxM === null ? "max —" : `max ${formatMetresMm(row.maxM)}`;
        const marginText =
          row.marginM === null ? "" : ` · Δ ${formatMarginMm(row.marginM)}`;
        drawWorldCaption(
          `L${row.layer} ${row.phaseName} ${formatMetresMm(row.bandM)} / ${limitText}${marginText}`,
          row.anchorX,
          row.y + labelRaise,
          strokeColor,
        );
        drawnBandWidths += 1;
      }
      ctx.globalAlpha = 1;
    }

    // c. Lightbox measure ruler (two-click dimension tool, lightbox only).
    //    Pink to stay distinct from the indigo pole-pitch ruler. The live
    //    preview (start point + cursor) is dashed; the locked dimension is
    //    solid with perpendicular end ticks. Draws only while expanded.
    if (expanded && measureMode && measureP1) {
      const end = measureP2 ?? measureCursor;
      if (end) {
        const ruler = computeMeasureRuler(measureP1, end);
        const placing = !measureP2;
        const tick = 0.0012;
        const dx = ruler.p2.x - ruler.p1.x;
        const dy = ruler.p2.y - ruler.p1.y;
        const len = Math.max(Math.hypot(dx, dy), 1e-9);
        const nx = -dy / len;
        const ny = dx / len;
        ctx.globalAlpha = placing ? 0.75 : 0.95;
        ctx.setLineDash(placing ? [4 / s, 3 / s] : []);
        ctx.strokeStyle = "#f472b6"; // pink-400: user measure ruler
        ctx.fillStyle = "#f9a8d4";
        ctx.lineWidth = 1.4 / s;
        ctx.beginPath();
        ctx.moveTo(ruler.p1.x, ruler.p1.y);
        ctx.lineTo(ruler.p2.x, ruler.p2.y);
        // Perpendicular end ticks.
        ctx.moveTo(ruler.p1.x, ruler.p1.y);
        ctx.lineTo(ruler.p1.x + nx * tick, ruler.p1.y + ny * tick);
        ctx.moveTo(ruler.p2.x, ruler.p2.y);
        ctx.lineTo(ruler.p2.x + nx * tick, ruler.p2.y + ny * tick);
        ctx.stroke();
        ctx.setLineDash([]);
        drawWorldCaption(
          formatMetresMm(ruler.mm / 1000),
          ruler.label.x,
          ruler.label.y,
          "#f9a8d4",
        );
        ctx.globalAlpha = 1;
      }
    }

    // -----------------------------------------------------------------
    // Overlays — back in virtual px, so they stay put under pan/zoom.
    // -----------------------------------------------------------------
    ctx.setTransform(k, 0, 0, k, 0, 0);

    // Legend (bottom-left).
    ctx.textAlign = "left";
    ctx.textBaseline = "alphabetic";
    ctx.font = "11px sans-serif";
    ctx.fillStyle = "#94a3b8";
    ctx.strokeStyle = "#94a3b8";
    ctx.lineWidth = 2.4;
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.moveTo(0, H - 8);
    ctx.lineTo(20, H - 8);
    ctx.stroke();
    ctx.fillText("active leg", 26, H - 4);
    ctx.lineWidth = 1;
    ctx.setLineDash([3, 2]);
    ctx.beginPath();
    ctx.moveTo(170, H - 8);
    ctx.lineTo(190, H - 8);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillText("end-turn", 196, H - 4);

    // Schematic disclaimer (bottom-right).
    ctx.textAlign = "right";
    ctx.font = "9px sans-serif";
    ctx.fillStyle = "#64748b";
    ctx.fillText(
      "Schematic — layers overlaid at true coordinates. Toggle per layer to inspect.",
      W - 8,
      H - 4,
    );

    drawnSegments = segments;
    drawnVias = vias;
    canvas.dataset.segments = String(drawnSegments);
    canvas.dataset.vias = String(drawnVias);
    canvas.dataset.polePitch = String(showPolePitch && poleRuler ? 1 : 0);
    canvas.dataset.bandWidths = String(drawnBandWidths);
    canvas.dataset.poleRegions = String(drawnPoleRegions);
    canvas.dataset.revision = String(drawRevision);
  }

  // Redraw whenever any input to the schematic changes. Draws the ACTIVE
  // canvas only — the inline pair while collapsed, the modal pair while
  // expanded. All reads below happen synchronously so Svelte auto-tracks
  // them (including the ones that `drawInto()` performs on the reactive
  // proxies). Runs even without coils so the "Awaiting coil generation…"
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
    drawInto(frame, canvas);
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
         data-pole-regions / data-revision are written imperatively in
         drawInto() (canvas.dataset), NOT bound reactively — see the counter
         comment above. -->
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
