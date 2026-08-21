# Coreless Linear Motor Permanent Magnet Mover Array Sizing

This specification defines the parametric algorithm for calculating the physical dimensions of permanent magnets and the overall magnet array for a coreless 3-phase linear motor.

---

### 1. Input Parameters

- $P_e$: Stator electrical pitch ($\text{mm}$) = $12.0\text{ mm}$
- $N_{poles}$: Total number of magnetic poles on the mover ($\text{integer} \ge 2$, preferably even)
- $k_{fill}$: Magnet pole fill factor ($\text{ratio}$, dimensionless) = $0.75$ (default range: $0.67 \le k_{fill} \le 0.83$)

---

### 2. Derived Output Parameters

- $\tau_p$: Magnetic pole pitch ($\text{mm}$)
- $W_m$: Individual magnet pole width ($\text{mm}$)
- $W_{gap}$: Physical gap between adjacent magnets ($\text{mm}$)
- $L_{array}$: Total active length of the magnet array ($\text{mm}$)
- $T_m$: Minimum recommended magnet thickness ($\text{mm}$)

---

### 3. Mathematical Model & Sizing Rules

**Pole Pitch ($\tau_p$) Calculation**
A full electrical cycle ($P_e = 360^\circ$) contains two alternating magnetic poles ($180^\circ$ each):

$$\tau_p = \frac{P_e}{2}$$

**Individual Magnet Width ($W_m$)**
Magnets must not cover the full pole pitch to prevent inter-pole magnetic flux leakage and high harmonics:

$$W_m = \tau_p \times k_{fill}$$

_Setting $k_{fill} = 0.75$ ($135^\circ$ electrical) optimizes fundamental magnetic flux while attenuating unwanted spatial harmonics._

**Inter-Pole Gap ($W_{gap}$)**
Space reserved between adjacent magnet blocks along the array:

$$W_{gap} = \tau_p - W_m = \tau_p \times (1 - k_{fill})$$

**Total Array Length ($L_{array}$)**
Length spans all magnetic pole pitches from the leading edge of pole 1 to the trailing edge of pole $N_{poles}$:

$$L_{array} = N_{poles} \times \tau_p$$

**Magnet Thickness ($T_m$)**
In a coreless motor, there is no iron core to direct magnetic flux. To ensure adequate flux density ($B_z$) across the air gap without iron backing teeth, thickness scales directly with pole pitch:

$$T_m = 0.50 \times \tau_p$$

---

### 4. Constraints & Parameter Validation Rules

1. **Pole Count ($N_{poles}$):** Must be an integer where $N_{poles} \ge 2$. Even pole counts are strongly recommended to ensure zero net unbalanced magnetic normal force on the mover ends.
2. **Fill Factor ($k_{fill}$):**

- $k_{fill} < 0.60$: Drops air-gap flux density too low, reducing motor thrust constant ($K_t$).
- $k_{fill} > 0.85$: Leads to severe flux leakage directly between adjacent magnets instead of passing through the coil plane.

3. **Array Alignment:** Mover poles must be magnetized alternately through the thickness plane ($Z$-axis): $[N, S, N, S, \dots]$.

---

### 5. Parameter Summary Table ($P_e = 12\text{ mm}$, $k_{fill} = 0.75$)

| Parameter                            | Formula              | 4-Pole Mover ($N=4$) | 8-Pole Mover ($N=8$) |
| ------------------------------------ | -------------------- | -------------------- | -------------------- |
| **Pole Pitch ($\tau_p$)**            | $P_e / 2$            | $6.0\text{ mm}$      | $6.0\text{ mm}$      |
| **Magnet Width ($W_m$)**             | $\tau_p \times 0.75$ | $4.5\text{ mm}$      | $4.5\text{ mm}$      |
| **Inter-Magnet Gap ($W_{gap}$)**     | $\tau_p \times 0.25$ | $1.5\text{ mm}$      | $1.5\text{ mm}$      |
| **Magnet Thickness ($T_m$)**         | $0.50 \times \tau_p$ | $3.0\text{ mm}$      | $3.0\text{ mm}$      |
| **Total Array Length ($L_{array}$)** | $N \times \tau_p$    | $24.0\text{ mm}$     | $48.0\text{ mm}$     |

---

### 6. Reference Implementation (Python)

```python
from dataclasses import dataclass


@dataclass(frozen=True)
class MagnetDimensions:
    pole_pitch_mm: float
    magnet_width_mm: float
    inter_magnet_gap_mm: float
    magnet_thickness_mm: float
    total_array_length_mm: float
    num_poles: int


def calculate_magnet_geometry(
    pitch_e_mm: float = 12.0, num_poles: int = 4, k_fill: float = 0.75
) -> MagnetDimensions:
    # Rule 1: Validate minimum pole count
    if num_poles < 2:
        raise ValueError("Mover must have at least 2 poles.")

    # Rule 2: Validate fill factor range
    if not (0.50 <= k_fill <= 0.85):
        raise ValueError(
            "Fill factor k_fill should be between 0.50 and 0.85 for optimal flux linkage."
        )

    # Core Calculations
    tau_p = pitch_e_mm / 2.0
    w_m = round(tau_p * k_fill, 3)
    w_gap = round(tau_p - w_m, 3)
    t_m = round(tau_p * 0.50, 3)
    l_array = round(num_poles * tau_p, 3)

    return MagnetDimensions(
        pole_pitch_mm=tau_p,
        magnet_width_mm=w_m,
        inter_magnet_gap_mm=w_gap,
        magnet_thickness_mm=t_m,
        total_array_length_mm=l_array,
        num_poles=num_poles,
    )


# Example Output for a 12mm Electrical Pitch Motor
spec = calculate_magnet_geometry(pitch_e_mm=12.0, num_poles=6, k_fill=0.75)
print(spec)

```
