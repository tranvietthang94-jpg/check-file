use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// FAT32/exFAT (common on camera cards) store modified time with coarse
/// granularity (commonly ~2s), while NTFS uses 100ns -- comparing
/// `SystemTime` for exact equality would treat a genuinely identical file
/// as different after crossing filesystems. Treat mtimes within this
/// window as the same moment.
const MTIME_TOLERANCE: Duration = Duration::from_secs(2);

pub enum DuplicateAction {
    /// No existing file at the destination path -- copy normally.
    Copy,
    /// A file already exists with the same name, size, and modified time --
    /// treated as the same file already offloaded; don't copy it again.
    Skip,
    /// A file exists with the same name but different size/time -- a
    /// different file that happens to share a name (e.g. two cards that
    /// both start clip numbering at C0001). Copy to this new path instead.
    Rename(PathBuf),
}

pub(crate) fn mtimes_close(a: SystemTime, b: SystemTime) -> bool {
    let diff = a.duration_since(b).or_else(|_| b.duration_since(a));
    matches!(diff, Ok(d) if d <= MTIME_TOLERANCE)
}

/// Decides what to do about a source file that would land at `dest_path`.
/// `skip_modification_check` drops modified-time out of the comparison
/// entirely (name + size only) -- for workflows where an intermediate copy
/// step or a coarse filesystem means a genuinely-identical file's timestamp
/// can't be trusted to still match.
pub fn resolve_duplicate(
    dest_path: &Path,
    source_size: u64,
    source_modified: SystemTime,
    skip_modification_check: bool,
) -> io::Result<DuplicateAction> {
    if !dest_path.exists() {
        return Ok(DuplicateAction::Copy);
    }

    let dest_meta = fs::metadata(dest_path)?;
    let dest_modified = dest_meta.modified()?;
    let modified_matches = skip_modification_check || mtimes_close(dest_modified, source_modified);

    if dest_meta.len() == source_size && modified_matches {
        return Ok(DuplicateAction::Skip);
    }

    Ok(DuplicateAction::Rename(next_available_name(dest_path)))
}

/// Finds the first `name 2.ext`, `name 3.ext`, ... that doesn't already exist.
pub(crate) fn next_available_name(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());

    let mut counter = 2u32;
    loop {
        let candidate_name = match &ext {
            Some(ext) => format!("{stem} {counter}.{ext}"),
            None => format!("{stem} {counter}"),
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_when_destination_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("clip.mp4");
        let action = resolve_duplicate(&dest, 1234, SystemTime::now(), false).unwrap();
        assert!(matches!(action, DuplicateAction::Copy));
    }

    #[test]
    fn skips_when_size_and_mtime_match() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("clip.mp4");
        fs::write(&dest, b"same content").unwrap();
        let meta = fs::metadata(&dest).unwrap();

        let action = resolve_duplicate(&dest, meta.len(), meta.modified().unwrap(), false).unwrap();
        assert!(matches!(action, DuplicateAction::Skip));
    }

    #[test]
    fn skips_when_mtime_differs_within_fat_rounding_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("clip.mp4");
        fs::write(&dest, b"same content").unwrap();
        let meta = fs::metadata(&dest).unwrap();

        let nudged = meta.modified().unwrap() + Duration::from_millis(500);
        let action = resolve_duplicate(&dest, meta.len(), nudged, false).unwrap();
        assert!(matches!(action, DuplicateAction::Skip));
    }

    #[test]
    fn renames_when_size_differs() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("clip.mp4");
        fs::write(&dest, b"existing content").unwrap();
        let meta = fs::metadata(&dest).unwrap();

        let action =
            resolve_duplicate(&dest, meta.len() + 1, meta.modified().unwrap(), false).unwrap();
        match action {
            DuplicateAction::Rename(path) => assert_eq!(path, dir.path().join("clip 2.mp4")),
            _ => panic!("expected Rename"),
        }
    }

    #[test]
    fn renames_when_mtime_differs_beyond_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("clip.mp4");
        fs::write(&dest, b"existing content").unwrap();
        let meta = fs::metadata(&dest).unwrap();

        let far_off = meta.modified().unwrap() + Duration::from_secs(60);
        let action = resolve_duplicate(&dest, meta.len(), far_off, false).unwrap();
        assert!(matches!(action, DuplicateAction::Rename(_)));
    }

    #[test]
    fn skip_modification_check_ignores_a_stale_mtime_but_still_catches_a_size_change() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("clip.mp4");
        fs::write(&dest, b"existing content").unwrap();
        let meta = fs::metadata(&dest).unwrap();

        let far_off = meta.modified().unwrap() + Duration::from_secs(3600);
        let action = resolve_duplicate(&dest, meta.len(), far_off, true).unwrap();
        assert!(
            matches!(action, DuplicateAction::Skip),
            "with the check skipped, a matching name+size must be treated as the same file \
             regardless of how far off its mtime is"
        );

        let action = resolve_duplicate(&dest, meta.len() + 1, far_off, true).unwrap();
        assert!(
            matches!(action, DuplicateAction::Rename(_)),
            "skipping the modification-date check must not also skip the size comparison"
        );
    }

    #[test]
    fn rename_finds_the_next_free_counter() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("clip.mp4"), b"a").unwrap();
        fs::write(dir.path().join("clip 2.mp4"), b"b").unwrap();

        let candidate = next_available_name(&dir.path().join("clip.mp4"));
        assert_eq!(candidate, dir.path().join("clip 3.mp4"));
    }

    #[test]
    fn rename_preserves_extensionless_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("README"), b"a").unwrap();

        let candidate = next_available_name(&dir.path().join("README"));
        assert_eq!(candidate, dir.path().join("README 2"));
    }
}
