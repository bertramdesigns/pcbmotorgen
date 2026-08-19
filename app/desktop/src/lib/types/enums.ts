/**
 * pcbmotorgen — frontend domain enums.
 *
 * Mirror the wire values of the Rust serde DTOs. All physical quantities
 * on the wire are SI (metres, Tesla, Amperes, Ohms, Watts, Newtons); the
 * UI store keeps human-readable mm values and converts at the invoke
 * boundary.
 */

/** NdFeB magnetization arrangement along the travel axis. */
export type MagnetArrangement =
  | "Alternating"
  | "AlternatingBackIron"
  | "Halbach"
  | "HalbachBackIron";

/** Coil slot pitch expressed as a vernier ratio of the pole pitch. */
export type SpacingRatio = "1:1" | "4:5" | "5:6";

/** Comutation strategy used by the force sweep. */
export type CommutationMode = "max_torque" | "phase_a_only";

/** Mover bearing type (drives the friction budget). */
export type BearingType = "ball" | "ptfe" | "plastic";