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
    // Nearest-snapped, span-aware convention (kata xb16; defaults: padding
    // 30, active 147, N=12, τ_p=6 → P_e=12): centre clamp [66, 141] mm,
    // φ_track = 4 mm lattice → min = nearest to 66 = 4 + 5·12 = 64 mm,
    // max = 4 + 11·12 = 136 mm (each ≤ P_e/2 = 6 mm from its bound), so
    // force-chart zeros land on the stable rests and the 72 mm sweep
    // approximates the 75 mm configured travel.
    expect(env.min_position_m).toBeCloseTo(0.064, 10);
    expect(env.max_position_m).toBeCloseTo(0.136, 10);

    const motion = new MotionStore(config);
    // Geometric fallback approximates the backend centre clamp (kata xb16):
    // array flush inside copper (padding 30 + half span 36 → min;
    // padding + active − half → max) WITHOUT the backend lattice snap.
    expect(motion.moverMinMm).toBeCloseTo(66, 3);
    expect(motion.moverMaxMm).toBeCloseTo(141, 3);
    motion.setEnvelope(env);
    expect(motion.moverMinMm).toBeCloseTo(64, 3);
    expect(motion.moverMaxMm).toBeCloseTo(136, 3);
    expect(motion.restPhaseMm).toBeCloseTo(4, 3);
    expect(motion.electricalPeriodMm).toBeCloseTo(12, 3);

    // MIN endpoint: strip edges at centre ± span/2 = 28 … 100 mm (leading
    // edge overhangs the copper start by 2 mm into the end-turn padding,
    // within the documented P_e/2 nearest-snap deviation).
    motion.commit(motion.moverMinMm);
    expect(motion.stripStartMm).toBeCloseTo(28, 6);
    expect(motion.stripEndMm).toBeCloseTo(100, 6);

    // MAX endpoint: strip edges at 100 … 172 mm (trailing edge 172 ≤ 177).
    motion.commit(motion.moverMaxMm);
    expect(motion.stripStartMm).toBeCloseTo(100, 6);
    expect(motion.stripEndMm).toBeCloseTo(172, 6);
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
    // Re-pinned to the nearest-snapped envelope (kata xb16): max = 136 mm.
    expect(motion.moverMaxMm).toBeCloseTo(136, 3);
  });
});
