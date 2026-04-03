//! macOS process info via libc::getpid.

use platform_core::process::ProcessInfo;

pub struct DarwinProcessInfo;

impl ProcessInfo for DarwinProcessInfo {
    fn pid(&self) -> i32 {
        unsafe { libc::getpid() }
    }

    fn is_main_thread(&self) -> bool {
        unsafe { libc::pthread_main_np() != 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_positive() {
        let p = DarwinProcessInfo;
        assert!(p.pid() > 0);
    }

    #[test]
    fn pid_stable() {
        let p = DarwinProcessInfo;
        assert_eq!(p.pid(), p.pid());
    }

    #[test]
    fn is_main_thread_returns_bool() {
        let p = DarwinProcessInfo;
        // cargo test may run on a worker thread, so just verify it
        // returns without crashing and produces a valid bool.
        let _is_main = p.is_main_thread();
    }
}
