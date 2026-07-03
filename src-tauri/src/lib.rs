mod cascade;
mod checksum;
mod commands;
mod copy_engine;
mod dedup;
mod disks;
mod media_scan;
mod metadata;
mod organize;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(copy_engine::JobRegistry::default())
        .setup(|app| {
            disks::start_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_disks,
            commands::cancel_copy,
            commands::start_transfer_group,
            commands::start_media_scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
