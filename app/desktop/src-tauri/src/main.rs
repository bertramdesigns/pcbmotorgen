//! pcbmotorgen Tauri host — entry point.
//!
//! Registers all `#[tauri::command]` async handlers from the `commands/`
//! module tree and the `ipc/` DTO layer with the Tauri v2 `Builder`. The
//! frontend (`app/src/`) calls these via `invoke("command_name", { config })`
//! (see `app/src/lib/tauri.ts`).
//!
//! Linear mode only (PRODUCT_GOALS.md §7.A). No radial commands are exposed.

mod commands;
mod config;
mod ipc;
mod plugins;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::physics::compute_config_derived,
            commands::physics::validate_config,
            commands::physics::get_magnet_grades,
            commands::physics::compute_height_stack,
            commands::physics::generate_coils,
            commands::physics::evaluate_force_sweep,
            commands::physics::sample_b_field,
            commands::physics::travel_envelope,
            commands::physics::compute_stackup,
            commands::physics::compute_power_budget,
            commands::physics::compute_friction,
            commands::kicad::connect_kicad,
            commands::kicad::write_coils_to_board,
            commands::kicad::ping_kicad,
            commands::kicad::get_board_diagnostics,
            commands::kicad::validate_write_preconditions,
            commands::kicad::preview_coils,
            commands::dxf::export_coils_dxf,
            commands::routing_plugins::list_routing_patterns,
            commands::routing_plugins::register_routing_plugin,
            commands::routing_plugins::routing_pattern_parameters,
            commands::routing_plugins::check_coil_interference,
            commands::routing_plugins::load_installed_plugins,
            commands::routing_plugins::list_installed_plugins,
            commands::routing_plugins::remove_routing_plugin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running pcbmotorgen tauri application");
}
