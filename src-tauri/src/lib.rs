mod commands;
mod copy_engine;
mod disks;

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
            commands::start_copy,
            commands::cancel_copy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
