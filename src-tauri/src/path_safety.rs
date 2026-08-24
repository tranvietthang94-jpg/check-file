use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Rejects any existing symlink/junction/reparse-point component between
/// `root` and `relative`, so writes cannot escape through filesystem links.
pub fn safe_destination(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    let mut current = root.to_path_buf();
    reject_link(&current)?;
    for component in relative.components() {
        use std::path::Component;
        match component {
            Component::Normal(part) => current.push(part),
            Component::CurDir => continue,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "destination path must be relative and cannot contain parent segments",
                ))
            }
        }
        reject_link(&current)?;
    }
    Ok(current)
}

fn reject_link(path: &Path) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if metadata.file_type().is_symlink() || is_windows_reparse_point(&metadata) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("filesystem link is not allowed in destination path: {}", path.display()),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn is_windows_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_windows_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}
