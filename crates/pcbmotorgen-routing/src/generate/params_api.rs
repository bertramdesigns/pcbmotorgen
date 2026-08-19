use super::runtime_registry::{bundled_registry, runtime};

/// Look up a pattern's declared user-editable parameters by id (bundled or
/// runtime-loaded). Returns an empty vec if the id is unknown.
pub fn pattern_parameters(id: &str) -> Vec<crate::PatternParameter> {
    if let Ok(guard) = runtime().lock() {
        if let Some(p) = guard.get(id) {
            return p.parameters();
        }
    }
    let reg = bundled_registry();
    match reg.get(id) {
        Some(p) => p.parameters(),
        None => Vec::new(),
    }
}

/// Look up a pattern's full metadata block (author/version/description) by id.
pub fn pattern_metadata(id: &str) -> Option<crate::PluginMetadata> {
    if let Ok(guard) = runtime().lock() {
        if let Some(p) = guard.get(id) {
            return Some(p.metadata());
        }
    }
    let reg = bundled_registry();
    reg.get(id).map(|p| p.metadata())
}

/// Validate a pattern id against its declared parameter schema, clamping /
/// rejecting out-of-range values. Returns a helpful error on the first bad
/// parameter.
pub fn validate_routing_params(
    id: &str,
    params: &std::collections::HashMap<String, f64>,
) -> Result<(), String> {
    let schema = pattern_parameters(id);
    if schema.is_empty() {
        return Ok(());
    }
    for p in &schema {
        if let Some(v) = params.get(&p.key) {
            if !v.is_finite() {
                return Err(format!(
                    "{} = {} is not finite for pattern \"{id}\" — use a finite number",
                    p.label, v
                ));
            }
            if matches!(p.param_type, crate::ParamType::Int)
                && (v.round() - *v).abs() > 1e-9
            {
                return Err(format!(
                    "{} = {} is not an integer for pattern \"{id}\"",
                    p.label, v
                ));
            }
            if let Some(min) = p.min {
                if *v < min - 1e-9 {
                    return Err(format!(
                        "{} = {} is below the minimum {} for pattern \"{id}\"",
                        p.label, v, min
                    ));
                }
            }
            if let Some(max) = p.max {
                if *v > max + 1e-9 {
                    return Err(format!(
                        "{} = {} exceeds the maximum {} for pattern \"{id}\"",
                        p.label, v, max
                    ));
                }
            }
        }
    }
    Ok(())
}
