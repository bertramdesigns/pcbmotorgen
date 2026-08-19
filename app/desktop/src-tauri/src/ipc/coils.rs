//! IPC coil-geometry DTOs (`CoilSegmentIpc` / `PhaseCoilIpc` / `CoilPathIpc`)
//! and their core `PhaseCoil` → IPC converters.

use serde::{Deserialize, Serialize};

use pcbmotorgen_routing::{
    CoilSegment as CoreCoilSegment, PhaseCoil as CorePhaseCoil,
    PoleRegion as CorePoleRegion,
    RoutingDimensions as CoreRoutingDimensions, SlotWidth as CoreSlotWidth,
};

const MM_TO_M: f64 = 1e-3;

// ===========================================================================
// Coil path (generate_coils)
// ===========================================================================

/// One straight segment of a coil path. `start`/`end` are `(x, y)` [m].
/// `is_active` distinguishes active vertical conductors from end-turn
/// connectors (PRODUCT_GOALS.md §5) for SVG colour-coding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CoilSegmentIpc {
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub is_active: bool,
}

/// One rounded corner / curve of a coil path — a quadratic Bézier
/// `(start, mid, end)` matching KiCad's `(arc ...)` primitive. `mid` is the
/// control point on the arc.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CoilArcIpc {
    pub start: [f64; 2],
    pub mid: [f64; 2],
    pub end: [f64; 2],
    pub is_active: bool,
}

/// A single phase coil on a single PCB layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PhaseCoilIpc {
    pub phase_idx: u32,
    pub layer_idx: u32,
    pub phase_name: String,
    pub pattern_id: String,
    pub segments: Vec<CoilSegmentIpc>,
    /// Rounded corners / curves (empty for straight-line-only patterns).
    #[serde(default)]
    pub corner_arcs: Vec<CoilArcIpc>,
    /// Center positions of inter-layer vias on this coil's layer+net, `(x, y)` [m].
    #[serde(default)]
    pub via_positions: Vec<[f64; 2]>,
    pub total_length_m: f64,
    pub active_length_m: f64,
    pub end_turn_length_m: f64,
    pub active_conductor_count: u32,
    /// `[min_x, min_y, max_x, max_y]` [m].
    pub bounding_box: [f64; 4],
    pub terminal_start: [f64; 2],
    pub terminal_end: [f64; 2],
}

/// One active trace bundle's calculated effective slot width.
///
/// Lengths are metres and `angle_rad` is relative to the direction of travel.
/// `margin_m < 0` means the bundle is wider than the phase band allowed by the
/// pole pitch; the backend reports that condition and does not alter geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SlotWidthIpc {
    pub layer: u32,
    pub net: String,
    pub trace_count: u32,
    pub trace_width_m: f64,
    pub trace_spacing_m: f64,
    pub angle_rad: f64,
    pub slot_width_m: f64,
    pub max_slot_width_m: Option<f64>,
    pub margin_m: Option<f64>,
}

/// Pole/phase dimensions returned with `generate_coils`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingDimensionsIpc {
    pub active_area_length_m: f64,
    pub total_routing_length_m: f64,
    pub board_width_m: f64,
    pub phases: u32,
    /// Full mover magnet-array span, when supplied by the config.
    pub magnet_array_span_m: Option<f64>,
    /// Centre-to-centre distance between adjacent north/south poles.
    pub pole_pitch_m: Option<f64>,
    pub period_pitch_m: Option<f64>,
    pub period_count: Option<u32>,
    /// Ideal phase-band pitch (`pole_pitch_m / phases`).
    pub slot_pitch_m: Option<f64>,
    pub phase_clearance_m: f64,
    pub max_slot_width_m: Option<f64>,
    pub slot_widths: Vec<SlotWidthIpc>,
    #[serde(default)]
    pub pole_regions: Vec<PoleRegionIpc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PoleRegionIpc {
    pub phase: String,
    pub pole_index: u32,
    pub start: [f64; 2],
    pub end: [f64; 2],
}

impl Default for RoutingDimensionsIpc {
    fn default() -> Self {
        Self {
            active_area_length_m: 0.0,
            total_routing_length_m: 0.0,
            board_width_m: 0.0,
            phases: 0,
            magnet_array_span_m: None,
            pole_pitch_m: None,
            period_pitch_m: None,
            period_count: None,
            slot_pitch_m: None,
            phase_clearance_m: 0.0,
            max_slot_width_m: None,
            slot_widths: Vec::new(),
            pole_regions: Vec::new(),
        }
    }
}

/// Complete coil geometry for all phases/layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CoilPathIpc {
    pub phases: Vec<PhaseCoilIpc>,
    pub layer_count: u32,
    /// Design dimensions calculated from the same context as the geometry.
    /// This is additive so existing consumers can continue using `phases` and
    /// `layer_count` alone.
    #[serde(default)]
    pub routing_dimensions: RoutingDimensionsIpc,
}

// ===========================================================================
// Geometry conversions (core PhaseCoil → IPC)
// ===========================================================================

#[allow(dead_code)]
impl CoilSegmentIpc {
    /// Convert a core `CoilSegment` (array coords) to the IPC form.
    /// Both already use `[f64; 2]` — this is a 1:1 passthrough.
    pub fn from_core(s: &CoreCoilSegment) -> Self {
        Self {
            start: [s.start.0 * MM_TO_M, s.start.1 * MM_TO_M],
            end: [s.end.0 * MM_TO_M, s.end.1 * MM_TO_M],
            is_active: s.is_active,
        }
    }
}

#[allow(dead_code)]
impl CoilArcIpc {
    /// Convert a core `CoilArc` (tuple coords) to the IPC form.
    pub fn from_core(a: &pcbmotorgen_routing::CoilArc) -> Self {
        Self {
            start: [a.start.0 * MM_TO_M, a.start.1 * MM_TO_M],
            mid: [a.mid.0 * MM_TO_M, a.mid.1 * MM_TO_M],
            end: [a.end.0 * MM_TO_M, a.end.1 * MM_TO_M],
            is_active: a.is_active,
        }
    }
}

#[allow(dead_code)]
impl PhaseCoilIpc {
    /// Convert a core `PhaseCoil` to the IPC wire format.
    pub fn from_core(coil: &CorePhaseCoil) -> Self {
        let segments: Vec<CoilSegmentIpc> =
            coil.segments.iter().map(CoilSegmentIpc::from_core).collect();
        let corner_arcs: Vec<CoilArcIpc> =
            coil.corner_arcs.iter().map(CoilArcIpc::from_core).collect();
        let via_positions: Vec<[f64; 2]> = coil
            .center_via_positions
            .iter()
            .map(|&(x, y)| [x * MM_TO_M, y * MM_TO_M])
            .collect();
        let bbox = coil.bounding_box();
        let terminal_start = coil
            .segments
            .first()
            .map(|s| [s.start.0 * MM_TO_M, s.start.1 * MM_TO_M])
            .unwrap_or([0.0, 0.0]);
        let terminal_end = coil
            .segments
            .last()
            .map(|s| [s.end.0 * MM_TO_M, s.end.1 * MM_TO_M])
            .unwrap_or([0.0, 0.0]);
        Self {
            phase_idx: coil.phase_idx,
            layer_idx: coil.layer_idx,
            phase_name: coil.phase_name.clone(),
            pattern_id: coil.pattern_id.clone(),
            segments,
            corner_arcs,
            via_positions,
            total_length_m: coil.total_length_mm() * MM_TO_M,
            active_length_m: coil.active_length_mm() * MM_TO_M,
            end_turn_length_m: coil.end_turn_length_mm() * MM_TO_M,
            active_conductor_count: coil.active_conductor_count() as u32,
            bounding_box: [
                bbox.0 * MM_TO_M,
                bbox.1 * MM_TO_M,
                bbox.2 * MM_TO_M,
                bbox.3 * MM_TO_M,
            ],
            terminal_start,
            terminal_end,
        }
    }
}

#[allow(dead_code)]
impl CoilPathIpc {
    /// Build the IPC coil path from a list of core `PhaseCoil`s.
    pub fn from_core(coils: &[CorePhaseCoil], layer_count: u32) -> Self {
        Self::from_core_with_dimensions(
            coils,
            layer_count,
            &CoreRoutingDimensions::default(),
        )
    }

    /// Build the IPC coil path and include the dimensions produced by the
    /// routing report.
    pub fn from_core_with_dimensions(
        coils: &[CorePhaseCoil],
        layer_count: u32,
        dimensions: &CoreRoutingDimensions,
    ) -> Self {
        Self {
            phases: coils.iter().map(PhaseCoilIpc::from_core).collect(),
            layer_count,
            routing_dimensions: RoutingDimensionsIpc::from_core(dimensions),
        }
    }
}

impl SlotWidthIpc {
    fn from_core(slot: &CoreSlotWidth) -> Self {
        Self {
            layer: slot.layer,
            net: slot.net.clone(),
            trace_count: slot.trace_count,
            trace_width_m: slot.trace_width_mm * MM_TO_M,
            trace_spacing_m: slot.trace_spacing_mm * MM_TO_M,
            angle_rad: slot.angle_rad,
            slot_width_m: slot.slot_width_mm * MM_TO_M,
            max_slot_width_m: slot.max_slot_width_mm.map(|v| v * MM_TO_M),
            margin_m: slot.margin_mm.map(|v| v * MM_TO_M),
        }
    }
}

impl RoutingDimensionsIpc {
    fn from_core(dimensions: &CoreRoutingDimensions) -> Self {
        Self {
            active_area_length_m: dimensions.active_area_length_mm * MM_TO_M,
            total_routing_length_m: dimensions.total_routing_length_mm * MM_TO_M,
            board_width_m: dimensions.board_width_mm * MM_TO_M,
            phases: dimensions.phases,
            magnet_array_span_m: dimensions.magnet_array_span_mm.map(|v| v * MM_TO_M),
            pole_pitch_m: dimensions.pole_pitch_mm.map(|v| v * MM_TO_M),
            period_pitch_m: dimensions.period_pitch_mm.map(|v| v * MM_TO_M),
            period_count: dimensions.period_count,
            slot_pitch_m: dimensions.slot_pitch_mm.map(|v| v * MM_TO_M),
            phase_clearance_m: dimensions.phase_clearance_mm * MM_TO_M,
            max_slot_width_m: dimensions.max_slot_width_mm.map(|v| v * MM_TO_M),
            slot_widths: dimensions
                .slot_widths
                .iter()
                .map(SlotWidthIpc::from_core)
                .collect(),
            pole_regions: dimensions
                .pole_regions
                .iter()
                .map(PoleRegionIpc::from_core)
                .collect(),
        }
    }
}

impl PoleRegionIpc {
    fn from_core(region: &CorePoleRegion) -> Self {
        Self {
            phase: region.phase.clone(),
            pole_index: region.pole_index,
            start: [region.start.x * MM_TO_M, region.start.y * MM_TO_M],
            end: [region.end.x * MM_TO_M, region.end.y * MM_TO_M],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_mm_is_converted_to_frontend_si() {
        let segment = CoreCoilSegment {
            start: (10.0, 0.0),
            end: (10.0, 20.0),
            is_active: true,
        };
        let dto = CoilSegmentIpc::from_core(&segment);
        assert_eq!(dto.start, [0.01, 0.0]);
        assert_eq!(dto.end, [0.01, 0.02]);

        let dimensions = CoreRoutingDimensions {
            active_area_length_mm: 195.0,
            total_routing_length_mm: 255.0,
            board_width_mm: 20.0,
            magnet_array_span_mm: Some(120.0),
            phases: 3,
            pole_pitch_mm: Some(12.0),
            period_pitch_mm: Some(12.0),
            period_count: Some(20),
            slot_pitch_mm: Some(4.0),
            phase_clearance_mm: 0.127,
            max_slot_width_mm: Some(3.873),
            slot_widths: vec![],
            pole_regions: vec![CorePoleRegion {
                phase: "A".to_string(),
                pole_index: 2,
                start: pcbmotorgen_routing::Point::new(0.0, 10.0),
                end: pcbmotorgen_routing::Point::new(12.0, 10.0),
            }],
        };
        let wire = RoutingDimensionsIpc::from_core(&dimensions);
        assert_eq!(wire.active_area_length_m, 0.195);
        assert_eq!(wire.pole_pitch_m, Some(0.012));
        assert_eq!(wire.phase_clearance_m, 0.000127);
        assert_eq!(wire.pole_regions.len(), 1);
        assert_eq!(wire.pole_regions[0].phase, "A");
        assert_eq!(wire.pole_regions[0].pole_index, 2);
        assert_eq!(wire.pole_regions[0].start, [0.0, 0.01]);
        assert_eq!(wire.pole_regions[0].end, [0.012, 0.01]);
    }
}
