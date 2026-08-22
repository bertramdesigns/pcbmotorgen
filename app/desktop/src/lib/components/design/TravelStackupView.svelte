<script lang="ts">
  import type { ConfigStore } from "../../stores/config.svelte";

  let { config }: { config: ConfigStore } = $props();

  // ====================================================================
  // FRONT-ON ORTHOGRAPHIC VIEW (Y–Z cross-section)
  // View rotated 90° from the side view: the SVG X-axis spans the board
  // width (Y in the config) and the SVG Y-axis is still Z+ UP. Each layer
  // is rendered as a full-width rectangle. Real mm thicknesses (modulo the
  // Z exaggeration that fits the stack inside ORTHO_H) are reported in the
  // dim-line labels on the right.
  // ====================================================================
  const ORTHO_W = 180;
  const ORTHO_H = 220;
  const ORTHO_PAD_L = 8;
  const ORTHO_PAD_R = 30; // reserved for the height-dimension labels
  const ORTHO_PAD_T = 12;
  const ORTHO_PAD_B = 16;

  let pcbThicknessMm = $derived(config.pcb_thickness_mm);
  let airGapMm = $derived(config.air_gap_mm);
  let magnetHeightMm = $derived(config.magnet_height_mm);
  let boardWidthMm = $derived(config.active_area_width_mm);
  let totalStackMm = $derived(
    pcbThicknessMm + airGapMm + magnetHeightMm,
  );

  // X-axis now spans the BOARD WIDTH (Y in the config) — was active
  // area length in the X–Z view. Z-axis is still up.
  let orthoXScale = $derived(
    (ORTHO_W - ORTHO_PAD_L - ORTHO_PAD_R) / Math.max(boardWidthMm, 1),
  );
  let orthoZScale = $derived(
    (ORTHO_H - ORTHO_PAD_T - ORTHO_PAD_B) / Math.max(totalStackMm, 0.1),
  );
  // Pixel width of the stackup rect (capped so very wide boards don't
  // overflow; the rect is left-padded to ORTHO_PAD_L regardless).
  let orthoStackPxW = $derived(boardWidthMm * orthoXScale);

  // Layer boundaries — z=0 at the BOTTOM, Z+ UP.
  let orthoBaseY = $derived(ORTHO_H - ORTHO_PAD_B);
  let orthoPcbTopY = $derived(orthoBaseY - pcbThicknessMm * orthoZScale);
  let orthoAirGapTopY = $derived(orthoPcbTopY - airGapMm * orthoZScale);
  let orthoMagnetTopY = $derived(orthoAirGapTopY - magnetHeightMm * orthoZScale);

  // X position of the height-dimension line in the orthographic view.
  let orthoDimX = $derived(ORTHO_W - 4);
</script>

<div class="min-w-0">
  <div class="text-[10px] uppercase tracking-wider text-slate-500 mb-1">
    Front view (Y–Z)
  </div>
  <svg viewBox="0 0 {ORTHO_W} {ORTHO_H}" class="w-full h-auto"
       role="img" aria-label="Front-on orthographic Y–Z cross-section of the height stack">
    <!-- PCB (bottom layer) — full width -->
    <rect x={ORTHO_PAD_L} y={orthoPcbTopY}
          width={orthoStackPxW}
          height={pcbThicknessMm * orthoZScale}
          fill="#1e293b" stroke="#475569" stroke-width="0.7" />
    <text x={ORTHO_PAD_L + 3} y={orthoBaseY - 2} class="fill-slate-300" style="font-size:7px">PCB</text>

    <!-- Air gap (light tint, only if there's room) — full width -->
    {#if airGapMm > 0 && airGapMm * orthoZScale >= 0.5}
      <rect x={ORTHO_PAD_L} y={orthoAirGapTopY}
            width={orthoStackPxW}
            height={airGapMm * orthoZScale}
            fill="rgba(100,116,139,0.15)" stroke="#475569" stroke-width="0.5" />
    {/if}

    <!-- Magnet block — solid rectangle (N/S alternation is along
         the travel direction, which is hidden in this view) -->
    <rect x={ORTHO_PAD_L} y={orthoMagnetTopY}
          width={orthoStackPxW}
          height={magnetHeightMm * orthoZScale}
          fill="#10b981" fill-opacity="0.45"
          stroke="#10b981" stroke-width="0.5" stroke-opacity="0.8" />
    <text x={ORTHO_PAD_L + orthoStackPxW / 2}
          y={orthoMagnetTopY + magnetHeightMm * orthoZScale / 2 + 2.5}
          text-anchor="middle" class="fill-emerald-200" style="font-size:7px">
      N · S
    </text>

    <!-- Stack height dimension line (right side) -->
    <g stroke="#94a3b8" stroke-width="0.5">
      <line x1={orthoDimX} y1={orthoBaseY} x2={orthoDimX} y2={orthoMagnetTopY} />
      <line x1={orthoDimX - 2} y1={orthoBaseY} x2={orthoDimX + 2} y2={orthoBaseY} />
      <line x1={orthoDimX - 2} y1={orthoPcbTopY} x2={orthoDimX + 2} y2={orthoPcbTopY} />
      <line x1={orthoDimX - 2} y1={orthoAirGapTopY} x2={orthoDimX + 2} y2={orthoAirGapTopY} />
      <line x1={orthoDimX - 2} y1={orthoMagnetTopY} x2={orthoDimX + 2} y2={orthoMagnetTopY} />
    </g>
    <!-- Layer thickness labels (only if there's room for the text) -->
    <text x={orthoDimX - 4} y={(orthoBaseY + orthoPcbTopY) / 2 + 2.5} text-anchor="end"
          class="fill-slate-300" style="font-size:7px">
      {pcbThicknessMm.toFixed(1)} mm
    </text>
    {#if airGapMm > 0 && airGapMm * orthoZScale > 7}
      <text x={orthoDimX - 4} y={(orthoPcbTopY + orthoAirGapTopY) / 2 + 2.5} text-anchor="end"
            class="fill-slate-300" style="font-size:7px">
        {airGapMm.toFixed(1)} mm
      </text>
    {/if}
    <text x={orthoDimX - 4} y={(orthoAirGapTopY + orthoMagnetTopY) / 2 + 2.5} text-anchor="end"
          class="fill-slate-300" style="font-size:7px">
      {magnetHeightMm.toFixed(1)} mm
    </text>
    <text x={orthoDimX - 4} y={orthoMagnetTopY - 3} text-anchor="end"
          class="fill-slate-200 font-semibold" style="font-size:7px">
      Total: {totalStackMm.toFixed(1)} mm
    </text>
  </svg>
</div>