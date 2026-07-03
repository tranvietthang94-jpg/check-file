use std::path::PathBuf;

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::copy_engine::{self, JobRegistry};
use crate::disks::{enumerate_disks, DiskInfo};

#[tauri::command]
pub fn list_disks() -> Vec<DiskInfo> {
    enumerate_disks()
}

#[tauri::command]
pub fn start_copy(
    app_handle: AppHandle,
    registry: State<JobRegistry>,
    source: String,
    destination: String,
) -> String {
    let job_id = Uuid::new_v4().to_string();
    let cancel_flag = registry.register(job_id.clone());

    let source_path = PathBuf::from(source);
    let dest_path = PathBuf::from(destination);
    let job_id_for_thread = job_id.clone();

    std::thread::spawn(move || {
        copy_engine::run_copy_job(
            app_handle,
            job_id_for_thread,
            source_path,
            dest_path,
            cancel_flag,
        );
    });

    job_id
}

#[tauri::command]
pub fn cancel_copy(registry: State<JobRegistry>, job_id: String) -> bool {
    registry.cancel(&job_id)
}
