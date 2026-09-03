use std::sync::{Mutex, OnceLock};

use crate::{RoutingPattern, RoutingRegistry};

/// Runtime-loaded patterns (native crate plugins / python runners) registered
/// by the app at runtime. Consulted before the bundled patterns on lookup.
static RUNTIME: OnceLock<Mutex<RoutingRegistry>> = OnceLock::new();

pub(crate) fn runtime() -> &'static Mutex<RoutingRegistry> {
    RUNTIME.get_or_init(|| Mutex::new(RoutingRegistry::new()))
}

/// Build the registry of bundled patterns.
pub fn bundled_registry() -> RoutingRegistry {
    crate::patterns::bundled()
}

/// List available pattern ids for the UI (bundled + runtime-loaded).
pub fn available_pattern_ids() -> Vec<(String, String)> {
    let mut out = bundled_registry().catalog();
    if let Ok(lock) = runtime().lock() {
        for (id, name) in lock.catalog() {
            if !out.iter().any(|(i, _)| i == &id) {
                out.push((id, name));
            }
        }
    }
    out
}

/// List available pattern metadata (bundled + runtime-loaded). Mirrors
/// [`available_pattern_ids`] but returns the full [`crate::PluginMetadata`]
/// block, including the pattern-declared layer-range constraints the UI
/// mirrors onto its inputs.
pub fn available_pattern_metadata() -> Vec<crate::PluginMetadata> {
    let mut out = bundled_registry().metadata_catalog();
    if let Ok(lock) = runtime().lock() {
        for m in lock.metadata_catalog() {
            if !out.iter().any(|e| e.id == m.id) {
                out.push(m);
            }
        }
    }
    out
}

/// Register a runtime-loaded pattern (native plugin or python runner) into the
/// app-wide registry. Returns `Err` with a helpful message if the `id` is
/// already taken by a bundled pattern (bundled patterns cannot be shadowed).
pub fn register_runtime_pattern(pattern: Box<dyn RoutingPattern>) -> Result<(), String> {
    let id = pattern.id().to_string();
    if bundled_registry().contains(&id) {
        return Err(format!(
            "routing pattern \"{id}\" is a built-in bundled pattern and cannot be overridden by a loaded plugin"
        ));
    }
    let mut guard = runtime()
        .lock()
        .map_err(|_| "runtime registry lock poisoned".to_string())?;
    guard.register_boxed(pattern);
    Ok(())
}

/// Unregister a runtime-loaded pattern by id (no-op for bundled patterns).
pub fn unregister_runtime_pattern(id: &str) {
    if bundled_registry().contains(id) {
        return;
    }
    if let Ok(mut guard) = runtime().lock() {
        guard.remove(id);
    }
}