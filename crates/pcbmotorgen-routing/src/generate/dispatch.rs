use super::adapters::routing_result_to_phase_coils;
use super::params_api::validate_routing_params;
use super::runtime_registry::{bundled_registry, register_runtime_pattern, runtime};
use crate::context::RoutingContext;
use crate::coil::PhaseCoil;
use crate::dimensions::RoutingDimensions;
use crate::model::RoutingResult;
use crate::report::RoutingReport;
use crate::validator::Validator;
use crate::RoutingPattern;

/// Generate a validated [`RoutingReport`] for the given context and pattern
/// id.  The report contains the canonical geometry plus the calculated
/// pole/slot dimensions used to hand the traces off to magnet-pattern code.
pub fn generate_routing_report(ctx: &RoutingContext, id: &str) -> Result<RoutingReport, String> {
    validate_routing_params(id, &ctx.params)?;

    if let Ok(guard) = runtime().lock() {
        if let Some(p) = guard.get(id) {
            return generate_with_report(p, ctx, id);
        }
    }

    let reg = bundled_registry();
    let pattern = reg.get(id).ok_or_else(|| {
        format!(
            "routing pattern \"{id}\" is not registered. Available: {}",
            reg.ids().join(", ")
        )
    })?;
    generate_with_report(pattern, ctx, id)
}

/// Generate a validated [`RoutingResult`] for the given context and pattern id.
///
/// Rejects out-of-range user parameters (validated against the pattern's
/// declared schema) before generation. Prefers a runtime-loaded pattern, then
/// falls back to the bundled registry.
pub fn generate_routing_result(ctx: &RoutingContext, id: &str) -> Result<RoutingResult, String> {
    generate_routing_report(ctx, id).map(|report| report.result)
}

fn generate_with_report(
    pattern: &dyn RoutingPattern,
    ctx: &RoutingContext,
    id: &str,
) -> Result<RoutingReport, String> {
    let result = pattern
        .generate(ctx)
        .map_err(|e| format!("pattern \"{id}\" failed: {e}"))?;
    Validator::validate(&result, ctx, pattern.expects_continuous()).map_err(|e| {
        format!("pattern \"{id}\" produced a malformed shape and was rejected: {e}")
    })?;

    // The plugin wire contract remains geometry-only.  The host computes the
    // dimensions after validation, using the exact same context that was used
    // to generate the geometry.  The bundled infinity braid has a known
    // diamond angle and strand count; generic plugins use their active
    // geometry and the conventional trace-count parameter when present.
    let dimensions = if id == "infinity-braid" {
        let trace_count = ctx.param("num_strands", 5.0).round().max(1.0) as u32;
        let (period_pitch, period_count) = infinity_period(ctx);
        RoutingDimensions::for_infinity(
            &result,
            ctx,
            trace_count,
            period_pitch,
            period_count,
        )
    } else {
        RoutingDimensions::from_result(&result, ctx)
    }
    .map_err(|e| format!("pattern \"{id}\" dimensions were rejected: {e}"))?;

    Ok(RoutingReport { result, dimensions })
}

/// Convenience: generate validated `PhaseCoil`s for a context (the presentation
/// consumed by the preview and the simulation/force model).
///
/// Returns an empty vec on a malformed / unresolved pattern — callers surface
/// the detailed error via [`generate_routing_result`] when needed.
pub fn generate_coils_from_context(ctx: &RoutingContext, id: &str) -> Vec<PhaseCoil> {
    match generate_routing_result(ctx, id) {
        Ok(result) => routing_result_to_phase_coils(&result, id),
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("routing generation failed: {e}");
            Vec::new()
        }
    }
}

/// Load and register a native `cdylib` routing-pattern plugin into the runtime
/// registry. The loaded pattern is validated on registration by generating for
/// the given probe context (a malformed plugin is rejected with a helpful
/// error).
pub fn register_native_plugin(
    path: &std::path::Path,
    probe: &RoutingContext,
) -> Result<String, String> {
    // SAFETY: loading an arbitrary native library executes its constructor/fini
    // code; only load plugins from a trusted source (the user explicitly
    // selected the file in the app).
    let plugin = unsafe { crate::loaders::native::NativePlugin::load(path) }
        .map_err(|e| format!("failed to load native plugin: {e}"))?;
    let id = plugin.id().to_string();
    // Probe: generate for the reference context to reject malformed plugins now.
    let result = plugin
        .generate(probe)
        .map_err(|e| format!("plugin \"{id}\" failed to generate: {e}"))?;
    Validator::validate(&result, probe, plugin.expects_continuous())
        .map_err(|e| format!("plugin \"{id}\" rejected on upload: {e}"))?;
    register_runtime_pattern(Box::new(plugin))?;
    Ok(id)
}

/// Load and register a Python runner routing pattern into the runtime registry.
/// The runner is probed against the given context and rejected on upload if its
/// output is malformed. Optional metadata (author/version/description) and
/// declared parameters are read from the runner's `--metadata` mode when
/// present; `custom_id` (optional) overrides the derived registry key.
pub fn register_python_runner(
    path: &std::path::Path,
    probe: &RoutingContext,
    custom_id: Option<&str>,
) -> Result<String, String> {
    let mut pattern = crate::loaders::python::PythonRunnerPattern::from_script(path.to_path_buf());
    // Apply metadata / declared parameters from the runner if it supports them.
    if let Ok(Some(meta)) = crate::loaders::python::python_metadata(path) {
        if let Some(id) = custom_id {
            pattern = crate::loaders::python::PythonRunnerPattern::new(
                id,
                meta.display_name.clone(),
                path,
            );
        } else if !meta.id.is_empty() {
            pattern = crate::loaders::python::PythonRunnerPattern::new(
                meta.id.clone(),
                meta.display_name.clone(),
                path,
            );
        }
        pattern.set_metadata(&crate::PluginMetadata {
            id: pattern.id().to_string(),
            display_name: meta.display_name,
            author: meta.author,
            version: meta.version,
            description: meta.description,
        });
        pattern.set_parameters(meta.parameters);
    } else if let Some(id) = custom_id {
        pattern = crate::loaders::python::PythonRunnerPattern::new(id, id, path);
    }

    let id = pattern.id().to_string();
    let result = pattern
        .generate(probe)
        .map_err(|e| format!("python runner \"{id}\" failed to generate: {e}"))?;
    Validator::validate(&result, probe, pattern.expects_continuous())
        .map_err(|e| format!("python runner \"{id}\" rejected on upload: {e}"))?;
    register_runtime_pattern(Box::new(pattern))?;
    Ok(id)
}

/// Return the exact period metadata used by the bundled infinity braid.
///
/// When a pole pitch is present the braid reserves its uniform phase/strand
/// interleave step, then uses complete periods that fit in the routable length;
/// each period is exactly one pole pitch. A runner or legacy probe without
/// magnet data uses its explicit fallback period count; its generated repeat
/// pitch is reported, but it is not presented as magnet-aligned because there
/// is no pole pitch to compare it with.
fn infinity_period(ctx: &RoutingContext) -> (Option<f64>, Option<u32>) {
    let total = ctx.active_area_length_mm + 2.0 * ctx.padding_mm;
    if let Some(pole_pitch) = ctx.magnet_pitch() {
        let phases = ctx.phases.max(1) as f64;
        let strands = ctx.param("num_strands", 5.0).max(2.0);
        let interleave_step = pole_pitch / (phases * strands);
        let periods = ((total + interleave_step) / pole_pitch).floor() as i64 - 1;
        return (Some(pole_pitch), Some(periods.max(1) as u32));
    }

    let phases = ctx.phases.max(1) as i64;
    let strands = (ctx.param("num_strands", 5.0) as i64).max(2);
    let periods = (ctx.param("n_periods", 4.0) as i64).max(1);
    let offset = total / ((periods + 1) * strands * phases - 1) as f64 * -1.0;
    let phase_span = total - ((offset * strands as f64 * -1.0) * phases as f64) - offset;
    (Some(phase_span / periods as f64), Some(periods as u32))
}
