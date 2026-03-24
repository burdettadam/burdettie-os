//! macOS process info via libc::getpid.

use platform_core::process::ProcessInfo;

pub struct DarwinProcessInfo;

impl ProcessInfo for DarwinProcessInfo {
    fn pid(&self) -> i32 {
        unsafe { libc::getpid() }
    }
}
