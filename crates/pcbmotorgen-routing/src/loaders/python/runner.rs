use std::io::Write;
use std::process::{Command, Stdio};

use crate::context::RoutingContext;
use crate::error::{RoutingError, RoutingErrorKind};
use crate::model::RoutingResult;

/// Run a Python runner at `script_path` with `ctx`, returning the parsed
/// (unvalidated) [`RoutingResult`]. Callers pass the result through
/// [`Validator::validate`](crate::validator::Validator::validate).
pub(crate) fn run_python_runner(
    script_path: &std::path::Path,
    ctx: &RoutingContext,
) -> Result<RoutingResult, RoutingError> {
    let input = serde_json::to_string(ctx).map_err(|e| {
        RoutingError::new(
            0,
            "context",
            RoutingErrorKind::Generation,
            format!("failed to serialise routing context for python runner: {e}"),
        )
    })?;

    let mut child = Command::new("python3")
        .arg(script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            RoutingError::new(
                0,
                "loader",
                RoutingErrorKind::Generation,
                format!("failed to start python3 runner {}: {e}", script_path.display()),
            )
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).map_err(|e| {
            RoutingError::new(
                0,
                "loader",
                RoutingErrorKind::Generation,
                format!("failed to write context to python runner: {e}"),
            )
        })?;
    }

    let output = child.wait_with_output().map_err(|e| {
        RoutingError::new(
            0,
            "loader",
            RoutingErrorKind::Generation,
            format!("python runner failed: {e}"),
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RoutingError::new(
            0,
            "loader",
            RoutingErrorKind::Generation,
            format!(
                "python runner {} exited with {}: {}",
                script_path.display(),
                output.status,
                stderr.trim()
            ),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<RoutingResult>(stdout.trim()).map_err(|e| {
        RoutingError::new(
            0,
            "runner_output",
            RoutingErrorKind::Malformed,
            format!(
                "python runner output is not a valid RoutingResult JSON: {e} — it must emit exactly one strict RoutingResult object on stdout",
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;
    use std::collections::HashMap;

    #[test]
    fn parses_valid_runner_json() {
        // Simulate a trivial inline runner by invoking python3 -c via a temp
        // script file that echoes a fixed RoutingResult.
        let dir = std::env::temp_dir().join(format!("pcbrt_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("runner.py");
        std::fs::write(&script, r#"
import json, sys
ctx = json.load(sys.stdin)
# emit a minimal but valid shape
out = {
    "segments": [{
        "start": {"x": 0.0, "y": 0.0},
         "end":   {"x": 0.0, "y": 20.0},
        "layer": 0,
        "net": "A",
        "is_active": True
    }],
    "curves": [],
    "vias": []
}
json.dump(out, sys.stdout)
"#)
        .unwrap();

        let ctx = RoutingContext {
            active_area_length_mm: 100.0,
            board_width_mm: 20.0,
            num_layers: 2,
            phases: 3,
            min_trace_mm: 0.1,
            min_space_mm: 0.1,
            padding_mm: 0.0,
            expects_continuous: false,
            params: HashMap::new(),
            ..RoutingContext::default()
        };

        match run_python_runner(&script, &ctx) {
            Ok(r) => {
                assert_eq!(r.segments.len(), 1);
                assert_eq!(r.segments[0].net, "A");
                assert_eq!(r.segments[0].start, Point::new(0.0, 0.0));
                crate::Validator::validate(&r, &ctx, false).expect("runner shape validates");
                let dimensions = crate::RoutingDimensions::from_result(&r, &ctx)
                    .expect("runner dimensions calculate");
                assert_eq!(dimensions.phase_band_widths.len(), 1);
                assert!(dimensions.pole_pitch_mm.is_none());
            }
            Err(e) => panic!("unexpected parse failure: {e}"),
        }
    }

    #[test]
    fn rejects_bad_runner_json() {
        let dir = std::env::temp_dir().join(format!("pcbrt_test_bad_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("runner.py");
        std::fs::write(&script, "print('not json at all')\n").unwrap();

        let ctx = RoutingContext {
            active_area_length_mm: 100.0,
            board_width_mm: 20.0,
            num_layers: 2,
            phases: 3,
            min_trace_mm: 0.1,
            min_space_mm: 0.1,
            padding_mm: 0.0,
            expects_continuous: false,
            params: HashMap::new(),
            ..RoutingContext::default()
        };

        let err = run_python_runner(&script, &ctx).unwrap_err();
        assert_eq!(err.kind, RoutingErrorKind::Malformed);
        assert!(err.message.contains("RoutingResult"));
    }
}
