# Scope, Bugs & TODO

---

## Deferred

The following features are explicitly **out of scope for now** and tracked here to prevent accidental implementation.

> ⚠️ **Radial (Rotary) mode is NOT YET IMPLEMENTED.**
> `AxialMotorConfig` exists as a design stub — it has no geometry generation, no radial coil patterns, and no torque sweep. The Linear/Radial toggle is **disabled** in the UI and clearly labelled `TODO / not-yet-implemented`. Do NOT attempt to implement radial geometry.

---

## Known Issues (Active Bugs)

This is the consolidated, high-level bug tracker. **All currently ACTIVE bugs are
tracked in the round sections below**

| #   | Bug                                                     | Status   | Notes |
| --- | ------------------------------------------------------- | -------- | ----- |
| 1   | `validate_write_preconditions` always reports 12 layers | RESOLVED | —     |
| 2   | Back iron graphics don't clear when toggling off        | RESOLVED | —     |
| 3   | Magnet preview bars are offset from the three-phase slot zones | RESOLVED | Anchor each pitch cell's right edge (solid magnet + trailing gap) on the pattern's B-phase slot centres (A1 +, B1 neutral, C1/A2 −, ...) using `routing_dimensions.pole_regions`. |

---

## Wishes (Deferred / Backlog)

Improvements the user has requested but deferred to a future round.

| #   | Wish                | Effort                  | Notes                                                                                                                                                                                                                                                                      |
| --- | ------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W1  | Backplane under PCB | Medium (future feature) | A second back-iron-like plate sits BELOW the PCB (below the bottom copper layer, below the coils) to increase the field for symmetry. The existing "above the magnets" back iron is unchanged. Requires a new config field and new image magnets in `build_image_magnets`. |

---
