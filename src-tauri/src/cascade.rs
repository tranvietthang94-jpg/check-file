use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use uuid::Uuid;

use crate::checksum::ChecksumAlgorithm;
use crate::copy_engine::{self, CopyOutcome, JobRegistry, VerificationMode};

pub const GROUP_JOB_ADDED_EVENT: &str = "transfer-group-job-added";

/// How a source fans out to multiple destinations.
/// - `Parallel`: every destination copies directly from the original source.
/// - `Cascade`: the source is read once into the first ("primary") destination,
///   then the remaining destinations copy from that primary instead of the
///   original source -- useful when the source is a slow card reader and the
///   primary destination is a fast local disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferGroupMode {
    Parallel,
    Cascade,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupJobAddedPayload {
    pub group_id: String,
    pub job_id: String,
    pub source: String,
    pub destination: String,
    /// 1 = copies from the original source. 2 = relayed from the primary
    /// destination (Cascade mode only, spawned after hop 1 succeeds).
    pub hop: u8,
}

/// A relay hop should only run if the file(s) it depends on are actually
/// intact on disk. Pure decision, no I/O, so it's trivially unit-testable
/// without touching Tauri.
fn should_cascade_continue(hop1_outcome: &CopyOutcome) -> bool {
    !hop1_outcome.cancelled && hop1_outcome.failed_files.is_empty()
}

fn spawn_job<R: Runtime>(
    app_handle: &AppHandle<R>,
    group_id: &str,
    source: PathBuf,
    destination: PathBuf,
    hop: u8,
    verification_mode: VerificationMode,
    checksum_algorithm: ChecksumAlgorithm,
) -> String {
    let job_id = Uuid::new_v4().to_string();
    let cancel_flag = app_handle.state::<JobRegistry>().register(job_id.clone());

    let _ = app_handle.emit(
        GROUP_JOB_ADDED_EVENT,
        GroupJobAddedPayload {
            group_id: group_id.to_string(),
            job_id: job_id.clone(),
            source: source.display().to_string(),
            destination: destination.display().to_string(),
            hop,
        },
    );

    let app_handle_thread = app_handle.clone();
    let job_id_thread = job_id.clone();
    std::thread::spawn(move || {
        copy_engine::run_copy_job(
            app_handle_thread,
            job_id_thread,
            source,
            destination,
            cancel_flag,
            verification_mode,
            checksum_algorithm,
        );
    });

    job_id
}

/// Kicks off a source -> many-destinations transfer. Returns immediately with
/// a group id; individual jobs (including cascade hop 2, which doesn't exist
/// yet when this returns) announce themselves via `GROUP_JOB_ADDED_EVENT` so
/// the frontend can start tracking each one as it's created.
pub fn start_transfer_group<R: Runtime>(
    app_handle: AppHandle<R>,
    source: PathBuf,
    destinations: Vec<PathBuf>,
    mode: TransferGroupMode,
    verification_mode: VerificationMode,
    checksum_algorithm: ChecksumAlgorithm,
) -> String {
    let group_id = Uuid::new_v4().to_string();

    match mode {
        TransferGroupMode::Parallel => {
            for destination in destinations {
                spawn_job(
                    &app_handle,
                    &group_id,
                    source.clone(),
                    destination,
                    1,
                    verification_mode,
                    checksum_algorithm,
                );
            }
        }
        TransferGroupMode::Cascade => {
            let mut iter = destinations.into_iter();
            let Some(primary) = iter.next() else {
                return group_id;
            };
            let rest: Vec<PathBuf> = iter.collect();

            let hop1_job_id = Uuid::new_v4().to_string();
            let cancel_flag = app_handle.state::<JobRegistry>().register(hop1_job_id.clone());
            let _ = app_handle.emit(
                GROUP_JOB_ADDED_EVENT,
                GroupJobAddedPayload {
                    group_id: group_id.clone(),
                    job_id: hop1_job_id.clone(),
                    source: source.display().to_string(),
                    destination: primary.display().to_string(),
                    hop: 1,
                },
            );

            let app_handle_thread = app_handle.clone();
            let group_id_thread = group_id.clone();
            let primary_thread = primary.clone();

            std::thread::spawn(move || {
                let outcome = copy_engine::run_copy_job(
                    app_handle_thread.clone(),
                    hop1_job_id,
                    source,
                    primary_thread.clone(),
                    cancel_flag,
                    verification_mode,
                    checksum_algorithm,
                );

                if !should_cascade_continue(&outcome) {
                    return;
                }

                for destination in rest {
                    spawn_job(
                        &app_handle_thread,
                        &group_id_thread,
                        primary_thread.clone(),
                        destination,
                        2,
                        verification_mode,
                        checksum_algorithm,
                    );
                }
            });
        }
    }

    group_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copy_engine::{run_copy_core, FailedFile, ProgressPayload, ProgressSink};
    use std::fs;
    use std::sync::atomic::AtomicBool;

    struct NoopSink;
    impl ProgressSink for NoopSink {
        fn on_scan(&self, _total_files: u64, _total_bytes: u64) {}
        fn on_progress(&self, _payload: ProgressPayload) {}
    }

    /// Exercises the concept a cascade relies on -- source -> primary ->
    /// secondary -- using the Tauri-agnostic copy core directly (two calls),
    /// without going through cascade.rs's threading/event orchestration.
    #[test]
    fn chaining_two_copies_relays_content_correctly() {
        let source_dir = tempfile::tempdir().unwrap();
        let primary_dir = tempfile::tempdir().unwrap();
        let secondary_dir = tempfile::tempdir().unwrap();

        fs::write(source_dir.path().join("clip.mov"), b"camera card footage").unwrap();

        let cancel_flag = AtomicBool::new(false);
        let hop1 = run_copy_core(
            &NoopSink,
            "hop1".to_string(),
            source_dir.path(),
            primary_dir.path(),
            &cancel_flag,
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
        );
        assert!(should_cascade_continue(&hop1));

        let hop2 = run_copy_core(
            &NoopSink,
            "hop2".to_string(),
            primary_dir.path(),
            secondary_dir.path(),
            &cancel_flag,
            VerificationMode::SourceAndDestination,
            ChecksumAlgorithm::Xxh64,
        );

        assert!(hop2.failed_files.is_empty());
        assert_eq!(
            fs::read(secondary_dir.path().join("clip.mov")).unwrap(),
            b"camera card footage"
        );
    }

    fn test_outcome(cancelled: bool, files_copied: u64, failed_files: Vec<FailedFile>) -> CopyOutcome {
        CopyOutcome {
            cancelled,
            files_copied,
            bytes_copied: 0,
            failed_files,
            verified_files: Vec::new(),
            skipped_files: Vec::new(),
            renamed_files: Vec::new(),
        }
    }

    #[test]
    fn cascade_continues_after_a_clean_hop() {
        let outcome = test_outcome(false, 3, Vec::new());
        assert!(should_cascade_continue(&outcome));
    }

    #[test]
    fn cascade_stops_if_hop_one_was_cancelled() {
        let outcome = test_outcome(true, 0, Vec::new());
        assert!(!should_cascade_continue(&outcome));
    }

    #[test]
    fn cascade_stops_if_hop_one_had_failures() {
        let outcome = test_outcome(
            false,
            2,
            vec![FailedFile {
                path: "bad.mov".to_string(),
                message: "checksum mismatch".to_string(),
            }],
        );
        assert!(
            !should_cascade_continue(&outcome),
            "relaying from a primary with a known-bad file would propagate the corruption"
        );
    }
}
