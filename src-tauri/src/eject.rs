use std::io;

/// `\\.\G:` -- the Win32 device-path form `CreateFileW` needs to open a
/// volume directly (as opposed to a file on it). Pulled out of the
/// Windows-only FFI module so it's testable on every platform.
fn windows_device_path(mount_point: &str) -> String {
    format!("\\\\.\\{}", mount_point.trim_end_matches('\\'))
}

#[cfg(target_os = "windows")]
mod imp {
    use super::windows_device_path;
    use std::ffi::OsStr;
    use std::io;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    // CreateFileW / DeviceIoControl / CloseHandle (kernel32.dll) and the
    // FSCTL_LOCK_VOLUME / FSCTL_DISMOUNT_VOLUME / IOCTL_STORAGE_EJECT_MEDIA
    // control codes -- signatures and numeric values verified against
    // Microsoft Learn and the windows-sys crate docs before use.
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileW(
            lpfilename: *const u16,
            dwdesiredaccess: u32,
            dwsharemode: u32,
            lpsecurityattributes: *mut core::ffi::c_void,
            dwcreationdisposition: u32,
            dwflagsandattributes: u32,
            htemplatefile: *mut core::ffi::c_void,
        ) -> *mut core::ffi::c_void;

        fn DeviceIoControl(
            hdevice: *mut core::ffi::c_void,
            dwiocontrolcode: u32,
            lpinbuffer: *mut core::ffi::c_void,
            ninbuffersize: u32,
            lpoutbuffer: *mut core::ffi::c_void,
            noutbuffersize: u32,
            lpbytesreturned: *mut u32,
            lpoverlapped: *mut core::ffi::c_void,
        ) -> i32;

        fn CloseHandle(hobject: *mut core::ffi::c_void) -> i32;
    }

    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const OPEN_EXISTING: u32 = 3;
    const FSCTL_LOCK_VOLUME: u32 = 589_848;
    const FSCTL_DISMOUNT_VOLUME: u32 = 589_856;
    const IOCTL_STORAGE_EJECT_MEDIA: u32 = 2_967_560;

    fn invalid_handle() -> *mut core::ffi::c_void {
        (-1isize) as *mut core::ffi::c_void
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    /// Locks, dismounts, then ejects the volume at `mount_point` (e.g.
    /// `"G:\\"`). Locking and dismounting are best-effort -- some drivers
    /// refuse them but still honor the eject IOCTL, so their failure alone
    /// doesn't abort the call; only a failed eject is reported as an error.
    pub fn eject(mount_point: &str) -> io::Result<()> {
        let wide_path = wide(&windows_device_path(mount_point));

        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            );
            if handle == invalid_handle() {
                return Err(io::Error::last_os_error());
            }

            let mut bytes_returned: u32 = 0;
            DeviceIoControl(
                handle,
                FSCTL_LOCK_VOLUME,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                0,
                &mut bytes_returned,
                ptr::null_mut(),
            );
            DeviceIoControl(
                handle,
                FSCTL_DISMOUNT_VOLUME,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                0,
                &mut bytes_returned,
                ptr::null_mut(),
            );
            let ejected = DeviceIoControl(
                handle,
                IOCTL_STORAGE_EJECT_MEDIA,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                0,
                &mut bytes_returned,
                ptr::null_mut(),
            );
            let eject_err = if ejected == 0 {
                Some(io::Error::last_os_error())
            } else {
                None
            };

            CloseHandle(handle);

            match eject_err {
                Some(err) => Err(err),
                None => Ok(()),
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::io;
    use std::process::Command;

    /// Shells out to `diskutil eject <mount point>` (e.g.
    /// `/Volumes/CARD`) -- a stable, documented CLI tool, chosen over a
    /// DiskArbitration/IOKit FFI binding this project has no way to verify
    /// without real Mac hardware (see the project plan's documented risk).
    pub fn eject(mount_point: &str) -> io::Result<()> {
        let status = Command::new("diskutil").arg("eject").arg(mount_point).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("diskutil eject exited with status {status}"),
            ))
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod imp {
    use std::io;
    pub fn eject(_mount_point: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "eject is not supported on this platform",
        ))
    }
}

pub fn eject_disk(mount_point: &str) -> io::Result<()> {
    imp::eject(mount_point)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_device_path_strips_trailing_backslash() {
        assert_eq!(windows_device_path("G:\\"), "\\\\.\\G:");
    }

    #[test]
    fn windows_device_path_handles_a_bare_drive_letter() {
        assert_eq!(windows_device_path("G:"), "\\\\.\\G:");
    }
}
