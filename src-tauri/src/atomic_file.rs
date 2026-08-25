use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Writes a complete file through a same-directory temporary path, flushes it
/// to the storage device, then renames it into place. Readers therefore see
/// either the previous complete file or the new complete file, never a
/// partially-written JSON/XML document.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut temp_name = file_name.to_os_string();
    temp_name.push(".tmp");
    let temp_path = path.with_file_name(temp_name);

    let result = (|| {
        let mut file = File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        replace_file(&temp_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(windows)]
pub fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "Kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, new_name: *const u16, flags: u32) -> i32;
    }

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let ok = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_replacement_keeps_the_previous_complete_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("record.json");
        fs::write(&path, b"old complete data").unwrap();
        let missing = dir.path().join("missing.tmp");

        assert!(replace_file(&missing, &path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old complete data");
    }

    #[test]
    fn replaces_existing_content_and_removes_the_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("record.json");
        fs::write(&path, b"old").unwrap();

        write_atomic(&path, b"new complete data").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new complete data");
        assert!(!dir.path().join("record.json.tmp").exists());
    }
}
