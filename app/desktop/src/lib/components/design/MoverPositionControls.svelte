<script lang="ts">
  /**
   * MoverPositionControls.svelte — shared mover-position controls for the
   * MotionStore: position number field and the travel slider, plus the live
   * readout. Motion input is continuous — a coreless motor has no
   * detent/cogging force. The slider endpoints come from the backend travel
   * envelope (the simulation crate's `travel_envelope_over_slots`, kata
   * 5c7r): the array edges sit exactly on the copper active-area bounds at
   * min and max, and the endpoints are limits, not stable rest positions —
   * the rests (spaced one electrical period P_e) are marked by the
   * force-chart zeros below, and the mover may hold position between them.
   * While only the placeholder envelope is active (backend unavailable /
   * not yet fetched) the endpoints are a fixed reference pin and the
   * store's `envelopeWarning` is rendered below — never silent (kata ab30).
   *
   * Used by the Design reflection (TravelDiagram) and the CoilPreview
   * lightbox so both screens expose identical position controls on the same
   * shared store.
   */
  import type { ConfigStore } from "../../stores/config.svelte";
  import type { MotionStore } from "../../stores/motion.svelte";
  import { stripBoundsDomainMm } from "../../previewGeometry";
  import NumberField from "../ui/NumberField.svelte";
  import HoldingForceChart from "./HoldingForceChart.svelte";

  let { config, motion }: { config: ConfigStore; motion: MotionStore } =
    $props();

  // Strip extent in the DOMAIN (routing) frame — exactly what the coil
  // canvas overlay and the iso view draw, so the printed numbers always
  // match the picture.
  let bounds = $derived(stripBoundsDomainMm(config, motion));
  const travelRangeMm = $derived(motion.moverMaxMm - motion.moverMinMm);
</script>

<div class="mt-3" aria-label="Mover position">
  <div class="flex items-center gap-2">
    <label
      for="mover-position-value"
      class="text-xs text-slate-400 whitespace-nowrap"
    >
      Position:
    </label>
    <NumberField
      id="mover-position-value"
      min={motion.moverMinMm}
      max={motion.moverMaxMm}
      step={0.1}
      value={motion.clampedPositionMm}
      ariaLabel="Mover position (mm)"
      class="w-24 rounded px-1.5 py-0.5 text-right font-mono text-emerald-300"
      onCommit={(value) => motion.commit(value)}
    />
    <span class="text-xs text-slate-400">mm</span>
  </div>

  <input
    type="range"
    min={motion.moverMinMm}
    max={motion.moverMaxMm}
    step="any"
    value={motion.clampedPositionMm}
    aria-label="Mover position slider (mm)"
    class="mt-2 w-full accent-emerald-500"
    oninput={(e) =>
      motion.commit(Number((e.currentTarget as HTMLInputElement).value))}
  />

  <!-- Normalized per-phase holding force vs position (3 waves at 120° for
       a 3-phase motor); phase-A zeros mark the stable rest positions. -->
  <HoldingForceChart {config} {motion} />

  <div class="mt-1 text-[10px] text-slate-500" aria-live="polite">
    Position: {motion.clampedPositionMm.toFixed(1)} mm · Mover extent:
    <span class="text-slate-300">{bounds.startMm.toFixed(1)} - {bounds.endMm.toFixed(1)} mm</span>
    · Offset from rest: {motion.offsetFromRestMm.toFixed(1)} / {travelRangeMm.toFixed(1)} mm
  </div>

  {#if motion.envelopeWarning}
    <!-- Placeholder travel envelope in effect (kata ab30): the bounds are a
         fixed reference pin, NOT backend physics — say so, loudly. -->
    <p class="mt-1 text-[10px] text-amber-400" role="status">
      {motion.envelopeWarning.message}
    </p>
  {/if}
</div>