# pcbmotorgen Desktop Frontend

A local-first tool for generating multi-layer PCB stator layouts for both linear and rotary (axial-flux) coreless electric motors.

---

## Primary Vision & Scope

The objective is to provide an intuitive, mathematically rigorous, and local-first toolchain for designing, simulating, and layout-drafting PCB stator motors. The toolchain is decoupled into three primary pillars:

1. **PCB Geometry Generation**: Analytical coil layout path generation.
2. **Magnetic & Multiphysics Simulation**: Force, torque, thermal, and friction modelling
3. **Drafting Integration**: Automating layout routing directly into KiCad 10 via IPC.

### Desired user outcomes

Using the tool should let the user understand the following:

- **Coil Path Generation**: Understand how PCB traces should be drawn and routed
- **Board size and mechanical travel**: Understand the motion parameters and dimensions of the design
- **Magnet size and spacing**: Understand how many magnets are needed, dimensions, and position
- **Cogging and smoothness**: Know the full-step, half-step, and other resulting characteristics of the movement
- **Estimated torque/power**: Know and approximation of the performance (strength) of the design
- **Estimated thermal and electical performance**: Know the rough thermal electrical performance of of the design

### Anti-Scope Creep Guardrails

- **One-Way Pipeline**: The workflow is stateless and strictly one-way (User Input $\to$ Optimization $\to$ KiCad IPC Write). There is no "read-from-board and edit" sync loop.

---

## Linear Geometry & Motion Model

There is **no padding offset** in the motor model: the copper coils are the
active copper area. The defined red/blue per-phase zones span the total
active pole regions, and the active copper area is exactly
$[0, \text{active\_area\_length}]$.

- **Routing domain = active area.** The routing crate lays out traces over
  exactly the active copper length (braid end turns included) — there is no
  separate PCB margin parameter.
- **Travel envelope = flush span-aware clamp.** The mover centre range is
  $[\text{span}/2,\ \text{active\_area\_length} - \text{span}/2]$ exactly:
  at the endpoints the magnet array edges sit exactly on the copper bounds,
  and the sweep equals the configured travel exactly
  ($\text{travel} = \text{active\_area\_length} - \text{span}$).
- **Endpoints are mechanical limits, not stable rests.** Stable rest
  positions come from the rest lattice ($x \equiv \varphi \pmod{P_e}$,
  reported as `rest_phase_m` + `electrical_period_m`) and drive the
  holding-force chart zeros; the mover may hold position between rests.
- **Verification tooling.** `pnpm shot:mover` (app/desktop) captures the
  running app at the slider min and max (full view + mover strip canvas)
  with the mover-extent readout pinned per endpoint.

---

## UI

The application adopts a single, highly integrated, state-contained **Dashboard**. This dashboard features:

1.  **Immediate Parameter Feedback**: Slide any variable and immediately see updated collateral calculations.
2.  **Dual-Fidelity Solvers**: Instant analytical approximations ($< 2\text{ms}$) coupled with an explicit, high-fidelity 3D Biot-Savart sweep on command.
3.  **Clean Separation of Concerns**: A high-level toggle between **Linear Motor** and **Radial (Rotary) Motor** cleanly shifts all UI labels, parameter scopes, and mathematical terminology.

---

## Crate Division

- `pcbmotorgen-routing` — traces & generation, generator plugins, trace width,
  via size, and overlap (DRC) detection (leaf, no internal deps).
- `pcbmotorgen-simulation` — all physics: B-field, Lorentz force, stackup,
  power, friction (depends on routing for coil geometry).
- `pcbmotorgen-export` — KiCad 10 IPC adapter that consumes routing's generic
  geometry model (depends on routing only). DXF export.
- `desktop` (parent, `app/desktop/src-tauri`) — master config, UI, Tauri IPC,
  orchestration; consumes the sub-crates.

---
