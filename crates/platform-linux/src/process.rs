//! Linux process info via libc.

use platform_core::process::ProcessInfo;
use std::sync::atomic::{AtomicU64, Ordering};

/// Thread ID of the first thread to call `is_main_thread()`.
/// Linux has no `pthread_main_np()`, so we record the main thread's
/// TID at init time and compare against it.
static MAIN_TID: AtomicU64 = AtomicU64::new(0);

pub struct LinuxProcessInfo;

impl LinuxProcessInfo {
    /// Record the current thread as the main thread.  Call this once
    /// from the main thread during startup.
    pub fn mark_main_thread() {
        let tid = current_tid();
        MAIN_TID.store(tid, Ordering::Relaxed);
    }
}

impl ProcessInfo for LinuxProcessInfo {
    fn pid(&self) -> i32 {
        unsafe { libc::getpid() }
    }

    fn is_main_thread(&self) -> bool {
        let main = MAIN_TID.load(Ordering::Relaxed);
        if main == 0 {
            // Not yet marked — assume first caller is main (and record it).
            let tid = current_tid();
            // If another thread races here, the first wins (CAS).
            let _ = MAIN_TID.compare_exchange(0, tid, Ordering::Relaxed, Ordering::Relaxed);
            MAIN_TID.load(Ordering::Relaxed) == tid
        } else {
            main == current_tid()
        }
    }
}

#[cfg(target_os = "linux")]
fn current_tid() -> u64 {
    unsafe { libc::syscall(libc::SYS_gettid) as u64 }
}

#[cfg(not(target_os = "linux"))]
fn current_tid() -> u64 {
    // Fallback for compiling on macOS: use pthread_self as an opaque ID.
    unsafe { libc::pthread_self() as u64 }
}
