# Mover Equilibrium Position Calculation

This specification details the mathematical algorithm for computing the equilibrium position ($x$) of a magnet mover's center relative to a 3-phase linear stator cycle.

**1. Input Parameters**

- $I_A$: Phase A current ($\text{A}$)
- $I_B$: Phase B current ($\text{A}$)
- $I_C$: Phase C current ($\text{A}$)
- $P_e$: Electrical pitch / period ($\text{mm}$) = $12.0\text{ mm}$

**2. Output Parameter**

- $x$: Equilibrium center position within the electrical cycle ($\text{mm}$), bounded by $0 \le x < P_e$.

---

**3. Mathematical Model**

**Step 1: Alpha-Beta Vector Transformation (Clarke Transform)**
Decompose the 3-phase spatial vectors ($\theta_A = 0$, $\theta_B = \frac{2\pi}{3}$, $\theta_C = \frac{4\pi}{3}$) into 2D Cartesian space components ($I_\alpha, I_\beta$):

$$I_\alpha = I_A - 0.5 I_B - 0.5 I_C$$

$$I_\beta = \frac{\sqrt{3}}{2} (I_B - I_C)$$

**Step 2: Electrical Angle ($\theta_e$) Determination**
Compute the resultant magnetic field vector angle using the two-argument arctangent function to preserve quadrant orientation:

$$\theta_e = \text{atan2}(I_\beta, I_\alpha) \quad \text{(in radians, range } [-\pi, \pi]\text{)}$$

Normalize $\theta_e$ into the continuous range $[0, 2\pi)$:

$$\text{if } \theta_e < 0: \quad \theta_e = \theta_e + 2\pi$$

**Step 3: Spatial Position Mapping**
Scale the electrical angle to the physical track pitch $P_e$:

$$x = \theta_e \times \left( \frac{P_e}{2\pi} \right)$$

---

**4. Special Conditions & Edge Cases**

- **Singularity ($I_\alpha = 0$ AND $I_\beta = 0$):** Occurs when $I_A = I_B = I_C = 0$. The motor generates zero holding force; position $x$ is undefined. Output a fault or maintain the last known valid position.
- **Current Offset / Unbalance:** The algorithm automatically handles non-balanced currents, though unbalanced conditions distort the ideal sinusoidal spatial holding profile.

---

**5. Reference Implementation (Python)**

```python
import math


def calculate_magnet_position(
    i_a: float, i_b: float, i_c: float, pitch_mm: float = 12.0
) -> float:
    # Step 1: Clarke Transformation
    i_alpha = i_a - 0.5 * i_b - 0.5 * i_c
    i_beta = (math.sqrt(3) / 2.0) * (i_b - i_c)

    # Step 2: Handle Zero-Current Singularity
    if math.isclose(i_alpha, 0.0, abs_tol=1e-6) and math.isclose(
        i_beta, 0.0, abs_tol=1e-6
    ):
        raise ValueError("Undefined position: Zero net magnetic field.")

    # Step 3: Compute Electrical Angle (radians)
    theta_e = math.atan2(i_beta, i_alpha)

    # Step 4: Normalize angle to [0, 2*pi)
    if theta_e < 0:
        theta_e += 2 * math.pi

    # Step 5: Convert to physical spatial position
    x_mm = theta_e * (pitch_mm / (2 * math.pi))

    return x_mm


# Example Verification: Ia = 1.0, Ib = 0.0, Ic = -1.0
pos = calculate_magnet_position(1.0, 0.0, -1.0, 12.0)
# Returns: 1.0 mm

```
