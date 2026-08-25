use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};

use serde::{Deserialize, Serialize};

/// Controls how many transfer jobs are allowed to actively copy at once,
/// mirroring OffShoot's Queuing modes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueueMode {
    /// Every job starts immediately -- no admission control (the default,
    /// matching this app's behavior before Queuing existed).
    #[default]
    Off,
    /// Only one source's jobs (its whole transfer group) run at a time;
    /// multiple destinations of that same source may still run together.
    /// The next source starts automatically once the current one finishes.
    SingleSource,
    /// Within one source, only one destination job runs at a time
    /// (destinations go sequentially); different sources' groups may still
    /// run concurrently with each other.
    SingleDestination,
    /// Only one job runs anywhere in the app at a time, even from the same
    /// source.
    SingleTransfer,
}

struct ActiveJob {
    job_id: String,
    group_id: String,
}

/// Pure admission check, extracted for testability: given what's already
/// running and the current mode, may a job from `group_id` start now?
fn can_admit(mode: QueueMode, active: &[ActiveJob], group_id: &str) -> bool {
    match mode {
        QueueMode::Off => true,
        QueueMode::SingleTransfer => active.is_empty(),
        QueueMode::SingleSource => {
            active.is_empty() || active.iter().all(|j| j.group_id == group_id)
        }
        QueueMode::SingleDestination => !active.iter().any(|j| j.group_id == group_id),
    }
}

#[derive(Default)]
struct QueueState {
    mode: QueueMode,
    active: Vec<ActiveJob>,
}

/// Gates when a registered job's actual copy work may begin, based on the
/// configured `QueueMode` and what else is currently running. Jobs still
/// register with `JobRegistry` and appear in the UI immediately on
/// creation -- this only delays the copy itself, so a job waiting its turn
/// stays visible (as "queued") rather than disappearing until admitted.
#[derive(Default)]
pub struct JobQueue {
    state: Mutex<QueueState>,
    condvar: Condvar,
}

impl JobQueue {
    pub fn set_mode(&self, mode: QueueMode) {
        let mut state = self.state.lock().unwrap();
        state.mode = mode;
        // A looser mode may free up jobs that were waiting under a stricter one.
        self.condvar.notify_all();
    }

    /// Blocks the calling thread until `job_id` (part of `group_id`) is
    /// allowed to start, or until `cancel_flag` is set while still waiting.
    /// Returns `false` if it returned early due to cancellation -- callers
    /// must not proceed to the actual copy in that case, since the job was
    /// never admitted (never added to the active set).
    pub fn wait_for_turn(&self, job_id: &str, group_id: &str, cancel_flag: &AtomicBool) -> bool {
        let mut state = self.state.lock().unwrap();
        loop {
            if cancel_flag.load(Ordering::SeqCst) {
                return false;
            }
            if can_admit(state.mode, &state.active, group_id) {
                state.active.push(ActiveJob {
                    job_id: job_id.to_string(),
                    group_id: group_id.to_string(),
                });
                return true;
            }
            state = self.condvar.wait(state).unwrap();
        }
    }

    /// Wakes every waiter so it re-checks admission -- called both when a
    /// job actually finishes (freeing up a slot) and when a still-queued
    /// job is cancelled (so it notices its own flag promptly instead of
    /// only on the next unrelated wake-up).
    pub fn notify_all(&self) {
        self.condvar.notify_all();
    }

    pub fn job_finished(&self, job_id: &str) {
        let mut state = self.state.lock().unwrap();
        state.active.retain(|j| j.job_id != job_id);
        self.condvar.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn job(id: &str, group: &str) -> ActiveJob {
        ActiveJob {
            job_id: id.to_string(),
            group_id: group.to_string(),
        }
    }

    #[test]
    fn off_mode_always_admits() {
        let active = vec![job("a", "g1"), job("b", "g2")];
        assert!(can_admit(QueueMode::Off, &active, "g3"));
    }

    #[test]
    fn single_transfer_only_admits_when_nothing_is_active() {
        assert!(can_admit(QueueMode::SingleTransfer, &[], "g1"));
        assert!(!can_admit(
            QueueMode::SingleTransfer,
            &[job("a", "g1")],
            "g1"
        ));
        assert!(!can_admit(
            QueueMode::SingleTransfer,
            &[job("a", "g1")],
            "g2"
        ));
    }

    #[test]
    fn single_source_admits_more_destinations_of_the_same_source() {
        let active = vec![job("a", "g1")];
        assert!(
            can_admit(QueueMode::SingleSource, &active, "g1"),
            "another destination of the already-running source should be allowed"
        );
        assert!(
            !can_admit(QueueMode::SingleSource, &active, "g2"),
            "a different source must wait for g1 to finish entirely"
        );
    }

    #[test]
    fn single_destination_serializes_within_a_source_but_not_across_sources() {
        let active = vec![job("a", "g1")];
        assert!(
            !can_admit(QueueMode::SingleDestination, &active, "g1"),
            "a second destination for the same source must wait"
        );
        assert!(
            can_admit(QueueMode::SingleDestination, &active, "g2"),
            "a different source may run concurrently"
        );
    }

    #[test]
    fn cancelling_a_still_queued_job_returns_false_without_blocking() {
        let queue = JobQueue::default();
        queue.set_mode(QueueMode::SingleTransfer);
        let already_cancelled = AtomicBool::new(true);
        assert!(!queue.wait_for_turn("b", "g2", &already_cancelled));
    }

    #[test]
    fn a_blocked_job_is_admitted_once_the_conflicting_job_finishes() {
        let queue = Arc::new(JobQueue::default());
        queue.set_mode(QueueMode::SingleTransfer);
        let cancel_a = AtomicBool::new(false);
        let cancel_b = AtomicBool::new(false);

        assert!(queue.wait_for_turn("a", "group-a", &cancel_a));

        let queue_thread = queue.clone();
        let handle =
            std::thread::spawn(move || queue_thread.wait_for_turn("b", "group-b", &cancel_b));

        std::thread::sleep(std::time::Duration::from_millis(50));
        queue.job_finished("a");

        assert!(
            handle.join().unwrap(),
            "b should be admitted once a finishes freeing the only slot"
        );
    }
}
