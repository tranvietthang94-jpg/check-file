use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

use tauri::{AppHandle, Emitter, Manager, Runtime};
use walkdir::WalkDir;

use crate::checksum::{self, ChecksumAlgorithm, StreamingHasher};
use crate::dedup::{self, DuplicateAction};
use crate::organize::{self, OrganizeSettings, TokenContext};

/// How thoroughly a transfer is checked for integrity.
/// - `Transfer`: no hashing, relies on the OS copy completing without an I/O error.
/// - `Source`: hash the source while streaming it (no extra read pass) and record it.
/// - `SourceAndDestination`: additionally re-reads the destination afterwards and
///   compares its hash to the source hash, catching corruption introduced during write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationMode {
    Transfer,
    Source,
    SourceAndDestination,
}

impl Default for VerificationMode {
    fn default() -> Self {
        VerificationMode::SourceAndDestination
    }
}

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
pub struct VerifiedFile {
    pub path: String,
    pub checksum: String,
    pub algorithm: ChecksumAlgorithm,
}

/// A file that already existed at the destination with the same name, size,
/// and modified time -- treated as already offloaded and not copied again.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFile {
    pub path: String,
}

/// A file that collided on name with a different file already at the
/// destination (different size or modified time), so it was copied under a
/// new name instead of overwriting.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RenamedFile {
    pub original_path: String,
    pub renamed_to: String,
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
    pub verified_files: Vec<VerifiedFile>,
    pub skipped_files: Vec<SkippedFile>,
    pub renamed_files: Vec<RenamedFile>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CancelledPayload {
    pub job_id: String,
}

/// Result of a finished (or cancelled) copy job, Tauri-agnostic so it can be
/// asserted on directly in unit tests.
#[derive(Clone)]
pub struct CopyOutcome {
    pub cancelled: bool,
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub failed_files: Vec<FailedFile>,
    pub verified_files: Vec<VerifiedFile>,
    pub skipped_files: Vec<SkippedFile>,
    pub renamed_files: Vec<RenamedFile>,
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
    modified: SystemTime,
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
        let meta = fs::metadata(source)?;
        let relative = PathBuf::from(source.file_name().unwrap_or_default());
        return Ok(vec![FileEntry {
            absolute: source.to_path_buf(),
            relative,
            size: meta.len(),
            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
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
            let meta = entry.metadata().ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            entries.push(FileEntry {
                absolute,
                relative,
                size,
                modified,
            });
        }
    }
    Ok(entries)
}

/// Outcome of copying a single file.
enum CopyFileOutcome {
    Cancelled,
    /// `source_hash` is `Some` only when verification requested it (Transfer
    /// mode skips hashing entirely to stay as fast as a plain OS copy).
    Completed { source_hash: Option<String> },
}

/// Copies one file in fixed-size chunks (streaming, not loaded fully into RAM),
/// checking the cancel flag between chunks and optionally hashing the source
/// bytes as they're read (no extra I/O pass). Returns `Cancelled` if cancelled
/// mid-file, in which case the partial destination file is removed so it can
/// never be mistaken for a completed copy.
fn copy_file_chunked(
    src: &Path,
    dst: &Path,
    buffer: &mut [u8],
    cancel_flag: &AtomicBool,
    tracker: &mut ProgressTracker,
    relative_display: &str,
    checksum_algorithm: ChecksumAlgorithm,
    compute_hash: bool,
    source_modified: SystemTime,
) -> std::io::Result<CopyFileOutcome> {
    let mut src_file = fs::File::open(src)?;
    let dst_file = fs::File::create(dst)?;
    let mut hasher = compute_hash.then(|| StreamingHasher::new(checksum_algorithm));

    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            drop(dst_file);
            let _ = fs::remove_file(dst);
            return Ok(CopyFileOutcome::Cancelled);
        }
        let n = src_file.read(buffer)?;
        if n == 0 {
            break;
        }
        (&dst_file).write_all(&buffer[..n])?;
        if let Some(h) = hasher.as_mut() {
            h.update(&buffer[..n]);
        }
        tracker.add_bytes(n as u64, relative_display);
    }

    // Mirror the source's modified time onto the destination. Without this,
    // every destination file gets "now" as its mtime, which would make
    // duplicate detection (name + size + mtime) misfire as a rename on any
    // second offload of the same card instead of recognizing it as already
    // copied.
    let _ = dst_file.set_modified(source_modified);

    Ok(CopyFileOutcome::Completed {
        source_hash: hasher.map(|h| h.finalize_hex()),
    })
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
    verification_mode: VerificationMode,
    checksum_algorithm: ChecksumAlgorithm,
    source_name: &str,
    organize: &OrganizeSettings,
) -> CopyOutcome {
    let mut entries = match scan_source(source) {
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
                verified_files: Vec::new(),
                skipped_files: Vec::new(),
                renamed_files: Vec::new(),
            };
        }
    };

    if let Some(rule) = &organize.bundle_ignore {
        let ignored_dirs = organize::find_ignored_bundle_dirs(source, rule);
        if !ignored_dirs.is_empty() {
            entries.retain(|e| !ignored_dirs.iter().any(|dir| e.absolute.starts_with(dir)));
        }
    }
    entries.retain(|e| organize::passes_selective_filter(&e.relative, &organize.selective_copy));

    let total_files = entries.len() as u64;
    let total_bytes: u64 = entries.iter().map(|e| e.size).sum();
    sink.on_scan(total_files, total_bytes);

    let job_started = SystemTime::now();
    let content_oldest = organize::compute_content_oldest_date(
        &entries
            .iter()
            .map(|e| (e.relative.clone(), e.modified))
            .collect::<Vec<_>>(),
        &organize.content_date_excluded_extensions,
    );
    // Source folders that still have at least one (post-filter) file in them
    // -- anything not in here is empty and, unless `ignore_empty_folders` is
    // disabled, never gets created at the destination.
    let dirs_with_files: HashSet<PathBuf> = entries
        .iter()
        .filter_map(|e| e.relative.parent().map(|p| p.to_path_buf()))
        .filter(|p| !p.as_os_str().is_empty())
        .collect();

    let compute_hash = verification_mode != VerificationMode::Transfer;
    let mut tracker = ProgressTracker::new(sink, job_id, total_bytes, total_files);
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut failed_files = Vec::new();
    let mut verified_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut renamed_files = Vec::new();
    let mut cancelled = false;

    for (index, entry) in entries.iter().enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }

        let file_stem = entry
            .relative
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let file_extension = entry
            .relative
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ctx = TokenContext {
            source_name: source_name.to_string(),
            job_started,
            counter: (index + 1) as u32,
            counter_padding: organize.counter_padding,
            file_stem,
            file_extension,
            file_modified: entry.modified,
            content_oldest,
        };
        let organized_relative = organize::build_destination_path(&entry.relative, &ctx, organize);

        let mut dest_path = destination.join(&organized_relative);
        if let Some(parent) = dest_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                failed_files.push(FailedFile {
                    path: organized_relative.display().to_string(),
                    message: err.to_string(),
                });
                continue;
            }
        }

        let relative_display = organized_relative.display().to_string();
        // Tracks the path actually written to, which diverges from
        // `relative_display` after a rename -- everything downstream
        // (copying, verification, error reporting) must refer to this one
        // so a renamed file is never reported under its original name.
        let mut copy_display = relative_display.clone();

        match dedup::resolve_duplicate(&dest_path, entry.size, entry.modified) {
            Ok(DuplicateAction::Copy) => {}
            Ok(DuplicateAction::Skip) => {
                tracker.add_bytes(entry.size, &relative_display);
                tracker.finish_file();
                skipped_files.push(SkippedFile {
                    path: relative_display,
                });
                continue;
            }
            Ok(DuplicateAction::Rename(new_path)) => {
                copy_display = entry
                    .relative
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(new_path.file_name().unwrap_or_default())
                    .display()
                    .to_string();
                renamed_files.push(RenamedFile {
                    original_path: relative_display,
                    renamed_to: copy_display.clone(),
                });
                dest_path = new_path;
            }
            Err(err) => {
                failed_files.push(FailedFile {
                    path: relative_display,
                    message: format!("could not check for duplicates: {err}"),
                });
                continue;
            }
        }

        match copy_file_chunked(
            &entry.absolute,
            &dest_path,
            &mut buffer,
            cancel_flag,
            &mut tracker,
            &copy_display,
            checksum_algorithm,
            compute_hash,
            entry.modified,
        ) {
            Ok(CopyFileOutcome::Completed { source_hash }) => {
                tracker.finish_file();
                if let Some(hash) = source_hash {
                    if verification_mode == VerificationMode::SourceAndDestination {
                        match checksum::verify_file_hash(&dest_path, &hash, checksum_algorithm) {
                            Ok(true) => verified_files.push(VerifiedFile {
                                path: copy_display,
                                checksum: hash,
                                algorithm: checksum_algorithm,
                            }),
                            Ok(false) => failed_files.push(FailedFile {
                                path: copy_display,
                                message:
                                    "checksum mismatch: source and destination differ after copy"
                                        .to_string(),
                            }),
                            Err(err) => failed_files.push(FailedFile {
                                path: copy_display,
                                message: format!(
                                    "could not re-read destination for verification: {err}"
                                ),
                            }),
                        }
                    } else {
                        // Source-only mode: nothing to compare against yet, just record it.
                        verified_files.push(VerifiedFile {
                            path: copy_display,
                            checksum: hash,
                            algorithm: checksum_algorithm,
                        });
                    }
                }
            }
            Ok(CopyFileOutcome::Cancelled) => {
                cancelled = true;
                break;
            }
            Err(err) => failed_files.push(FailedFile {
                path: copy_display,
                message: err.to_string(),
            }),
        }
    }

    if !cancelled {
        tracker.emit(""); // final flush so the UI can reach 100%

        if !organize.ignore_empty_folders
            && !organize.flatten
            && organize.folder_template.is_none()
        {
            let _ = organize::mirror_empty_source_dirs(source, destination, &dirs_with_files);
        }
    }

    CopyOutcome {
        cancelled,
        files_copied: tracker.files_copied,
        bytes_copied: tracker.bytes_copied,
        failed_files,
        verified_files,
        skipped_files,
        renamed_files,
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
/// `copy-cancelled` event. Intended to run on its own thread. Returns the
/// outcome so callers orchestrating multiple jobs (e.g. cascading transfers)
/// can decide whether to proceed based on how this one went.
pub fn run_copy_job<R: Runtime>(
    app_handle: AppHandle<R>,
    job_id: String,
    source: PathBuf,
    destination: PathBuf,
    cancel_flag: Arc<AtomicBool>,
    verification_mode: VerificationMode,
    checksum_algorithm: ChecksumAlgorithm,
    source_name: String,
    organize: OrganizeSettings,
) -> CopyOutcome {
    let sink = TauriProgressSink {
        app_handle: &app_handle,
        job_id: job_id.clone(),
    };
    let outcome = run_copy_core(
        &sink,
        job_id.clone(),
        &source,
        &destination,
        &cancel_flag,
        verification_mode,
        checksum_algorithm,
        &source_name,
        &organize,
    );

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
                failed_files: outcome.failed_files.clone(),
                verified_files: outcome.verified_files.clone(),
                skipped_files: outcome.skipped_files.clone(),
                renamed_files: outcome.renamed_files.clone(),
            },
        );
    }

    app_handle.state::<JobRegistry>().remove(&job_id);
    outcome
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
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
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
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
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
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
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
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
        );

        // The final flush always emits at least one progress event.
        let calls = sink.calls.lock().unwrap();
        assert!(!calls.is_empty());
        assert_eq!(*calls.last().unwrap(), 200);
    }

    #[test]
    fn transfer_mode_skips_hashing() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("a.txt"), b"hello").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-transfer".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
        );

        assert!(!outcome.cancelled);
        assert!(outcome.failed_files.is_empty());
        assert!(
            outcome.verified_files.is_empty(),
            "Transfer mode must not compute checksums"
        );
    }

    #[test]
    fn source_mode_records_checksum_without_rereading_destination() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("a.txt"), b"hello").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-source".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Source,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
        );

        assert!(outcome.failed_files.is_empty());
        assert_eq!(outcome.verified_files.len(), 1);
        assert_eq!(outcome.verified_files[0].path, "a.txt");
        assert_eq!(
            outcome.verified_files[0].checksum,
            checksum::hash_file(&src_dir.path().join("a.txt"), ChecksumAlgorithm::Xxh64).unwrap()
        );
    }

    #[test]
    fn source_and_destination_mode_verifies_successful_copy() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("a.txt"), vec![42u8; 10_000]).unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-verify".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Md5,
            "Source",
            &OrganizeSettings::default(),
        );

        assert!(outcome.failed_files.is_empty());
        assert_eq!(outcome.verified_files.len(), 1);
        assert_eq!(outcome.verified_files[0].algorithm, ChecksumAlgorithm::Md5);
        // Confirms the recorded checksum really is the destination's checksum,
        // not just an unchecked copy of whatever the source hasher produced.
        let dest_hash =
            checksum::hash_file(&dst_dir.path().join("a.txt"), ChecksumAlgorithm::Md5).unwrap();
        assert_eq!(outcome.verified_files[0].checksum, dest_hash);
    }

    #[test]
    fn re_running_the_same_transfer_skips_already_offloaded_files() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("clip.mp4"), b"camera footage").unwrap();

        let run = |cancel: &AtomicBool| {
            run_copy_core(
                &NoopSink,
                "job-dedup".to_string(),
                src_dir.path(),
                dst_dir.path(),
                cancel,
                VerificationMode::Transfer,
                ChecksumAlgorithm::Xxh64,
                "Source",
                &OrganizeSettings::default(),
            )
        };

        let first = run(&AtomicBool::new(false));
        assert_eq!(first.files_copied, 1);
        assert!(first.skipped_files.is_empty());

        // Re-offloading the same card to the same destination (a real DIT
        // workflow: verifying a card offloaded correctly before formatting
        // it) must not re-copy or corrupt what's already there.
        let second = run(&AtomicBool::new(false));
        assert_eq!(second.skipped_files.len(), 1);
        assert_eq!(second.skipped_files[0].path, "clip.mp4");
        assert_eq!(
            fs::read(dst_dir.path().join("clip.mp4")).unwrap(),
            b"camera footage"
        );
    }

    #[test]
    fn name_collision_with_different_content_is_renamed_not_overwritten() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();

        // Two different cards that both start clip numbering at C0001.mp4.
        fs::write(src_dir.path().join("C0001.mp4"), b"card A footage").unwrap();
        fs::write(dst_dir.path().join("C0001.mp4"), b"card B footage, already offloaded").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-collision".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
        );

        assert!(outcome.failed_files.is_empty());
        assert!(outcome.skipped_files.is_empty());
        assert_eq!(outcome.renamed_files.len(), 1);
        assert_eq!(outcome.renamed_files[0].original_path, "C0001.mp4");
        assert_eq!(outcome.renamed_files[0].renamed_to, "C0001 2.mp4");

        // The pre-existing file must survive untouched, and the new one
        // lands under the renamed path with the new card's actual content.
        assert_eq!(
            fs::read(dst_dir.path().join("C0001.mp4")).unwrap(),
            b"card B footage, already offloaded"
        );
        assert_eq!(
            fs::read(dst_dir.path().join("C0001 2.mp4")).unwrap(),
            b"card A footage"
        );
    }

    #[test]
    fn copy_preserves_the_source_modified_time() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();

        let src_path = src_dir.path().join("clip.mp4");
        fs::write(&src_path, b"camera footage").unwrap();

        // Simulate camera footage recorded years ago, not just-now test
        // fixture data -- a same-second mtime wouldn't be able to tell
        // "preserved the source's time" apart from "defaulted to now".
        let years_ago = SystemTime::now() - std::time::Duration::from_secs(5 * 365 * 24 * 3600);
        fs::OpenOptions::new()
            .write(true)
            .open(&src_path)
            .unwrap()
            .set_modified(years_ago)
            .unwrap();

        let cancel_flag = AtomicBool::new(false);
        run_copy_core(
            &NoopSink,
            "job-mtime".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
        );

        let dest_modified = fs::metadata(dst_dir.path().join("clip.mp4"))
            .unwrap()
            .modified()
            .unwrap();
        let source_modified = fs::metadata(&src_path).unwrap().modified().unwrap();
        assert_eq!(
            dest_modified, source_modified,
            "destination mtime must mirror the source's, or a second offload of the \
             same card would misdetect every file as a rename-worthy collision instead \
             of an already-copied duplicate"
        );
    }

    #[test]
    fn organize_rename_and_folder_templates_control_the_destination_layout() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("C0001.MP4"), b"footage").unwrap();

        let mut organize = OrganizeSettings::default();
        organize.rename_template = Some("{Source Name}_{File Counter}".to_string());
        organize.folder_template = Some("{File Extension}".to_string());

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-organize-rename".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "A-Cam",
            &organize,
        );

        assert!(outcome.failed_files.is_empty());
        assert_eq!(
            fs::read(dst_dir.path().join("MP4").join("A-Cam_00001.MP4")).unwrap(),
            b"footage"
        );
    }

    #[test]
    fn organize_selective_filter_excludes_matching_files_from_the_copy() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("clip.mp4"), b"footage").unwrap();
        fs::write(src_dir.path().join("clip.xml"), b"sidecar").unwrap();

        let mut organize = OrganizeSettings::default();
        organize.selective_copy = crate::organize::SelectiveCopyFilter {
            mode: crate::organize::SelectiveCopyMode::Exclude,
            patterns: vec![".xml".to_string()],
        };

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-organize-filter".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &organize,
        );

        assert_eq!(outcome.files_copied, 1);
        assert!(dst_dir.path().join("clip.mp4").exists());
        assert!(!dst_dir.path().join("clip.xml").exists());
    }

    #[test]
    fn organize_bundle_ignore_skips_a_small_bundle_but_keeps_a_populated_one() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();

        let empty_bundle = src_dir.path().join("PRIVATE");
        fs::create_dir_all(&empty_bundle).unwrap();
        fs::write(empty_bundle.join("stub.bin"), vec![0u8; 10]).unwrap();
        fs::write(src_dir.path().join("clip.mp4"), vec![0u8; 10_000]).unwrap();

        let mut organize = OrganizeSettings::default();
        organize.bundle_ignore = Some(crate::organize::BundleIgnoreRule {
            name: "PRIVATE".to_string(),
            max_size_bytes: 100,
        });

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-organize-bundle".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &organize,
        );

        assert_eq!(outcome.files_copied, 1);
        assert!(dst_dir.path().join("clip.mp4").exists());
        assert!(!dst_dir.path().join("PRIVATE").exists());
    }
}
