<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";
  import { isoProject, isoCenter as computeIsoCenter, isoBoxPath, hasBackIron } from "../../geometry";

  let {
    config,
    moverPosMm,
    magnetStartMm,
    magnetEndMm,
  }: {
    config: ConfigStore;
    moverPosMm: number;
    magnetStartMm: number;
    magnetEndMm: number;
  } = $props();

  // ====================================================================
  // 3/4 ISOMETRIC VIEW — axonometric projection of the assembly (PCB
  // wireframe + magnet wireframe + optional back-iron). Z is exaggerated
  // so the thin stackup is visible. The magnet block sits at the current
  // `moverPosMm` along the travel axis.
  // ====================================================================
  const ISO_W = 220;
  const ISO_H = 220;
  const ISO_PAD = 14;
  const Z_EXAG = 10; // vertical exaggeration factor for the iso view

  let isoScale = $derived(
    (ISO_W - 2 * ISO_PAD) / Math.max(config.active_area_length_mm * 1.45, 1),
  );

  // Axonometric projection (delegates to the shared lib/geometry helper
  // with the Z-exaggerated scale baked in):
  //     sx = cx + (x + 0.45·y) · sXY
  //     sy = cy + (-z·Z_EXAG + 0.45·y) · sXY
  const project = (
    x: number, y: number, z: number, cx: number, cy: number,
  ): [number, number] => isoProject(x, y, z, cx, cy, isoScale, isoScale * Z_EXAG);

  // Center the assembly's bounding box in the iso canvas.
  let isoCenter = $derived(
    computeIsoCenter(
      {
        length: config.active_area_length_mm,
        width: config.active_area_width_mm,
        totalHeight:
          config.pcb_thickness_mm +
          config.air_gap_mm +
          config.magnet_height_mm +
          config.back_iron_thickness_mm,
      },
      ISO_W,
      ISO_H,
      project,
    ),
  );

  // Back-iron visibility predicate. Mirrors the same `has_back_iron` rune
  // found in FluxDiagram.svelte and the orthographic stackup view so both
  // views agree on when the steel back-iron should be drawn.
  let has_back_iron = $derived(
    hasBackIron(config.magnet_arrangement, config.back_iron_thickness_mm),
  );

  let isoGeom = $derived.by(() => {
    const pcbT = config.pcb_thickness_mm;
    const ag = config.air_gap_mm;
    const mh = config.magnet_height_mm;
    const bi = config.back_iron_thickness_mm;
    return {
      L: config.active_area_length_mm,
      W: config.active_area_width_mm,
      pcbT,
      ag,
      mh,
      bi,
      // Z-stack (stator at z=0, then PCB → air gap → magnet → optional back iron).
      // The magnet must sit ABOVE the PCB with the air gap in between, so the
      // magnet's bottom is at z = pcbT + ag (previously just `ag`, which made the
      // magnet wireframe overlap the PCB wireframe in the 3/4 iso view).
      pcbZTop: pcbT,
      airGapZBottom: pcbT,
      magnetZBottom: pcbT + ag,
      backIronZBottom: pcbT + ag + mh,
      // `moverPosMm` is the CENTER of the magnet array; the magnet
      // extends from `moverPosMm - coilSpan/2` to `moverPosMm + coilSpan/2`.
      magnetStartX: magnetStartMm,
      magnetEndX: magnetEndMm,
    };
  });

  // Pre-computed wireframe boxes for the iso view.
  let isoStatorBox = $derived(
    isoBoxPath(0, 0, 0, isoGeom.L, isoGeom.W, isoGeom.pcbT, isoCenter.cx, isoCenter.cy, project),
  );
  let isoMagnetBox = $derived(
    isoBoxPath(isoGeom.magnetStartX, 0, isoGeom.magnetZBottom,
      isoGeom.magnetEndX - isoGeom.magnetStartX, isoGeom.W, isoGeom.mh,
      isoCenter.cx, isoCenter.cy, project),
  );
  let isoBackIronBox = $derived(
    has_back_iron
      ? isoBoxPath(isoGeom.magnetStartX, 0, isoGeom.backIronZBottom,
          isoGeom.magnetEndX - isoGeom.magnetStartX, isoGeom.W, isoGeom.bi,
          isoCenter.cx, isoCenter.cy, project)
      : null,
  );
</script>

<div class="min-w-0">
  <div class="text-[10px] uppercase tracking-wider text-slate-500 mb-1">3/4 view</div>
  <svg viewBox="0 0 {ISO_W} {ISO_H}" class="w-full h-auto"
       role="img" aria-label="Three-quarter isometric view of the assembly">
    <!-- Stator (PCB) wireframe box -->
    <path d={isoStatorBox.d} fill="#1e293b" fill-opacity="0.35" stroke="#94a3b8" stroke-width="1" />
    <text x={isoStatorBox.corners[0][0] - 4} y={isoStatorBox.corners[0][1] + 12} text-anchor="end"
          class="fill-slate-400" style="font-size:9px">PCB</text>

    <!-- Magnet array wireframe box (positioned by the mover-position field) -->
    <path d={isoMagnetBox.d} fill="#065f46" fill-opacity="0.35" stroke="#10b981" stroke-width="1" />
    <text x={(isoMagnetBox.corners[4][0] + isoMagnetBox.corners[5][0]) / 2}
          y={isoMagnetBox.corners[4][1] - 4} text-anchor="middle"
          class="fill-emerald-300" style="font-size:9px">Magnets</text>

    <!-- Back iron wireframe (if present) -->
    {#if isoBackIronBox}
      <path d={isoBackIronBox.d} fill="#a16207" fill-opacity="0.35" stroke="#ca8a04" stroke-width="1" />
    {/if}

    <!-- Axis legend (bottom-left corner) -->
    <g style="font-size:8px" stroke-linecap="round">
      <!-- X axis: right (red) -->
      <line x1="20" y1={ISO_H - 22} x2="38" y2={ISO_H - 22} stroke="#ef4444" stroke-width="1.4" />
      <text x="40" y={ISO_H - 19} class="fill-red-300">X</text>
      <!-- Y axis: diagonal down-right (green) -->
      <line x1="20" y1={ISO_H - 22} x2="33" y2={ISO_H - 14} stroke="#22c55e" stroke-width="1.4" />
      <text x="34" y={ISO_H - 10} class="fill-green-300">Y</text>
      <!-- Z axis: up (blue) -->
      <line x1="20" y1={ISO_H - 22} x2="20" y2={ISO_H - 40} stroke="#3b82f6" stroke-width="1.4" />
      <text x="23" y={ISO_H - 40} class="fill-blue-300">Z</text>
    </g>
  </svg>
</div>