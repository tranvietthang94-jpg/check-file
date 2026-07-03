use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::cascade::{self, TransferGroupMode};
use crate::checksum::ChecksumAlgorithm;
use crate::copy_engine::{JobRegistry, VerificationMode};
use crate::disks::{enumerate_disks, DiskInfo};
use crate::eject;
use crate::media_scan;
use crate::organize::OrganizeSettings;
use crate::presets::{self, Preset};
use crate::queue::QueueMode;
use crate::transfer_log::{self, TransferLogEntry};

#[tauri::command]
pub fn list_disks() -> Vec<DiskInfo> {
    enumerate_disks()
}

#[tauri::command]
pub fn cancel_copy(registry: State<JobRegistry>, job_id: String) -> bool {
    registry.cancel(&job_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn start_transfer_group(
    app_handle: AppHandle,
    source: String,
    destinations: Vec<String>,
    mode: TransferGroupMode,
    verification_mode: VerificationMode,
    checksum_algorithm: ChecksumAlgorithm,
    source_name: String,
    organize: OrganizeSettings,
) -> String {
    cascade::start_transfer_group(
        app_handle,
        PathBuf::from(source),
        destinations.into_iter().map(PathBuf::from).collect(),
        mode,
        verification_mode,
        checksum_algorithm,
        source_name,
        organize,
    )
}

#[tauri::command]
pub fn start_media_scan(app_handle: AppHandle, folder: String) -> String {
    media_scan::start_media_scan(app_handle, PathBuf::from(folder))
}

#[tauri::command]
pub fn save_preset(app_handle: AppHandle, preset: Preset) -> Result<(), String> {
    presets::save_preset(&app_handle, &preset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_presets(app_handle: AppHandle) -> Result<Vec<Preset>, String> {
    presets::list_presets(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_preset(app_handle: AppHandle, name: String) -> Result<(), String> {
    presets::delete_preset(&app_handle, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_transfer_logs(app_handle: AppHandle) -> Result<Vec<TransferLogEntry>, String> {
    transfer_log::list_logs(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn eject_disk(mount_point: String) -> Result<(), String> {
    eject::eject_disk(&mount_point).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_prevent_sleep(registry: State<JobRegistry>, enabled: bool) {
    registry.set_sleep_prevention_enabled(enabled);
}

#[tauri::command]
pub fn set_queue_mode(registry: State<JobRegistry>, mode: QueueMode) {
    registry.set_queue_mode(mode);
}
