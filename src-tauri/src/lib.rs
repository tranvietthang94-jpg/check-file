mod atomic_file;
mod cascade;
mod checksum;
mod commands;
mod copy_engine;
mod dedup;
mod disks;
mod eject;
pub mod explorer_integration;
mod media_scan;
mod metadata;
mod mhl;
mod organize;
mod path_safety;
mod power;
mod presets;
mod queue;
mod reports;
mod transfer_log;
mod volume_rename;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    let startup_activation =
        explorer_integration::parse_explorer_activation(std::env::args_os());
    #[cfg(not(windows))]
    let startup_activation = explorer_integration::ExplorerActivation::None;

    let builder = tauri::Builder::default()
        .manage(explorer_integration::ExplorerPendingState::new(
            startup_activation,
        ));

    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
        explorer_integration::handle_secondary_instance(app, args);
    }));

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(copy_engine::JobRegistry::default())
        .setup(|app| {
            disks::start_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_disks,
            commands::get_volume_signature,
            commands::cancel_copy,
            commands::resolve_broken_media,
            commands::start_transfer_group,
            commands::start_media_scan,
            commands::save_preset,
            commands::list_presets,
            commands::delete_preset,
            commands::list_transfer_logs,
            commands::eject_disk,
            commands::rename_disk,
            commands::set_prevent_sleep,
            commands::set_queue_mode,
            commands::generate_report,
            commands::verify_mhl,
            commands::plan_mhl_repair,
            commands::repair_mhl_entry,
            commands::verify_mhls_in_folder,
            explorer_integration::explorer_frontend_ready,
            explorer_integration::acknowledge_explorer_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
