//! Native application menu (kata 0cgm) — the File dropdown carries the
//! project actions (Open / Save / Save As); the top-bar buttons are gone.
//!
//! Rust owns the menu bar. Each project item's id equals the Tauri event
//! name it is forwarded as (see `on_menu_event` in `main.rs`), where
//! `bindProjectMenuActions` (`src/lib/ipc/project.ts`) dispatches into the
//! `ProjectStore` flows — dialogs, busy-guard, and dirty tracking stay in
//! the existing frontend store.

use tauri::{menu::{Menu, MenuItem, PredefinedMenuItem, Submenu}, Wry};

/// Menu item id == webview event name for Open.
pub const OPEN_ID: &str = "menu:open-project";
/// Menu item id == webview event name for Save.
pub const SAVE_ID: &str = "menu:save-project";
/// Menu item id == webview event name for Save As.
pub const SAVE_AS_ID: &str = "menu:save-project-as";

/// Build and install the application menu.
///
/// A custom menu replaces the platform default, so the standard macOS app
/// submenu and an Edit submenu (clipboard) are provided explicitly to keep
/// copy/paste working in text inputs.
pub fn install(app: &tauri::AppHandle) -> tauri::Result<()> {
    let file = file_submenu(app)?;
    let edit = edit_submenu(app)?;

    #[cfg(target_os = "macos")]
    let menu = {
        let app_menu = app_submenu(app)?;
        Menu::with_items(app, &[&app_menu, &file, &edit])?
    };
    #[cfg(not(target_os = "macos"))]
    let menu = Menu::with_items(app, &[&file, &edit])?;

    app.set_menu(menu)?;
    Ok(())
}

/// File menu — the project actions (kata 0cgm).
fn file_submenu(app: &tauri::AppHandle) -> tauri::Result<Submenu<Wry>> {
    let open = MenuItem::with_id(app, OPEN_ID, "Open…", true, Some("CmdOrCtrl+O"))?;
    let save = MenuItem::with_id(app, SAVE_ID, "Save", true, Some("CmdOrCtrl+S"))?;
    let save_as = MenuItem::with_id(
        app,
        SAVE_AS_ID,
        "Save As…",
        true,
        Some("CmdOrCtrl+Shift+S"),
    )?;
    Submenu::with_id_and_items(app, "file", "File", true, &[&open, &save, &save_as])
}

/// Edit menu with the standard clipboard items.
fn edit_submenu(app: &tauri::AppHandle) -> tauri::Result<Submenu<Wry>> {
    #[cfg(target_os = "macos")]
    {
        let undo = PredefinedMenuItem::undo(app, None)?;
        let redo = PredefinedMenuItem::redo(app, None)?;
        let sep1 = PredefinedMenuItem::separator(app)?;
        let cut = PredefinedMenuItem::cut(app, None)?;
        let copy = PredefinedMenuItem::copy(app, None)?;
        let paste = PredefinedMenuItem::paste(app, None)?;
        let sep2 = PredefinedMenuItem::separator(app)?;
        let select_all = PredefinedMenuItem::select_all(app, None)?;
        Submenu::with_id_and_items(
            app,
            "edit",
            "Edit",
            true,
            &[&undo, &redo, &sep1, &cut, &copy, &paste, &sep2, &select_all],
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        let cut = PredefinedMenuItem::cut(app, None)?;
        let copy = PredefinedMenuItem::copy(app, None)?;
        let paste = PredefinedMenuItem::paste(app, None)?;
        let sep = PredefinedMenuItem::separator(app)?;
        let select_all = PredefinedMenuItem::select_all(app, None)?;
        Submenu::with_id_and_items(
            app,
            "edit",
            "Edit",
            true,
            &[&cut, &copy, &paste, &sep, &select_all],
        )
    }
}

/// macOS application submenu (About / Hide / Quit) so the custom menu keeps
/// the standard platform shape.
#[cfg(target_os = "macos")]
fn app_submenu(app: &tauri::AppHandle) -> tauri::Result<Submenu<Wry>> {
    let about = PredefinedMenuItem::about(app, None, None)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let services = PredefinedMenuItem::services(app, None)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let hide = PredefinedMenuItem::hide(app, None)?;
    let hide_others = PredefinedMenuItem::hide_others(app, None)?;
    let show_all = PredefinedMenuItem::show_all(app, None)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit = PredefinedMenuItem::quit(app, None)?;
    Submenu::with_id_and_items(
        app,
        "app",
        "pcbmotorgen",
        true,
        &[
            &about, &sep1, &services, &sep2, &hide, &hide_others, &show_all, &sep3, &quit,
        ],
    )
}
