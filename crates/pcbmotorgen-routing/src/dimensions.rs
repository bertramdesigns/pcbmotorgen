//! Board-to-magnet dimensions reported alongside generated routing geometry.
//!
//! A [`RoutingResult`] deliberately contains only raw
//! geometry.  Consumers that need to place or evaluate the mover also need the
//! dimensions that gave that geometry meaning: the centre-to-centre pole pitch
//! and the width of each active conductor band (phase band).  This module owns those
//! calculations so the application, Python runners, and native plugins all use
//! the same equations.
//!
//! All lengths are millimetres.  `angle_rad` is measured from the direction of
//! motion (the x axis).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::context::RoutingContext;
use crate::error::{RoutingError, RoutingErrorKind};
use crate::model::{Layer, Point, PoleRegion, RoutingResult};

const EPS: f64 = 1e-12;

/// A calculated active-conductor band for one `(layer, net)` group.
///
/// The band width is calculated from the perpendicular bundle thickness and
/// the trace angle:
///
/// `band_width = (N * trace_width + (N - 1) * trace_spacing) / sin(theta)`
///
/// `max_band_width_mm` is the top-down phase-band limit when a pole pitch is
/// available.  A negative `margin_mm` means the trace bundle does not fit in the
/// available phase band; geometry is never silently shortened to make it fit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseBandWidth {
    /// Copper layer carrying this band.
    pub layer: Layer,
    /// Phase/net label carrying this band.
    pub net: String,
    /// Number of parallel strands (conductors) in the coil-side bundle (`N`).
    pub trace_count: u32,
    /// Width of one trace (`w_t`) [mm].
    pub trace_width_mm: f64,
    /// Clearance between adjacent traces (`s`) [mm].
    pub trace_spacing_mm: f64,
    /// Trace angle relative to the x/travel axis (`theta`) [rad].
    pub angle_rad: f64,
    /// Effective band width measured along the travel axis (`w_s`) [mm].
    #[serde(alias = "slot_width_mm")]
    pub band_width_mm: f64,
    /// Maximum width allowed by the pole pitch and phase count [mm].
    #[serde(default, alias = "max_slot_width_mm")]
    pub max_band_width_mm: Option<f64>,
    /// `max_band_width_mm - band_width_mm`, when a pole pitch is known [mm].
    #[serde(default)]
    pub margin_mm: Option<f64>,
}

/// Dimensions needed to hand generated traces off to magnet-pattern and
/// analysis code.
///
/// `pole_pitch_mm` is the centre-to-centre distance between adjacent north and
/// south poles (`tau_p` in the design equation).  `phase_band_pitch_mm` is the
/// ideal phase-band pitch `tau_p / phases`; it is intentionally separate from
/// the calculated conductor-band width in [`PhaseBandWidth::band_width_mm`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingDimensions {
    /// Active-area length supplied by the board context [mm].
    pub active_area_length_mm: f64,
    /// Total routable length [mm]. The routing domain equals the active
    /// area — there is no end padding.
    pub total_routing_length_mm: f64,
    /// Board width perpendicular to travel [mm].
    pub board_width_mm: f64,
    /// Full mover magnet-array span supplied by the core [mm], when known.
    /// Together with `pole_pitch_mm` this lets a consumer recover the number of
    /// pole intervals without reaching back into the application config.
    #[serde(default)]
    pub magnet_array_span_mm: Option<f64>,
    /// Number of electrical phases used for the phase-band calculation.
    pub phases: u32,
    /// Centre-to-centre adjacent-pole distance (`tau_p`) [mm].
    #[serde(default)]
    pub pole_pitch_mm: Option<f64>,
    /// Repeating routing period [mm].  For the magnet-aware infinity braid this
    /// equals `pole_pitch_mm`; it is optional for patterns without a repeating
    /// unit.
    #[serde(default)]
    pub period_pitch_mm: Option<f64>,
    /// Number of complete repeating periods emitted by a pattern, when known.
    #[serde(default)]
    pub period_count: Option<u32>,
    /// Ideal phase-band pitch (pole_pitch / phases, i.e. spacing_ratio = 1.0) [mm].
    #[serde(default, alias = "slot_pitch_mm")]
    pub phase_band_pitch_mm: Option<f64>,
    /// Minimum gap reserved between adjacent phase bands (`g_phase`) [mm].
    pub phase_clearance_mm: f64,
    /// Maximum phase-band width (pole_pitch / phases − g_phase) [mm].
    #[serde(default, alias = "max_slot_width_mm")]
    pub max_phase_band_width_mm: Option<f64>,
    /// One calculated band for each active `(layer, net)` group.
    #[serde(default, alias = "slot_widths")]
    pub phase_band_widths: Vec<PhaseBandWidth>,
    /// Pattern-defined pole regions copied from the canonical result.
    #[serde(default)]
    pub pole_regions: Vec<PoleRegion>,
}

impl Default for RoutingDimensions {
    fn default() -> Self {
        Self {
            active_area_length_mm: 0.0,
            total_routing_length_mm: 0.0,
            board_width_mm: 0.0,
            magnet_array_span_mm: None,
            phases: 0,
            pole_pitch_mm: None,
            period_pitch_mm: None,
            period_count: None,
            phase_band_pitch_mm: None,
            phase_clearance_mm: 0.0,
            max_phase_band_width_mm: None,
            phase_band_widths: Vec::new(),
            pole_regions: Vec::new(),
        }
    }
}

impl RoutingDimensions {
    /// Calculate dimensions from a generic routing result.
    ///
    /// Generic patterns may expose a `num_strands`, `trace_count`, `turns`, or
    /// `windings_per_phase` parameter.  The first present value is used as the
    /// trace count for the returned phase-band width records; otherwise one
    /// trace is reported.  A pattern that has no pole pitch still gets the
    /// bottom-up phase-band width calculation, but no top-down maximum can be
    /// reported.
    pub fn from_result(result: &RoutingResult, ctx: &RoutingContext) -> Result<Self, RoutingError> {
        let trace_count = trace_count_hint(ctx);
        Self::from_result_with_options(result, ctx, trace_count, None, None, None)
    }

    /// Build the dimensions for the bundled infinity braid.
    ///
    /// The diamond edge angle is known from the board width and the repeating
    /// period, so it is not estimated from an arbitrary crossing segment.  If
    /// a magnet layout is present, the period is the pole pitch exactly.
    pub fn for_infinity(
        result: &RoutingResult,
        ctx: &RoutingContext,
        trace_count: u32,
        period_pitch_mm: Option<f64>,
        period_count: Option<u32>,
    ) -> Result<Self, RoutingError> {
        let angle = period_pitch_mm.map(|period| (ctx.board_width_mm / period).atan());
        Self::from_result_with_options(
            result,
            ctx,
            trace_count,
            angle,
            period_pitch_mm,
            period_count,
        )
    }

    /// Return true when every calculated phase band fits.
    pub fn all_phase_bands_fit(&self) -> bool {
        self.phase_band_widths
            .iter()
            .all(|band| band.margin_mm.map(|margin| margin >= -EPS).unwrap_or(true))
    }

    /// Centre-to-centre adjacent North/South pole distance, using the motor
    /// design terminology requested by magnet-pattern consumers.
    pub fn pole_to_pole_pitch_mm(&self) -> Option<f64> {
        self.pole_pitch_mm
    }

    fn from_result_with_options(
        result: &RoutingResult,
        ctx: &RoutingContext,
        trace_count: u32,
        angle_override_rad: Option<f64>,
        period_pitch_mm: Option<f64>,
        period_count: Option<u32>,
    ) -> Result<Self, RoutingError> {
        validate_context_dimensions(ctx)?;
        if trace_count == 0 {
            return Err(dimension_error(
                "dimensions.trace_count",
                "trace count N must be at least 1",
            ));
        }

        let phases = ctx.phases.max(1);
        let total_length = ctx.active_area_length_mm;
        let pole_pitch_mm = match ctx.magnet_pitch_mm {
            Some(pitch) => {
                if !pitch.is_finite() || pitch <= 0.0 {
                    return Err(dimension_error(
                        "context.magnet_pitch_mm",
                        "pole pitch must be finite and greater than zero",
                    ));
                }
                Some(pitch)
            }
            None => None,
        };

        let phase_band_pitch_mm = pole_pitch_mm.map(|pitch| pitch / phases as f64);
        let max_phase_band_width_mm = phase_band_pitch_mm.map(|pitch| pitch - ctx.min_space_mm);

        let mut groups: BTreeMap<(Layer, String), Vec<(Point, Point)>> = BTreeMap::new();
        for segment in result.segments.iter().filter(|segment| segment.is_active) {
            groups
                .entry((segment.layer, segment.net.clone()))
                .or_default()
                .push((segment.start, segment.end));
        }
        for curve in result.curves.iter().filter(|curve| curve.is_active) {
            groups
                .entry((curve.layer, curve.net.clone()))
                .or_default()
                .push((curve.start, curve.end));
        }

        let mut phase_band_widths = Vec::with_capacity(groups.len());
        for ((layer, net), paths) in groups {
            let angle_rad = angle_override_rad
                .or_else(|| paths.iter().find_map(|(start, end)| path_angle(*start, *end)))
                .ok_or_else(|| {
                    dimension_error(
                        "dimensions.phase_band_widths.angle_rad",
                        format!(
                            "cannot calculate a trace angle for layer {} net {}: active geometry is parallel to the travel axis",
                            layer, net
                        ),
                    )
                })?;
            let band_width_mm = phase_band_width_from_trace_geometry_mm(
                trace_count,
                ctx.min_trace_mm,
                ctx.min_space_mm,
                angle_rad,
            )
            .map_err(|message| dimension_error("dimensions.phase_band_widths.band_width_mm", message))?;
            let margin_mm = max_phase_band_width_mm.map(|max| max - band_width_mm);
            phase_band_widths.push(PhaseBandWidth {
                layer,
                net,
                trace_count,
                trace_width_mm: ctx.min_trace_mm,
                trace_spacing_mm: ctx.min_space_mm,
                angle_rad,
                band_width_mm,
                max_band_width_mm: max_phase_band_width_mm,
                margin_mm,
            });
        }

        Ok(Self {
            active_area_length_mm: ctx.active_area_length_mm,
            total_routing_length_mm: total_length,
            board_width_mm: ctx.board_width_mm,
            magnet_array_span_mm: ctx.magnet_array_span(),
            phases,
            pole_pitch_mm,
            period_pitch_mm,
            period_count,
            phase_band_pitch_mm,
            phase_clearance_mm: ctx.min_space_mm,
            max_phase_band_width_mm,
            phase_band_widths,
            pole_regions: result.pole_regions.clone(),
        })
    }
}

/// Bottom-up phase-band width equation.
///
/// `theta_rad` is the angle between the trace and the direction of motion.
/// The function rejects an angle parallel to the motion because its projected
/// band width is undefined (division by `sin(theta)`).
pub fn phase_band_width_from_trace_geometry_mm(
    trace_count: u32,
    trace_width_mm: f64,
    trace_spacing_mm: f64,
    theta_rad: f64,
) -> Result<f64, String> {
    if trace_count == 0 {
        return Err("trace count N must be at least 1".to_string());
    }
    if !trace_width_mm.is_finite() || trace_width_mm <= 0.0 {
        return Err("trace width w_t must be finite and greater than zero".to_string());
    }
    if !trace_spacing_mm.is_finite() || trace_spacing_mm < 0.0 {
        return Err("trace spacing s must be finite and non-negative".to_string());
    }
    if !theta_rad.is_finite() {
        return Err("trace angle theta must be finite".to_string());
    }

    let sin_theta = theta_rad.sin().abs();
    if sin_theta <= EPS {
        return Err(
            "trace angle theta must not be parallel to the travel axis (sin(theta) is zero)"
                .to_string(),
        );
    }

    let perpendicular_bundle_width = trace_count as f64 * trace_width_mm
        + (trace_count.saturating_sub(1) as f64) * trace_spacing_mm;
    Ok(perpendicular_bundle_width / sin_theta)
}

/// Top-down maximum phase-band width equation:
///
/// `max_phase_band_width = pole_pitch / phases - phase_clearance`.
pub fn max_phase_band_width_from_pole_pitch_mm(
    pole_pitch_mm: f64,
    phases: u32,
    phase_clearance_mm: f64,
) -> Result<f64, String> {
    if !pole_pitch_mm.is_finite() || pole_pitch_mm <= 0.0 {
        return Err("pole pitch tau_p must be finite and greater than zero".to_string());
    }
    if phases == 0 {
        return Err("phase count m must be at least 1".to_string());
    }
    if !phase_clearance_mm.is_finite() || phase_clearance_mm < 0.0 {
        return Err("phase clearance g_phase must be finite and non-negative".to_string());
    }
    Ok(pole_pitch_mm / phases as f64 - phase_clearance_mm)
}

fn validate_context_dimensions(ctx: &RoutingContext) -> Result<(), RoutingError> {
    if !ctx.active_area_length_mm.is_finite() || ctx.active_area_length_mm <= 0.0 {
        return Err(dimension_error(
            "context.active_area_length_mm",
            "active-area length must be finite and greater than zero",
        ));
    }
    if !ctx.board_width_mm.is_finite() || ctx.board_width_mm <= 0.0 {
        return Err(dimension_error(
            "context.board_width_mm",
            "board width must be finite and greater than zero",
        ));
    }
    if !ctx.min_trace_mm.is_finite() || ctx.min_trace_mm <= 0.0 {
        return Err(dimension_error(
            "context.min_trace_mm",
            "trace width must be finite and greater than zero",
        ));
    }
    if !ctx.min_space_mm.is_finite() || ctx.min_space_mm < 0.0 {
        return Err(dimension_error(
            "context.min_space_mm",
            "trace/phase clearance must be finite and non-negative",
        ));
    }
    Ok(())
}

fn path_angle(start: Point, end: Point) -> Option<f64> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if !dx.is_finite() || !dy.is_finite() || (dx.abs() <= EPS && dy.abs() <= EPS) {
        return None;
    }
    let angle = dy.abs().atan2(dx.abs());
    if angle <= EPS {
        None
    } else {
        Some(angle)
    }
}

fn trace_count_hint(ctx: &RoutingContext) -> u32 {
    ["num_strands", "trace_count", "turns", "windings_per_phase"]
        .iter()
        .find_map(|key| {
            let value = ctx.params.get(*key).copied()?;
            if value.is_finite() && value >= 1.0 {
                Some(value.round() as u32)
            } else {
                None
            }
        })
        .unwrap_or(1)
}

fn dimension_error(field: impl Into<String>, message: impl Into<String>) -> RoutingError {
    RoutingError::new(0, field, RoutingErrorKind::Generation, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RouteSegment, RoutingResult};
    use std::collections::HashMap;

    fn context() -> RoutingContext {
        RoutingContext {
            active_area_length_mm: 195.0,
            board_width_mm: 20.0,
            num_layers: 2,
            phases: 3,
            min_trace_mm: 0.2,
            min_space_mm: 0.15,
            magnet_pitch_mm: Some(12.0),
            params: HashMap::from([("num_strands".to_string(), 4.0)]),
            ..RoutingContext::default()
        }
    }

    #[test]
    fn bottom_up_equation_matches_reference_example() {
        let width = phase_band_width_from_trace_geometry_mm(4, 0.2, 0.15, 45_f64.to_radians())
            .expect("valid phase-band width inputs");
        assert!((width - 1.76776695).abs() < 1e-8, "width = {width}");
    }

    #[test]
    fn top_down_equation_matches_reference_example() {
        let width = max_phase_band_width_from_pole_pitch_mm(12.0, 3, 0.0).unwrap();
        assert!((width - 4.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_trace_parallel_to_motion() {
        let error = phase_band_width_from_trace_geometry_mm(1, 0.2, 0.15, 0.0).unwrap_err();
        assert!(error.contains("parallel"));
    }

    #[test]
    fn reports_pole_pitch_and_band_widths_per_layer_and_net() {
        let result = RoutingResult {
            segments: vec![
                RouteSegment {
                    start: Point::new(0.0, 0.0),
                    end: Point::new(10.0, 10.0),
                    layer: 0,
                    net: "A".to_string(),
                    is_active: true,
                },
                RouteSegment {
                    start: Point::new(0.0, 10.0),
                    end: Point::new(10.0, 0.0),
                    layer: 1,
                    net: "A".to_string(),
                    is_active: true,
                },
            ],
            ..RoutingResult::default()
        };
        let dimensions = RoutingDimensions::from_result(&result, &context()).unwrap();
        assert_eq!(dimensions.pole_pitch_mm, Some(12.0));
        assert_eq!(dimensions.pole_to_pole_pitch_mm(), Some(12.0));
        assert_eq!(dimensions.magnet_array_span_mm, None);
        assert_eq!(dimensions.phase_band_pitch_mm, Some(4.0));
        assert_eq!(dimensions.phase_band_widths.len(), 2);
        assert!(dimensions.all_phase_bands_fit(), "bands: {:?}", dimensions.phase_band_widths);
        assert_eq!(dimensions.phase_band_widths[0].trace_count, 4);
    }

    #[test]
    fn dimensions_round_trip_as_json() {
        let dimensions = RoutingDimensions::from_result(
            &RoutingResult {
                segments: vec![RouteSegment {
                    start: Point::new(0.0, 0.0),
                    end: Point::new(10.0, 10.0),
                    layer: 0,
                    net: "A".into(),
                    is_active: true,
                }],
                ..RoutingResult::default()
            },
            &context(),
        )
        .unwrap();
        let json = serde_json::to_string(&dimensions).unwrap();
        let back: RoutingDimensions = serde_json::from_str(&json).unwrap();
        assert_eq!(back, dimensions);
    }
}
