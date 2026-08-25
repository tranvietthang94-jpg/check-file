use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use uuid::Uuid;

use crate::copy_engine::{self, CopyOutcome, JobRegistry};
use crate::source_selection::SourceSelection;

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
    pub selected_paths: Option<Vec<String>>,
}

/// A relay hop should only run if the file(s) it depends on are actually
/// intact on disk. Pure decision, no I/O, so it's trivially unit-testable
/// without touching Tauri.
fn should_cascade_continue(hop1_outcome: &CopyOutcome) -> bool {
    !hop1_outcome.cancelled && hop1_outcome.failed_files.is_empty()
}

fn selection_for_relay(
    selection: Option<&SourceSelection>,
    primary: &std::path::Path,
    hop1_outcome: &CopyOutcome,
) -> std::io::Result<Option<SourceSelection>> {
    let Some(selection) = selection else {
        return Ok(None);
    };
    let output_paths: Vec<PathBuf> = hop1_outcome
        .mhl_entries
        .iter()
        .map(|entry| primary.join(&entry.relative_path))
        .chain(
            hop1_outcome
                .skipped_files
                .iter()
                .map(|entry| primary.join(&entry.path)),
        )
        .collect();
    if output_paths.is_empty() {
        return selection.rebase(primary.to_path_buf()).map(Some);
    }
    SourceSelection::new(primary.to_path_buf(), output_paths).map(Some)
}

struct SpawnJobRequest {
    source: PathBuf,
    source_selection: Option<SourceSelection>,
    destination: PathBuf,
    hop: u8,
    options: copy_engine::TransferOptions,
}

fn spawn_job<R: Runtime>(
    app_handle: &AppHandle<R>,
    group_id: &str,
    request: SpawnJobRequest,
) -> String {
    let SpawnJobRequest {
        source,
        source_selection,
        destination,
        hop,
        options,
    } = request;
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
            selected_paths: source_selection.as_ref().map(|selection| {
                selection
                    .selected_paths()
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect()
            }),
        },
    );

    let app_handle_thread = app_handle.clone();
    let job_id_thread = job_id.clone();
    let group_id_thread = group_id.to_string();
    std::thread::spawn(move || {
        let admitted = app_handle_thread.state::<JobRegistry>().wait_for_turn(
            &job_id_thread,
            &group_id_thread,
            &cancel_flag,
        );
        if !admitted {
            let _ = app_handle_thread.emit(
                copy_engine::CANCELLED_EVENT,
                copy_engine::CancelledPayload {
                    job_id: job_id_thread.clone(),
                },
            );
            app_handle_thread
                .state::<JobRegistry>()
                .remove(&job_id_thread);
            return;
        }

        copy_engine::run_copy_job(
            app_handle_thread,
            copy_engine::CopyJobRequest {
                job_id: job_id_thread,
                source,
                source_selection,
                destination,
                cancel_flag,
                options,
            },
        );
    });

    job_id
}

/// Kicks off a source -> many-destinations transfer. Returns immediately with
/// a group id; individual jobs (including cascade hop 2, which doesn't exist
/// yet when this returns) announce themselves via `GROUP_JOB_ADDED_EVENT` so
/// the frontend can start tracking each one as it's created.
pub struct TransferGroupRequest {
    pub source: PathBuf,
    pub source_selection: Option<SourceSelection>,
    pub destinations: Vec<PathBuf>,
    pub mode: TransferGroupMode,
    pub options: copy_engine::TransferOptions,
}

pub fn start_transfer_group<R: Runtime>(
    app_handle: AppHandle<R>,
    request: TransferGroupRequest,
) -> String {
    let TransferGroupRequest {
        source,
        source_selection,
        destinations,
        mode,
        options,
    } = request;
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
            let effective_move = options.move_after_transfer && single_destination;
            let effective_same_volume_move = options.move_same_volume && single_destination;
            for destination in destinations {
                let mut job_options = options.clone();
                job_options.move_after_transfer = effective_move;
                job_options.move_same_volume = effective_same_volume_move;
                spawn_job(
                    &app_handle,
                    &group_id,
                    SpawnJobRequest {
                        source: source.clone(),
                        source_selection: source_selection.clone(),
                        destination,
                        hop: 1,
                        options: job_options,
                    },
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
            let cancel_flag = app_handle
                .state::<JobRegistry>()
                .register(hop1_job_id.clone());
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
                    selected_paths: source_selection.as_ref().map(|selection| {
                        selection
                            .selected_paths()
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect()
                    }),
                },
            );

            let app_handle_thread = app_handle.clone();
            let group_id_thread = group_id.clone();
            let primary_thread = primary.clone();
            let options_thread = options;
            let hop1_job_id_thread = hop1_job_id.clone();
            let source_selection_thread = source_selection.clone();

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
                    app_handle_thread
                        .state::<JobRegistry>()
                        .remove(&hop1_job_id_thread);
                    return;
                }

                // Hop 1 has exactly one destination (primary) by construction,
                // so it's always the unambiguous single-destination case Move
                // is safe for -- regardless of how many `rest` destinations
                // this cascade relays to afterward.
                let outcome = copy_engine::run_copy_job(
                    app_handle_thread.clone(),
                    copy_engine::CopyJobRequest {
                        job_id: hop1_job_id,
                        source,
                        source_selection: source_selection_thread.clone(),
                        destination: primary_thread.clone(),
                        cancel_flag,
                        options: options_thread.clone(),
                    },
                );

                if !should_cascade_continue(&outcome) {
                    return;
                }

                let relay_selection = match selection_for_relay(
                    source_selection_thread.as_ref(),
                    &primary_thread,
                    &outcome,
                ) {
                    Ok(selection) => selection,
                    Err(error) => {
                        eprintln!("Cannot preserve selected paths for cascade relay: {error}");
                        return;
                    }
                };

                for destination in rest {
                    // Hop 2's "source" is the primary destination we just
                    // wrote and verified -- it must never be deleted, so this
                    // is never Move- or same-volume-move-eligible regardless
                    // of the caller's settings. The paper-trail settings
                    // aren't destructive, so hop 2 still honors them.
                    let mut relay_options = options_thread.clone();
                    relay_options.move_after_transfer = false;
                    relay_options.move_same_volume = false;
                    spawn_job(
                        &app_handle_thread,
                        &group_id_thread,
                        SpawnJobRequest {
                            source: primary_thread.clone(),
                            source_selection: relay_selection.clone(),
                            destination,
                            hop: 2,
                            options: relay_options,
                        },
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
    use crate::checksum::ChecksumAlgorithm;
    use crate::copy_engine::{run_copy_core, FailedFile, ProgressPayload, ProgressSink};
    use crate::copy_engine::VerificationMode;
    use crate::organize::OrganizeSettings;
    use crate::source_selection::SourceSelection;
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
            crate::copy_engine::CopyOptions::new(
                VerificationMode::SourceAndDestination,
                ChecksumAlgorithm::Xxh64,
                "Source",
                &OrganizeSettings::default(),
                false,
                false,
                None,
            ),
        );
        assert!(should_cascade_continue(&hop1));

        let hop2 = run_copy_core(
            &NoopSink,
            "hop2".to_string(),
            primary_dir.path(),
            secondary_dir.path(),
            &cancel_flag,
            crate::copy_engine::CopyOptions::new(
                VerificationMode::SourceAndDestination,
                ChecksumAlgorithm::Xxh64,
                "Source",
                &OrganizeSettings::default(),
                false,
                false,
                None,
            ),
        );

        assert!(hop2.failed_files.is_empty());
        assert_eq!(
            fs::read(secondary_dir.path().join("clip.mov")).unwrap(),
            b"camera card footage"
        );
    }

    #[test]
    fn selected_cascade_rebases_the_same_layout_onto_the_primary() {
        let source = tempfile::tempdir().unwrap();
        let primary = tempfile::tempdir().unwrap();
        let selected_folder = source.path().join("CARD_A");
        fs::create_dir_all(&selected_folder).unwrap();
        fs::write(selected_folder.join("clip.mov"), b"footage").unwrap();
        let selection =
            SourceSelection::new(source.path().to_path_buf(), vec![selected_folder]).unwrap();

        let outcome = crate::copy_engine::run_copy_core_with_selection(
            &NoopSink,
            "relay-layout".to_string(),
            &selection,
            primary.path(),
            &AtomicBool::new(false),
            crate::copy_engine::CopyOptions::new(
                VerificationMode::SourceAndDestination,
                ChecksumAlgorithm::Xxh64,
                "Source",
                &OrganizeSettings::default(),
                false,
                false,
                None,
            ),
        );
        let relay = selection_for_relay(Some(&selection), primary.path(), &outcome).unwrap();

        let relay = relay.expect("selected cascades must stay selected on hop 2");
        assert_eq!(relay.common_root(), primary.path());
        assert_eq!(
            relay.selected_paths(),
            &[primary.path().join("CARD_A").join("clip.mov")]
        );
    }

    #[test]
    fn selected_cascade_relays_actual_flattened_output_paths() {
        let source = tempfile::tempdir().unwrap();
        let primary = tempfile::tempdir().unwrap();
        let selected_folder = source.path().join("CARD_A");
        fs::create_dir_all(selected_folder.join("DCIM")).unwrap();
        fs::write(selected_folder.join("DCIM").join("clip.mov"), b"footage").unwrap();
        let selection =
            SourceSelection::new(source.path().to_path_buf(), vec![selected_folder]).unwrap();
        let organize = OrganizeSettings {
            flatten: true,
            ..OrganizeSettings::default()
        };
        let outcome = crate::copy_engine::run_copy_core_with_selection(
            &NoopSink,
            "flatten-hop1".to_string(),
            &selection,
            primary.path(),
            &AtomicBool::new(false),
            crate::copy_engine::CopyOptions::new(
                VerificationMode::SourceAndDestination,
                ChecksumAlgorithm::Xxh64,
                "Source",
                &organize,
                false,
                false,
                None,
            ),
        );

        let relay = selection_for_relay(Some(&selection), primary.path(), &outcome).unwrap();

        let relay = relay.expect("selected cascade relay");
        assert_eq!(relay.selected_paths(), &[primary.path().join("clip.mov")]);
    }

    fn test_outcome(
        cancelled: bool,
        files_copied: u64,
        failed_files: Vec<FailedFile>,
    ) -> CopyOutcome {
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
            missing_files: Vec::new(),
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
