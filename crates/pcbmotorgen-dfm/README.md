# pcbmotorgen-dfm

Design-for-manufacturing (DFM) rules and diagnostics for pcbmotorgen —
extracted from the routing crate (kata 0rgs) so manufacturability is a strictly
downstream concern: any routing is allowed in the generator, and DFM is
checked later, downstream, as diagnostics only.

## Scope and ownership

The crate owns:

- **`DesignRules`** (`rules`) — the trace width / clearance / via sizing
  authority, including the plated IO pad sizing helper
  `io_tht_pad_diameter_mm()` (drill + 2 × annular ring) and the
  `io_fanout_options()` bridge that hands rule-derived sizes to the host IO
  fanout generator (kata xa0f). Downstream consumers
  (the KiCad writer, the DXF exporter) read sizes from this spec — they never
  decide them.
- **`check_interference` / `InterferenceViolation`** (`interference`) — DRC
  copper-clearance diagnostics over a validated `RoutingResult`:
  same-layer different-net segment clearance (`min_trace_mm + min_space_mm`),
  via-pad-to-trace clearance (`via_pad_radius + trace_width/2 +
  min_space`), and — since kata xa0f — the IO elements: `io_traces[]` join
  the same-layer clearance checks (against segments and each other, and as
  via-pad clearance targets), and `io_pads[]` are checked against
  different-net copper / other pads on the layers they declare
  (`io_pad_clearance`). Violations are **diagnostics only** — reported, never
  used to silently alter geometry.

The strict-shape validator (bounds / finite / degenerate / continuity) is wire
-contract validation and stays in `pcbmotorgen-routing` — it is not DFM.

## Dependency direction

```
pcbmotorgen-dfm  →  pcbmotorgen-routing
```

The checks need the canonical geometry model, so this crate sits **downstream**
of routing. The routing crate keeps no DFM types.

## Phase-clearance semantics (`g_phase`, context fields)

`RoutingContext.min_trace_mm`, `min_space_mm`, and `phase_clearance_mm`
(`g_phase`) are part of the routing wire contract — patterns consume them for
layout and phase-band math (routing API §10.1). They stay in the routing
crate. The application bridges the same config values into a `DesignRules`
snapshot (`LinearMotorConfig::design_rules()`); this crate reads the snapshot
for sizing and clearance diagnostics. Phase clearance is a band-budget concept
owned by the routing dimension math, not a DRC input here.

## Units

All values in millimetres; x = travel axis, y = across board width.

## Commands

```bash
cargo test -p pcbmotorgen-dfm
cargo test --workspace
```

## Reference

- Routing plugin contract: [`../pcbmotorgen-routing/docs/API.md`](../pcbmotorgen-routing/docs/API.md)
- `DesignRules`: `src/rules.rs`
- Interference checks: `src/interference.rs`
