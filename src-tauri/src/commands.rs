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
use crate::source_selection::SourceSelection;
use crate::transfer_log::{self, TransferLogEntry};

#[tauri::command]
pub fn list_disks() -> Vec<DiskInfo> {
    enumerate_disks()
}

/// The stable volume identifier backing `path` right now (Windows serial
/// number / macOS Volume UUID), or `None` if it can't be determined --
/// powers Resume's Source Index check by letting the frontend compare
/// "what's plugged in now" against what a job recorded at its original start.
#[tauri::command]
pub fn get_volume_signature(path: String) -> Option<String> {
    crate::disks::volume_signature(&path)
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTransferGroupRequest {
    source: String,
    selected_paths: Option<Vec<String>>,
    destinations: Vec<String>,
    mode: TransferGroupMode,
    verification_mode: VerificationMode,
    checksum_algorithm: ChecksumAlgorithm,
    source_name: String,
    organize: OrganizeSettings,
    move_after_transfer: bool,
    move_same_volume: bool,
    legacy_checksum_algorithm: Option<ChecksumAlgorithm>,
    save_log_to_destination: bool,
    create_per_file_mhl: bool,
}

#[tauri::command]
pub fn start_transfer_group(
    app_handle: AppHandle,
    request: StartTransferGroupRequest,
) -> Result<String, String> {
    let source = PathBuf::from(request.source);
    let source_selection = request
        .selected_paths
        .map(|paths| {
            SourceSelection::new(
                source.clone(),
                paths.into_iter().map(PathBuf::from).collect(),
            )
        })
        .transpose()
        .map_err(|error| error.to_string())?;
    Ok(cascade::start_transfer_group(
        app_handle,
        cascade::TransferGroupRequest {
            source,
            source_selection,
            destinations: request
                .destinations
                .into_iter()
                .map(PathBuf::from)
                .collect(),
            mode: request.mode,
            options: crate::copy_engine::TransferOptions {
                verification_mode: request.verification_mode,
                checksum_algorithm: request.checksum_algorithm,
                source_name: request.source_name,
                organize: request.organize,
                move_after_transfer: request.move_after_transfer,
                move_same_volume: request.move_same_volume,
                legacy_checksum_algorithm: request.legacy_checksum_algorithm,
                save_log_to_destination: request.save_log_to_destination,
                create_per_file_mhl: request.create_per_file_mhl,
            },
        },
    ))
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
pub fn rename_disk(mount_point: String, label: String) -> Result<(), String> {
    crate::volume_rename::rename_volume(&mount_point, &label).map_err(|e| e.to_string())
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

#[tauri::command]
pub fn plan_mhl_repair(
    mhl_path: String,
    relative_path: String,
    candidate_roots: Vec<String>,
) -> Result<mhl::RepairPlan, String> {
    let roots: Vec<PathBuf> = candidate_roots.into_iter().map(PathBuf::from).collect();
    mhl::plan_mhl_repair(
        &PathBuf::from(mhl_path),
        &PathBuf::from(relative_path),
        &roots,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn repair_mhl_entry(
    mhl_path: String,
    relative_path: String,
    source_root: String,
    approved: bool,
) -> Result<MhlVerifyReport, String> {
    mhl::repair_mhl_entry_from_report(
        &PathBuf::from(mhl_path),
        &PathBuf::from(relative_path),
        &PathBuf::from(source_root),
        approved,
    )
    .map_err(|e| e.to_string())
}

/// Verifies every `.mhl` file found directly inside `folder` -- the "verify
/// all MHLs on this drive/folder" batch action.
#[tauri::command]
pub fn verify_mhls_in_folder(folder: String) -> Result<Vec<MhlVerifyReport>, String> {
    mhl::verify_mhls_in_folder(&PathBuf::from(folder)).map_err(|e| e.to_string())
}
