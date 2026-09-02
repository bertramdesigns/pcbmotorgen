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
 * continuous: a coreless motor has no detent/cogging force. The RANGE
 * endpoints come from the backend travel envelope — the span-aware FLUSH
 * limits of the copper active area (kata 5c7r, mirroring the Rust
 * `travel_envelope_over_slots`): centre ∈ [span/2, active_area_length_m −
 * span/2] with the glossary "Mover Span" span = N·τ_p, so the array edges
 * sit exactly on the copper bounds at both endpoints and the swept range
 * equals the configured free travel EXACTLY. The endpoints are MECHANICAL
 * LIMITS, NOT stable rest positions — the rests (x ≡ φ (mod P_e)) are
 * reported by the envelope's rest_phase_m/electrical_period_m and marked on
 * the holding-force chart; the mover may hold position between rests. The
 * pre-xb16 edge rule (leading edge ≥ track start at min, trailing edge ≤
 * track end at max) is superseded by the flush clamp — the clamp bounds the
 * CENTRE, not the array edges. Until an envelope arrives the store falls
 * back to the geometric "array flush inside the copper" range below.
 */

import type { ConfigStore } from "./config.svelte";
import type { TravelEnvelopeDto } from "../types";

export class MotionStore {
  /** Raw mover-centre input (mm) before clamping. */
  positionMm = $state(60);

  /** Backend travel envelope (SI, metres); null until fetched. */
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
  // This IS the same flush clamp the backend computes (kata 5c7r):
  // centre ∈ [span/2, active_area_length − span/2] — array flush inside
  // the copper active area — so fallback and backend envelope agree on the
  // limits by construction; only the rest phase (φ, N-dependent) arrives
  // with the envelope, which the client approximates via fallbackRestPhaseMm
  // until then.
  private geometricMinMm = $derived.by(() => this.moverSpanMm / 2);
  private geometricMaxMm = $derived.by(() =>
    Math.max(
      this.geometricMinMm,
      this.config.active_area_length_mm - this.moverSpanMm / 2,
    ),
  );

  /**
   * Leftmost allowed mover centre: the span-aware FLUSH limit of the copper
   * active area (kata 5c7r) — the array's left edge sits exactly on the
   * copper bound. A mechanical limit, not a stable rest position.
   */
  moverMinMm = $derived(
    this.envelope ? this.envelope.min_position_m * 1000 : this.geometricMinMm,
  );
  /**
   * Rightmost allowed mover centre: the span-aware FLUSH limit of the copper
   * active area (kata 5c7r) — the array's right edge sits exactly on the
   * copper bound. A mechanical limit, not a stable rest position.
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
  /** Strip shift from the leftmost flush limit (mm): 0 at moverMinMm. */
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
