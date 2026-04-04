//! Mach kernel primitive shims.
//!
//! Provides controlled implementations of Mach primitives that macOS
//! binaries commonly use at startup: task self port, absolute time,
//! and timebase info.
//!
//! On macOS these delegate to the real host values. On Linux they use
//! portable equivalents (`clock_gettime(CLOCK_MONOTONIC)` for time,
//! a synthetic task-port value, and {1,1} timebase).
//!
//! NOTE: `_mach_task_self_` is a DATA symbol on macOS (a global
//! `mach_port_t` variable, not a function). On macOS we let it
//! fall through to the StubResolver so dlsym returns the correct
//! data address. On non-macOS we register a static holding a
//! synthetic port value.

use crate::table::ShimTable;

/// Install Mach primitive shims.
pub struct MachShims;

/// `mach_timebase_info_data_t` layout matching `<mach/mach_time.h>`.
#[repr(C)]
pub struct MachTimebaseInfoData {
    pub numer: u32,
    pub denom: u32,
}

/// Synthetic Mach task-self port for non-macOS platforms.
/// This is a DATA symbol — the address of this static is what gets
/// bound into guest GOT slots, so loads from it yield the port value.
#[cfg(not(target_os = "macos"))]
static MACH_TASK_SELF_PORT: u32 = 0x103;

impl MachShims {
    /// Register Mach primitive shims in the table.
    pub fn install(table: &mut ShimTable) {
        // On macOS, _mach_task_self_ is a DATA symbol (global mach_port_t).
        // We do NOT register it as a shim — the StubResolver will dlsym it
        // and return the host's data address, which is correct.
        // On non-macOS, register our synthetic data symbol.
        #[cfg(not(target_os = "macos"))]
        table.register(
            "_mach_task_self_",
            &MACH_TASK_SELF_PORT as *const u32 as u64,
            "mach_task_self_ (synthetic data)",
        );

        table.register(
            "_mach_absolute_time",
            shim_mach_absolute_time as *const () as u64,
            "mach_absolute_time (portable)",
        );
        table.register(
            "_mach_timebase_info",
            shim_mach_timebase_info as *const () as u64,
            "mach_timebase_info (portable)",
        );

        table.register(
            "_clock",
            shim_clock as *const () as u64,
            "clock (delegate to host)",
        );
        table.register(
            "_sys_icache_invalidate",
            shim_sys_icache_invalidate as *const () as u64,
            "sys_icache_invalidate (no-op in lab)",
        );
        table.register(
            "__dyld_register_func_for_add_image",
            shim_dyld_register_func_for_add_image as *const () as u64,
            "_dyld_register_func_for_add_image (no-op, prevents host callback confusion)",
        );
        table.register(
            "__dyld_image_count",
            shim_dyld_image_count as *const () as u64,
            "_dyld_image_count (returns 0 for guest)",
        );
        table.register(
            "__dyld_get_image_header",
            shim_dyld_get_image_header as *const () as u64,
            "_dyld_get_image_header (returns null for guest)",
        );
        table.register(
            "__dyld_get_image_name",
            shim_dyld_get_image_name as *const () as u64,
            "_dyld_get_image_name (returns empty for guest)",
        );
        table.register(
            "__dyld_get_image_vmaddr_slide",
            shim_dyld_get_image_vmaddr_slide as *const () as u64,
            "_dyld_get_image_vmaddr_slide (returns 0 for guest)",
        );

        #[cfg(not(target_os = "macos"))]
        log::debug!("MachShims: registered 10 mach shims (with synthetic task port)");
        #[cfg(target_os = "macos")]
        log::debug!("MachShims: registered 9 mach shims (task port via host dlsym)");
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

// ── clock ──────────────────────────────────────────────────────────

/// `clock_t clock(void)` — delegate to host libc.
extern "C" fn shim_clock() -> i64 {
    extern "C" {
        fn clock() -> i64;
    }
    unsafe { clock() }
}

// ── sys_icache_invalidate ──────────────────────────────────────────

/// `void sys_icache_invalidate(void *start, size_t len)`
///
/// On real hardware this flushes the instruction cache. In the lab
/// environment this is a no-op since we don't generate code at runtime.
extern "C" fn shim_sys_icache_invalidate(_start: *mut u8, _len: usize) {
    log::trace!("sys_icache_invalidate: no-op");
}

// ── dyld introspection shims ───────────────────────────────────────

/// `void _dyld_register_func_for_add_image(void (*func)(const struct mach_header*, intptr_t))`
///
/// Intercept to prevent host dyld from calling guest callbacks with host
/// image headers. The guest ObjC runtime and LLVM `PrintStackTraceOnError`
/// often register callbacks here during constructors.
extern "C" fn shim_dyld_register_func_for_add_image(
    _func: Option<extern "C" fn(*const u8, isize)>,
) {
    log::trace!("_dyld_register_func_for_add_image: intercepted (no-op)");
}

/// `uint32_t _dyld_image_count(void)` — returns 0 for guest (we don't
/// expose our manually-mapped images through the dyld introspection API).
extern "C" fn shim_dyld_image_count() -> u32 {
    log::trace!("_dyld_image_count: returning 0 (guest)");
    0
}

/// `const struct mach_header* _dyld_get_image_header(uint32_t index)` — returns null.
extern "C" fn shim_dyld_get_image_header(_index: u32) -> *const u8 {
    log::trace!("_dyld_get_image_header({}): returning null (guest)", _index);
    std::ptr::null()
}

/// `const char* _dyld_get_image_name(uint32_t index)` — returns empty string.
static EMPTY_CSTR: &[u8] = b"\0";
extern "C" fn shim_dyld_get_image_name(_index: u32) -> *const u8 {
    log::trace!("_dyld_get_image_name({}): returning empty (guest)", _index);
    EMPTY_CSTR.as_ptr()
}

/// `intptr_t _dyld_get_image_vmaddr_slide(uint32_t index)` — returns 0.
extern "C" fn shim_dyld_get_image_vmaddr_slide(_index: u32) -> isize {
    log::trace!("_dyld_get_image_vmaddr_slide({}): returning 0 (guest)", _index);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mach_shims_registered() {
        let mut table = ShimTable::new();
        MachShims::install(&mut table);
        assert!(table.resolve("_mach_absolute_time").is_some());
        assert!(table.resolve("_mach_timebase_info").is_some());
        assert!(table.resolve("_clock").is_some());
        assert!(table.resolve("_sys_icache_invalidate").is_some());
        assert!(table.resolve("__dyld_register_func_for_add_image").is_some());
        assert!(table.resolve("__dyld_image_count").is_some());
        assert!(table.resolve("__dyld_get_image_header").is_some());
        assert!(table.resolve("__dyld_get_image_name").is_some());
        assert!(table.resolve("__dyld_get_image_vmaddr_slide").is_some());
        #[cfg(target_os = "macos")]
        {
            assert!(table.resolve("_mach_task_self_").is_none());
            assert_eq!(table.len(), 9);
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(table.resolve("_mach_task_self_").is_some());
            assert_eq!(table.len(), 10);
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn mach_task_self_synthetic_port() {
        assert_eq!(MACH_TASK_SELF_PORT, 0x103);
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
