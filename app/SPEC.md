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

### Out of Scope (Forward-Looking)

The following glossary-normative kinematics are **documented but intentionally unimplemented** anywhere in the app or sub-crates (decision recorded 2026-09-02, kata `mspc`). They remain in the glossary as normative definitions for future work; no simulation or UI currently computes or displays them:

- **Back-EMF** ($V_{emf} = K_e \cdot v$) — no induced-voltage outputs exist.
- **Force constant** ($K_f$) as a first-class result — force is evaluated from the Lorentz law directly; $K_f$ is not exposed.
- **Electrical frequency** ($f_e = v / (2\tau_p)$) derived from velocity — `drive_frequency_hz` is a skin-depth input only.
- **Step modes** (full / half / microstep) — no step-mode drive math or displacement readouts; the travel-envelope and holding-force charts model fixed excitation only.

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
