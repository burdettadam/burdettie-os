//! Mach kernel primitive shims.
//!
//! Provides controlled implementations of Mach primitives that macOS
//! binaries commonly use at startup: task self port, absolute time,
//! and timebase info.
//!
//! On macOS these delegate to the real host values. On Linux they use
//! portable equivalents (`clock_gettime(CLOCK_MONOTONIC)` for time,
//! a synthetic task-port value, and {1,1} timebase).

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
            "mach_task_self_ (portable)",
        );
        table.register(
            "_mach_absolute_time",
            shim_mach_absolute_time as u64,
            "mach_absolute_time (portable)",
        );
        table.register(
            "_mach_timebase_info",
            shim_mach_timebase_info as u64,
            "mach_timebase_info (portable)",
        );

        log::debug!("MachShims: registered 3 mach shims");
    }
}

// ── mach_task_self ─────────────────────────────────────────────────

/// `mach_port_t mach_task_self(void)` — returns the task self port.
///
/// On macOS: delegates to the real Mach port.
/// On Linux: returns a synthetic non-zero value (0x103 matches XNU default).
extern "C" fn shim_mach_task_self() -> u32 {
    #[cfg(target_os = "macos")]
    {
        #[allow(deprecated)]
        unsafe {
            libc::mach_task_self()
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        0x103 // synthetic task port, matching XNU's typical value
    }
}

// ── mach_absolute_time ─────────────────────────────────────────────

/// `uint64_t mach_absolute_time(void)` — returns monotonic time in
/// abstract ticks (on macOS) or nanoseconds (on Linux).
extern "C" fn shim_mach_absolute_time() -> u64 {
    #[cfg(target_os = "macos")]
    {
        #[allow(deprecated)]
        unsafe {
            libc::mach_absolute_time()
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: clock_gettime with CLOCK_MONOTONIC is always valid.
        unsafe {
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
        }
        ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
    }
}

// ── mach_timebase_info ─────────────────────────────────────────────

/// `kern_return_t mach_timebase_info(mach_timebase_info_data_t *info)`
///
/// On macOS: delegates to the real Mach timebase.
/// On Linux: returns {1, 1} since `clock_gettime(CLOCK_MONOTONIC)` already
/// reports nanoseconds.
extern "C" fn shim_mach_timebase_info(info: *mut MachTimebaseInfoData) -> i32 {
    if info.is_null() {
        return 1; // KERN_INVALID_ARGUMENT
    }
    #[cfg(target_os = "macos")]
    {
        // SAFETY: info is non-null, and libc::mach_timebase_info writes to the same layout
        #[allow(deprecated)]
        unsafe {
            libc::mach_timebase_info(info.cast::<libc::mach_timebase_info>())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        // clock_gettime already returns nanoseconds — ratio is 1:1.
        // SAFETY: info is non-null, we own the pointer.
        unsafe {
            (*info).numer = 1;
            (*info).denom = 1;
        }
        0 // KERN_SUCCESS
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
