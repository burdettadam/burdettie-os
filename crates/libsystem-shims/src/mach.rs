//! Mach kernel primitive shims.
//!
//! Provides controlled implementations of Mach primitives that macOS
//! binaries commonly use at startup: task self port, absolute time,
//! and timebase info. These delegate to the real host values since
//! we're running in the same process.

use crate::table::ShimTable;

/// Install Mach primitive shims.
pub struct MachShims;

/// `mach_timebase_info_data_t` layout matching `<mach/mach_time.h>`.
#[repr(C)]
pub struct MachTimebaseInfoData {
    pub numer: u32,
    pub denom: u32,
}

impl MachShims {
    /// Register Mach primitive shims in the table.
    pub fn install(table: &mut ShimTable) {
        table.register(
            "_mach_task_self_",
            shim_mach_task_self as u64,
            "mach_task_self_ (real)",
        );
        table.register(
            "_mach_absolute_time",
            shim_mach_absolute_time as u64,
            "mach_absolute_time (real)",
        );
        table.register(
            "_mach_timebase_info",
            shim_mach_timebase_info as u64,
            "mach_timebase_info (real)",
        );

        log::debug!("MachShims: registered 3 mach shims");
    }
}

/// `mach_port_t mach_task_self(void)` — returns the task self port.
///
/// Since we're running in the same process, this is the real value.
#[allow(deprecated)] // libc::mach_task_self deprecated in favor of mach2 crate
extern "C" fn shim_mach_task_self() -> u32 {
    // SAFETY: mach_task_self_ is a well-known Mach port
    unsafe { libc::mach_task_self() }
}

/// `uint64_t mach_absolute_time(void)` — returns monotonic nanosecond-ish time.
#[allow(deprecated)]
extern "C" fn shim_mach_absolute_time() -> u64 {
    // SAFETY: mach_absolute_time is a safe read-only syscall
    unsafe { libc::mach_absolute_time() }
}

/// `kern_return_t mach_timebase_info(mach_timebase_info_data_t *info)`
#[allow(deprecated)]
extern "C" fn shim_mach_timebase_info(info: *mut MachTimebaseInfoData) -> i32 {
    if info.is_null() {
        return 1; // KERN_INVALID_ARGUMENT
    }
    // SAFETY: info is non-null, and libc::mach_timebase_info writes to the same layout
    #[allow(deprecated)]
    unsafe {
        libc::mach_timebase_info(info as *mut libc::mach_timebase_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mach_shims_registered() {
        let mut table = ShimTable::new();
        MachShims::install(&mut table);
        assert!(table.resolve("_mach_task_self_").is_some());
        assert!(table.resolve("_mach_absolute_time").is_some());
        assert!(table.resolve("_mach_timebase_info").is_some());
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn mach_task_self_nonzero() {
        let port = shim_mach_task_self();
        assert_ne!(port, 0);
    }

    #[test]
    fn mach_absolute_time_advances() {
        let t1 = shim_mach_absolute_time();
        let t2 = shim_mach_absolute_time();
        assert!(t2 >= t1);
    }

    #[test]
    fn mach_timebase_info_succeeds() {
        let mut info = MachTimebaseInfoData { numer: 0, denom: 0 };
        let ret = shim_mach_timebase_info(&mut info);
        assert_eq!(ret, 0);
        assert_ne!(info.numer, 0);
        assert_ne!(info.denom, 0);
    }
}
