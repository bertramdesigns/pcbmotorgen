/**
 * pcbmotorgen — frontend domain enums.
 *
 * Mirror the wire values of the Rust serde DTOs. All physical quantities
 * on the wire are SI (metres, Tesla, Amperes, Ohms, Watts, Newtons); the
 * UI store keeps human-readable mm values and converts at the invoke
 * boundary.
 */

/** Commutation strategy used by the force sweep. */
export type CommutationMode = "max_thrust" | "phase_a_only";
