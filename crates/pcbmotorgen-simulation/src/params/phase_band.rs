//! Declared phase-band positions handed to the simulation (kata hzs2).
//!
//! The routing contract (kata hzs2) carries first-class phase-band geometry:
//! patterns declare their bands on `RoutingResult.phase_bands` and the host
//! resolves them (declared or derived from the ideal phase-band pitch) in the
//! `RoutingDimensions.phase_bands` sidecar. This module carries those
//! positions into the simulation config in SI metres, so commutation and
//! equilibrium can consume the laid-out band positions when present and fall
//! back to the analytic derivations when not.

use serde::{Deserialize, Serialize};

use crate::units::mm;

/// One declared phase-band position record [m] (kata hzs2).
///
/// Records may repeat a phase label when a phase occupies several layers —
/// both layer copies share the phase's electrical position, so consumers
/// match by label and the first match wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseBandPosition {
    /// Phase/net label of the band (e.g. `"A"`).
    pub phase: String,
    /// Centerline x of the band's first repeating instance [m] — the phase
    /// reference position the per-coil electrical offsets are derived from.
    pub centerline_m: f64,
    /// Along-travel extent start [m], as the pattern lays the band out.
    pub start_m: f64,
    /// Along-travel extent end [m].
    pub end_m: f64,
}

impl PhaseBandPosition {
    /// Convert one resolved routing band (millimetres) to a simulation
    /// record (metres).
    #[must_use]
    pub fn from_resolved_routing_band(
        band: &pcbmotorgen_routing::ResolvedPhaseBand,
    ) -> Self {
        Self {
            phase: band.band.net.clone(),
            centerline_m: mm(band.band.centerline_x_mm),
            start_m: mm(band.band.start_x_mm),
            end_m: mm(band.band.end_x_mm),
        }
    }
}

/// Convert resolved routing phase bands (millimetres, per `(layer, net)`)
/// into simulation records (metres), preserving order.
#[must_use]
pub fn phase_bands_from_routing(
    bands: &[pcbmotorgen_routing::ResolvedPhaseBand],
) -> Vec<PhaseBandPosition> {
    bands
        .iter()
        .map(PhaseBandPosition::from_resolved_routing_band)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pcbmotorgen_routing::{PhaseBand, PhaseBandShape};

    #[test]
    fn converts_millimetres_to_metres_and_keeps_labels() {
        let resolved = ResolvedPhaseBandPair::single();
        let record = PhaseBandPosition::from_resolved_routing_band(&resolved);
        assert_eq!(record.phase, "A");
        assert!((record.centerline_m - 0.002).abs() < 1e-15);
        assert!((record.start_m - 0.0).abs() < 1e-15);
        assert!((record.end_m - 0.004).abs() < 1e-15);
    }

    #[test]
    fn batch_conversion_preserves_order_and_duplicates() {
        let bands = ResolvedPhaseBandPair::two_layers();
        let records = phase_bands_from_routing(&bands);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].phase, "A");
        assert_eq!(records[1].phase, "A");
        assert_eq!(records[0], records[1], "layer copies share the phase position");
    }

    /// Local helpers building resolved routing bands without needing a full
    /// routing generation.
    struct ResolvedPhaseBandPair;

    impl ResolvedPhaseBandPair {
        fn single() -> pcbmotorgen_routing::ResolvedPhaseBand {
            pcbmotorgen_routing::ResolvedPhaseBand {
                band: band_mm("A", 0),
                derived: false,
            }
        }

        fn two_layers() -> Vec<pcbmotorgen_routing::ResolvedPhaseBand> {
            vec![
                pcbmotorgen_routing::ResolvedPhaseBand {
                    band: band_mm("A", 0),
                    derived: false,
                },
                pcbmotorgen_routing::ResolvedPhaseBand {
                    band: band_mm("A", 1),
                    derived: false,
                },
            ]
        }
    }

    fn band_mm(net: &str, layer: u32) -> PhaseBand {
        PhaseBand {
            layer,
            net: net.to_string(),
            centerline_x_mm: 2.0,
            start_x_mm: 0.0,
            end_x_mm: 4.0,
            y_min_mm: 0.0,
            y_max_mm: 20.0,
            shape: PhaseBandShape::Linear,
        }
    }
}
