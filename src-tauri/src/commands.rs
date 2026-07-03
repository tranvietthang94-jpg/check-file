use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::cascade::{self, TransferGroupMode};
use crate::checksum::ChecksumAlgorithm;
use crate::copy_engine::{JobRegistry, VerificationMode};
use crate::disks::{enumerate_disks, DiskInfo};

#[tauri::command]
pub fn list_disks() -> Vec<DiskInfo> {
    enumerate_disks()
}

#[tauri::command]
pub fn cancel_copy(registry: State<JobRegistry>, job_id: String) -> bool {
    registry.cancel(&job_id)
}

#[tauri::command]
pub fn start_transfer_group(
    app_handle: AppHandle,
    source: String,
    destinations: Vec<String>,
    mode: TransferGroupMode,
    verification_mode: VerificationMode,
    checksum_algorithm: ChecksumAlgorithm,
) -> String {
    cascade::start_transfer_group(
        app_handle,
        PathBuf::from(source),
        destinations.into_iter().map(PathBuf::from).collect(),
        mode,
        verification_mode,
        checksum_algorithm,
    )
}
