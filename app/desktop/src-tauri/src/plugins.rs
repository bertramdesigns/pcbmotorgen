//! Persistence of installed routing-pattern generators.
//!
//! Uploaded generators (native `cdylib` or Python runners) are copied into the
//! app's data directory under `plugins/`, and a `plugins.json` manifest records
//! each installed plugin. On app startup the manifest is re-read and each
//! plugin re-registered so installed generators survive restarts.
//!
//! Metadata (author / version / description) is captured from the plugin at
//! registration time and stored alongside its file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

const PLUGINS_SUBDIR: &str = "plugins";
const MANIFEST_FILE: &str = "plugins.json";

/// A persistently-installed routing-pattern generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    /// Registry id (also the stored filename stem).
    pub id: String,
    /// `"native"` (cdylib) or `"python"` (runner).
    pub kind: String,
    /// Filename inside the plugins data dir.
    pub file: String,
    pub display_name: String,
    pub author: String,
    pub version: String,
    pub description: String,
}

impl InstalledPlugin {
    /// Build a manifest entry from the runtime metadata of a just-registered
    /// pattern, copying `src` into the plugins dir under a safe filename.
    pub fn persist(
        app: &tauri::AppHandle,
        id: &str,
        kind: &str,
        src: &Path,
        meta: &pcbmotorgen_routing::PluginMetadata,
    ) -> Result<Self, String> {
        let dir = plugins_dir(app)?;
        let file = format!("{}.{}", slug(id), extension_for(kind));
        let dest = dir.join(&file);
        std::fs::copy(src, &dest)
            .map_err(|e| format!("failed to copy generator into app storage: {e}"))?;
        Ok(InstalledPlugin {
            id: id.to_string(),
            kind: kind.to_string(),
            file,
            display_name: if meta.display_name.is_empty() { id.to_string() } else { meta.display_name.clone() },
            author: meta.author.clone(),
            version: meta.version.clone(),
            description: meta.description.clone(),
        })
    }

    /// The absolute path to the stored file.
    pub fn stored_path(&self, app: &tauri::AppHandle) -> Result<PathBuf, String> {
        Ok(plugins_dir(app)?.join(&self.file))
    }
}

/// Register a just-loaded pattern into the persistent store: copy the source
/// file into the app data dir and append its metadata to the manifest.
pub fn register_and_persist(
    app: &tauri::AppHandle,
    id: &str,
    kind: &str,
    src: &Path,
) -> Result<(), String> {
    // Fetch the runtime metadata for the freshly-registered pattern.
    let meta = pcbmotorgen_routing::pattern_metadata(id)
        .ok_or_else(|| format!("pattern {id} registered but metadata lookup failed"))?;
    let plugin = InstalledPlugin::persist(app, id, kind, src, &meta)?;
    save_plugin(app, &plugin)
}

/// The app's plugins data directory (created if missing).
pub fn plugins_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("could not resolve app data dir: {e}"))?;
    let dir = base.join(PLUGINS_SUBDIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create plugins dir: {e}"))?;
    Ok(dir)
}

fn manifest_path(dir: &Path) -> PathBuf {
    dir.join(MANIFEST_FILE)
}

/// Read the installed-plugins manifest (empty list if absent/corrupt).
pub fn read_manifest(app: &tauri::AppHandle) -> Vec<InstalledPlugin> {
    match plugins_dir(app).ok().and_then(|d| std::fs::read_to_string(manifest_path(&d)).ok()) {
        Some(s) => serde_json::from_str(&s).unwrap_or_default(),
        None => Vec::new(),
    }
}

fn write_manifest(app: &tauri::AppHandle, plugins: &[InstalledPlugin]) -> Result<(), String> {
    let dir = plugins_dir(app)?;
    let s = serde_json::to_string_pretty(plugins).map_err(|e| format!("serialize manifest: {e}"))?;
    std::fs::write(manifest_path(&dir), s).map_err(|e| format!("write manifest: {e}"))
}

/// Add or update a plugin in the manifest and persist it.
pub fn save_plugin(app: &tauri::AppHandle, plugin: &InstalledPlugin) -> Result<(), String> {
    let mut all = read_manifest(app);
    all.retain(|p| p.id != plugin.id);
    all.push(plugin.clone());
    write_manifest(app, &all)
}

/// Remove a plugin from the manifest and delete its stored file.
pub fn remove_plugin(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let dir = plugins_dir(app)?;
    let mut all = read_manifest(app);
    if let Some(idx) = all.iter().position(|p| p.id == id) {
        let p = all.remove(idx);
        let dest = dir.join(&p.file);
        if dest.exists() {
            let _ = std::fs::remove_file(&dest);
        }
    }
    write_manifest(app, &all)
}

/// List installed plugins (read from the manifest).
pub fn list_plugins(app: &tauri::AppHandle) -> Vec<InstalledPlugin> {
    read_manifest(app)
}

/// Sanitise a plugin id into a safe lowercase filename stem.
fn slug(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

fn extension_for(kind: &str) -> &'static str {
    match kind {
        "python" => "py",
        "native" => {
            if cfg!(target_os = "macos") {
                "dylib"
            } else if cfg!(target_os = "windows") {
                "dll"
            } else {
                "so"
            }
        }
        _ => "bin",
    }
}
