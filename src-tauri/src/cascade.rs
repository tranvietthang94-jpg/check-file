use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use uuid::Uuid;

use crate::checksum::ChecksumAlgorithm;
use crate::copy_engine::{self, CopyOutcome, JobRegistry, VerificationMode};
use crate::organize::OrganizeSettings;

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
    /// The physical volume backing `source` at the moment this job started
    /// (see `disks::volume_signature`), recorded so a later Resume can
    /// confirm the same disk is still the one plugged in before it
    /// re-reads from `source`. `None` when it can't be determined -- the
    /// frontend treats that as "can't verify" rather than blocking Resume.
    pub source_volume_signature: Option<String>,
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
    source_name: String,
    organize: OrganizeSettings,
    move_after_transfer: bool,
    move_same_volume: bool,
    legacy_checksum_algorithm: Option<ChecksumAlgorithm>,
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
            source_volume_signature: crate::disks::volume_signature(&source.display().to_string()),
        },
    );

    let app_handle_thread = app_handle.clone();
    let job_id_thread = job_id.clone();
    let group_id_thread = group_id.to_string();
    std::thread::spawn(move || {
        let admitted = app_handle_thread
            .state::<JobRegistry>()
            .wait_for_turn(&job_id_thread, &group_id_thread, &cancel_flag);
        if !admitted {
            let _ = app_handle_thread.emit(
                copy_engine::CANCELLED_EVENT,
                copy_engine::CancelledPayload {
                    job_id: job_id_thread.clone(),
                },
            );
            app_handle_thread.state::<JobRegistry>().remove(&job_id_thread);
            return;
        }

        copy_engine::run_copy_job(
            app_handle_thread,
            job_id_thread,
            source,
            destination,
            cancel_flag,
            verification_mode,
            checksum_algorithm,
            source_name,
            organize,
            move_after_transfer,
            move_same_volume,
            legacy_checksum_algorithm,
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
    source_name: String,
    organize: OrganizeSettings,
    move_after_transfer: bool,
    move_same_volume: bool,
    legacy_checksum_algorithm: Option<ChecksumAlgorithm>,
) -> String {
    let group_id = Uuid::new_v4().to_string();

    match mode {
        TransferGroupMode::Parallel => {
            // Every destination reads the same source independently -- with
            // more than one, there's no single point where it's safe to
            // delete the source without racing the other destination's read.
            // Move (and the same-volume fast-move rename, which deletes the
            // source just as surely) only ever applies to the unambiguous
            // single-destination case.
            let single_destination = destinations.len() == 1;
            let effective_move = move_after_transfer && single_destination;
            let effective_same_volume_move = move_same_volume && single_destination;
            for destination in destinations {
                spawn_job(
                    &app_handle,
                    &group_id,
                    source.clone(),
                    destination,
                    1,
                    verification_mode,
                    checksum_algorithm,
                    source_name.clone(),
                    organize.clone(),
                    effective_move,
                    effective_same_volume_move,
                    legacy_checksum_algorithm,
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
                    source_volume_signature: crate::disks::volume_signature(
                        &source.display().to_string(),
                    ),
                },
            );

            let app_handle_thread = app_handle.clone();
            let group_id_thread = group_id.clone();
            let primary_thread = primary.clone();
            let source_name_thread = source_name.clone();
            let organize_thread = organize.clone();
            let hop1_job_id_thread = hop1_job_id.clone();

            std::thread::spawn(move || {
                let admitted = app_handle_thread.state::<JobRegistry>().wait_for_turn(
                    &hop1_job_id_thread,
                    &group_id_thread,
                    &cancel_flag,
                );
                if !admitted {
                    let _ = app_handle_thread.emit(
                        copy_engine::CANCELLED_EVENT,
                        copy_engine::CancelledPayload {
                            job_id: hop1_job_id_thread.clone(),
                        },
                    );
                    app_handle_thread.state::<JobRegistry>().remove(&hop1_job_id_thread);
                    return;
                }

                // Hop 1 has exactly one destination (primary) by construction,
                // so it's always the unambiguous single-destination case Move
                // is safe for -- regardless of how many `rest` destinations
                // this cascade relays to afterward.
                let outcome = copy_engine::run_copy_job(
                    app_handle_thread.clone(),
                    hop1_job_id,
                    source,
                    primary_thread.clone(),
                    cancel_flag,
                    verification_mode,
                    checksum_algorithm,
                    source_name_thread.clone(),
                    organize_thread.clone(),
                    move_after_transfer,
                    move_same_volume,
                    legacy_checksum_algorithm,
                );

                if !should_cascade_continue(&outcome) {
                    return;
                }

                for destination in rest {
                    // Hop 2's "source" is the primary destination we just
                    // wrote and verified -- it must never be deleted, so this
                    // is never Move- or same-volume-move-eligible regardless
                    // of the caller's settings.
                    spawn_job(
                        &app_handle_thread,
                        &group_id_thread,
                        primary_thread.clone(),
                        destination,
                        2,
                        verification_mode,
                        checksum_algorithm,
                        source_name_thread.clone(),
                        organize_thread.clone(),
                        false,
                        false,
                        legacy_checksum_algorithm,
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
            "Source",
            &OrganizeSettings::default(),
            false,
            false,
            None, // legacy_checksum_algorithm
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
            "Source",
            &OrganizeSettings::default(),
            false,
            false,
            None, // legacy_checksum_algorithm
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
            mhl_entries: Vec::new(),
            deleted_source_files: Vec::new(),
            move_delete_failed: Vec::new(),
            broken_media_files: Vec::new(),
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
