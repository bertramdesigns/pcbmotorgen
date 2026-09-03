//! Pattern registry.
//!
//! Patterns register into a [`RoutingRegistry`] (bundled at startup, or added
//! at runtime from a loaded crate plugin / Python runner). The registry is the
//! single point that resolves a pattern `id` → concrete [`RoutingPattern`].

use std::collections::BTreeMap;

use crate::pattern::RoutingPattern;

// Re-export for convenience in the public prelude.
pub use crate::error::RoutingErrorKind;

/// A registry of loadable routing patterns keyed by pattern id.
#[derive(Default)]
pub struct RoutingRegistry {
    patterns: BTreeMap<String, Box<dyn RoutingPattern>>,
}

impl RoutingRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pattern; replaces any existing pattern with the same id.
    pub fn register(&mut self, pattern: impl RoutingPattern + 'static) {
        let id = pattern.id().to_string();
        self.patterns.insert(id, Box::new(pattern));
    }

    /// Register a boxed pattern.
    pub fn register_boxed(&mut self, pattern: Box<dyn RoutingPattern>) {
        let id = pattern.id().to_string();
        self.patterns.insert(id, pattern);
    }

    /// Remove a pattern by id. Returns true if a pattern was removed.
    pub fn remove(&mut self, id: &str) -> bool {
        self.patterns.remove(id).is_some()
    }

    /// Look up a pattern by id.
    pub fn get(&self, id: &str) -> Option<&dyn RoutingPattern> {
        self.patterns.get(id).map(|p| p.as_ref())
    }

    /// Is a pattern with this id registered?
    pub fn contains(&self, id: &str) -> bool {
        self.patterns.contains_key(id)
    }

    /// All registered pattern ids, sorted.
    pub fn ids(&self) -> Vec<String> {
        self.patterns.keys().cloned().collect()
    }

    /// (id, display_name) pairs for the UI selector.
    pub fn catalog(&self) -> Vec<(String, String)> {
        self.patterns
            .values()
            .map(|p| (p.id().to_string(), p.display_name().to_string()))
            .collect()
    }

    /// Full metadata blocks for the UI catalog (includes the layer-range
    /// constraints declared via the trait's default methods).
    pub fn metadata_catalog(&self) -> Vec<crate::PluginMetadata> {
        self.patterns.values().map(|p| p.metadata()).collect()
    }

    /// Number of registered patterns.
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}
