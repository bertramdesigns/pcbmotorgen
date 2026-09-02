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
    // Nearest-snapped, span-aware convention (kata xb16; defaults: active
    // 147, N=12, τ_p=6 → P_e=12; the copper active area is the whole track
    // [0, 147] — no padding, kata hrd8): centre clamp [36, 111] mm,
    // φ_track = 10 mm lattice → min = nearest to 36 = 10 + 2·12 = 34 mm,
    // max = 10 + 8·12 = 106 mm (each ≤ P_e/2 = 6 mm from its bound), so
    // force-chart zeros land on the stable rests and the 72 mm sweep
    // approximates the 75 mm configured travel.
    expect(env.min_position_m).toBeCloseTo(0.034, 10);
    expect(env.max_position_m).toBeCloseTo(0.106, 10);

    const motion = new MotionStore(config);
    // Geometric fallback approximates the backend centre clamp (kata xb16):
    // array flush inside copper (half span 36 → min;
    // active − half → max) WITHOUT the backend lattice snap.
    expect(motion.moverMinMm).toBeCloseTo(36, 3);
    expect(motion.moverMaxMm).toBeCloseTo(111, 3);
    motion.setEnvelope(env);
    expect(motion.moverMinMm).toBeCloseTo(34, 3);
    expect(motion.moverMaxMm).toBeCloseTo(106, 3);
    expect(motion.restPhaseMm).toBeCloseTo(10, 3);
    expect(motion.electricalPeriodMm).toBeCloseTo(12, 3);

    // MIN endpoint: strip edges at centre ± span/2 = −2 … 70 mm (leading
    // edge overhangs the copper start by 2 mm — the nearest-snap deviation
    // is bounded by P_e/2; the out-hanging magnets see no conductors).
    motion.commit(motion.moverMinMm);
    expect(motion.stripStartMm).toBeCloseTo(-2, 6);
    expect(motion.stripEndMm).toBeCloseTo(70, 6);

    // MAX endpoint: strip edges at 70 … 142 mm (trailing edge 142 ≤ 147).
    motion.commit(motion.moverMaxMm);
    expect(motion.stripStartMm).toBeCloseTo(70, 6);
    expect(motion.stripEndMm).toBeCloseTo(142, 6);
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
    // Re-pinned to the nearest-snapped envelope (kata xb16): max = 106 mm.
    expect(motion.moverMaxMm).toBeCloseTo(106, 3);
  });
});
