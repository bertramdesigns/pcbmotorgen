//! Default KiCad IPC socket path resolution & client name generation.

use std::path::Path;

/// Resolves the default KiCad IPC socket path.
///
/// Priority:
/// 1. `KICAD_API_SOCKET` environment variable (handled by caller).
/// 2. Flatpak cache path (if it exists on non-Windows).
/// 3. Platform default:
///    - macOS/Linux: `ipc:///tmp/kicad/api.sock`
///    - Windows: `ipc://%TEMP%\kicad\api.sock`
pub(super) fn default_socket_path() -> String {
    if cfg!(target_os = "windows") {
        let temp = std::env::var("TEMP").unwrap_or_else(|_| "C:\\temp".to_string());
        format!("ipc://{temp}\\kicad\\api.sock")
    } else {
        // Check for KiCad flatpak socket on non-Windows.
        if let Some(home) = std::env::var_os("HOME") {
            let flatpak_socket = Path::new(&home)
                .join(".var/app/org.kicad.KiCad/cache/tmp/kicad/api.sock");
            if flatpak_socket.exists() {
                return format!("ipc://{}", flatpak_socket.display());
            }
        }
        "ipc:///tmp/kicad/api.sock".to_string()
    }
}

/// Generates a random client name: `pcbmotorgen-<8 alphanumeric chars>`.
///
/// Mirrors the Python `kipy._random_client_name()` pattern.
pub(crate) fn random_client_name() -> String {
    use rand::Rng;
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    format!("pcbmotorgen-{suffix}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_client_name_format() {
        let name = random_client_name();
        assert!(name.starts_with("pcbmotorgen-"));
        assert_eq!(name.len(), "pcbmotorgen-".len() + 8);
    }

    #[test]
    fn test_default_socket_path_non_windows() {
        // On non-Windows, should be the default unless flatpak path exists.
        // We can't control the flatpak path in CI, so just check the format.
        let path = default_socket_path();
        assert!(
            path.starts_with("ipc://"),
            "socket path should start with ipc://, got: {path}"
        );
    }
}