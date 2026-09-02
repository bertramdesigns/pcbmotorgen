import { describe, expect, it } from "vitest";
import { fetchTravelEnvelope } from "./ipc";
import { ConfigStore } from "./stores/config.svelte";
import { MotionStore } from "./stores/motion.svelte";

describe("envelope consumption chain", () => {
  // PRODUCT REFERENCE PINS — if the slider endpoints move, this test fails.
  it("mock envelope drives MotionStore bounds in the track frame", async () => {
    const config = new ConfigStore();
    const ipc = config.toIpc();
    const env = await fetchTravelEnvelope(ipc);
    // Flush, span-aware convention (kata 5c7r; defaults: active 147, N=12,
    // τ_p=6 → P_e=12; the copper active area is the whole track [0, 147] —
    // no padding, kata hrd8): span = 72 mm → flush limits [36, 111] mm.
    // The array edges sit exactly on the copper bounds at the endpoints
    // (strip 0–72 mm at min, 75–147 mm at max) and the 75 mm sweep equals
    // the configured travel EXACTLY. rest_phase 10 mm still marks the
    // stable rests for the force-chart zeros.
    expect(env.min_position_m).toBeCloseTo(0.036, 10);
    expect(env.max_position_m).toBeCloseTo(0.111, 10);

    const motion = new MotionStore(config);
    // Geometric fallback = the same flush clamp (the backend envelope adds
    // only the rest phase; the limits agree by construction under kata 5c7r).
    expect(motion.moverMinMm).toBeCloseTo(36, 3);
    expect(motion.moverMaxMm).toBeCloseTo(111, 3);
    motion.setEnvelope(env);
    expect(motion.moverMinMm).toBeCloseTo(36, 3);
    expect(motion.moverMaxMm).toBeCloseTo(111, 3);
    expect(motion.restPhaseMm).toBeCloseTo(10, 3);
    expect(motion.electricalPeriodMm).toBeCloseTo(12, 3);

    // MIN endpoint: strip edges at centre ± span/2 = 0 … 72 mm — leading
    // edge EXACTLY on the copper start (flush, kata 5c7r).
    motion.commit(motion.moverMinMm);
    expect(motion.stripStartMm).toBeCloseTo(0, 6);
    expect(motion.stripEndMm).toBeCloseTo(72, 6);

    // MAX endpoint: strip edges at 75 … 147 mm — trailing edge EXACTLY on
    // the copper end.
    motion.commit(motion.moverMaxMm);
    expect(motion.stripStartMm).toBeCloseTo(75, 6);
    expect(motion.stripEndMm).toBeCloseTo(147, 6);
  });

  it("rejects implausible envelopes (mm reported as metres) and keeps fallback", async () => {
    const config = new ConfigStore();
    const motion = new MotionStore(config);
    const geometricMin = motion.moverMinMm;
    // The regression from the measured-track unit bug: a 203 554 mm track
    // shipped as metres → slider max ≈ 203 554 mm. Must be rejected.
    motion.setEnvelope({
      min_position_m: 0.022,
      max_position_m: 190.0,
      rest_phase_m: 0.01,
      electrical_period_m: 0.012,
    });
    expect(motion.envelope).toBeNull();
    expect(motion.moverMinMm).toBeCloseTo(geometricMin, 6);
    // A valid envelope still installs after a rejection.
    const env = await fetchTravelEnvelope(config.toIpc());
    motion.setEnvelope(env);
    expect(motion.envelope).not.toBeNull();
    // Re-pinned to the flush envelope (kata 5c7r): max = 111 mm.
    expect(motion.moverMaxMm).toBeCloseTo(111, 3);
  });
});
