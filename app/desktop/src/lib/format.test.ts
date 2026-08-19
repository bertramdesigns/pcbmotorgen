import { describe, expect, it } from "vitest";
import { warningClasses, warningLabel, pluralize, rippleStatus } from "./format";

describe("warningClasses", () => {
  it("returns a distinct tailwind triple per severity", () => {
    const error = warningClasses("error");
    const warning = warningClasses("warning");
    const info = warningClasses("info");
    expect(error).toContain("rose");
    expect(warning).toContain("amber");
    expect(info).toContain("sky");
    const set = new Set([error, warning, info]);
    expect(set.size).toBe(3);
  });
});

describe("warningLabel", () => {
  it("uppercases the severity", () => {
    expect(warningLabel("error")).toBe("ERROR");
    expect(warningLabel("warning")).toBe("WARNING");
    expect(warningLabel("info")).toBe("INFO");
  });
});

describe("pluralize", () => {
  it("adds s only for counts ≠ 1", () => {
    expect(pluralize(1, "line")).toBe("1 line");
    expect(pluralize(0, "line")).toBe("0 lines");
    expect(pluralize(3, "line")).toBe("3 lines");
  });
});

describe("rippleStatus", () => {
  it("classifies ripple thresholds", () => {
    expect(rippleStatus(0)).toBe("ok");
    expect(rippleStatus(4.9)).toBe("ok");
    expect(rippleStatus(5)).toBe("warn");
    expect(rippleStatus(9.9)).toBe("warn");
    expect(rippleStatus(10)).toBe("bad");
    expect(rippleStatus(null)).toBe("na");
    expect(rippleStatus(undefined)).toBe("na");
    expect(rippleStatus(Number.NaN)).toBe("na");
  });
});