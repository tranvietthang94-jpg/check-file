use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::checksum::ChecksumAlgorithm;
use crate::copy_engine::VerificationMode;
use crate::organize::OrganizeSettings;

/// A saved job configuration -- verification mode, checksum algorithm, and
/// every Organize option -- but deliberately no source/destination paths,
/// since those change per transfer while the rest of a workflow doesn't.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub name: String,
    pub verification_mode: VerificationMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub organize: OrganizeSettings,
}

/// Keeps preset names safe to use as file names -- no path separators or
/// `.` (which would otherwise allow `..` traversal), so a crafted preset
/// name can never write or read outside the presets directory.
fn sanitize_filename(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn preset_file_path(dir: &Path, name: &str) -> io::Result<PathBuf> {
    let sanitized = sanitize_filename(name);
    if sanitized.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "preset name must not be empty",
        ));
    }
    Ok(dir.join(format!("{sanitized}.json")))
}

/// Tauri-agnostic core so it can be unit tested with a plain temp directory --
/// no app runtime needed (`tauri::test::mock_app` crashes on Windows, see
/// copy_engine's `ProgressSink` for the same workaround).
pub fn save_preset_to_dir(dir: &Path, preset: &Preset) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = preset_file_path(dir, &preset.name)?;
    let json = serde_json::to_string_pretty(preset).map_err(|e| io::Error::other(e.to_string()))?;
    fs::write(path, json)
}

pub fn list_presets_from_dir(dir: &Path) -> io::Result<Vec<Preset>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut presets = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        // A corrupt or hand-edited file shouldn't hide every other preset --
        // skip it rather than failing the whole list.
        if let Ok(contents) = fs::read_to_string(entry.path()) {
            if let Ok(preset) = serde_json::from_str::<Preset>(&contents) {
                presets.push(preset);
            }
        }
    }
    presets.sort_by_key(|preset| preset.name.to_lowercase());
    Ok(presets)
}

pub fn delete_preset_from_dir(dir: &Path, name: &str) -> io::Result<()> {
    fs::remove_file(preset_file_path(dir, name)?)
}

pub fn presets_dir<R: Runtime>(app_handle: &AppHandle<R>) -> io::Result<PathBuf> {
    let base = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(base.join("presets"))
}

pub fn save_preset<R: Runtime>(app_handle: &AppHandle<R>, preset: &Preset) -> io::Result<()> {
    save_preset_to_dir(&presets_dir(app_handle)?, preset)
}

pub fn list_presets<R: Runtime>(app_handle: &AppHandle<R>) -> io::Result<Vec<Preset>> {
    list_presets_from_dir(&presets_dir(app_handle)?)
}

pub fn delete_preset<R: Runtime>(app_handle: &AppHandle<R>, name: &str) -> io::Result<()> {
    delete_preset_from_dir(&presets_dir(app_handle)?, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_preset(name: &str) -> Preset {
        Preset {
            name: name.to_string(),
            verification_mode: VerificationMode::SourceAndDestination,
            checksum_algorithm: ChecksumAlgorithm::Xxh64,
            organize: OrganizeSettings::default(),
        }
    }

    #[test]
    fn round_trips_a_saved_preset() {
        let dir = tempfile::tempdir().unwrap();
        let preset = sample_preset("A-Cam Sort");
        save_preset_to_dir(dir.path(), &preset).unwrap();

        let loaded = list_presets_from_dir(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "A-Cam Sort");
        assert_eq!(loaded[0].checksum_algorithm, ChecksumAlgorithm::Xxh64);
    }

    #[test]
    fn listing_an_empty_or_missing_directory_returns_no_presets() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist-yet");
        assert!(list_presets_from_dir(&missing).unwrap().is_empty());
    }

    #[test]
    fn presets_are_listed_alphabetically_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        save_preset_to_dir(dir.path(), &sample_preset("zebra")).unwrap();
        save_preset_to_dir(dir.path(), &sample_preset("Apple")).unwrap();

        let names: Vec<String> = list_presets_from_dir(dir.path())
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, vec!["Apple".to_string(), "zebra".to_string()]);
    }

    #[test]
    fn saving_the_same_name_again_overwrites_the_previous_version() {
        let dir = tempfile::tempdir().unwrap();
        let mut preset = sample_preset("My Preset");
        save_preset_to_dir(dir.path(), &preset).unwrap();

        preset.checksum_algorithm = ChecksumAlgorithm::Md5;
        save_preset_to_dir(dir.path(), &preset).unwrap();

        let loaded = list_presets_from_dir(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].checksum_algorithm, ChecksumAlgorithm::Md5);
    }

    #[test]
    fn deletes_a_preset_by_name() {
        let dir = tempfile::tempdir().unwrap();
        save_preset_to_dir(dir.path(), &sample_preset("Temp Preset")).unwrap();
        assert_eq!(list_presets_from_dir(dir.path()).unwrap().len(), 1);

        delete_preset_from_dir(dir.path(), "Temp Preset").unwrap();
        assert!(list_presets_from_dir(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn rejects_an_empty_or_whitespace_only_name() {
        let dir = tempfile::tempdir().unwrap();
        let err = save_preset_to_dir(dir.path(), &sample_preset("   ")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn sanitizes_path_traversal_attempts_in_the_name() {
        let dir = tempfile::tempdir().unwrap();
        let preset = sample_preset("../../evil");
        save_preset_to_dir(dir.path(), &preset).unwrap();

        // Must land inside `dir`, never escape it via ".." segments.
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
        let written_path = entries.into_iter().next().unwrap().unwrap().path();
        assert_eq!(written_path.parent().unwrap(), dir.path());
    }

    #[test]
    fn a_corrupt_preset_file_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(dir.path().join("broken.json"), b"{ not valid json").unwrap();
        save_preset_to_dir(dir.path(), &sample_preset("Good Preset")).unwrap();

        let loaded = list_presets_from_dir(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Good Preset");
    }
}
