use serde::Serialize;
use std::thread;
use std::time::Duration;
use sysinfo::Disks;
use tauri::{AppHandle, Emitter};

const DISKS_CHANGED_EVENT: &str = "disks-changed";
const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub id: String,
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub is_removable: bool,
    pub file_system: String,
}

pub fn enumerate_disks() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|d| {
            let mount_point = d.mount_point().to_string_lossy().to_string();
            let name = {
                let n = d.name().to_string_lossy().to_string();
                if n.trim().is_empty() {
                    mount_point.clone()
                } else {
                    n
                }
            };
            DiskInfo {
                id: mount_point.clone(),
                name,
                mount_point,
                total_bytes: d.total_space(),
                available_bytes: d.available_space(),
                is_removable: d.is_removable(),
                file_system: d.file_system().to_string_lossy().to_string(),
            }
        })
        .collect()
}

/// Polls the OS disk list on a background thread and emits `disks-changed`
/// to the frontend whenever the mounted volumes differ from the last poll.
/// Covers newly inserted SD cards / USB drives without a manual refresh.
pub fn start_watcher(app_handle: AppHandle) {
    thread::spawn(move || {
        let mut last = enumerate_disks();
        let _ = app_handle.emit(DISKS_CHANGED_EVENT, &last);
        loop {
            thread::sleep(POLL_INTERVAL);
            let current = enumerate_disks();
            if current != last {
                if app_handle.emit(DISKS_CHANGED_EVENT, &current).is_err() {
                    // Window/app has closed; stop polling.
                    break;
                }
                last = current;
            }
        }
    });
}
