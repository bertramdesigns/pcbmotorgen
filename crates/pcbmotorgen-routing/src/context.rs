//! Flat snapshot of board dimensions + DFM rules + phase count handed to every
//! routing pattern.
//!
//! Patterns must not depend on the concrete physics-core `LinearMotorConfig`,
//! keeping this crate decoupled. Pattern-specific knobs ride in [`params`].
//!
//! Every length in this context is millimetres. The parent application converts
//! its SI/metre config at the routing boundary.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Everything a routing pattern may need to produce geometry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoutingContext {
    /// Physical length of the stator copper region [mm] (primary input).
    pub active_area_length_mm: f64,
    /// PCB dimension perpendicular to travel [mm].
    pub board_width_mm: f64,
    /// Number of copper layers in the stack.
    pub num_layers: u32,
    /// Number of electrical phases.
    pub phases: u32,
    /// Minimum manufacturable trace width [mm].
    pub min_trace_mm: f64,
    /// Minimum trace-to-trace clearance [mm].
    pub min_space_mm: f64,
    /// Whether the pattern declares that its conductors are continuous (end→
    /// start connect within a tolerance) so the validator checks continuity.
    pub expects_continuous: bool,
    /// Pattern-specific numeric parameters (e.g. `num_strands`, `angle_deg`).
    pub params: HashMap<String, f64>,
    /// Magnetic pole pitch [mm] (`magnet_width + magnet_gap`) of the mover the
    /// stator serves. This is the centre-to-centre distance between adjacent
    /// north and south poles (`tau_p` in the phase-band width equations).
    /// Optional so patterns that do not care about the magnet layout stay
    /// decoupled; patterns that DO use it can align their repeating unit (e.g.
    /// the braid's diamond period) to the pole pitch and regenerate geometry
    /// when the magnet pattern changes.
    #[serde(default)]
    pub magnet_pitch_mm: Option<f64>,
    /// Full span of the mover's magnet array [mm] (`magnet_count × pitch`).
    /// Together with `magnet_pitch_mm` lets a pattern derive how many repeating
    /// units cover the mover (e.g. braid periods over the magnet-array span).
    #[serde(default)]
    pub magnet_array_span_mm: Option<f64>,
    /// Legacy alias for [`RoutingContext::magnet_array_span_mm`], retained for
    /// plugin/runner JSON compatibility. Both are populated together.
    #[serde(default)]
    pub coil_span_mm: Option<f64>,
}

impl RoutingContext {
    /// Convenience: read a numeric parameter or fall back to `default`.
    pub fn param(&self, key: &str, default: f64) -> f64 {
        self.params.get(key).copied().unwrap_or(default)
    }

    /// Resolved pole pitch [mm] when the magnet layout was provided.
    pub fn magnet_pitch(&self) -> Option<f64> {
        self.magnet_pitch_mm.filter(|p| *p > 0.0)
    }

    /// Alias for [`RoutingContext::magnet_pitch`] using the motor-design term
    /// used by the handoff documentation.
    pub fn pole_pitch(&self) -> Option<f64> {
        self.magnet_pitch()
    }

    /// Resolved mover magnet-array span [mm] when the magnet layout was
    /// provided.
    pub fn magnet_array_span(&self) -> Option<f64> {
        self.magnet_array_span_mm.or(self.coil_span_mm)
    }

    /// Resolved mover magnet-array span [mm] when the magnet layout was
    /// provided.
    ///
    /// Legacy alias for [`RoutingContext::magnet_array_span`], retained for
    /// plugin/runner compatibility.
    #[deprecated(since = "0.5.0", note = "use `magnet_array_span()` instead")]
    pub fn coil_span(&self) -> Option<f64> {
        self.magnet_array_span()
    }
}
