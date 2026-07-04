use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::cascade::{self, TransferGroupMode};
use crate::checksum::ChecksumAlgorithm;
use crate::copy_engine::{JobRegistry, VerificationMode};
use crate::disks::{enumerate_disks, DiskInfo};
use crate::eject;
use crate::media_scan;
use crate::mhl::{self, MhlVerifyReport};
use crate::organize::OrganizeSettings;
use crate::presets::{self, Preset};
use crate::queue::QueueMode;
use crate::reports::{self, ReportRequest};
use crate::transfer_log::{self, TransferLogEntry};

#[tauri::command]
pub fn list_disks() -> Vec<DiskInfo> {
    enumerate_disks()
}

#[tauri::command]
pub fn cancel_copy(registry: State<JobRegistry>, job_id: String) -> bool {
    registry.cancel(&job_id)
}

/// Resolves a pending Broken Media alert for `job_id`: `proceed = true`
/// continues the copy, `false` aborts it (reported as cancelled).
#[tauri::command]
pub fn resolve_broken_media(registry: State<JobRegistry>, job_id: String, proceed: bool) {
    registry.resolve_broken_media(&job_id, proceed);
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
    move_after_transfer: bool,
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
        move_after_transfer,
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

#[tauri::command]
pub fn generate_report(app_handle: AppHandle, request: ReportRequest) -> Result<String, String> {
    let all_logs = transfer_log::list_logs(&app_handle).map_err(|e| e.to_string())?;
    let entries: Vec<TransferLogEntry> = request
        .job_ids
        .iter()
        .filter_map(|id| all_logs.iter().find(|log| &log.job_id == id).cloned())
        .collect();
    let path = reports::save_report(&app_handle, &entries, &request).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

/// Verifies one `.mhl` file against the real files on disk, without running
/// a transfer -- the "double-click an MHL to verify it" action.
#[tauri::command]
pub fn verify_mhl(path: String) -> Result<MhlVerifyReport, String> {
    mhl::verify_mhl_file(&PathBuf::from(path)).map_err(|e| e.to_string())
}

/// Verifies every `.mhl` file found directly inside `folder` -- the "verify
/// all MHLs on this drive/folder" batch action.
#[tauri::command]
pub fn verify_mhls_in_folder(folder: String) -> Result<Vec<MhlVerifyReport>, String> {
    mhl::verify_mhls_in_folder(&PathBuf::from(folder)).map_err(|e| e.to_string())
}
