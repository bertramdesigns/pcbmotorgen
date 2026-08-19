//! Canonical output shape for routing patterns.
//!
//! Every routing pattern — bundled, Rust crate plugin, or Python runner — must
//! ultimately produce one [`RoutingResult`] conforming to this strict serde
//! model. Units are millimetres, x = travel axis, y = across board width.
//!
//! # Contract versioning
//!
//! `RoutingResult.format_version` identifies the wire contract that produced
//! the document. It defaults to [`FORMAT_VERSION`] when absent for runners
//! authored against the current millimetre contract.
//! Adding fields to this model is non-breaking (additive); removing or
//! reinterpreting a field, or changing the coordinate/unit conventions, is
//! breaking and MUST bump `FORMAT_VERSION`. See `docs/routing-pattern-handoff.md`.

use serde::{Deserialize, Serialize};

/// The current routing handoff contract version (see `model.rs` docs).
pub const FORMAT_VERSION: u32 = 2;

fn default_format_version() -> u32 {
    FORMAT_VERSION
}

/// A 2D point in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Index into the board's copper stack `[0, num_layers)`.
pub type Layer = u32;

/// A phase net label (e.g. `"A"`, `"B"`, `"C"`). The writer prefixes `/`.
pub type Net = String;

/// One straight trace element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteSegment {
    pub start: Point,
    pub end: Point,
    pub layer: Layer,
    pub net: Net,
    /// Active force-producing conductor (`true`) vs end-turn (`false`).
    pub is_active: bool,
}

/// One arc (rounded corner / curve) — maps to KiCad's `(arc start mid end)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteCurve {
    pub start: Point,
    pub mid: Point,
    pub end: Point,
    pub layer: Layer,
    pub net: Net,
    pub is_active: bool,
}

/// One via / inter-layer connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Via {
    pub position: Point,
    pub from_layer: Layer,
    pub to_layer: Layer,
    pub net: Net,
}

/// One pole-pitch region assigned by a routing pattern to a phase.
///
/// These boundaries are pattern-owned: different routing algorithms may use
/// completely different geometry to determine where a phase's pole region
/// starts and ends. Coordinates are in millimetres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoleRegion {
    /// Phase/net label associated with this region.
    pub phase: Net,
    /// Zero-based pole-pitch index within the phase.
    pub pole_index: u32,
    pub start: Point,
    pub end: Point,
}

/// The complete geometry a routing pattern produces for a board.
///
/// All elements carry their own `layer` and `net` — the pattern owns layer
/// semantics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingResult {
    /// Wire-contract version (see module docs). Defaults to [`FORMAT_VERSION`]
    /// when the field is absent in serialized JSON.
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    pub segments: Vec<RouteSegment>,
    pub curves: Vec<RouteCurve>,
    pub vias: Vec<Via>,
    /// Pattern-defined pole regions, one entry per phase and pole pitch.
    #[serde(default)]
    pub pole_regions: Vec<PoleRegion>,
}

impl Default for RoutingResult {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            segments: Vec::new(),
            curves: Vec::new(),
            vias: Vec::new(),
            pole_regions: Vec::new(),
        }
    }
}

impl RoutingResult {
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() && self.curves.is_empty() && self.vias.is_empty()
    }

    /// Total number of geometric elements.
    pub fn element_count(&self) -> usize {
        self.segments.len() + self.curves.len() + self.vias.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_result_is_format_version_2() {
        let r = RoutingResult::default();
        assert_eq!(r.format_version, FORMAT_VERSION);
        assert!(r.is_empty());
    }

    #[test]
    fn absent_format_version_defaults_to_current() {
        // Python runners may omit `format_version`; the current contract is
        // selected by the serde default.
        let json = r#"{"segments": [], "curves": [], "vias": []}"#;
        let r: RoutingResult = serde_json::from_str(json).expect("valid result JSON");
        assert_eq!(r.format_version, FORMAT_VERSION);
    }

    #[test]
    fn format_version_round_trips() {
        let r = RoutingResult::default();
        let json = serde_json::to_string(&r).expect("serialize");
        assert!(json.contains("\"format_version\":2"));
        let back: RoutingResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, r);
    }

    #[test]
    fn arc_serialization_keeps_is_active_with_default() {
        // Arc without is_active (older serialized coil) defaults to false.
        let json = r#"{"start": [0.0, 0.0], "mid": [1.0, 1.0], "end": [2.0, 0.0]}"#;
        let arc: crate::coil::CoilArc = serde_json::from_str(json).expect("valid arc");
        assert!(!arc.is_active);
    }
}
