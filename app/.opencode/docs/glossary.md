# PCB Linear Motor Reference Glossary

Technical domain terminology, physical definitions, and governing kinematic equations for printed circuit board (PCB) linear electromagnetic motors, structured for embedded software, motor control algorithms, and AI coding agent ingestion.

---

## Core System Components

- **Stator (Primary / Forcer)**: The stationary PCB assembly containing etched or multi-layer conductive coil trace patterns. Electrically energized across defined electrical phases to modulate magnetic flux fields and drive linear motion.
- **Mover (Secondary / Rotor)**: The mobile assembly containing permanent magnets (or reactive material) positioned parallel to the stator. Referred to as a _rotor_ in rotational variations.
- **Coil**: The complete, continuous conductive trace loop (spiral, wave, or multi-layer planar pattern) belonging to a single phase circuit. A coil consists of **active legs** (conductive traces passing directly through the magnetic field to generate Lorentz force) and **end turns** (inactive trace connections outside the active magnetic zone that complete the loop). In overlapping multi-layer planar layouts, one phase coil spans multiple PCB layers; a `(layer, net)` record is a **coil group** (phase-layer portion), not a whole coil.
- **Slot**: A single spatial track position or layout grid channel allocated on the PCB to house an active conductor leg. _Key Distinction_: A slot contains only a single active leg (a forward or return conductor segment) of a coil, **not the full coil winding**. In overlapping or multi-layer planar coils, individual phase coils cross over each other, distributing their active legs into distinct slots across different PCB layers.
- **Slot Width**: The width of the track space housing **one active leg**. For a diagonal trace at angle $\theta$ to the travel axis, a single-leg slot's along-travel width is $w_t / \sin\theta$. _Warning_: Do **not** use "slot width" for the electrical period $P_e$, for the slot pitch $\tau_s$, or for the full phase-band width — those are distinct quantities (see Disambiguation).
- **Strand**: One of several parallel serpentine conductive paths per phase on the same PCB layer (stacked across the board width) used to increase copper cross-section. Distinct from a _winding_: a winding (coil) is one complete conductive loop, not a parallel path count.
- **Net**: The ECAD connectivity label (e.g. KiCad net `/A`) assigned to routed geometry. One net per phase; **phase** is the motor concept, **net** is the layout artifact. Prefer "phase" in motor-design context and "net" in KiCad/layout context.
- **Phase**: An independent, electrically isolated circuit winding group (e.g., Phase A, Phase B, Phase C in a 3-phase system) excited in coordinated sequence by the motor driver. Phase trace resistance $R_{phase}$ is governed by:

$$R_{phase} = \rho \cdot \frac{L_{trace}}{A_{trace}}$$

where $\rho$ is the electrical resistivity of copper, $L_{trace}$ is total trace path length, and $A_{trace} = \text{width} \times \text{thickness}$ is the cross-sectional area.

---

## Spatial & Geometric Parameters

- **Coil Pitch ($\tau_c$)**: Center-to-center physical distance between the active forward leg and active return leg of the same coil loop. Maximum electromagnetic coupling efficiency occurs when coil pitch equals pole pitch ($\tau_c = \tau_p$).
- **Phase Band**: The full conductor bundle of one coil side — all $N$ parallel strands of one phase on one layer, including the spacing between them. Distinct from a slot (which houses a single leg). Related quantities:
  - **Phase-band width**: the along-travel x-extent of the bundle, $w_{band} = \dfrac{N \cdot w_t + (N-1)\cdot s}{\sin\theta}$, where $w_t$ is trace width, $s$ trace spacing, $\theta$ the trace angle to the travel axis.
  - **Phase-band pitch (ideal)**: $\tau_{band} = \tau_p / \text{phases}$ — centerline distance between adjacent phase bands. This is the spacing at ratio 1.0; the ideal phase-band pitch feeds the top-down width limit $w_{band,max} = \tau_p/\text{phases} - g_{phase}$.
  - **Phase clearance ($g_{phase}$)**: minimum gap reserved between adjacent phase bands.
- **Slot Pitch ($\tau_s$)**: Spatial distance between the centerlines of consecutive adjacent conductor slots along the stator track trajectory:

$$\tau_s = \frac{L_{stator}}{N_{slots}}$$

where $L_{stator}$ is the total track length and $N_{slots}$ is the total number of slots. _Disambiguation_: $\tau_s = \tau_p / \text{phases}$ (the ideal phase-band pitch) holds only for uniform 1-slot-per-pole-per-phase windings. For braided slotless patterns the effective leg pitch is the **interleave step** $\tau_p / (\text{phases} \cdot N_{strands})$.

- **Pole Pitch ($\tau_p$)**: Center-to-center physical distance between two consecutive opposite magnetic poles (North to South) on the mover magnet array. Serves as the fundamental conversion constant between mechanical displacement ($x$) and electrical angle ($\theta_e$):

$$\theta_e = \pi \cdot \frac{x}{\tau_p}$$

The relationship between linear velocity ($v$) and electrical excitation frequency ($f_e$) is given by:

$$f_e = \frac{v}{2 \tau_p}$$

_Forward-looking_: $f_e$ is not yet derived from velocity anywhere in the toolchain; the only frequency input is the skin-depth drive frequency (see SPEC.md, "Out of Scope (Forward-Looking)").

- **Electrical Period ($P_e$)**: Length of one full 360° electrical cycle along the travel axis:

$$P_e = 2 \tau_p$$

Equivalent forms of the electrical angle: $\theta_e = 2\pi \cdot x / P_e = \pi \cdot x / \tau_p$. _Warning_: $P_e$ is **not** a slot dimension — it is twice the pole pitch (the historical "slot width" UI label was a misnomer and has been renamed Electrical Pitch).
- **Mover Span (Magnet Array Span)**: Total travel-axis extent of the mover's magnet array: $N_{magnets} \cdot \tau_p$ measured center-of-first-magnet to one pitch past the last magnet. The physical end-to-end span is one inter-magnet gap shorter. _Do not_ call this "coil span" — a coil is a stator trace loop.
- **Magnet Pitch (mechanical)**: Center-to-center spacing of adjacent magnets, $W_m + W_{gap}$. For the alternating (non-Halbach) array, adjacent magnets are consecutive opposite poles, so the magnet pitch **equals the pole pitch** $\tau_p$.
- **Pole Fill Factor ($k_{fill}$)**: Fraction of the pole pitch occupied by magnet width: $k_{fill} = W_m / \tau_p$, with **inter-magnet gap** $W_{gap} = \tau_p - W_m$. The magnet spans $180° \cdot k_{fill}$ of electrical angle.
- **Active Area (Copper Region)**: The stator track length populated by active conductor legs ($L_{stator}$ in the equations above). The **routing domain** EQUALS the active area: $L_{routing} = L_{active}$ — the braid's end turns are part of the routed pattern and there is no end padding (kata hrd8, 2026-09-02: the padding offset feature was removed). Never refer to the whole active area as a "slot".
- **Vernier Spacing Ratio & Rest Offset**: A winding layout may compress the phase-band pitch by a spacing ratio $r \in (0, 1]$ (applied pitch $= r \cdot \tau_p/\text{phases}$). The **rest offset** is the unresolvable remainder $\left(\frac{\tau_p}{\text{phases}}\right)(1 - r)$ between coil centers and pole centers; zero at $r = 1$.

- **Air Gap ($g$)**: Physical mechanical clearance between the surface of the stationary PCB stator and the moving magnet array. Magnetic flux density ($B$) decays exponentially across the air gap:

$$B(g) \approx B_0 \cdot e^{-\frac{\pi g}{\tau_p}}$$

where $B_0$ is the surface flux density at the magnets.

---

## Construction Topologies

- **Slotted (Iron-Core)**: Topologies where PCB coils are backed by or integrated around ferromagnetic (iron) cores or teeth to concentrate magnetic flux. Maximizes force density but introduces magnetic attraction and detent forces.
- **Slotless (Ironless)**: Topologies where coils are placed on non-magnetic backing substrates. Eliminates detent forces, yielding zero magnetic cogging and smooth motion across all velocity profiles.
- **Coreless (Ironless)**: PCB motor architecture constructed entirely without a ferromagnetic substrate (e.g., standard FR4 or polyimide flexible layers). Equivalent to a slotless configuration, eliminating magnetic cogging, hysteresis losses, and attraction forces between stator and mover.

---

## Kinematics & Physical Dynamics

- **Thrust (Linear Force, $F$)**: Output force generated parallel to the direction of motion, measured in Newtons (N). Linear machines produce **thrust**; "torque" applies only to rotary variants. Derived from the Lorentz force law and motor force constant ($K_f$):

$$F = N \cdot B \cdot I \cdot L_{active} \cdot \sin(\theta_e) = K_f \cdot I$$

where $N$ is the number of active conductors cutting the field per phase (per slot in slotted machines), $B$ is flux density, $I$ is phase current, $L_{active}$ is active trace length within the magnetic field, and $\theta_e$ is the electrical phase angle.

_Forward-looking_: $K_f$ as a first-class output is not implemented — force is evaluated from the Lorentz law per coil and no $K_f$ result is exposed (see SPEC.md, "Out of Scope (Forward-Looking)").

- **Back EMF ($V_{emf}$)**: Counter-electromotive force voltage induced across the stator coils by the moving magnetic field:

$$V_{emf} = K_e \cdot v$$

where $K_e$ is the back-EMF constant (in $\text{V}/(\text{m/s})$) and $v$ is linear velocity. In SI units, $K_e \approx K_f$.

_Forward-looking_: $V_{emf}$ and $K_e$ are not implemented; the simulation crate produces no back-EMF outputs (see SPEC.md, "Out of Scope (Forward-Looking)").

- **Detent Force (Cogging Force)**: Passive magnetic attraction between mover magnets and ferromagnetic structures in slotted stators when de-energized. Measured as positional force ripple; zero in coreless/slotless topologies. "Cogging" is an accepted alias; prefer "detent" in user-facing labels.
- **Thrust Ripple**: Periodic variation of the commanded thrust along travel caused by commutation and winding geometry (spatial period on the order of the phase-band pitch). Distinct from detent force: thrust ripple exists when energized; detent force exists when de-energized.
- **Stable Rest Position (Equilibrium)**: A mover position where the net force at fixed excitation is zero with restoring slope. With the baseline excitation ($I_A = +I$, $I_B = 0$, $I_C = -I$), stable rest positions of the array center recur every electrical period $P_e$ at a fixed **rest phase** $\varphi$: $x \equiv \varphi \pmod{P_e}$.
- **Travel Envelope**: The span of valid mover positions: the span-aware flush clamp that keeps the mover inside the copper active area — centre ∈ [span/2, $L_{active}$ − span/2] (Mover Span span = N·τ_p). The input **travel** is the free-travel range the array sweeps (center-to-center), equal to the active-area length minus the mover span — realized EXACTLY by the flush endpoints (kata 5c7r). The endpoints are mechanical limits, NOT stable rest positions; the stable rests (x ≡ φ (mod P_e)) are reported via `rest_phase_m` and marked on the holding-force chart.
- **Holding Force**: The normalized per-phase force profile at fixed excitation as a function of position, $F(x) \propto -\sin\!\left(2\pi (x - \varphi)/P_e\right)$ per phase; phase-A zeros mark stable rest positions. Used to visualize equilibria, not absolute thrust.
- **Backlash**: Positional play or mechanical slack experienced during motion reversal. In direct-drive PCB linear motors, mechanical backlash is effectively zero.
- **Thermal Resistance ($R_{th}$) & Power Dissipation**: Thermal impedance of the PCB substrate (measured in °C/W) defining continuous power dissipation limits ($P_{loss}$) and temperature rise ($\Delta T$):

$$P_{loss} = I_{rms}^2 \cdot R_{phase}$$

$$\Delta T = P_{loss} \cdot R_{th}$$

---

## Drive Modes & Control Operations

- **Commutation**: Algorithmic timing and switching of stator coil phase currents based on real-time mover position to maintain an optimal 90° electrical angle ($\theta_e = \frac{\pi}{2}$) between the stator field and mover field for maximum vector force. For a balanced 3-phase system:

$$I_A = I_{peak} \sin(\theta_e)$$

$$I_B = I_{peak} \sin\left(\theta_e - \frac{2\pi}{3}\right)$$

$$I_C = I_{peak} \sin\left(\theta_e - \frac{4\pi}{3}\right)$$

_General per-coil offset law_: a coil displaced spatially by $\Delta x$ from the reference coil carries an electrical phase shift of $\pi \cdot \Delta x / \tau_p$. Coils spaced one phase-band pitch apart ($\Delta x = \tau_p/\text{phases}$) therefore run a $\pi \cdot \tau_{band}/\tau_p$ offset — $60°$ for the default 3-phase 1:1 layout. The classic $120°$ balanced law above corresponds to coils spaced $2\tau_p/3$. Whether the per-phase current is $\sin$ or $\cos$ of $\theta_e$ is a reference-frame (d-axis alignment) convention, not a physics difference.
- **FOC (Field-Oriented Control)**: Sinusoidal commutation scheme that regulates the phase-current vector orthogonal to the rotor field (the q-axis) to maximize thrust per ampere; the concrete realization of the commutation law above.

- **Full Step**: Drive mode operating at maximum alternating phase currents, advancing the mover by one full pole pitch ($\tau_p$) per control state change. _Forward-looking: not implemented (see SPEC.md, "Out of Scope")._
- **Half Step**: Interleaved drive mode alternating between single-phase and dual-phase energization to double spatial resolution ($\frac{\tau_p}{2}$) per step cycle. _Forward-looking: not implemented (see SPEC.md, "Out of Scope")._
- **Microstepping**: Pulse-width modulation (PWM) drive technique using continuous sine/cosine current vectors to sub-divide full steps into micro-increments, maximizing positional resolution and suppressing resonance. Microstep resolution displacement ($\Delta x$) is defined by:

$$\Delta x = \frac{\tau_p}{N_{microsteps}}$$

where $N_{microsteps}$ is the division factor per electrical cycle. _Forward-looking_: step-mode drive math and displacement readouts are not implemented (see SPEC.md, "Out of Scope (Forward-Looking)").

_Drive-mode vs fixed-excitation note_: step modes re-commutate the phase currents as the mover advances (full step re-anchors every $\tau_p$). The travel envelope and holding-force charts instead model **fixed excitation** (one constant current vector), whose stable rest positions recur every $P_e = 2\tau_p$. Both views are correct; do not mix their displacement figures.

---

## Disambiguation: Slot vs Phase Band vs Pole Region

These three layout quantities are frequently conflated. One name, one meaning:

| Term | Spatial extent | Contains | Typical size (3-phase, $\tau_p$ = 6 mm, 2 strands) |
|---|---|---|---|
| **Slot** | One track channel | One active leg (forward **or** return trace) | ~0.15 mm single-leg width |
| **Phase band** | One coil side of one phase per layer | All $N$ parallel strands of that coil side | ~0.4–2 mm width, 2 mm pitch |
| **Pole region** | One pole-pitch interval for one phase | The pattern's leg zones between pole boundaries | $\tau_p$ = 6 mm wide |
| **Electrical period** | One full 360° cycle | Two alternating poles (N+S) | $P_e = 2\tau_p$ = 12 mm |
| **Copper active area** | The whole energized track length | All phases, all slots | $L_{active}$ (tens to hundreds of mm) |

Rules of thumb:
- Never label the electrical period $P_e$, the slot pitch $\tau_s$, or the phase-band width as "slot width".
- Never call the mover's $N_{magnets} \cdot \tau_p$ extent a "coil span" — coils are stator copper.
- "Torque" only in rotary variants (rotor, radial/axial-flux machines); the linear quantity is **thrust**.
- "Winding" means a complete conductive loop; parallel paths per phase are **strands**.
- In coreless/slotless braided constructions there are no physical slots; slot quantities are an **equivalent leg-pitch model** of the trace layout.

## Rotary Variants (Out of Scope for Linear Terms)

- **Rotor**: The rotating counterpart of the mover in rotary machines.
- **Radial-flux / Axial-flux**: Rotary machine geometries classified by the air-gap flux orientation relative to the rotation axis; the desktop UI currently targets the linear (flat) architecture only.
