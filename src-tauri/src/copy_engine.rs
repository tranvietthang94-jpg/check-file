use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Instant, SystemTime};

use tauri::{AppHandle, Emitter, Manager, Runtime};
use walkdir::WalkDir;

use crate::checksum::{self, ChecksumAlgorithm, StreamingHasher};
use crate::dedup::{self, DuplicateAction};
use crate::mhl::{self, MhlFileEntry};
use crate::organize::{self, OrganizeSettings, TokenContext};
use crate::transfer_log::{self, TransferLogEntry};

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
pub const BROKEN_MEDIA_EVENT: &str = "copy-broken-media";

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB
const PROGRESS_THROTTLE: std::time::Duration = std::time::Duration::from_millis(200);
/// A file gets this many attempts total (1 initial + retries) before it's
/// reported as failed -- a transient read/write error (e.g. a flaky card
/// reader) often clears up if the source is simply read again.
const MAX_COPY_ATTEMPTS: u32 = 3;
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(200);

/// Pure decision extracted for testability -- never retries a cancelled
/// copy (the user asked it to stop, retrying would ignore that) or once
/// the attempt budget is spent.
fn should_retry_copy(attempt: u32, cancelled: bool) -> bool {
    attempt < MAX_COPY_ATTEMPTS && !cancelled
}

/// Deletes one source file once its copy is confirmed safe at the
/// destination (verified or an identical skip). A deletion failure (e.g. a
/// read-only card, or the file still open elsewhere) is recorded rather than
/// treated as a copy failure -- the destination copy is already good either way.
fn try_move_delete_source(
    absolute: &Path,
    relative_display: &str,
    deleted_source_files: &mut Vec<String>,
    move_delete_failed: &mut Vec<FailedFile>,
) {
    match fs::remove_file(absolute) {
        Ok(()) => deleted_source_files.push(relative_display.to_string()),
        Err(err) => move_delete_failed.push(FailedFile {
            path: relative_display.to_string(),
            message: err.to_string(),
        }),
    }
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FailedFile {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedFile {
    pub path: String,
    pub checksum: String,
    pub algorithm: ChecksumAlgorithm,
    pub legacy_checksum: Option<String>,
    pub legacy_algorithm: Option<ChecksumAlgorithm>,
}

/// A file that already existed at the destination with the same name, size,
/// and modified time -- treated as already offloaded and not copied again.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFile {
    pub path: String,
}

/// A file that collided on name with a different file already at the
/// destination (different size or modified time), so it was copied under a
/// new name instead of overwriting.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub deleted_source_files: Vec<String>,
    pub move_delete_failed: Vec<FailedFile>,
    /// Source-relative paths of every 0-byte file found on the source,
    /// whether or not the job actually paused on them (empty when
    /// `auto_continue_on_broken_media` was on or none were found).
    pub broken_media_files: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CancelledPayload {
    pub job_id: String,
}

/// Emitted once, right before the copy loop starts, when one or more 0-byte
/// files are found on the source -- a common symptom of a card that dropped
/// out mid-recording. The job blocks (see `JobRegistry::wait_for_broken_media_decision`)
/// until the frontend resolves it, unless `auto_continue_on_broken_media` skips
/// the alert entirely.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BrokenMediaPayload {
    pub job_id: String,
    pub files: Vec<String>,
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
    /// One entry per successfully (and, when verification ran, successfully
    /// verified) copied file -- feeds the MHL written at the destination root.
    pub mhl_entries: Vec<MhlFileEntry>,
    /// Source-relative paths removed from `source` after `move_after_transfer`
    /// confirmed each one was safely verified (or already identical) at the
    /// destination.
    pub deleted_source_files: Vec<String>,
    /// A file whose copy succeeded (and was verified) but whose *source* copy
    /// could not be deleted -- e.g. the card is read-only or the file is
    /// still open elsewhere. Never treated as a copy failure: the data is
    /// safely at the destination either way.
    pub move_delete_failed: Vec<FailedFile>,
    /// Source-relative paths of every 0-byte file found on the source. See
    /// `BrokenMediaPayload`.
    pub broken_media_files: Vec<String>,
}

/// Destination for scan/progress notifications. Kept separate from Tauri so
/// the copy core can be unit tested without spinning up an app runtime.
pub trait ProgressSink {
    fn on_scan(&self, total_files: u64, total_bytes: u64);
    fn on_progress(&self, payload: ProgressPayload);
    /// Called once, before the copy loop starts, when broken (0-byte) source
    /// files were found and the job isn't set to auto-continue past them.
    /// Returns `true` to proceed with the copy, `false` to abort it. The
    /// default (used by every sink that doesn't need real gating, e.g. tests)
    /// always continues.
    fn on_broken_media(&self, _files: &[String]) -> bool {
        true
    }
}

/// Blocks a job past a Broken Media alert until the frontend resolves it
/// (Continue/Cancel), or its cancel flag fires while still waiting. Mirrors
/// `crate::queue::JobQueue`'s condvar-gate shape, just keyed by decision
/// instead of admission.
#[derive(Default)]
struct BrokenMediaGate {
    decisions: Mutex<HashMap<String, Option<bool>>>,
    condvar: Condvar,
}

impl BrokenMediaGate {
    fn wait_for_decision(&self, job_id: &str, cancel_flag: &AtomicBool) -> bool {
        let mut decisions = self.decisions.lock().unwrap();
        decisions.insert(job_id.to_string(), None);
        loop {
            if cancel_flag.load(Ordering::SeqCst) {
                decisions.remove(job_id);
                return false;
            }
            if let Some(decision) = decisions.get(job_id).copied().flatten() {
                decisions.remove(job_id);
                return decision;
            }
            decisions = self.condvar.wait(decisions).unwrap();
        }
    }

    fn resolve(&self, job_id: &str, proceed: bool) {
        let mut decisions = self.decisions.lock().unwrap();
        if let Some(slot) = decisions.get_mut(job_id) {
            *slot = Some(proceed);
        }
        self.condvar.notify_all();
    }

    fn notify_all(&self) {
        self.condvar.notify_all();
    }
}

/// Tracks cancellation flags for in-flight copy jobs, keyed by job id,
/// keeps the system awake for as long as any of them are running, and gates
/// when each one's copy work may actually begin per the configured queue mode.
#[derive(Default)]
pub struct JobRegistry {
    cancel_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
    sleep_guard: crate::power::SleepGuard,
    job_queue: crate::queue::JobQueue,
    broken_media_gate: BrokenMediaGate,
}

impl JobRegistry {
    pub fn register(&self, job_id: String) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.cancel_flags.lock().unwrap().insert(job_id, flag.clone());
        self.sleep_guard.job_started();
        flag
    }

    /// Returns true if a job with this id was found and signalled to stop.
    pub fn cancel(&self, job_id: &str) -> bool {
        match self.cancel_flags.lock().unwrap().get(job_id) {
            Some(flag) => {
                flag.store(true, Ordering::SeqCst);
                // Wakes it immediately if it's still waiting in the queue or
                // blocked on a Broken Media alert, instead of only noticing
                // the cancellation once admitted / resolved.
                self.job_queue.notify_all();
                self.broken_media_gate.notify_all();
                true
            }
            None => false,
        }
    }

    pub fn remove(&self, job_id: &str) {
        // Only release sleep prevention/queue state for a job that
        // genuinely existed -- an unmatched call would decrement past zero.
        if self.cancel_flags.lock().unwrap().remove(job_id).is_some() {
            self.sleep_guard.job_finished();
            self.job_queue.job_finished(job_id);
        }
    }

    pub fn set_sleep_prevention_enabled(&self, enabled: bool) {
        self.sleep_guard.set_enabled(enabled);
    }

    pub fn set_queue_mode(&self, mode: crate::queue::QueueMode) {
        self.job_queue.set_mode(mode);
    }

    /// Blocks until `job_id` (part of `group_id`) is allowed to start
    /// copying, or returns `false` early if cancelled while still waiting.
    pub fn wait_for_turn(&self, job_id: &str, group_id: &str, cancel_flag: &AtomicBool) -> bool {
        self.job_queue.wait_for_turn(job_id, group_id, cancel_flag)
    }

    /// Blocks until the frontend resolves the Broken Media alert for `job_id`
    /// (via `resolve_broken_media`) or `cancel_flag` fires while still
    /// waiting, in which case it returns `false` as if the user chose to abort.
    pub fn wait_for_broken_media_decision(&self, job_id: &str, cancel_flag: &AtomicBool) -> bool {
        self.broken_media_gate.wait_for_decision(job_id, cancel_flag)
    }

    /// Resolves a pending Broken Media alert: `true` continues the copy,
    /// `false` aborts it. A no-op if no job is currently waiting under this id.
    pub fn resolve_broken_media(&self, job_id: &str, proceed: bool) {
        self.broken_media_gate.resolve(job_id, proceed);
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
    /// `legacy_hash` is `Some` only when a legacy checksum algorithm was
    /// also requested (OffShoot's "Also generate legacy checksums").
    Completed {
        source_hash: Option<String>,
        legacy_hash: Option<String>,
    },
}

/// The path a file is streamed into while its copy (and, for Source &
/// Destination mode, its verification) is still in progress. Never renamed
/// to `dest_path` until the copy -- and, when applicable, the destination
/// re-read -- has actually succeeded, so an interrupted copy (cancelled,
/// crashed, or a plain I/O failure) never leaves anything behind under the
/// final name for Duplicate Detection to later mistake for a genuinely
/// different, unrelated file of the same name. A leftover from a previous
/// crashed attempt is silently overwritten by `fs::File::create` the next
/// time the same destination path is attempted, so no separate cleanup pass
/// is needed at job start.
fn staging_path_for(dest_path: &Path) -> PathBuf {
    let mut name = dest_path.file_name().unwrap_or_default().to_os_string();
    name.push(".ofkit-partial");
    dest_path.with_file_name(name)
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
    legacy_checksum_algorithm: Option<ChecksumAlgorithm>,
    source_modified: SystemTime,
) -> std::io::Result<CopyFileOutcome> {
    let mut src_file = fs::File::open(src)?;
    let dst_file = fs::File::create(dst)?;
    let mut hasher = compute_hash.then(|| StreamingHasher::new(checksum_algorithm));
    let mut legacy_hasher = compute_hash
        .then_some(legacy_checksum_algorithm)
        .flatten()
        .map(StreamingHasher::new);

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
        if let Some(h) = legacy_hasher.as_mut() {
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
        legacy_hash: legacy_hasher.map(|h| h.finalize_hex()),
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
    move_after_transfer: bool,
    move_same_volume: bool,
    legacy_checksum_algorithm: Option<ChecksumAlgorithm>,
) -> CopyOutcome {
    // OffShoot's "Don't copy but move data when a Source and Destination are
    // located on the same volume": when both resolve to the same physical
    // disk, an `fs::rename` relocates the file instantly with no byte
    // duplication -- there's nothing to "copy" since it's already the same
    // bytes on the same filesystem. Checked once per job, not per file, since
    // the source/destination roots don't change mid-job.
    let same_volume_move_active = move_same_volume
        && crate::disks::volume_signature(&source.display().to_string())
            .zip(crate::disks::volume_signature(&destination.display().to_string()))
            .is_some_and(|(a, b)| a == b);
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
                mhl_entries: Vec::new(),
                deleted_source_files: Vec::new(),
                move_delete_failed: Vec::new(),
                broken_media_files: Vec::new(),
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
    entries.retain(|e| !organize::is_os_junk_file(&e.relative));

    let total_files = entries.len() as u64;
    let total_bytes: u64 = entries.iter().map(|e| e.size).sum();
    sink.on_scan(total_files, total_bytes);

    // Broken Media Detection: a 0-byte file is almost always a card that
    // dropped out mid-recording. Checked once up front (not that it would
    // matter mid-loop -- an empty file copies instantly either way), so the
    // alert can't be missed by copying past it before the user notices.
    let broken_media_files: Vec<String> = entries
        .iter()
        .filter(|e| e.size == 0)
        .map(|e| e.relative.display().to_string())
        .collect();
    if !broken_media_files.is_empty()
        && !organize.auto_continue_on_broken_media
        && !sink.on_broken_media(&broken_media_files)
    {
        return CopyOutcome {
            cancelled: true,
            files_copied: 0,
            bytes_copied: 0,
            failed_files: Vec::new(),
            verified_files: Vec::new(),
            skipped_files: Vec::new(),
            renamed_files: Vec::new(),
            mhl_entries: Vec::new(),
            deleted_source_files: Vec::new(),
            move_delete_failed: Vec::new(),
            broken_media_files,
        };
    }

    let job_started = organize::effective_job_date(SystemTime::now(), &organize.date_override);
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
    // Deleting the only copy of a source file is never worth the speed of
    // Transfer mode's size-only check -- Move only ever acts on a file once
    // it's been through an actual hash comparison.
    let can_move = move_after_transfer && compute_hash;
    // MHL Awareness: if the source already carries an MHL (e.g. from a camera
    // system or a prior offload) recording a matching size/mtime for this
    // exact checksum algorithm, its checksum can stand in for hashing the
    // source again -- the file itself still gets copied and, in
    // Source & Destination mode, the destination is still independently
    // re-read and compared, so no integrity guarantee is weakened.
    let source_mhl_index = if compute_hash {
        mhl::load_source_mhl_index(source)
    } else {
        HashMap::new()
    };
    let mut tracker = ProgressTracker::new(sink, job_id, total_bytes, total_files);
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut failed_files = Vec::new();
    let mut verified_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut renamed_files = Vec::new();
    let mut mhl_entries = Vec::new();
    let mut deleted_source_files = Vec::new();
    let mut move_delete_failed = Vec::new();
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
            elements: organize.elements.clone(),
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

        match dedup::resolve_duplicate(
            &dest_path,
            entry.size,
            entry.modified,
            organize.skip_modification_date_check,
        ) {
            Ok(DuplicateAction::Copy) => {}
            Ok(DuplicateAction::Skip) => {
                tracker.add_bytes(entry.size, &relative_display);
                tracker.finish_file();
                if can_move {
                    try_move_delete_source(
                        &entry.absolute,
                        &relative_display,
                        &mut deleted_source_files,
                        &mut move_delete_failed,
                    );
                }
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

        let known_hash = mhl::reusable_checksum(
            &source_mhl_index,
            &entry.relative,
            entry.size,
            entry.modified,
            checksum_algorithm,
        );
        let compute_hash_for_this_file = compute_hash && known_hash.is_none();

        if same_volume_move_active {
            // Still hash the file first (a plain read, same as any other
            // verification mode would do) so Reports/MHLs get a real
            // checksum -- only the write side skips the redundant
            // copy-then-delete dance a cross-volume Move needs.
            let (hash, legacy_hash) = if compute_hash_for_this_file {
                match checksum::hash_file_dual(
                    &entry.absolute,
                    checksum_algorithm,
                    legacy_checksum_algorithm,
                ) {
                    Ok((h, legacy)) => (Some(h), legacy),
                    Err(err) => {
                        failed_files.push(FailedFile {
                            path: copy_display.clone(),
                            message: format!("could not hash file before moving: {err}"),
                        });
                        continue;
                    }
                }
            } else {
                (known_hash.clone(), None)
            };
            match fs::rename(&entry.absolute, &dest_path) {
                Ok(()) => {
                    tracker.add_bytes(entry.size, &copy_display);
                    tracker.finish_file();
                    if let Some(h) = &hash {
                        verified_files.push(VerifiedFile {
                            path: copy_display.clone(),
                            checksum: h.clone(),
                            algorithm: checksum_algorithm,
                            legacy_checksum: legacy_hash.clone(),
                            legacy_algorithm: legacy_hash.as_ref().and(legacy_checksum_algorithm),
                        });
                    }
                    mhl_entries.push(MhlFileEntry {
                        relative_path: copy_display.clone(),
                        size: entry.size,
                        modified: entry.modified,
                        checksum: hash,
                        algorithm: checksum_algorithm,
                        hashed_at: SystemTime::now(),
                        legacy_checksum: legacy_hash,
                        legacy_algorithm: legacy_checksum_algorithm,
                    });
                    deleted_source_files.push(copy_display);
                }
                Err(err) => {
                    failed_files.push(FailedFile {
                        path: copy_display,
                        message: format!("could not move file on the same volume: {err}"),
                    });
                }
            }
            continue;
        }

        let staging_path = staging_path_for(&dest_path);

        let mut attempt = 1;
        let copy_result = loop {
            // A failed attempt may have streamed some bytes into the
            // tracker before erroring out -- roll that back before retrying
            // from scratch, or the progress/byte totals would double-count
            // bytes from the doomed attempt.
            let bytes_before_attempt = tracker.bytes_copied;
            let result = copy_file_chunked(
                &entry.absolute,
                &staging_path,
                &mut buffer,
                cancel_flag,
                &mut tracker,
                &copy_display,
                checksum_algorithm,
                compute_hash_for_this_file,
                legacy_checksum_algorithm,
                entry.modified,
            );
            if result.is_err() && should_retry_copy(attempt, cancel_flag.load(Ordering::SeqCst)) {
                tracker.bytes_copied = bytes_before_attempt;
                attempt += 1;
                std::thread::sleep(RETRY_DELAY);
                continue;
            }
            break result;
        };

        match copy_result {
            Ok(CopyFileOutcome::Completed { source_hash, legacy_hash }) => {
                tracker.finish_file();
                // Falls back to the MHL-Awareness checksum when this file's
                // hashing was skipped because it was already known-good.
                // MHL Awareness never carries a legacy checksum forward
                // (the reused entry only recorded the primary algorithm),
                // so a known-good file simply has no legacy hash this run.
                let source_hash = source_hash.or_else(|| known_hash.clone());
                // Feeds the MHL: `None` means Transfer mode (size-only entry, no
                // hash computed); a failed/mismatched verification below clears
                // `record_in_mhl` so a bad copy is never recorded as trustworthy.
                let mut mhl_checksum: Option<String> = None;
                let mut record_in_mhl = true;

                // Only a Source & Destination re-read proves the bytes landed
                // intact -- every other path (Transfer's size-only check,
                // Source's stream-time-only hash) treats the streaming copy
                // finishing without an I/O error as all the proof its mode
                // ever promised. Either way, nothing is renamed into its
                // final `dest_path` name until this check passes.
                let verified_ok = if let Some(hash) = &source_hash {
                    if verification_mode == VerificationMode::SourceAndDestination {
                        match checksum::verify_file_hash(&staging_path, hash, checksum_algorithm) {
                            Ok(true) => true,
                            Ok(false) => {
                                failed_files.push(FailedFile {
                                    path: copy_display.clone(),
                                    message:
                                        "checksum mismatch: source and destination differ after copy"
                                            .to_string(),
                                });
                                record_in_mhl = false;
                                false
                            }
                            Err(err) => {
                                failed_files.push(FailedFile {
                                    path: copy_display.clone(),
                                    message: format!(
                                        "could not re-read destination for verification: {err}"
                                    ),
                                });
                                record_in_mhl = false;
                                false
                            }
                        }
                    } else {
                        true
                    }
                } else {
                    true
                };

                if verified_ok {
                    match fs::rename(&staging_path, &dest_path) {
                        Ok(()) => {
                            if let Some(hash) = &source_hash {
                                verified_files.push(VerifiedFile {
                                    path: copy_display.clone(),
                                    checksum: hash.clone(),
                                    algorithm: checksum_algorithm,
                                    legacy_checksum: legacy_hash.clone(),
                                    legacy_algorithm: legacy_hash.as_ref().and(legacy_checksum_algorithm),
                                });
                                mhl_checksum = Some(hash.clone());
                            }
                            if can_move {
                                try_move_delete_source(
                                    &entry.absolute,
                                    &copy_display,
                                    &mut deleted_source_files,
                                    &mut move_delete_failed,
                                );
                            }
                        }
                        Err(err) => {
                            failed_files.push(FailedFile {
                                path: copy_display.clone(),
                                message: format!("copied file could not be moved into place: {err}"),
                            });
                            record_in_mhl = false;
                            let _ = fs::remove_file(&staging_path);
                        }
                    }
                } else {
                    let _ = fs::remove_file(&staging_path);
                }

                if record_in_mhl {
                    mhl_entries.push(MhlFileEntry {
                        relative_path: copy_display,
                        size: entry.size,
                        modified: entry.modified,
                        checksum: mhl_checksum,
                        algorithm: checksum_algorithm,
                        hashed_at: SystemTime::now(),
                        legacy_checksum: legacy_hash,
                        legacy_algorithm: legacy_checksum_algorithm,
                    });
                }
            }
            Ok(CopyFileOutcome::Cancelled) => {
                cancelled = true;
                break;
            }
            Err(err) => {
                // The streaming copy never finished -- nothing under the
                // final `dest_path` name to worry about, but the staging
                // file it was writing into could still be sitting there
                // partially written.
                let _ = fs::remove_file(&staging_path);
                failed_files.push(FailedFile {
                    path: copy_display,
                    message: if attempt > 1 {
                        format!("{err} (failed after {attempt} attempts)")
                    } else {
                        err.to_string()
                    },
                });
            }
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
        mhl_entries,
        deleted_source_files,
        move_delete_failed,
        broken_media_files,
    }
}

struct TauriProgressSink<'a, R: Runtime> {
    app_handle: &'a AppHandle<R>,
    job_id: String,
    cancel_flag: Arc<AtomicBool>,
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

    fn on_broken_media(&self, files: &[String]) -> bool {
        let _ = self.app_handle.emit(
            BROKEN_MEDIA_EVENT,
            BrokenMediaPayload {
                job_id: self.job_id.clone(),
                files: files.to_vec(),
            },
        );
        self.app_handle
            .state::<JobRegistry>()
            .wait_for_broken_media_decision(&self.job_id, &self.cancel_flag)
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
    move_after_transfer: bool,
    move_same_volume: bool,
    legacy_checksum_algorithm: Option<ChecksumAlgorithm>,
    save_log_to_destination: bool,
    create_per_file_mhl: bool,
) -> CopyOutcome {
    let sink = TauriProgressSink {
        app_handle: &app_handle,
        job_id: job_id.clone(),
        cancel_flag: cancel_flag.clone(),
    };
    let started_at = SystemTime::now();
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
        move_after_transfer,
        move_same_volume,
        legacy_checksum_algorithm,
    );
    let finished_at = SystemTime::now();

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
                deleted_source_files: outcome.deleted_source_files.clone(),
                move_delete_failed: outcome.move_delete_failed.clone(),
                broken_media_files: outcome.broken_media_files.clone(),
            },
        );
    }

    // Recorded either way (Stop & Resume): even a Stopped job already has
    // real, verified work worth an audit trail and an MHL for what it did
    // manage to copy -- Resume is just a fresh job over the same
    // source/destination afterward, relying on Duplicate Detection (and,
    // when the algorithm matches, MHL Awareness) to skip what's already here.
    let mhl_path = mhl::write_mhl(&destination, &outcome.mhl_entries, started_at, finished_at)
        .ok()
        .flatten()
        .map(|p| p.display().to_string());

    if create_per_file_mhl {
        mhl::write_per_file_mhls(&destination, &outcome.mhl_entries, started_at, finished_at);
    }

    let log_entry = TransferLogEntry {
        job_id: job_id.clone(),
        source_name,
        source: source.display().to_string(),
        destination: destination.display().to_string(),
        verification_mode,
        checksum_algorithm,
        started_at: mhl::iso8601(started_at),
        finished_at: mhl::iso8601(finished_at),
        files_copied: outcome.files_copied,
        bytes_copied: outcome.bytes_copied,
        failed_files: outcome.failed_files.clone(),
        verified_files: outcome.verified_files.clone(),
        skipped_files: outcome.skipped_files.clone(),
        renamed_files: outcome.renamed_files.clone(),
        deleted_source_files: outcome.deleted_source_files.clone(),
        move_delete_failed: outcome.move_delete_failed.clone(),
        broken_media_files: outcome.broken_media_files.clone(),
        mhl_path,
        cancelled: outcome.cancelled,
    };
    let _ = transfer_log::save_log(&app_handle, &log_entry);
    // OffShoot's "Include Transfer Logs ... on Destination" -- the JSON log
    // is always saved locally (above); this additionally drops a copy at
    // the destination root, mirroring where the MHL already always lands.
    if save_log_to_destination {
        let _ = transfer_log::save_log_to_dir(&destination, &log_entry);
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
    fn retries_up_to_the_attempt_budget_then_gives_up() {
        assert!(should_retry_copy(1, false));
        assert!(should_retry_copy(2, false));
        assert!(
            !should_retry_copy(MAX_COPY_ATTEMPTS, false),
            "the last allotted attempt must not trigger yet another retry"
        );
    }

    #[test]
    fn never_retries_a_cancelled_copy() {
        assert!(
            !should_retry_copy(1, true),
            "retrying after cancellation would ignore the user's request to stop"
        );
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert!(outcome.cancelled);
        assert!(!dst_dir.path().join("big.bin").exists());
        assert!(
            !dst_dir.path().join("big.bin.ofkit-partial").exists(),
            "the staging file it was streaming into must be cleaned up too, or it would sit \
             there forever under a name Duplicate Detection never even looks at"
        );
    }

    #[test]
    fn a_stale_staging_leftover_from_a_previous_crash_is_silently_overwritten() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("clip.mov"), b"camera footage").unwrap();

        // Simulates the app being killed mid-copy on a previous run: a
        // half-written staging file left behind under the same name this
        // run will also pick for its own staging path.
        fs::write(dst_dir.path().join("clip.mov.ofkit-partial"), b"GARBAGE-FROM-A-CRASHED-RUN")
            .unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-stale-staging".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert!(outcome.failed_files.is_empty());
        assert_eq!(
            fs::read(dst_dir.path().join("clip.mov")).unwrap(),
            b"camera footage",
            "a stale leftover staging file must not corrupt or block a fresh retry"
        );
        assert!(!dst_dir.path().join("clip.mov.ofkit-partial").exists());
    }

    #[test]
    fn checksum_mismatch_in_source_and_destination_mode_leaves_no_file_under_any_name() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mov");
        fs::write(&src_file, b"camera footage").unwrap();
        let meta = fs::metadata(&src_file).unwrap();

        // A source MHL claiming a checksum that doesn't match the real bytes
        // (MHL Awareness reuses it instead of rehashing) deterministically
        // forces the Source & Destination re-read below to detect a
        // mismatch, without needing to corrupt anything mid-write.
        let bogus_entry = MhlFileEntry {
            relative_path: "clip.mov".to_string(),
            size: meta.len(),
            modified: meta.modified().unwrap(),
            checksum: Some("0000000000000000".to_string()),
            algorithm: ChecksumAlgorithm::Xxh64,
            hashed_at: SystemTime::now(),
            legacy_checksum: None, legacy_algorithm: None,
        };
        mhl::write_mhl(src_dir.path(), &[bogus_entry], SystemTime::now(), SystemTime::now()).unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-mismatch".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        // `write_mhl` drops the source MHL directly into `src_dir`, so the
        // scan also picks up that `.mhl` file itself as an ordinary (and
        // perfectly verifiable) file to copy -- assertions below are scoped
        // to `clip.mov` specifically rather than the whole outcome.
        assert_eq!(outcome.failed_files.len(), 1);
        assert_eq!(outcome.failed_files[0].path, "clip.mov");
        assert!(!outcome.verified_files.iter().any(|f| f.path == "clip.mov"));
        assert!(!outcome.mhl_entries.iter().any(|e| e.relative_path == "clip.mov"));
        assert!(
            !dst_dir.path().join("clip.mov").exists(),
            "an unverified copy must never be renamed into its final, trusted-looking name"
        );
        assert!(
            !dst_dir.path().join("clip.mov.ofkit-partial").exists(),
            "the failed staging attempt must be cleaned up, not left behind indefinitely"
        );
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
                false,
                false, // move_same_volume
                None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
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
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert_eq!(outcome.files_copied, 1);
        assert!(dst_dir.path().join("clip.mp4").exists());
        assert!(!dst_dir.path().join("PRIVATE").exists());
    }

    #[test]
    fn os_junk_files_are_never_copied() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("clip.mp4"), b"footage").unwrap();
        fs::write(src_dir.path().join(".DS_Store"), b"finder metadata").unwrap();
        fs::write(src_dir.path().join("Thumbs.db"), b"thumbnail cache").unwrap();
        fs::create_dir_all(src_dir.path().join("System Volume Information")).unwrap();
        fs::write(
            src_dir.path().join("System Volume Information/IndexerVolumeGuid"),
            b"junk",
        )
        .unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-junk".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert_eq!(outcome.files_copied, 1);
        assert!(dst_dir.path().join("clip.mp4").exists());
        assert!(!dst_dir.path().join(".DS_Store").exists());
        assert!(!dst_dir.path().join("Thumbs.db").exists());
        assert!(!dst_dir.path().join("System Volume Information").exists());
    }

    #[test]
    fn verified_copies_populate_mhl_entries_with_the_matching_checksum() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("clip.mp4"), b"camera footage").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-mhl".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert_eq!(outcome.mhl_entries.len(), 1);
        assert_eq!(outcome.mhl_entries[0].relative_path, "clip.mp4");
        assert_eq!(
            outcome.mhl_entries[0].checksum.as_deref(),
            Some(outcome.verified_files[0].checksum.as_str()),
        );
    }

    #[test]
    fn a_legacy_checksum_algorithm_is_computed_alongside_the_primary_one() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-legacy-checksum".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            Some(ChecksumAlgorithm::Sha1), // legacy_checksum_algorithm
        );

        let expected_legacy = checksum::hash_file(&src_file, ChecksumAlgorithm::Sha1).unwrap();
        assert_eq!(
            outcome.verified_files[0].legacy_checksum.as_deref(),
            Some(expected_legacy.as_str())
        );
        assert_eq!(
            outcome.verified_files[0].legacy_algorithm,
            Some(ChecksumAlgorithm::Sha1)
        );
        assert_eq!(
            outcome.mhl_entries[0].legacy_checksum.as_deref(),
            Some(expected_legacy.as_str())
        );

        let xml = mhl::render_mhl(&outcome.mhl_entries, SystemTime::now(), SystemTime::now());
        assert!(xml.contains(&format!("<sha1>{expected_legacy}</sha1>")));
    }

    #[test]
    fn transfer_mode_copies_still_populate_mhl_entries_without_a_checksum() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("clip.mp4"), b"camera footage").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-mhl-transfer".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert_eq!(outcome.mhl_entries.len(), 1);
        assert!(
            outcome.mhl_entries[0].checksum.is_none(),
            "Transfer mode never computes a hash, so the MHL entry must be size-only"
        );
    }

    #[test]
    fn mhl_awareness_reuses_a_matching_source_checksum_instead_of_rehashing() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();
        let meta = fs::metadata(&src_file).unwrap();

        // Plants a source MHL recording a deliberately *wrong* checksum: if
        // the copy engine's MHL Awareness reuses it instead of rehashing the
        // real bytes, the recorded checksum will be this bogus value rather
        // than the file's true XXH64.
        let fake_entry = MhlFileEntry {
            relative_path: "clip.mp4".to_string(),
            size: meta.len(),
            modified: meta.modified().unwrap(),
            checksum: Some("deadbeefdeadbeef".to_string()),
            algorithm: ChecksumAlgorithm::Xxh64,
            hashed_at: SystemTime::now(),
            legacy_checksum: None, legacy_algorithm: None,
        };
        mhl::write_mhl(src_dir.path(), &[fake_entry], SystemTime::now(), SystemTime::now()).unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-mhl-awareness".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Source,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert!(outcome.failed_files.is_empty());
        let clip_verification = outcome
            .verified_files
            .iter()
            .find(|f| f.path == "clip.mp4")
            .expect("clip.mp4 should have been copied and verified");
        assert_eq!(
            clip_verification.checksum, "deadbeefdeadbeef",
            "the reused MHL checksum should be recorded, proving the source wasn't rehashed"
        );
    }

    #[test]
    fn mhl_awareness_ignores_a_source_mhl_entry_whose_size_no_longer_matches() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();
        let meta = fs::metadata(&src_file).unwrap();

        // The MHL's recorded size no longer matches the live file, so its
        // (bogus) checksum must be ignored and the real content rehashed.
        let stale_entry = MhlFileEntry {
            relative_path: "clip.mp4".to_string(),
            size: meta.len() + 1,
            modified: meta.modified().unwrap(),
            checksum: Some("deadbeefdeadbeef".to_string()),
            algorithm: ChecksumAlgorithm::Xxh64,
            hashed_at: SystemTime::now(),
            legacy_checksum: None, legacy_algorithm: None,
        };
        mhl::write_mhl(src_dir.path(), &[stale_entry], SystemTime::now(), SystemTime::now()).unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-mhl-awareness-stale".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Source,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        let clip_verification = outcome
            .verified_files
            .iter()
            .find(|f| f.path == "clip.mp4")
            .expect("clip.mp4 should have been copied and verified");
        assert_eq!(
            clip_verification.checksum,
            checksum::hash_file(&src_file, ChecksumAlgorithm::Xxh64).unwrap(),
            "a stale MHL entry must never override a freshly computed checksum"
        );
    }

    #[test]
    fn move_deletes_the_source_once_verified() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-move".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            true,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert!(outcome.failed_files.is_empty());
        assert_eq!(outcome.deleted_source_files, vec!["clip.mp4".to_string()]);
        assert!(outcome.move_delete_failed.is_empty());
        assert!(!src_file.exists(), "the source copy should be gone once verified");
        assert_eq!(
            fs::read(dst_dir.path().join("clip.mp4")).unwrap(),
            b"camera footage",
            "the destination copy must still be intact"
        );
    }

    #[test]
    fn same_volume_move_renames_instead_of_copying_and_still_records_a_checksum() {
        // Both tempdirs land under the OS temp root, i.e. the same real
        // volume in any normal test environment -- exercising the actual
        // `disks::volume_signature` comparison, not a stub.
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-same-volume-move".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Source,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            true, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert!(outcome.failed_files.is_empty());
        assert!(!src_file.exists(), "the source file should have been renamed away");
        assert_eq!(outcome.deleted_source_files, vec!["clip.mp4".to_string()]);
        assert_eq!(
            fs::read(dst_dir.path().join("clip.mp4")).unwrap(),
            b"camera footage"
        );
        assert_eq!(
            outcome.verified_files.first().map(|f| &f.checksum),
            Some(&checksum::hash_file(dst_dir.path().join("clip.mp4").as_path(), ChecksumAlgorithm::Xxh64).unwrap()),
            "a checksum should still be recorded even though the write side was a rename"
        );
    }

    #[test]
    fn same_volume_move_is_a_noop_fast_path_when_disabled() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-same-volume-move-disabled".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Source,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert!(outcome.failed_files.is_empty());
        assert!(
            src_file.exists(),
            "with the setting off, a same-volume transfer must still copy (not move) the source"
        );
        assert!(outcome.deleted_source_files.is_empty());
    }

    #[test]
    fn move_never_deletes_anything_in_transfer_mode() {
        // Transfer mode never hashes, so there's no cryptographic proof the
        // copy is intact -- Move must refuse to delete the only copy of a
        // file on that basis alone, regardless of the caller's setting.
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &NoopSink,
            "job-move-transfer".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            true,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert!(outcome.deleted_source_files.is_empty());
        assert!(src_file.exists(), "Transfer mode gives no verification to delete on");
    }

    #[test]
    fn move_deletes_the_source_of_an_already_offloaded_skip() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();

        // First offload leaves an identical file at the destination.
        run_copy_core(
            &NoopSink,
            "job-move-skip-1".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &AtomicBool::new(false),
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );
        assert!(src_file.exists());

        // Re-running with Move on should still remove the source even though
        // this pass only *skips* the file (dedup, not a fresh verified copy)
        // -- the data is already safely and identically at the destination.
        let outcome = run_copy_core(
            &NoopSink,
            "job-move-skip-2".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &AtomicBool::new(false),
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            true,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert_eq!(outcome.skipped_files.len(), 1);
        assert_eq!(outcome.deleted_source_files, vec!["clip.mp4".to_string()]);
        assert!(!src_file.exists());
    }

    #[test]
    fn broken_media_is_detected_and_still_copied_when_the_sink_allows_it() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("good.mp4"), b"footage").unwrap();
        fs::write(src_dir.path().join("dropped.mp4"), b"").unwrap();

        let cancel_flag = AtomicBool::new(false);
        // NoopSink's default `on_broken_media` continues, mirroring
        // `auto_continue_on_broken_media` being on from the sink's point of view.
        let outcome = run_copy_core(
            &NoopSink,
            "job-broken-media".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert_eq!(outcome.broken_media_files, vec!["dropped.mp4".to_string()]);
        // Still copied -- Broken Media Detection is an alert, not a filter.
        assert_eq!(outcome.files_copied, 2);
        assert!(dst_dir.path().join("dropped.mp4").exists());
    }

    #[test]
    fn broken_media_alert_can_abort_the_job_before_anything_is_copied() {
        struct AbortingSink;
        impl ProgressSink for AbortingSink {
            fn on_scan(&self, _total_files: u64, _total_bytes: u64) {}
            fn on_progress(&self, _payload: ProgressPayload) {}
            fn on_broken_media(&self, _files: &[String]) -> bool {
                false
            }
        }

        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("good.mp4"), b"footage").unwrap();
        fs::write(src_dir.path().join("dropped.mp4"), b"").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &AbortingSink,
            "job-broken-media-abort".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert!(outcome.cancelled);
        assert_eq!(outcome.files_copied, 0);
        assert_eq!(outcome.broken_media_files, vec!["dropped.mp4".to_string()]);
        assert!(
            !dst_dir.path().join("good.mp4").exists(),
            "aborting on the alert must happen before any file (even a healthy one) is copied"
        );
    }

    #[test]
    fn auto_continue_on_broken_media_skips_the_alert_entirely() {
        struct PanicsIfAskedSink;
        impl ProgressSink for PanicsIfAskedSink {
            fn on_scan(&self, _total_files: u64, _total_bytes: u64) {}
            fn on_progress(&self, _payload: ProgressPayload) {}
            fn on_broken_media(&self, _files: &[String]) -> bool {
                panic!("must not be called when auto_continue_on_broken_media is set");
            }
        }

        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(src_dir.path().join("dropped.mp4"), b"").unwrap();

        let mut organize = OrganizeSettings::default();
        organize.auto_continue_on_broken_media = true;

        let cancel_flag = AtomicBool::new(false);
        let outcome = run_copy_core(
            &PanicsIfAskedSink,
            "job-broken-media-auto".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &cancel_flag,
            VerificationMode::Transfer,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &organize,
            false,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        // Still recorded for the transfer log/report even though nobody was asked.
        assert_eq!(outcome.broken_media_files, vec!["dropped.mp4".to_string()]);
        assert_eq!(outcome.files_copied, 1);
    }

    #[test]
    fn move_never_deletes_a_file_that_failed_to_copy() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(src_dir.path().join("sub")).unwrap();
        let src_file = src_dir.path().join("sub").join("clip.mp4");
        fs::write(&src_file, b"camera footage").unwrap();

        // A plain file already sitting where the destination's parent
        // directory needs to go forces `create_dir_all` to fail -- portable
        // and independent of any real disk-full/permission error.
        fs::write(dst_dir.path().join("sub"), b"not a directory").unwrap();

        let outcome = run_copy_core(
            &NoopSink,
            "job-move-failed".to_string(),
            src_dir.path(),
            dst_dir.path(),
            &AtomicBool::new(false),
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
            "Source",
            &OrganizeSettings::default(),
            true,
            false, // move_same_volume
            None, // legacy_checksum_algorithm
        );

        assert_eq!(outcome.failed_files.len(), 1);
        assert!(outcome.deleted_source_files.is_empty());
        assert!(src_file.exists(), "a failed copy must never delete the only remaining copy");
    }
}
