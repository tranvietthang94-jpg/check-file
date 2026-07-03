use crate::disks::{enumerate_disks, DiskInfo};

#[tauri::command]
pub fn list_disks() -> Vec<DiskInfo> {
    enumerate_disks()
}
