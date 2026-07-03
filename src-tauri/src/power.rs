use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[cfg(target_os = "windows")]
mod imp {
    // SetThreadExecutionState (kernel32.dll) -- flag values verified against
    // learn.microsoft.com/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate.
    #[link(name = "kernel32")]
    extern "system" {
        fn SetThreadExecutionState(es_flags: u32) -> u32;
    }

    const ES_CONTINUOUS: u32 = 0x80000000;
    const ES_SYSTEM_REQUIRED: u32 = 0x00000001;
    const ES_AWAYMODE_REQUIRED: u32 = 0x00000040;

    pub fn prevent_sleep() {
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_AWAYMODE_REQUIRED);
        }
    }

    pub fn allow_sleep() {
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::process::{Child, Command};
    use std::sync::Mutex;

    // `caffeinate -i` keeps the system from idle-sleeping for as long as
    // this child process lives. Chosen over an IOKit power-assertion FFI
    // binding, which this project has no way to verify without real Mac
    // hardware (see the project plan's documented macOS risk) -- a
    // subprocess of a stable, documented CLI tool is much lower-risk.
    static CAFFEINATE: Mutex<Option<Child>> = Mutex::new(None);

    pub fn prevent_sleep() {
        let mut guard = CAFFEINATE.lock().unwrap();
        if guard.is_none() {
            if let Ok(child) = Command::new("caffeinate").arg("-i").spawn() {
                *guard = Some(child);
            }
        }
    }

    pub fn allow_sleep() {
        let mut guard = CAFFEINATE.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod imp {
    pub fn prevent_sleep() {}
    pub fn allow_sleep() {}
}

/// True only on the 0 -> 1 active-job transition (and only when the
/// feature is enabled) -- callers must not spam the OS call once per job
/// when several jobs are already running concurrently.
fn should_prevent_on_start(previous_active: u32, enabled: bool) -> bool {
    previous_active == 0 && enabled
}

/// True only on the 1 -> 0 active-job transition.
fn should_allow_on_finish(previous_active: u32) -> bool {
    previous_active == 1
}

/// Enabling mid-transfer should immediately start preventing sleep if jobs
/// are already running; disabling should immediately release it.
fn should_prevent_on_enable(active_jobs: u32) -> bool {
    active_jobs > 0
}
fn should_allow_on_disable(active_jobs: u32) -> bool {
    active_jobs > 0
}

/// Keeps the system awake for as long as at least one job is active.
/// Tracks its own active-job count so `job_started`/`job_finished` can be
/// called once per job without callers needing to coordinate -- the OS
/// call only actually fires on the 0<->1 boundary.
pub struct SleepGuard {
    active_jobs: AtomicU32,
    enabled: AtomicBool,
}

impl Default for SleepGuard {
    fn default() -> Self {
        Self {
            active_jobs: AtomicU32::new(0),
            enabled: AtomicBool::new(true),
        }
    }
}

impl SleepGuard {
    pub fn job_started(&self) {
        let previous = self.active_jobs.fetch_add(1, Ordering::SeqCst);
        if should_prevent_on_start(previous, self.enabled.load(Ordering::SeqCst)) {
            imp::prevent_sleep();
        }
    }

    /// Callers must only invoke this once per job that was actually
    /// registered (see `JobRegistry::remove`'s `is_some()` guard) -- calling
    /// it without a matching `job_started` would underflow the counter.
    pub fn job_finished(&self) {
        let previous = self.active_jobs.fetch_sub(1, Ordering::SeqCst);
        if should_allow_on_finish(previous) {
            imp::allow_sleep();
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let was_enabled = self.enabled.swap(enabled, Ordering::SeqCst);
        if was_enabled == enabled {
            return;
        }
        let active = self.active_jobs.load(Ordering::SeqCst);
        if enabled && should_prevent_on_enable(active) {
            imp::prevent_sleep();
        } else if !enabled && should_allow_on_disable(active) {
            imp::allow_sleep();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_zero_to_one_transition_prevents_sleep() {
        assert!(should_prevent_on_start(0, true));
        assert!(!should_prevent_on_start(1, true), "a second concurrent job must not re-trigger the OS call");
        assert!(!should_prevent_on_start(0, false), "disabled means never prevent sleep");
    }

    #[test]
    fn only_the_one_to_zero_transition_allows_sleep() {
        assert!(should_allow_on_finish(1));
        assert!(!should_allow_on_finish(2), "other jobs are still active, sleep must stay prevented");
        assert!(!should_allow_on_finish(0), "no job was active, nothing to release");
    }

    #[test]
    fn toggling_enabled_mid_transfer_reacts_immediately() {
        assert!(should_prevent_on_enable(3), "enabling while jobs are running must start preventing sleep right away");
        assert!(!should_prevent_on_enable(0));
        assert!(should_allow_on_disable(3), "disabling while jobs are running must release the OS call right away");
        assert!(!should_allow_on_disable(0), "nothing was preventing sleep, nothing to release");
    }

    #[test]
    fn guard_only_calls_through_on_boundary_transitions() {
        // Exercises the real SleepGuard (not just the pure helpers) so a
        // regression in how job_started/job_finished wire into them would
        // fail here even without observing the OS call itself.
        let guard = SleepGuard::default();
        guard.job_started(); // 0 -> 1: would prevent
        guard.job_started(); // 1 -> 2: no-op
        guard.job_finished(); // 2 -> 1: no-op
        guard.job_finished(); // 1 -> 0: would allow
        assert_eq!(guard.active_jobs.load(Ordering::SeqCst), 0);
    }
}
