use std::io;

/// `SetVolumeLabelW` requires the root path to end with a trailing
/// backslash (verified against Microsoft Learn's
/// `winbase.h`/`SetVolumeLabelW` docs before use) -- e.g. `"G:"` must become
/// `"G:\\"`. Pulled out of the Windows-only FFI module so it's testable on
/// every platform.
fn windows_root_path(mount_point: &str) -> String {
    let trimmed = mount_point.trim_end_matches('\\');
    format!("{trimmed}\\")
}

#[cfg(target_os = "windows")]
mod imp {
    use super::windows_root_path;
    use std::ffi::OsStr;
    use std::io;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    // SetVolumeLabelW (kernel32.dll) -- signature verified against
    // Microsoft Learn's winbase.h reference before use: nonzero return on
    // success, 0 on failure (check GetLastError), NULL label clears it.
    #[link(name = "kernel32")]
    extern "system" {
        fn SetVolumeLabelW(lp_root_path_name: *const u16, lp_volume_name: *const u16) -> i32;
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    /// Renames the volume label at `mount_point` (e.g. `"G:\\"`) to `label`.
    /// The 32-char (11 for FAT) length limit is enforced by Windows itself;
    /// a too-long label simply fails the call rather than being silently
    /// truncated, so no client-side validation is duplicated here.
    pub fn rename(mount_point: &str, label: &str) -> io::Result<()> {
        let wide_root = wide(&windows_root_path(mount_point));
        let wide_label = wide(label);

        let ok = unsafe { SetVolumeLabelW(wide_root.as_ptr(), wide_label.as_ptr()) };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::io;
    use std::process::Command;

    /// Shells out to `diskutil rename <mount point> <label>` -- a stable,
    /// documented CLI tool, chosen for the same reason as `eject.rs`'s
    /// `diskutil eject`: no way to verify a raw DiskArbitration/IOKit FFI
    /// binding without real Mac hardware.
    pub fn rename(mount_point: &str, label: &str) -> io::Result<()> {
        let status = Command::new("diskutil")
            .arg("rename")
            .arg(mount_point)
            .arg(label)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("diskutil rename exited with status {status}"),
            ))
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod imp {
    use std::io;
    pub fn rename(_mount_point: &str, _label: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "renaming a volume is not supported on this platform",
        ))
    }
}

pub fn rename_volume(mount_point: &str, label: &str) -> io::Result<()> {
    imp::rename(mount_point, label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_root_path_adds_trailing_backslash() {
        assert_eq!(windows_root_path("G:"), "G:\\");
    }

    #[test]
    fn windows_root_path_keeps_a_single_trailing_backslash() {
        assert_eq!(windows_root_path("G:\\"), "G:\\");
    }
}
