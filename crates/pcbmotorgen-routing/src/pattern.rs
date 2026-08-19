//! The [`RoutingPattern`] trait — the documented interface every pattern
//! (bundled, Rust crate plugin, or Python runner) implements.

use crate::{error::RoutingError, model::RoutingResult, context::RoutingContext};

pub use ParamType::*;

/// The data type of a pattern parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    /// An integer-valued parameter (e.g. number of strands).
    Int,
    /// A floating-point parameter (e.g. an angle in degrees).
    Float,
}

/// A single user-settable parameter a pattern exposes.
///
/// Patterns declare their configurable knobs here so the UI can render a
/// control and the core can seed defaults. Parameters are carried to the
/// pattern at generate-time via [`RoutingContext::params`]. Quantities that are
/// *derived* from the board (e.g. the infinity braid's amplitude `A` and total
/// length `D_tot`, computed from the active-area width / length) are NOT
/// declared here — only user-editable knobs are.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PatternParameter {
    /// The key used in `RoutingContext.params` (e.g. `"num_strands"`).
    pub key: String,
    /// Human-readable label for the UI.
    pub label: String,
    /// Optional longer description / tooltip.
    pub description: String,
    /// The expected type (renders an int vs float input).
    pub param_type: ParamType,
    /// Default value.
    pub default: f64,
    /// Optional inclusive minimum.
    pub min: Option<f64>,
    /// Optional inclusive maximum.
    pub max: Option<f64>,
    /// Optional step for the spinner.
    pub step: Option<f64>,
}

impl PatternParameter {
    pub fn int(
        key: impl Into<String>,
        label: impl Into<String>,
        default: f64,
        min: f64,
        max: f64,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            description: String::new(),
            param_type: ParamType::Int,
            default,
            min: Some(min),
            max: Some(max),
            step: Some(1.0),
        }
    }

    pub fn float(
        key: impl Into<String>,
        label: impl Into<String>,
        default: f64,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            description: String::new(),
            param_type: ParamType::Float,
            default,
            min: None,
            max: None,
            step: None,
        }
    }

    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
}

/// Optional structured metadata a plugin author may attach to a pattern.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    /// Plugin id (registry key).
    pub id: String,
    /// Human-readable display name (may be empty during partial construction).
    pub display_name: String,
    /// Author name / handle.
    pub author: String,
    /// Semantic version of the plugin (e.g. "1.2.0").
    pub version: String,
    /// One-line description.
    pub description: String,
}

impl PluginMetadata {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Self::default()
        }
    }
}

/// A loadable, validated routing pattern.
///
/// Implementors produce a [`RoutingResult`] for a [`RoutingContext`]. The
/// generated result is always passed through the strict-shape [`Validator`]
/// (crate::validator) before being written or previewed; a malformed result is
/// rejected, never sanitised.
pub trait RoutingPattern: Send + Sync {
    /// Stable identifier used as the registry / config key (e.g. `"infinity-braid"`).
    fn id(&self) -> &str;

    /// Human-readable display name.
    fn display_name(&self) -> &str;

    /// Author of the plugin (defaults to empty).
    fn author(&self) -> &str {
        ""
    }

    /// Semantic version of the plugin (defaults to empty).
    fn version(&self) -> &str {
        ""
    }

    /// Optional one-line description.
    fn description(&self) -> &str {
        ""
    }

    /// The user-editable parameters this pattern exposes. Defaults to none.
    ///
    /// Derived-from-board quantities must NOT be listed here.
    fn parameters(&self) -> Vec<PatternParameter> {
        Vec::new()
    }

    /// Compose the full metadata block from the trait accessors.
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: self.id().to_string(),
            display_name: self.display_name().to_string(),
            author: self.author().to_string(),
            version: self.version().to_string(),
            description: self.description().to_string(),
        }
    }

    /// Whether this pattern's output is expected to be a continuous path per
    /// (layer, net). Drives the validator's continuity check.
    fn expects_continuous(&self) -> bool {
        false
    }

    /// Produce the coil geometry for the given context.
    fn generate(&self, ctx: &RoutingContext) -> Result<RoutingResult, RoutingError>;
}
