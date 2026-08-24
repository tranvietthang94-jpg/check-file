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
        match fs::rename(&temp_path, path) {
            Ok(()) => Ok(()),
            Err(rename_err) if path.exists() => {
                // Windows stdlib rename cannot replace an existing file. Keep
                // the old complete document until the new one is fully synced,
                // then use the smallest available replacement window.
                fs::remove_file(path)?;
                fs::rename(&temp_path, path).map_err(|replace_err| {
                    io::Error::new(
                        replace_err.kind(),
                        format!(
                            "failed to replace {} after rename error {rename_err}: {replace_err}",
                            path.display()
                        ),
                    )
                })
            }
            Err(err) => Err(err),
        }
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
