//! NSRunLoop shim: run loop for scheduling timers and input sources.
//!
//! Object layout (singleton per thread — we only support main):
//!
//!   offset  0: isa     (u64 — pointer to NSRunLoop class)
//!   offset  8: running (u64 — 1 while running, 0 otherwise)
//!
//! Total: 16 bytes.

use std::sync::OnceLock;

static MAIN_RUN_LOOP: OnceLock<u64> = OnceLock::new();

fn get_or_create_main_run_loop() -> u64 {
    *MAIN_RUN_LOOP.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        if obj == 0 { return 0; }
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsrunloop_class_addr();
        }
        obj
    })
}

// ── Mode string constants ──────────────────────────────────────────

static NSDEFAULT_RUN_LOOP_MODE: OnceLock<u64> = OnceLock::new();
static NSRUN_LOOP_COMMON_MODES: OnceLock<u64> = OnceLock::new();

/// Address of the NSDefaultRunLoopMode constant string.
pub fn nsdefault_run_loop_mode_addr() -> u64 {
    *NSDEFAULT_RUN_LOOP_MODE.get_or_init(|| {
        let cstr = std::ffi::CString::new("kCFRunLoopDefaultMode").unwrap();
        crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
    })
}

/// Address of the NSRunLoopCommonModes constant string.
pub fn nsrun_loop_common_modes_addr() -> u64 {
    *NSRUN_LOOP_COMMON_MODES.get_or_init(|| {
        let cstr = std::ffi::CString::new("kCFRunLoopCommonModes").unwrap();
        crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
    })
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSRunLoop currentRunLoop]`
pub extern "C" fn shim_nsrunloop_current_run_loop(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_main_run_loop()
}

/// `+[NSRunLoop mainRunLoop]`
pub extern "C" fn shim_nsrunloop_main_run_loop(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_main_run_loop()
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSRunLoop runUntilDate:]` — stub: returns immediately.
pub extern "C" fn shim_nsrunloop_run_until_date(_obj: u64, _cmd: u64, _date: u64) {
    log::debug!("NSRunLoop runUntilDate: (stub, returning immediately)");
}

/// `-[NSRunLoop runMode:beforeDate:]` — stub: returns YES.
pub extern "C" fn shim_nsrunloop_run_mode_before_date(_obj: u64, _cmd: u64, _mode: u64, _date: u64) -> u8 {
    log::debug!("NSRunLoop runMode:beforeDate: (stub)");
    1
}

/// `-[NSRunLoop run]` — stub: returns immediately.
pub extern "C" fn shim_nsrunloop_run(_obj: u64, _cmd: u64) {
    log::debug!("NSRunLoop run (stub, returning immediately)");
}

/// `-[NSRunLoop addTimer:forMode:]` — stub: no-op.
pub extern "C" fn shim_nsrunloop_add_timer_for_mode(_obj: u64, _cmd: u64, _timer: u64, _mode: u64) {
    log::debug!("NSRunLoop addTimer:forMode: (stub)");
}

/// `-[NSRunLoop currentMode]` — returns NSDefaultRunLoopMode.
pub extern "C" fn shim_nsrunloop_current_mode(_obj: u64, _cmd: u64) -> u64 {
    nsdefault_run_loop_mode_addr()
}
