---
name: linear-motor-magnet-design
description: Analyzes coreless 3-phase linear motor geometry and computes mover equilibrium positions and magnet array dimensions using reference specifications.
compatibility: opencode
metadata:
  domain: motion-control
  motor-type: coreless-linear-3phase
---

## Purpose

This skill provides standard workflows for analyzing, simulating, and sizing coreless 3-phase linear motor magnet systems. It relies on domain specification rules stored in project reference files.

## Reference Files

When performing calculations or writing motor control code, reference the following specification documents:

1. **Equilibrium Position Calculation:** `./reference/mover-equilibrium-calculation.md`
   - Implements Clarke transformation ($I_\alpha, I_\beta$) from phase currents ($I_A, I_B, I_C$).
   - Calculates electrical angle $\theta_e = \text{atan2}(I_\beta, I_\alpha)$ normalized to $[0, 2\pi)$.
   - Scales $\theta_e$ to physical track position $x = \theta_e \times \left(\frac{P_e}{2\pi}\right)$.

2. **Magnet Mover Array Sizing:** `./reference/magnet-mover-array-sizing.md`
   - Derives pole pitch $\tau_p = \frac{P_e}{2}$.
   - Calculates individual magnet width $W_m = \tau_p \times k_{fill}$ (default $k_{fill} = 0.75$).
   - Calculates inter-magnet gap $W_{gap} = \tau_p - W_m$ and thickness $T_m = 0.5 \times \tau_p$.
   - Determines total mover array length $L_{array} = N_{poles} \times \tau_p$.

## Execution Rules

- **Input Validation:** Ensure $N_{poles} \ge 2$ (preferably even) and fill factor $0.50 \le k_{fill} \le 0.85$. Default electrical pitch $P_e = 12.0\text{ mm}$ unless specified otherwise.
- **Singularity Handling:** Flag zero-current states ($I_A = I_B = I_C = 0$) as undefined holding states.
- **Unbalanced Current Warnings:** If $I_A + I_B + I_C \neq 0$, notify the user that non-balanced DC currents will distort the ideal spatial holding force profile.
- **Code Output:** Ensure output code modules import standard library math routines and output dimensions in millimeters ($\text{mm}$).
