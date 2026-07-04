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

/// `C:\Users\...` -> `C:\` -- `GetVolumeInformationW` needs the volume's
/// root path (trailing backslash required), not an arbitrary path under it,
/// but a Source's path can be any folder on the card (the per-endpoint
/// "Folder Path" override). Pulled out of the Windows-only FFI module so
/// it's testable on every platform, same split `eject.rs` uses for its own
/// device-path helper.
fn windows_volume_root(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        Some(format!("{}:\\", (bytes[0] as char).to_ascii_uppercase()))
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
mod volume_signature_imp {
    use super::windows_volume_root;
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    // Signature verified against Microsoft Learn's fileapi.h reference
    // before use: `BOOL GetVolumeInformationW(LPCWSTR, LPWSTR, DWORD,
    // LPDWORD, LPDWORD, LPDWORD, LPWSTR, DWORD)`, exported from
    // Kernel32.dll. Only the serial number output is needed here, so every
    // other out-parameter is passed as NULL/0, which the docs say is valid
    // ("This parameter can be NULL if ... is not required").
    #[link(name = "kernel32")]
    extern "system" {
        fn GetVolumeInformationW(
            lpRootPathName: *const u16,
            lpVolumeNameBuffer: *mut u16,
            nVolumeNameSize: u32,
            lpVolumeSerialNumber: *mut u32,
            lpMaximumComponentLength: *mut u32,
            lpFileSystemFlags: *mut u32,
            lpFileSystemNameBuffer: *mut u16,
            nFileSystemNameSize: u32,
        ) -> i32;
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    /// The OS-assigned volume serial number for whichever disk backs
    /// `path` -- reformatted on every real format/reformat of that volume,
    /// so it changes if a different card ends up mounted at the same drive
    /// letter between a job's original run and a later Resume.
    pub fn volume_signature(path: &str) -> Option<String> {
        let root = windows_volume_root(path)?;
        let wide_root = wide(&root);
        let mut serial: u32 = 0;
        let ok = unsafe {
            GetVolumeInformationW(
                wide_root.as_ptr(),
                ptr::null_mut(),
                0,
                &mut serial,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                0,
            )
        };
        if ok != 0 {
            Some(format!("{serial:08X}"))
        } else {
            None
        }
    }
}

#[cfg(target_os = "macos")]
mod volume_signature_imp {
    use std::process::Command;

    /// Shells out to `diskutil info <path>` and reads the "Volume UUID"
    /// line -- same reasoning as `eject.rs`'s macOS path: a stable,
    /// documented CLI tool chosen over a DiskArbitration/IOKit FFI binding
    /// this project has no way to verify without real Mac hardware.
    pub fn volume_signature(path: &str) -> Option<String> {
        let output = Command::new("diskutil").arg("info").arg(path).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix("Volume UUID:") {
                let uuid = rest.trim();
                if !uuid.is_empty() {
                    return Some(uuid.to_string());
                }
            }
        }
        None
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod volume_signature_imp {
    pub fn volume_signature(_path: &str) -> Option<String> {
        None
    }
}

/// A stable identifier for whichever physical volume backs `path` right
/// now (Windows: volume serial number; macOS: Volume UUID) -- used to
/// detect "a different disk is now mounted at this same path" between a
/// job's original run and a Resume of it. `None` when it can't be
/// determined (unsupported platform, or a path that isn't under a
/// recognizable local drive letter/volume); callers should treat that as
/// "can't verify" and fail open rather than block Resume on ambiguity.
pub fn volume_signature(path: &str) -> Option<String> {
    volume_signature_imp::volume_signature(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_volume_root_extracts_the_drive_letter_root() {
        assert_eq!(
            windows_volume_root("C:\\Users\\tranv\\Desktop"),
            Some("C:\\".to_string())
        );
        assert_eq!(windows_volume_root("g:\\PRIVATE\\M4ROOT"), Some("G:\\".to_string()));
    }

    #[test]
    fn windows_volume_root_rejects_a_non_drive_letter_path() {
        assert_eq!(windows_volume_root("\\\\server\\share"), None);
        assert_eq!(windows_volume_root(""), None);
        assert_eq!(windows_volume_root("relative/path"), None);
    }
}
