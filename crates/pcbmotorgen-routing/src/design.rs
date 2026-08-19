//! Design rules (DFM) that govern trace width, clearance, and via sizing.
//!
//! The routing crate owns trace width and via size. A [`DesignRules`] snapshot
//! is applied when (a) checking interference on a [`RoutingResult`](crate::model::RoutingResult)
//! and (b) describing the geometry to downstream consumers (the KiCad writer),
//! which converts these millimetre sizes into nanometres without re-deriving them.
//!
//! All values in millimetres.

use serde::{Deserialize, Serialize};

/// Minimum manufacturable trace width [mm].
const DEFAULT_MIN_TRACE_MM: f64 = 0.127; // 5 mil
/// Minimum trace-to-trace clearance [mm].
const DEFAULT_MIN_SPACE_MM: f64 = 0.127; // 5 mil
/// Minimum via drill diameter [mm].
const DEFAULT_MIN_VIA_DRILL_MM: f64 = 0.2;
/// Minimum via annular ring width [mm].
const DEFAULT_MIN_VIA_ANNULAR_RING_MM: f64 = 0.1;

/// A snapshot of the DFM rules the routing layer applies to generated geometry.
///
/// Patterns receive the board + phase dimensions via [`RoutingContext`](crate::context::RoutingContext);
/// trace width, clearance, and via sizing live here so the routing layer is the
/// authority on those values (per the crate division). Downstream consumers
/// (e.g. the KiCad writer) read the sizes from this spec — they never decide
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DesignRules {
    /// Minimum manufacturable trace width [mm].
    pub min_trace_mm: f64,
    /// Minimum trace-to-trace clearance [mm].
    pub min_space_mm: f64,
    /// Minimum via drill diameter [mm].
    pub min_via_drill_mm: f64,
    /// Minimum via annular ring width [mm].
    pub min_via_annular_ring_mm: f64,
}

impl Default for DesignRules {
    fn default() -> Self {
        Self {
            min_trace_mm: DEFAULT_MIN_TRACE_MM,
            min_space_mm: DEFAULT_MIN_SPACE_MM,
            min_via_drill_mm: DEFAULT_MIN_VIA_DRILL_MM,
            min_via_annular_ring_mm: DEFAULT_MIN_VIA_ANNULAR_RING_MM,
        }
    }
}

impl DesignRules {
    /// Minimum via pad diameter [mm] = drill + 2 × annular ring.
    pub fn via_pad_diameter_mm(&self) -> f64 {
        self.min_via_drill_mm + 2.0 * self.min_via_annular_ring_mm
    }

    /// Minimum via pad radius [mm].
    pub fn via_pad_radius_mm(&self) -> f64 {
        self.via_pad_diameter_mm() / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_millimetres() {
        let rules = DesignRules::default();
        assert_eq!(rules.min_trace_mm, 0.127);
        assert_eq!(rules.min_space_mm, 0.127);
        assert_eq!(rules.min_via_drill_mm, 0.2);
        assert_eq!(rules.min_via_annular_ring_mm, 0.1);
        assert_eq!(rules.via_pad_diameter_mm(), 0.4);
    }
}
