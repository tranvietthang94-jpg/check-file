use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Rejects any existing symlink/junction/reparse-point component between
/// `root` and `relative`, so writes cannot escape through filesystem links.
pub fn safe_destination(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    guarded_path(root, relative, "destination")
}

/// Rechecks an already-built destination pathname immediately before a
/// filesystem operation. This narrows pathname races but cannot make separate
/// path validation and mutation atomic on every supported platform.
pub fn revalidate_destination(root: &Path, path: &Path) -> io::Result<()> {
    revalidate_path(root, path, "destination")
}

/// Rechecks an already-scanned source pathname immediately before opening or
/// mutating it. Selected Explorer paths use this after scanning so a swapped
/// link or reparse point fails closed instead of being followed.
pub fn revalidate_source(root: &Path, path: &Path) -> io::Result<()> {
    revalidate_path(root, path, "source")
}

fn revalidate_path(root: &Path, path: &Path, kind: &str) -> io::Result<()> {
    let relative = path.strip_prefix(root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{kind} path escaped its root"),
        )
    })?;
    guarded_path(root, relative, kind).map(|_| ())
}

fn guarded_path(root: &Path, relative: &Path, kind: &str) -> io::Result<PathBuf> {
    let mut current = root.to_path_buf();
    reject_link(&current, kind)?;
    for component in relative.components() {
        use std::path::Component;
        match component {
            Component::Normal(part) => current.push(part),
            Component::CurDir => continue,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{kind} path must be relative and cannot contain parent segments"),
                ))
            }
        }
        reject_link(&current, kind)?;
    }
    Ok(current)
}

fn reject_link(path: &Path, kind: &str) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if metadata.file_type().is_symlink() || is_windows_reparse_point(&metadata) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "filesystem link is not allowed in {kind} path: {}",
                path.display()
            ),
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
