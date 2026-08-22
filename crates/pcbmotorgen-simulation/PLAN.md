# Scope, Bugs & TODO

---

## Deferred

The following features are explicitly **out of scope for now** and tracked here to prevent accidental implementation.

| #   | Deferral                               | Notes                                                                                                                                                                                                                                                                                                                                          |
| --- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | FOC rewrite (`@pcb-motor-expert` spec) | Replace the legacy cos-FOC with a closed-form law that handles Vernier spacing ratios, phase-loss tolerance, and a refined electrical-angle definition. Marked `// TODO: FOC-rewrite-pcb-motor-expert` in `src/magnetic/force_eval/commutation.rs` and `src/magnetic/force_eval/mod.rs`. The 3-point guard stays as the live self-calibration. |
| D2  | Proper Halbach model                   | REMOVED — the magnet arrangement options (Halbach and back-iron variants) were removed from the product scope so the mover is always the plain alternating array. The old single X-polarised interleave-per-gap approximation and the method-of-images back iron live in git history. |
| D4  | FOC ripple closed-form bounds          | `test_foc_rewrite_ripple_target_1_1` / `test_foc_rewrite_ripple_target_4_5_vernier` are `#[ignore]`d pending the rewritten FOC spec.                                                                                                                                                                                                           |

---

## Known Issues (Active Bugs)

This is the consolidated, high-level bug tracker. **All currently ACTIVE bugs are
tracked in the round sections below**

| #   | Bug | Status | Notes |
| --- | --- | ------ | ----- |

---

## Wishes (Deferred / Backlog)

Improvements the user has requested but deferred to a future round.

| #   | Wish                           | Effort | Notes                                                                                                                                                                                                                                        |
| --- | ------------------------------ | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W1  | FOC error-injection test hooks | Medium | Add a `foc_variant: FocVariant` field / `phase_offset_override` to `ForceEvaluator` so the 3-point guard can be unit-tested against a 90° sin error and a wrong per-coil offset. Currently `#[ignore]`d in `src/magnetic/force_eval/mod.rs`. |

---
