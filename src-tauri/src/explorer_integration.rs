use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_EXPLORER_PATHS: usize = 100;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExplorerAction {
    SetSource,
    SetDestination,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerRequest {
    pub id: String,
    pub action: ExplorerAction,
    pub paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerRequestError {
    message: String,
}

impl ExplorerRequestError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ExplorerRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ExplorerRequestError {}

pub fn parse_explorer_request<I>(args: I) -> Result<ExplorerRequest, ExplorerRequestError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let _executable = args.next();
    let remaining: Vec<OsString> = args.collect();
    let mut action = None;
    let mut paths = Vec::new();
    let mut index = 0;

    while index < remaining.len() {
        match remaining[index].as_os_str() {
            value if value == OsStr::new("--explorer-action") => {
                if action.is_some() {
                    return Err(ExplorerRequestError::new(
                        "Explorer action may only be specified once",
                    ));
                }
                let value = remaining
                    .get(index + 1)
                    .ok_or_else(|| ExplorerRequestError::new("Explorer action value is missing"))?;
                action = Some(parse_action(value)?);
                index += 2;
            }
            value if value == OsStr::new("--path") => {
                let value = remaining
                    .get(index + 1)
                    .ok_or_else(|| ExplorerRequestError::new("Explorer path value is missing"))?;
                if value.is_empty() {
                    return Err(ExplorerRequestError::new("Explorer path cannot be empty"));
                }
                paths.push(PathBuf::from(value));
                if paths.len() > MAX_EXPLORER_PATHS {
                    return Err(ExplorerRequestError::new(
                        "Explorer request cannot contain more than 100 paths",
                    ));
                }
                index += 2;
            }
            value => {
                return Err(ExplorerRequestError::new(format!(
                    "Unknown Explorer argument: {}",
                    value.to_string_lossy()
                )));
            }
        }
    }

    let action = action.ok_or_else(|| ExplorerRequestError::new("Explorer action is missing"))?;
    validate_paths(&action, &paths)?;

    Ok(ExplorerRequest {
        id: uuid::Uuid::new_v4().to_string(),
        action,
        paths,
    })
}

fn parse_action(value: &OsStr) -> Result<ExplorerAction, ExplorerRequestError> {
    if value == OsStr::new("set-source") {
        Ok(ExplorerAction::SetSource)
    } else if value == OsStr::new("set-destination") {
        Ok(ExplorerAction::SetDestination)
    } else {
        Err(ExplorerRequestError::new("Unknown Explorer action"))
    }
}

fn validate_paths(action: &ExplorerAction, paths: &[PathBuf]) -> Result<(), ExplorerRequestError> {
    if paths.is_empty() {
        return Err(ExplorerRequestError::new(
            "Explorer request requires at least one path",
        ));
    }
    if matches!(action, ExplorerAction::SetDestination) && paths.len() != 1 {
        return Err(ExplorerRequestError::new(
            "Explorer destination requires exactly one path",
        ));
    }

    for path in paths {
        let metadata = fs::metadata(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ExplorerRequestError::new(format!(
                    "Explorer path does not exist: {}",
                    path.display()
                ))
            } else {
                ExplorerRequestError::new(format!(
                    "Cannot inspect Explorer path {}: {error}",
                    path.display()
                ))
            }
        })?;
        reject_filesystem_links(path)?;
        if matches!(action, ExplorerAction::SetDestination) && !metadata.is_dir() {
            return Err(ExplorerRequestError::new(
                "Explorer destination path must be a directory",
            ));
        }
    }

    Ok(())
}

fn reject_filesystem_links(path: &Path) -> Result<(), ExplorerRequestError> {
    for component_path in path.ancestors() {
        let metadata = fs::symlink_metadata(component_path).map_err(|error| {
            ExplorerRequestError::new(format!(
                "Cannot inspect Explorer path {}: {error}",
                component_path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() || is_windows_reparse_point(&metadata) {
            return Err(ExplorerRequestError::new(format!(
                "Filesystem link or reparse point is not allowed: {}",
                component_path.display()
            )));
        }
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

#[cfg(test)]
mod tests {
    use super::{parse_explorer_request, ExplorerAction};
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn explorer_args(action: &str, paths: &[&Path]) -> Vec<OsString> {
        let mut args = vec![
            OsString::from("OffloadKit.exe"),
            OsString::from("--explorer-action"),
            OsString::from(action),
        ];
        for path in paths {
            args.push(OsString::from("--path"));
            args.push(path.as_os_str().to_owned());
        }
        args
    }

    #[test]
    fn parses_set_source_with_unicode_and_spaces_exactly() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("Adobe Premiere Pro Tự động lưu");
        fs::create_dir(&path).unwrap();

        let request = parse_explorer_request(explorer_args("set-source", &[&path])).unwrap();

        assert_eq!(request.action, ExplorerAction::SetSource);
        assert_eq!(request.paths, vec![path]);
        assert!(!request.id.is_empty());
    }

    #[test]
    fn parses_set_destination() {
        let temp = tempdir().unwrap();

        let request =
            parse_explorer_request(explorer_args("set-destination", &[temp.path()])).unwrap();

        assert_eq!(request.action, ExplorerAction::SetDestination);
        assert_eq!(request.paths, vec![temp.path().to_path_buf()]);
    }

    #[test]
    fn rejects_missing_path_argument() {
        let args = vec![
            OsString::from("OffloadKit.exe"),
            OsString::from("--explorer-action"),
            OsString::from("set-source"),
        ];

        let error = parse_explorer_request(args).unwrap_err();

        assert!(error.to_string().contains("path"));
    }

    #[test]
    fn rejects_unknown_action() {
        let temp = tempdir().unwrap();

        let error = parse_explorer_request(explorer_args("copy", &[temp.path()])).unwrap_err();

        assert!(error.to_string().contains("action"));
    }

    #[test]
    fn accepts_repeated_paths_up_to_one_hundred() {
        let temp = tempdir().unwrap();
        let paths = vec![temp.path(); 100];

        let request = parse_explorer_request(explorer_args("set-source", &paths)).unwrap();

        assert_eq!(request.paths.len(), 100);
    }

    #[test]
    fn rejects_more_than_one_hundred_paths() {
        let temp = tempdir().unwrap();
        let paths = vec![temp.path(); 101];

        let error = parse_explorer_request(explorer_args("set-source", &paths)).unwrap_err();

        assert!(error.to_string().contains("100"));
    }

    #[test]
    fn rejects_empty_path() {
        let args = vec![
            OsString::from("OffloadKit.exe"),
            OsString::from("--explorer-action"),
            OsString::from("set-source"),
            OsString::from("--path"),
            OsString::new(),
        ];

        let error = parse_explorer_request(args).unwrap_err();

        assert!(error.to_string().contains("empty"));
    }

    #[test]
    fn rejects_nonexistent_path() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("missing");

        let error = parse_explorer_request(explorer_args("set-source", &[&path])).unwrap_err();

        assert!(error.to_string().contains("does not exist"));
    }

    #[test]
    fn destination_requires_exactly_one_directory() {
        let temp = tempdir().unwrap();
        let other = tempdir().unwrap();
        let file = temp.path().join("clip.mov");
        fs::write(&file, b"test").unwrap();

        let file_error =
            parse_explorer_request(explorer_args("set-destination", &[&file])).unwrap_err();
        let count_error = parse_explorer_request(explorer_args(
            "set-destination",
            &[temp.path(), other.path()],
        ))
        .unwrap_err();

        assert!(file_error.to_string().contains("directory"));
        assert!(count_error.to_string().contains("exactly one"));
    }

    #[test]
    fn rejects_filesystem_links() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::create_dir(&target).unwrap();
        create_dir_link(&target, &link);

        let error = parse_explorer_request(explorer_args("set-source", &[&link])).unwrap_err();

        assert!(error.to_string().contains("link") || error.to_string().contains("reparse"));
    }

    #[cfg(windows)]
    fn create_dir_link(target: &Path, link: &Path) {
        std::os::windows::fs::symlink_dir(target, link).unwrap();
    }

    #[cfg(unix)]
    fn create_dir_link(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }
}
