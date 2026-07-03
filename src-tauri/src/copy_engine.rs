use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use walkdir::WalkDir;

pub const SCAN_EVENT: &str = "copy-scan";
pub const PROGRESS_EVENT: &str = "copy-progress";
pub const COMPLETE_EVENT: &str = "copy-complete";
pub const CANCELLED_EVENT: &str = "copy-cancelled";

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB
const PROGRESS_THROTTLE: std::time::Duration = std::time::Duration::from_millis(200);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub job_id: String,
    pub current_file: String,
    pub bytes_copied: u64,
    pub total_bytes: u64,
    pub files_copied: u64,
    pub total_files: u64,
    pub bytes_per_sec: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FailedFile {
    pub path: String,
    pub message: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanPayload {
    pub job_id: String,
    pub total_files: u64,
    pub total_bytes: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompletePayload {
    pub job_id: String,
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub failed_files: Vec<FailedFile>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CancelledPayload {
    pub job_id: String,
}

/// Result of a finished (or cancelled) copy job, Tauri-agnostic so it can be
/// asserted on directly in unit tests.
pub struct CopyOutcome {
    pub cancelled: bool,
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub failed_files: Vec<FailedFile>,
}

/// Destination for scan/progress notifications. Kept separate from Tauri so
/// the copy core can be unit tested without spinning up an app runtime.
pub trait ProgressSink {
    fn on_scan(&self, total_files: u64, total_bytes: u64);
    fn on_progress(&self, payload: ProgressPayload);
}

/// Tracks cancellation flags for in-flight copy jobs, keyed by job id.
#[derive(Default)]
pub struct JobRegistry {
    cancel_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl JobRegistry {
    pub fn register(&self, job_id: String) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.cancel_flags.lock().unwrap().insert(job_id, flag.clone());
        flag
    }

    /// Returns true if a job with this id was found and signalled to stop.
    pub fn cancel(&self, job_id: &str) -> bool {
        match self.cancel_flags.lock().unwrap().get(job_id) {
            Some(flag) => {
                flag.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub fn remove(&self, job_id: &str) {
        self.cancel_flags.lock().unwrap().remove(job_id);
    }
}

struct FileEntry {
    absolute: PathBuf,
    relative: PathBuf,
    size: u64,
}

/// Emits throttled progress notifications so the UI isn't flooded on fast local copies.
struct ProgressTracker<'a> {
    sink: &'a dyn ProgressSink,
    job_id: String,
    total_bytes: u64,
    total_files: u64,
    bytes_copied: u64,
    files_copied: u64,
    last_emit: Instant,
    last_emit_bytes: u64,
}

impl<'a> ProgressTracker<'a> {
    fn new(sink: &'a dyn ProgressSink, job_id: String, total_bytes: u64, total_files: u64) -> Self {
        Self {
            sink,
            job_id,
            total_bytes,
            total_files,
            bytes_copied: 0,
            files_copied: 0,
            last_emit: Instant::now(),
            last_emit_bytes: 0,
        }
    }

    fn add_bytes(&mut self, n: u64, current_file: &str) {
        self.bytes_copied += n;
        if self.last_emit.elapsed() >= PROGRESS_THROTTLE {
            self.emit(current_file);
        }
    }

    fn finish_file(&mut self) {
        self.files_copied += 1;
    }

    fn emit(&mut self, current_file: &str) {
        let elapsed_secs = self.last_emit.elapsed().as_secs_f64().max(0.001);
        let delta_bytes = self.bytes_copied.saturating_sub(self.last_emit_bytes);
        let bytes_per_sec = (delta_bytes as f64 / elapsed_secs) as u64;
        self.sink.on_progress(ProgressPayload {
            job_id: self.job_id.clone(),
            current_file: current_file.to_string(),
            bytes_copied: self.bytes_copied,
            total_bytes: self.total_bytes,
            files_copied: self.files_copied,
            total_files: self.total_files,
            bytes_per_sec,
        });
        self.last_emit = Instant::now();
        self.last_emit_bytes = self.bytes_copied;
    }
}

fn scan_source(source: &Path) -> std::io::Result<Vec<FileEntry>> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("source path not found: {}", source.display()),
        ));
    }
    if source.is_file() {
        let size = fs::metadata(source)?.len();
        let relative = PathBuf::from(source.file_name().unwrap_or_default());
        return Ok(vec![FileEntry {
            absolute: source.to_path_buf(),
            relative,
            size,
        }]);
    }

    let mut entries = Vec::new();
    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let absolute = entry.path().to_path_buf();
            let relative = absolute
                .strip_prefix(source)
                .unwrap_or(&absolute)
                .to_path_buf();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            entries.push(FileEntry {
                absolute,
                relative,
                size,
            });
        }
    }
    Ok(entries)
}

/// Copies one file in fixed-size chunks (streaming, not loaded fully into RAM),
/// checking the cancel flag between chunks. Returns Ok(false) if cancelled
/// mid-file, in which case the partial destination file is removed so it
/// can never be mistaken for a completed copy.
fn copy_file_chunked(
    src: &Path,
    dst: &Path,
    buffer: &mut [u8],
    cancel_flag: &AtomicBool,
    tracker: &mut ProgressTracker,
    relative_display: &str,
) -> std::io::Result<bool> {
    let mut src_file = fs::File::open(src)?;
    let mut dst_file = fs::File::create(dst)?;

    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            drop(dst_file);
            let _ = fs::remove_file(dst);
            return Ok(false);
        }
        let n = src_file.read(buffer)?;
        if n == 0 {
            break;
        }
        dst_file.write_all(&buffer[..n])?;
        tracker.add_bytes(n as u64, relative_display);
    }
    Ok(true)
}

/// Tauri-agnostic copy core: walks `source`, streams every file to the mirrored
/// path under `destination`, and reports progress through `sink`. Safe to unit
/// test directly with a stub sink and no app runtime.
pub fn run_copy_core(
    sink: &dyn ProgressSink,
    job_id: String,
    source: &Path,
    destination: &Path,
    cancel_flag: &AtomicBool,
) -> CopyOutcome {
    let entries = match scan_source(source) {
        Ok(e) => e,
        Err(err) => {
            return CopyOutcome {
                cancelled: false,
                files_copied: 0,
                bytes_copied: 0,
                failed_files: vec![FailedFile {
                    path: source.display().to_string(),
                    message: err.to_string(),
                }],
            };
        }
    };

    let total_files = entries.len() as u64;
    let total_bytes: u64 = entries.iter().map(|e| e.size).sum();
    sink.on_scan(total_files, total_bytes);

    let mut tracker = ProgressTracker::new(sink, job_id, total_bytes, total_files);
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut failed_files = Vec::new();
    let mut cancelled = false;

    for entry in &entries {
        if cancel_flag.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }

        let dest_path = destination.join(&entry.relative);
        if let Some(parent) = dest_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                failed_files.push(FailedFile {
                    path: entry.relative.display().to_string(),
                    message: err.to_string(),
                });
                continue;
            }
        }

        let relative_display = entry.relative.display().to_string();
        match copy_file_chunked(
            &entry.absolute,
            &dest_path,
            &mut buffer,
            cancel_flag,
            &mut tracker,
            &relative_display,
        ) {
            Ok(true) => tracker.finish_file(),
            Ok(false) => {
                cancelled = true;
                break;
            }
            Err(err) => failed_files.push(FailedFile {
                path: relative_display,
                message: err.to_string(),
            }),
        }
    }

    if !cancelled {
        tracker.emit(""); // final flush so the UI can reach 100%
    }

    CopyOutcome {
        cancelled,
        files_copied: tracker.files_copied,
        bytes_copied: tracker.bytes_copied,
        failed_files,
    }
}

struct TauriProgressSink<'a, R: Runtime> {
    app_handle: &'a AppHandle<R>,
    job_id: String,
}

impl<'a, R: Runtime> ProgressSink for TauriProgressSink<'a, R> {
    fn on_scan(&self, total_files: u64, total_bytes: u64) {
        let _ = self.app_handle.emit(
            SCAN_EVENT,
            ScanPayload {
                job_id: self.job_id.clone(),
                total_files,
                total_bytes,
            },
        );
    }

    fn on_progress(&self, payload: ProgressPayload) {
        let _ = self.app_handle.emit(PROGRESS_EVENT, payload);
    }
}

/// Runs a single source -> destination transfer to completion (or cancellation),
/// emitting `copy-scan`, `copy-progress`, and a terminal `copy-complete` /
/// `copy-cancelled` event. Intended to run on its own thread.
pub fn run_copy_job<R: Runtime>(
    app_handle: AppHandle<R>,
    job_id: String,
    source: PathBuf,
    destination: PathBuf,
    cancel_flag: Arc<AtomicBool>,
) {
    let sink = TauriProgressSink {
        app_handle: &app_handle,
        job_id: job_id.clone(),
    };
    let outcome = run_copy_core(&sink, job_id.clone(), &source, &destination, &cancel_flag);

    if outcome.cancelled {
        let _ = app_handle.emit(
            CANCELLED_EVENT,
            CancelledPayload {
                job_id: job_id.clone(),
            },
        );
    } else {
        let _ = app_handle.emit(
            COMPLETE_EVENT,
            CompletePayload {
                job_id: job_id.clone(),
                files_copied: outcome.files_copied,
                bytes_copied: outcome.bytes_copied,
                failed_files: outcome.failed_files,
            },
        );
    }

    app_handle.state::<JobRegistry>().remove(&job_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopSink;
    impl ProgressSink for NoopSink {
        fn on_scan(&self, _total_files: u64, _total_bytes: u64) {}
        fn on_progress(&self, _payload: ProgressPayload) {}
    }

    #[test]
    fn copies_nested_files_and_preserves_bytes() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();

        fs::write(src_dir.path().join("a.txt"), b"hello world").unwrap();
        fs::create_dir_all(src_dir.path().join("nested")).unwrap();
        fs::write(src_dir.path().join("nested").join("b.bin"), vec![7u8; 5000]).unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "test-job".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
        );

        assert!(!outcome.cancelled);
        assert_eq!(outcome.files_copied, 2);
        assert!(outcome.failed_files.is_empty());
        assert_eq!(outcome.bytes_copied, 11 + 5000);

        assert_eq!(fs::read(dst_dir.path().join("a.txt")).unwrap(), b"hello world");
        assert_eq!(
            fs::read(dst_dir.path().join("nested").join("b.bin")).unwrap(),
            vec![7u8; 5000]
        );
    }

    #[test]
    fn cancelling_removes_partial_destination_file() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();

        // Large enough to span multiple 1 MiB chunks so cancellation lands mid-file.
        fs::write(src_dir.path().join("big.bin"), vec![1u8; 5 * 1024 * 1024]).unwrap();

        let cancel_flag = AtomicBool::new(true); // pre-cancelled
        let outcome = run_copy_core(
            &NoopSink,
            "test-job-cancel".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
        );

        assert!(outcome.cancelled);
        assert!(!dst_dir.path().join("big.bin").exists());
    }

    #[test]
    fn missing_source_reports_failure_without_panicking() {
        let dst_dir = tempfile::tempdir().unwrap();
        let cancel_flag = AtomicBool::new(false);

        let outcome = run_copy_core(
            &NoopSink,
            "test-job-missing".to_string(),
            Path::new("Z:\\this\\path\\does\\not\\exist"),
            dst_dir.path(),
            &cancel_flag,
        );

        assert_eq!(outcome.files_copied, 0);
        assert_eq!(outcome.failed_files.len(), 1);
    }

    #[test]
    fn progress_events_report_growing_byte_count() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("f.bin"), vec![9u8; 200]).unwrap();

        struct RecordingSink {
            calls: Mutex<Vec<u64>>,
        }
        impl ProgressSink for RecordingSink {
            fn on_scan(&self, _total_files: u64, _total_bytes: u64) {}
            fn on_progress(&self, payload: ProgressPayload) {
                self.calls.lock().unwrap().push(payload.bytes_copied);
            }
        }

        let sink = RecordingSink {
            calls: Mutex::new(Vec::new()),
        };
        let cancel_flag = AtomicBool::new(false);
        run_copy_core(
            &sink,
            "job".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
        );

        // The final flush always emits at least one progress event.
        let calls = sink.calls.lock().unwrap();
        assert!(!calls.is_empty());
        assert_eq!(*calls.last().unwrap(), 200);
    }
}
