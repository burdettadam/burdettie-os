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
