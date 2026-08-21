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
    // Coil-capture convention (defaults: padding 30, active 147, P_e 12):
    // min = 30 + 2/3·12 = 38 mm, max = 177 − 3/4·12 = 168 mm; track-frame
    // phase (30+10) mod 12 = 4 mm so force-chart zeros land on rests.
    expect(env.min_position_m).toBeCloseTo(0.038, 10);
    expect(env.max_position_m).toBeCloseTo(0.168, 10);

    const motion = new MotionStore(config);
    // Geometric fallback: array flush inside the routing domain
    // (padding 30 + half span 36 → min; padding + active − half → max).
    expect(motion.moverMinMm).toBeCloseTo(66, 3);
    expect(motion.moverMaxMm).toBeCloseTo(141, 3);
    motion.setEnvelope(env);
    expect(motion.moverMinMm).toBeCloseTo(38, 3);
    expect(motion.moverMaxMm).toBeCloseTo(168, 3);
    expect(motion.restPhaseMm).toBeCloseTo(4, 3);
    expect(motion.electricalPeriodMm).toBeCloseTo(12, 3);

    // MIN endpoint: strip edges at centre ± span/2 = 2 … 74 mm.
    motion.commit(motion.moverMinMm);
    expect(motion.stripStartMm).toBeCloseTo(2, 6);
    expect(motion.stripEndMm).toBeCloseTo(74, 6);

    // MAX endpoint: strip edges at 132 … 204 mm.
    motion.commit(motion.moverMaxMm);
    expect(motion.stripStartMm).toBeCloseTo(132, 6);
    expect(motion.stripEndMm).toBeCloseTo(204, 6);
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
    expect(motion.moverMaxMm).toBeCloseTo(168, 3);
  });
});
