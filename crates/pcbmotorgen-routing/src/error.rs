//! Routing error types.
//!
//! [`RoutingError`] is the single error type returned by patterns and the
//! validator. It carries the index, field, error kind, and a human-helpful
//! message so the app can surface a precise, actionable rejection.

use std::fmt;

/// The class of failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingErrorKind {
    /// The algorithm produced a malformed / non-finite shape.
    Malformed,
    /// A coordinate fell outside the board bounds.
    OutOfBounds,
    /// A layer index was outside the valid range.
    BadLayer,
    /// A degenerate (zero-length / collapsed) element.
    Degenerate,
    /// An invalid net label.
    BadNet,
    /// A required field / element was missing.
    Missing,
    /// Internal generate() failure unrelated to shape validation.
    Generation,
}

impl fmt::Display for RoutingErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => write!(f, "malformed"),
            Self::OutOfBounds => write!(f, "out_of_bounds"),
            Self::BadLayer => write!(f, "bad_layer"),
            Self::Degenerate => write!(f, "degenerate"),
            Self::BadNet => write!(f, "bad_net"),
            Self::Missing => write!(f, "missing"),
            Self::Generation => write!(f, "generation"),
        }
    }
}

/// A field-level routing error.
#[derive(Debug, Clone)]
pub struct RoutingError {
    /// 1-based index of the offending element (0 = whole-result error).
    pub index: usize,
    /// The offending field name (e.g. `"segments[3].end.y"`).
    pub field: String,
    /// The error kind.
    pub kind: RoutingErrorKind,
    /// A human-helpful message.
    pub message: String,
}

impl RoutingError {
    pub fn new(
        index: usize,
        field: impl Into<String>,
        kind: RoutingErrorKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            index,
            field: field.into(),
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for RoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.index == 0 {
            write!(f, "{}: {}", self.field, self.message)
        } else {
            write!(
                f,
                "{} (element #{}): {}",
                self.field, self.index, self.message
            )
        }
    }
}

impl std::error::Error for RoutingError {}

impl From<RoutingError> for String {
    fn from(e: RoutingError) -> Self {
        e.to_string()
    }
}
