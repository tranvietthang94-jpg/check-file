use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::cascade::{self, TransferGroupMode};
use crate::checksum::ChecksumAlgorithm;
use crate::copy_engine::{JobRegistry, VerificationMode};
use crate::disks::{enumerate_disks, DiskInfo};
use crate::media_scan;
use crate::organize::OrganizeSettings;

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
