/**
 * Shared mover-position state for the design reflection.
 *
 * Owned by App.svelte and passed to both the TravelDiagram (position slider /
 * iso view) and the CoilPreview (magnet strip shading), so dragging the
 * position in one place moves the magnet overlay in the other.
 *
 * The value is the CENTER of the magnet array in ABSOLUTE TRACK coordinates
 * (the routing/domain frame: the copper active area is [0, active_area_length_m],
 * no padding offset) — the same frame every preview draws in. Motion input is
 * continuous: a coreless motor has no detent/cogging force, and the slider
 * endpoints are stable rest positions spaced one electrical period P_e apart
 * (full-step commutation would re-anchor every τp). The RANGE endpoints are
 * physics-derived: they come from the backend
 * travel envelope — the first/last STABLE EQUILIBRIUM rest positions of the
 * array CENTRE under the baseline excitation (IA=+I, IB=0, IC=−I), computed
 * over the MEASURED routed track (kata xb16 spec, mirroring the Rust
 * `travel_envelope_over_slots`): the centre is clamped so the array stays
 * inside the copper active area — centre ∈ [span/2, active_area_length_m −
 * span/2] with the glossary "Mover Span" span = N·τ_p, a
 * range that WIDENS as N shrinks and has the width of the configured free
 * travel — and both endpoints are snapped to the NEAREST point of the rest
 * lattice `x ≡ φ_track (mod P_e)` (ties inward), deviating by ≤ P_e/2 per
 * endpoint so the swept range approximates the configured travel. This is
 * the glossary-normative "first/last stable rest position inside the copper
 * active area"; the pre-xb16 edge rule (leading edge ≥ track start at min,
 * trailing edge ≤ track end at max) is superseded — the clamp bounds the
 * CENTRE, not the array edges. Until an envelope arrives the store falls
 * back to the geometric "array flush inside the copper" range below.
 */

import type { ConfigStore } from "./config.svelte";
import type { TravelEnvelopeDto } from "../types";

export class MotionStore {
  /** Raw mover-centre input (mm) before clamping. */
  positionMm = $state(60);

  /** Backend equilibrium envelope (SI, metres); null until fetched. */
  envelope = $state<TravelEnvelopeDto | null>(null);

  constructor(private config: ConfigStore) { }

  moverSpanMm = $derived.by(() => this.config.mover_span_mm);

  /** Electrical period P_e (mm): one full 360° cycle = 2 pole pitches. */
  electricalPeriodMm = $derived(this.config.pole_pitch_mm * 2);

  /**
   * Rest phase φ (mm): stable rest centres satisfy x ≡ φ (mod P_e).
   * Mirrors the backend equilibrium formula for the baseline currents.
   * Used before/without an envelope.
   */
  restPhaseMm = $derived.by(() => {
    if (this.envelope) return this.envelope.rest_phase_m * 1000;
    return fallbackRestPhaseMm(this.config.magnet_count, this.electricalPeriodMm);
  });

  // --- Geometric fallback bounds ------------------------------------------
  // Approximates the backend clamp — the array flush inside the copper
  // active area [0, active_area_length] — WITHOUT the backend nearest-rest
  // lattice snap. The residual difference is unknowable here without φ
  // (only the backend envelope carries it), so these bounds are the
  // UNSNAPPED clamp range and may sit up to P_e/2 per endpoint away from
  // the true stable rests.
  private geometricMinMm = $derived.by(() => this.moverSpanMm / 2);
  private geometricMaxMm = $derived.by(() =>
    Math.max(
      this.geometricMinMm,
      this.config.active_area_length_mm - this.moverSpanMm / 2,
    ),
  );

  /**
   * Leftmost allowed mover centre: first stable equilibrium rest position
   * of the array centre inside the copper active area (lattice-snapped,
   * span-aware — kata xb16).
   */
  moverMinMm = $derived(
    this.envelope ? this.envelope.min_position_m * 1000 : this.geometricMinMm,
  );
  /**
   * Rightmost allowed mover centre: last stable equilibrium rest position
   * of the array centre inside the copper active area (lattice-snapped,
   * span-aware — kata xb16).
   */
  moverMaxMm = $derived.by(() => {
    if (!this.envelope) return this.geometricMaxMm;
    return Math.max(this.moverMinMm, this.envelope.max_position_m * 1000);
  });

  /** Clamped mover centre; all consumers read this. */
  clampedPositionMm = $derived(
    Number.isFinite(this.positionMm)
      ? Math.max(this.moverMinMm, Math.min(this.moverMaxMm, this.positionMm))
      : this.moverMinMm,
  );
  /** Strip shift from the leftmost stable centre (mm): 0 at moverMinMm. */
  offsetFromRestMm = $derived(this.clampedPositionMm - this.moverMinMm);

  /**
   * Mover strip extent (mm) in the DOMAIN frame — centred on the current
   * position, so the drawn edges always equal position ± mover_span/2 and
   * match every printed number exactly.
   */
  stripStartMm = $derived(this.clampedPositionMm - this.moverSpanMm / 2);
  stripEndMm = $derived(this.clampedPositionMm + this.moverSpanMm / 2);

  /** Commit a raw position (slider or number field) into the store. */
  commit(value: number): void {
    if (!Number.isFinite(value)) return;
    this.positionMm = value;
  }

  /**
   * Install a fetched backend envelope (SI metres → internal mm).
   *
   * Sanity-gated: positions must be plausible coordinates ON this board
   * (finite, ordered, non-negative, and no farther than the routing domain
   * plus 1% slack). A backend unit slip — e.g. millimetre geometry reported
   * as metres — would otherwise blow the slider up to ~200 000 mm and shrink
   * the CoilPreview camera to a speck; rejecting keeps the geometric
   * fallback range instead of trusting garbage.
   */
  setEnvelope(dto: TravelEnvelopeDto): void {
    const minMm = dto.min_position_m * 1000;
    const maxMm = dto.max_position_m * 1000;
    const domainMm = this.config.trace_total_length_mm;
    const plausible =
      Number.isFinite(minMm) &&
      Number.isFinite(maxMm) &&
      minMm >= 0 &&
      maxMm >= minMm &&
      maxMm <= domainMm * 1.01;
    if (!plausible) return;
    this.envelope = dto;
  }
}

/**
 * Client-side fallback of the backend rest-phase formula (mm):
 * φ = (P_e/12 + ((N−1)/2)·τ_p) mod P_e with τ_p = P_e/2.
 * Exported pure so the chart + tests share one implementation.
 */
export function fallbackRestPhaseMm(magnetCount: number, electricalPeriodMm: number): number {
  const pe = electricalPeriodMm;
  if (!(pe > 0)) return 0;
  const tau = pe / 2;
  const xPeak = pe / 12;
  return (((xPeak + ((magnetCount - 1) / 2) * tau) % pe) + pe) % pe;
}
