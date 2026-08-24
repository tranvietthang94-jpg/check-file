mod atomic_file;
mod cascade;
mod checksum;
mod commands;
mod copy_engine;
mod dedup;
mod disks;
mod eject;
mod media_scan;
mod metadata;
mod mhl;
mod organize;
mod power;
mod presets;
mod queue;
mod reports;
mod transfer_log;
mod volume_rename;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            commands::verify_mhls_in_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
