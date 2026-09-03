//! IO (input/output) result elements: connector/IC pads and terminal fanout
//! traces.
//!
//! These types extend the [`RoutingResult`](crate::model::RoutingResult) wire
//! contract **additively** — both `RoutingResult` fields carrying them are
//! serde-defaulted, so payloads from patterns that emit no IO (and legacy
//! payloads) deserialize unchanged and `FORMAT_VERSION` stays put (see the
//! version policy in `docs/API.md` §15).
//!
//! This module is DEFINITIONS ONLY. A routing pattern declares the pads it
//! needs and the fanout traces connecting coil terminals to them; the host,
//! validator, and export crate only transport, validate, and emit them.
//! Host-side IO fanout *generation* (kata xa0f) lives in the generate
//! facade (`generate::io_fanout`) and appends these elements after pattern
//! generation, before validation.
//!
//! - [`IoPad`] — one connector/IC pad: position, pad-stack dimensions, net,
//!   copper layer(s), and pad kind (SMD / THT / board-edge).
//! - [`IoTrace`] — one fanout trace, a distinct element family from
//!   [`RouteSegment`](crate::model::RouteSegment) so later DFM checks can
//!   treat IO routing differently (it is never a force-producing conductor).
//!
//! Sizing authority: pad dimensions remain governed by the DFM rules
//! (`DesignRules`, downstream in `pcbmotorgen-dfm` since kata 0rgs) —
//! patterns read sizes from there (e.g. `DesignRules::io_tht_pad_diameter_mm`)
//! or size explicitly; downstream writers only carry the declared sizes
//! through and never decide them.
//!
//! Units are millimetres; x = travel axis, y = across board width (matching
//! the rest of the crate).

use serde::{Deserialize, Serialize};

use crate::model::{Layer, Net, Point};

/// Copper footprint of a pad [mm]: `x` along the travel axis, `y` across the
/// board width. A circular pad sets `x == y` (the diameter).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PadSize {
    pub x: f64,
    pub y: f64,
}

impl PadSize {
    /// True when the pad is circular (equal x/y extents within `tol`).
    pub fn is_round(&self, tol: f64) -> bool {
        (self.x - self.y).abs() <= tol
    }
}

/// The physical kind of an [`IoPad`]. Maps onto KiCad's `PadType`
/// (`PT_SMD` / `PT_PTH` / `PT_EDGE_CONNECTOR`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoPadKind {
    /// Surface-mounted pad (no drill).
    #[default]
    Smd,
    /// Plated through-hole pad (`drill_mm` is required).
    Tht,
    /// Castellated / board-edge connector pad — surface copper on the board
    /// outline (no drill).
    BoardEdge,
}

/// One connector/IC pad declared by a routing pattern for IO routing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoPad {
    /// Pad centre [mm], inside the routing domain (the active area).
    pub position: Point,
    /// Pad copper size [mm] — the pattern derives this from the DFM rules;
    /// see the [module docs](self).
    pub size: PadSize,
    /// Plated drill diameter [mm]. Required for [`IoPadKind::Tht`], rejected
    /// for surface pads (SMD / board-edge).
    #[serde(default)]
    pub drill_mm: Option<f64>,
    /// Copper layers the pad occupies, as zero-based board-stack indices.
    /// Empty means "the exporter's default set for the pad kind" (all copper
    /// layers for THT, the top layer for SMD / board-edge).
    #[serde(default)]
    pub layers: Vec<Layer>,
    /// Physical pad kind. Defaults to [`IoPadKind::Smd`].
    #[serde(default)]
    pub kind: IoPadKind,
    /// Phase/net label the pad belongs to (e.g. `"A"`).
    pub net: Net,
    /// Optional pad number / pin label carried through to KiCad (e.g. `"1"`).
    #[serde(default)]
    pub number: Option<String>,
}

/// What an [`IoTrace`] connects.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoTraceRole {
    /// Coil terminal → IO pad fanout.
    #[default]
    Fanout,
    /// IO pad → board-edge exit tail.
    Tail,
}

/// One IO fanout trace — a routed connection from a coil terminal to an IO
/// pad (or a tail from a pad to the board edge).
///
/// Deliberately a distinct element family from
/// [`RouteSegment`](crate::model::RouteSegment): an IO trace is never a
/// force-producing conductor, so DFM checks and consumers can filter or
/// re-rule it without inspecting `is_active` flags.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IoTrace {
    pub start: Point,
    pub end: Point,
    pub layer: Layer,
    pub net: Net,
    /// What the trace connects. Defaults to [`IoTraceRole::Fanout`].
    #[serde(default)]
    pub role: IoTraceRole,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_size_round_detects_circles() {
        let round = PadSize { x: 0.6, y: 0.6 };
        assert!(round.is_round(0.0));
        // A 1 nm (1e-6 mm) mismatch is "round" at 1e-5 tolerance, not at 1e-7.
        let hair_off = PadSize { x: 0.6, y: 0.6 + 1e-6 };
        assert!(hair_off.is_round(1e-5));
        assert!(!hair_off.is_round(1e-7));
        let rect = PadSize { x: 1.0, y: 0.6 };
        assert!(!rect.is_round(1e-9));
    }

    #[test]
    fn io_pad_kind_defaults_to_smd_and_round_trips() {
        // Absent `kind` in JSON defaults to Smd (serde field default).
        let json = r#"{"position": {"x": 0.0, "y": 0.0}, "size": {"x": 0.6, "y": 0.6}, "net": "A"}"#;
        let pad: IoPad = serde_json::from_str(json).expect("valid pad JSON");
        assert_eq!(pad.kind, IoPadKind::Smd);
        assert_eq!(pad.drill_mm, None);
        assert_eq!(pad.number, None);
        assert!(pad.layers.is_empty());

        let round: IoPad = serde_json::from_str(
            r#"{"position": {"x": 1.0, "y": 2.0}, "size": {"x": 0.4, "y": 0.4},
                "drill_mm": 0.2, "layers": [0, 1], "kind": "tht", "net": "B", "number": "3"}"#,
        )
        .expect("valid THT pad JSON");
        assert_eq!(round.kind, IoPadKind::Tht);
        assert_eq!(round.drill_mm, Some(0.2));
        assert_eq!(round.number.as_deref(), Some("3"));
        let back: IoPad =
            serde_json::from_str(&serde_json::to_string(&round).expect("serialize")).expect("round-trip");
        assert_eq!(back, round);
    }

    #[test]
    fn io_trace_role_defaults_to_fanout_and_round_trips() {
        let json = r#"{"start": {"x": 0.0, "y": 0.0}, "end": {"x": 5.0, "y": 0.0},
                       "layer": 1, "net": "A"}"#;
        let trace: IoTrace = serde_json::from_str(json).expect("valid trace JSON");
        assert_eq!(trace.role, IoTraceRole::Fanout);

        let tail = IoTrace {
            start: Point::new(0.0, 0.0),
            end: Point::new(5.0, 0.0),
            layer: 1,
            net: "A".into(),
            role: IoTraceRole::Tail,
        };
        let json = serde_json::to_string(&tail).expect("serialize");
        assert!(json.contains(r#""role":"tail""#));
        let back: IoTrace = serde_json::from_str(&json).expect("round-trip");
        assert_eq!(back, tail);
    }
}
