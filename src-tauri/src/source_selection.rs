use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceSelection {
    common_root: PathBuf,
    selected_paths: Vec<PathBuf>,
}

impl SourceSelection {
    pub fn new(common_root: PathBuf, selected_paths: Vec<PathBuf>) -> io::Result<Self> {
        validate_absolute_path(&common_root, "common root")?;
        let root_metadata = fs::metadata(&common_root)?;
        if !root_metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "source selection common root must be a directory",
            ));
        }
        reject_linked_ancestors(&common_root)?;

        let selected_paths = normalize_selected_paths(selected_paths)?;
        for selected in &selected_paths {
            let relative = strip_prefix_platform(selected, &common_root).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "selected path is outside the common root: {}",
                        selected.display()
                    ),
                )
            })?;
            validate_relative_path(relative)?;
        }

        Ok(Self {
            common_root,
            selected_paths,
        })
    }

    pub fn from_paths(selected_paths: Vec<PathBuf>) -> io::Result<Self> {
        let selected_paths = normalize_selected_paths(selected_paths)?;
        let first = selected_paths.first().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "source selection requires at least one path",
            )
        })?;
        let first_root = first.parent().unwrap_or(first.as_path());
        let common_root = first_root
            .ancestors()
            .find(|candidate| {
                selected_paths
                    .iter()
                    .all(|selected| path_starts_with(selected, candidate))
            })
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "selected paths do not share a common root",
                )
            })?;

        Self::new(common_root, selected_paths)
    }

    pub fn common_root(&self) -> &Path {
        &self.common_root
    }

    pub fn selected_paths(&self) -> &[PathBuf] {
        &self.selected_paths
    }

    pub fn rebase(&self, common_root: PathBuf) -> io::Result<Self> {
        let selected_paths = self
            .selected_paths
            .iter()
            .map(|selected| {
                strip_prefix_platform(selected, &self.common_root)
                    .map(|relative| common_root.join(relative))
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "selected path escaped its common root",
                        )
                    })
            })
            .collect::<io::Result<Vec<_>>>()?;
        Self::new(common_root, selected_paths)
    }
}

fn normalize_selected_paths(paths: Vec<PathBuf>) -> io::Result<Vec<PathBuf>> {
    if paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source selection requires at least one path",
        ));
    }

    let mut seen = HashSet::new();
    let mut inspected = Vec::new();
    for path in paths {
        validate_absolute_path(&path, "selected path")?;
        let metadata = fs::metadata(&path)?;
        if !metadata.is_file() && !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("selected path is not a file or directory: {}", path.display()),
            ));
        }
        reject_linked_ancestors(&path)?;
        if metadata.is_dir() {
            reject_links_below(&path)?;
        }

        let canonical = fs::canonicalize(&path)?;
        if seen.insert(path_key(&canonical)) {
            inspected.push((path, canonical, metadata.is_dir()));
        }
    }

    let mut normalized = Vec::new();
    for (index, (path, canonical, _)) in inspected.iter().enumerate() {
        let nested = inspected.iter().enumerate().any(
            |(other_index, (_, other_canonical, other_is_dir))| {
                index != other_index
                    && *other_is_dir
                    && canonical != other_canonical
                    && path_starts_with(canonical, other_canonical)
            },
        );
        if !nested {
            normalized.push(path.clone());
        }
    }

    Ok(normalized)
}

fn validate_absolute_path(path: &Path, label: &str) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{label} must be absolute: {}", path.display()),
        ));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{label} cannot contain parent traversal: {}", path.display()),
        ));
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> io::Result<()> {
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "selected path relative to common root contains an invalid component",
                ))
            }
        }
    }
    Ok(())
}

fn reject_links_below(root: &Path) -> io::Result<()> {
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| io::Error::other(error.to_string()))?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() || is_windows_reparse_point(&metadata) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "filesystem link or reparse point is not allowed in source selection: {}",
                    entry.path().display()
                ),
            ));
        }
    }
    Ok(())
}

fn reject_linked_ancestors(path: &Path) -> io::Result<()> {
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor)?;
        if metadata.file_type().is_symlink() || is_windows_reparse_point(&metadata) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "filesystem link or reparse point is not allowed in source selection: {}",
                    ancestor.display()
                ),
            ));
        }
    }
    Ok(())
}

fn strip_prefix_platform<'a>(path: &'a Path, base: &Path) -> Option<&'a Path> {
    path.strip_prefix(base).ok()
}

fn path_starts_with(path: &Path, base: &Path) -> bool {
    let path_components: Vec<String> = path
        .components()
        .map(|component| component_key(component.as_os_str()))
        .collect();
    let base_components: Vec<String> = base
        .components()
        .map(|component| component_key(component.as_os_str()))
        .collect();
    path_components.starts_with(&base_components)
}

fn path_key(path: &Path) -> String {
    path.components()
        .map(|component| component_key(component.as_os_str()))
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(windows)]
fn component_key(value: &std::ffi::OsStr) -> String {
    value.to_string_lossy().to_lowercase()
}

#[cfg(not(windows))]
fn component_key(value: &std::ffi::OsStr) -> String {
    value.to_string_lossy().into_owned()
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
