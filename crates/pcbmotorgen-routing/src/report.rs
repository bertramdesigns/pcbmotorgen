//! Enriched generation output for callers that need design dimensions as well
//! as raw route geometry.

use serde::{Deserialize, Serialize};

use crate::dimensions::RoutingDimensions;
use crate::model::RoutingResult;

/// The application-facing result of a validated routing generation.
///
/// Plugin authors still return the strict [`RoutingResult`] shape.  The host
/// wraps that result in this report after validation and calculates the
/// board/magnet dimensions from the same context.  Keeping the dimensions in
/// a sidecar preserves the canonical plugin wire shape while allowing
/// `generate_coils` and analysis clients to receive the additional information
/// needed to place a magnet array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingReport {
    /// Validated segments, curves, and vias emitted by the pattern.
    pub result: RoutingResult,
    /// Pole pitch and calculated active conductor-band widths.
    pub dimensions: RoutingDimensions,
}
