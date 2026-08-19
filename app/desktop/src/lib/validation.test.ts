import { describe, expect, it } from "vitest";
import { validateDesign, hasErrors, type DesignConfigInput } from "./validation";

/** Valid baseline — all checks should pass. */
function validConfig(overrides: Partial<DesignConfigInput> = {}): DesignConfigInput {
  return {
    travel_mm: 75,
    coil_span_mm: 120,
    magnet_count: 10,
    magnet_gap_mm: 2,
    magnet_cross_width_mm: 10,
    active_area_width_mm: 20,
    num_layers: 4,
    max_layers: 12,
    routing_pattern: "infinity-braid",
    windings_per_phase: 2,
    min_trace_mm: 0.127,
    min_space_mm: 0.127,
    peak_force_n: 1.0,
    target_force_n: 0.5,
    ...overrides,
  };
}

const ids = (config: DesignConfigInput) => validateDesign(config).map((f) => f.id);

describe("validateDesign", () => {
  it("returns no findings for a valid config", () => {
    expect(validateDesign(validConfig())).toEqual([]);
  });

  it("flags non-positive travel as an error", () => {
    expect(ids(validConfig({ travel_mm: 0 }))).toContain("travel-negative");
    expect(ids(validConfig({ travel_mm: -10 }))).toContain("travel-negative");
    expect(ids(validConfig({ travel_mm: Number.NaN }))).toContain("travel-negative");
  });

  it("requires an even magnet count ≥ 2", () => {
    expect(ids(validConfig({ magnet_count: 1 }))).toContain("magnet-count");
    expect(ids(validConfig({ magnet_count: 9 }))).toContain("magnet-count");
    expect(ids(validConfig({ magnet_count: 8 }))).not.toContain("magnet-count");
  });

  it("rejects negative magnet gaps", () => {
    expect(ids(validConfig({ magnet_gap_mm: -0.1 }))).toContain("magnet-gap");
  });

  it("warns when the magnet cross width exceeds the active area width", () => {
    const findings = validateDesign(validConfig({ magnet_cross_width_mm: 25 }));
    expect(findings.find((f) => f.id === "magnet-cross-width")?.level).toBe("warning");
  });

  it("requires an even layer count within the configured maximum", () => {
    expect(ids(validConfig({ num_layers: 3 }))).toContain("layer-count");
    expect(ids(validConfig({ num_layers: 6 }))).not.toContain("layer-count");
    const over = validateDesign(validConfig({ num_layers: 14 }));
    expect(over.find((f) => f.id === "layer-limit")?.level).toBe("error");
  });

  it("requires two layers for the infinity braid", () => {
    expect(ids(validConfig({ num_layers: 2, routing_pattern: "infinity-braid" }))).not.toContain(
      "infinity-layer-requirement",
    );
    expect(ids(validConfig({ num_layers: 1, routing_pattern: "infinity-braid" }))).toContain(
      "infinity-layer-requirement",
    );
  });

  it("warns when multi-strand windings cannot fit trace + clearance", () => {
    const findings = validateDesign(validConfig({ windings_per_phase: 4 }));
    // 20mm / 4 = 5mm per strand — plenty of room
    expect(findings.find((f) => f.id === "strand-clearance")).toBeUndefined();
    const tight = validateDesign(validConfig({ active_area_width_mm: 0.4, windings_per_phase: 2 }));
    expect(tight.find((f) => f.id === "strand-clearance")?.level).toBe("warning");
  });

  it("warns when peak force is below the continuous target", () => {
    const findings = validateDesign(validConfig({ peak_force_n: 0.3, target_force_n: 0.5 }));
    expect(findings.find((f) => f.id === "force-target-order")?.level).toBe("warning");
  });
});

describe("hasErrors", () => {
  it("is true only when at least one finding is error-level", () => {
    expect(hasErrors(validateDesign(validConfig({ travel_mm: 0 })))).toBe(true);
    expect(hasErrors(validateDesign(validConfig({ magnet_cross_width_mm: 25 })))).toBe(false);
  });
});