use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::checksum::ChecksumAlgorithm;
use crate::copy_engine::{FailedFile, RenamedFile, SkippedFile, VerificationMode, VerifiedFile};

/// A persisted record of one completed transfer job -- everything from the
/// live `copy-complete` event, plus the settings used and where its MHL (if
/// any) landed, so a past transfer stays reviewable after the job itself is
/// gone from the in-memory transfers list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferLogEntry {
    pub job_id: String,
    pub source_name: String,
    pub source: String,
    pub destination: String,
    pub verification_mode: VerificationMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    /// RFC3339 -- `SystemTime` itself isn't serde-serializable.
    pub started_at: String,
    pub finished_at: String,
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub failed_files: Vec<FailedFile>,
    pub verified_files: Vec<VerifiedFile>,
    pub skipped_files: Vec<SkippedFile>,
    pub renamed_files: Vec<RenamedFile>,
    pub mhl_path: Option<String>,
}

/// Tauri-agnostic core so it can be unit tested with a plain temp directory,
/// same split used by presets.rs and copy_engine's `ProgressSink`. The job
/// id is already a filesystem-safe UUID, so no name sanitization is needed
/// here the way presets.rs needs it for user-typed names.
pub fn save_log_to_dir(dir: &Path, entry: &TransferLogEntry) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.json", entry.job_id));
    let json = serde_json::to_string_pretty(entry)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    fs::write(path, json)
}

/// Newest-first, since that's how a log viewer wants to present history.
pub fn list_logs_from_dir(dir: &Path) -> io::Result<Vec<TransferLogEntry>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut logs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        // A corrupt log shouldn't hide every other one -- skip it.
        if let Ok(contents) = fs::read_to_string(entry.path()) {
            if let Ok(log) = serde_json::from_str::<TransferLogEntry>(&contents) {
                logs.push(log);
            }
        }
    }
    logs.sort_by(|a, b| b.finished_at.cmp(&a.finished_at));
    Ok(logs)
}

pub fn logs_dir<R: Runtime>(app_handle: &AppHandle<R>) -> io::Result<PathBuf> {
    let base = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    Ok(base.join("transfer_logs"))
}

pub fn save_log<R: Runtime>(app_handle: &AppHandle<R>, entry: &TransferLogEntry) -> io::Result<()> {
    save_log_to_dir(&logs_dir(app_handle)?, entry)
}

pub fn list_logs<R: Runtime>(app_handle: &AppHandle<R>) -> io::Result<Vec<TransferLogEntry>> {
    list_logs_from_dir(&logs_dir(app_handle)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(job_id: &str, finished_at: &str) -> TransferLogEntry {
        TransferLogEntry {
            job_id: job_id.to_string(),
            source_name: "A-Cam".to_string(),
            source: "G:\\CLIP".to_string(),
            destination: "D:\\Offload".to_string(),
            verification_mode: VerificationMode::SourceAndDestination,
            checksum_algorithm: ChecksumAlgorithm::Xxh64,
            started_at: "2026-07-03T20:00:00Z".to_string(),
            finished_at: finished_at.to_string(),
            files_copied: 3,
            bytes_copied: 12345,
            failed_files: Vec::new(),
            verified_files: Vec::new(),
            skipped_files: Vec::new(),
            renamed_files: Vec::new(),
            mhl_path: Some("D:\\Offload\\20260703_200005.mhl".to_string()),
        }
    }

    #[test]
    fn round_trips_a_saved_log() {
        let dir = tempfile::tempdir().unwrap();
        let entry = sample_entry("job-1", "2026-07-03T20:00:05Z");
        save_log_to_dir(dir.path(), &entry).unwrap();

        let logs = list_logs_from_dir(dir.path()).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].job_id, "job-1");
        assert_eq!(logs[0].files_copied, 3);
        assert_eq!(logs[0].mhl_path.as_deref(), Some("D:\\Offload\\20260703_200005.mhl"));
    }

    #[test]
    fn listing_an_empty_or_missing_directory_returns_no_logs() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist-yet");
        assert!(list_logs_from_dir(&missing).unwrap().is_empty());
    }

    #[test]
    fn logs_are_listed_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        save_log_to_dir(&dir.path(), &sample_entry("older", "2026-07-01T00:00:00Z")).unwrap();
        save_log_to_dir(&dir.path(), &sample_entry("newer", "2026-07-03T00:00:00Z")).unwrap();

        let ids: Vec<String> = list_logs_from_dir(dir.path())
            .unwrap()
            .into_iter()
            .map(|l| l.job_id)
            .collect();
        assert_eq!(ids, vec!["newer".to_string(), "older".to_string()]);
    }

    #[test]
    fn a_corrupt_log_file_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(dir.path().join("broken.json"), b"{ not valid json").unwrap();
        save_log_to_dir(dir.path(), &sample_entry("good", "2026-07-03T00:00:00Z")).unwrap();

        let logs = list_logs_from_dir(dir.path()).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].job_id, "good");
    }
}
