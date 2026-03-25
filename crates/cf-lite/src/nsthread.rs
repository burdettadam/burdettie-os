//! NSThread shim: thread management Foundation class.
//!
//! Object layout (24 bytes):
//!   - [0..8)   ISA pointer
//!   - [8..16)  thread name (NSString *), or 0
//!   - [16..24) is_main_thread flag (u64: 1 = main, 0 = not)
//!
//! NSThread wraps POSIX threads and provides ObjC-level thread
//! management. The main thread is identified via the platform's
//! `ProcessInfo::is_main_thread()`.

use crate::cfstring;
use crate::nsobj;

/// Object size for NSThread instances.
const NSTHREAD_SIZE: usize = 24;

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSThread currentThread]` — returns an NSThread for the calling thread.
///
/// Each call allocates a new NSThread object (no caching). The is_main
/// flag is set based on `ProcessInfo::is_main_thread()`.
pub extern "C" fn shim_nsthread_current_thread(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSTHREAD_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    let is_main = crate::platform().process().is_main_thread();
    unsafe {
        *(obj as *mut u64) = nsobj::nsthread_class_addr();
        // is_main_thread at offset 16
        *((obj as usize + 16) as *mut u64) = if is_main { 1 } else { 0 };
    }
    obj
}

/// `+[NSThread isMainThread]` — returns YES if called from the main thread.
pub extern "C" fn shim_nsthread_is_main_thread_class(_cls: u64, _cmd: u64) -> u8 {
    if crate::platform().process().is_main_thread() {
        1
    } else {
        0
    }
}

/// `+[NSThread isMultiThreaded]` — always returns YES (we support threading).
pub extern "C" fn shim_nsthread_is_multi_threaded(_cls: u64, _cmd: u64) -> u8 {
    1
}

/// `+[NSThread sleepForTimeInterval:]` — sleeps the current thread.
pub extern "C" fn shim_nsthread_sleep_for_time_interval(
    _cls: u64,
    _cmd: u64,
    interval: f64,
) -> u64 {
    crate::platform().clock().sleep_secs(interval);
    0 // void return through u64
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSThread isMainThread]` — returns YES if this is the main thread.
pub extern "C" fn shim_nsthread_is_main_thread(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flag = unsafe { *((obj as usize + 16) as *const u64) };
    if flag != 0 {
        1
    } else {
        0
    }
}

/// `-[NSThread name]` — returns the thread name (NSString or nil).
pub extern "C" fn shim_nsthread_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + 8) as *const u64) }
}

/// `-[NSThread setName:]` — sets the thread name.
pub extern "C" fn shim_nsthread_set_name(obj: u64, _cmd: u64, name: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + 8) as *mut u64) = name;
    }
}

/// `-[NSThread description]` — returns a description string.
pub extern "C" fn shim_nsthread_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let is_main = unsafe { *((obj as usize + 16) as *const u64) };
    let desc = if is_main != 0 {
        c"<NSThread: main>"
    } else {
        c"<NSThread: secondary>"
    };
    cfstring::shim_cf_string_create_with_cstring(0, desc.as_ptr(), 0)
}
