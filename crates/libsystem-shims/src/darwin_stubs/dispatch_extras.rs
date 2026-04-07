//! Extra GCD dispatch stubs.
//!
//! Supplements the core dispatch shims in `crate::dispatch` with
//! additional primitives used by Xcode tools.

use crate::table::ShimTable;
use super::dispatch_queue;

pub struct DispatchExtrasStubs;

impl DispatchExtrasStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_dispatch_activate", shim_dispatch_void1 as *const () as u64, "dispatch_activate (stub → void)"),
            ("_dispatch_assert_queue", shim_dispatch_void1 as *const () as u64, "dispatch_assert_queue (stub → void)"),
            ("_dispatch_assert_queue_barrier", shim_dispatch_void1 as *const () as u64, "dispatch_assert_queue_barrier (stub → void)"),
            ("_dispatch_assert_queue_not", shim_dispatch_void1 as *const () as u64, "dispatch_assert_queue_not (stub → void)"),
            ("_dispatch_block_create", shim_dispatch_block_create as *const () as u64, "dispatch_block_create (stub → passthrough block)"),
            ("_dispatch_block_create_with_qos_class", shim_dispatch_block_create_qos as *const () as u64, "dispatch_block_create_with_qos_class (stub → passthrough)"),
            ("_dispatch_block_cancel", shim_dispatch_void1 as *const () as u64, "dispatch_block_cancel (stub → void)"),
            ("_dispatch_block_testcancel", shim_dispatch_zero1 as *const () as u64, "dispatch_block_testcancel (stub → 0)"),
            ("_dispatch_block_wait", shim_dispatch_zero2 as *const () as u64, "dispatch_block_wait (stub → 0)"),
            ("_dispatch_block_notify", shim_dispatch_void3 as *const () as u64, "dispatch_block_notify (stub → void)"),
            ("_dispatch_block_perform", shim_dispatch_void2 as *const () as u64, "dispatch_block_perform (stub → void)"),
            ("_dispatch_io_create", shim_dispatch_null3 as *const () as u64, "dispatch_io_create (stub → NULL)"),
            ("_dispatch_io_read", shim_dispatch_void4 as *const () as u64, "dispatch_io_read (stub → void)"),
            ("_dispatch_io_write", shim_dispatch_void4 as *const () as u64, "dispatch_io_write (stub → void)"),
            ("_dispatch_io_close", shim_dispatch_void2 as *const () as u64, "dispatch_io_close (stub → void)"),
            ("_dispatch_io_set_high_water", shim_dispatch_void2 as *const () as u64, "dispatch_io_set_high_water (stub → void)"),
            ("_dispatch_io_set_low_water", shim_dispatch_void2 as *const () as u64, "dispatch_io_set_low_water (stub → void)"),
            ("_dispatch_queue_attr_make_with_autorelease_frequency", shim_dispatch_first_arg as *const () as u64, "dispatch_queue_attr_make_with_autorelease_frequency (stub → attr passthrough)"),
            ("_dispatch_queue_get_specific", shim_dispatch_null1 as *const () as u64, "dispatch_queue_get_specific (stub → NULL)"),
            ("_dispatch_source_get_data", shim_dispatch_zero1 as *const () as u64, "dispatch_source_get_data (stub → 0)"),
            ("_dispatch_source_get_handle", shim_dispatch_zero1 as *const () as u64, "dispatch_source_get_handle (stub → 0)"),
            ("_dispatch_source_merge_data", shim_dispatch_void2 as *const () as u64, "dispatch_source_merge_data (stub → void)"),
            // dispatch_apply — serial loop implementation
            ("_dispatch_apply", shim_dispatch_apply as *const () as u64, "dispatch_apply (serial loop)"),
            ("_dispatch_apply_f", shim_dispatch_apply_f as *const () as u64, "dispatch_apply_f (serial loop)"),
            // dispatch_queue_create — returns a sentinel queue pointer
            ("_dispatch_queue_create", shim_dispatch_queue_create as *const () as u64, "dispatch_queue_create (→ sentinel)"),
            ("__dispatch_queue_attr_concurrent", &DISPATCH_QUEUE_ATTR_CONCURRENT as *const _ as u64, "_dispatch_queue_attr_concurrent (data sentinel)"),
            // ── dispatch_after / dispatch_time ──────────────────
            ("_dispatch_after", shim_dispatch_after as *const () as u64, "dispatch_after (timer + thread)"),
            ("_dispatch_after_f", shim_dispatch_after_f as *const () as u64, "dispatch_after_f (timer + thread)"),
            ("_dispatch_time", shim_dispatch_time as *const () as u64, "dispatch_time (nanosecond math)"),
            ("_dispatch_walltime", shim_dispatch_walltime as *const () as u64, "dispatch_walltime (wall clock)"),
            // ── dispatch_async_f / dispatch_sync_f ──────────────
            ("_dispatch_async_f", shim_dispatch_async_f as *const () as u64, "dispatch_async_f (threaded)"),
            ("_dispatch_sync_f", shim_dispatch_sync_f as *const () as u64, "dispatch_sync_f (inline)"),
            // ── dispatch_barrier ─────────────────────────────────
            ("_dispatch_barrier_sync", shim_dispatch_barrier_sync as *const () as u64, "dispatch_barrier_sync (inline)"),
            ("_dispatch_barrier_async", shim_dispatch_barrier_async as *const () as u64, "dispatch_barrier_async (threaded)"),
            ("_dispatch_barrier_async_f", shim_dispatch_barrier_async_f as *const () as u64, "dispatch_barrier_async_f (threaded)"),
            // ── dispatch_queue_get_label ─────────────────────────
            ("_dispatch_queue_get_label", shim_dispatch_queue_get_label as *const () as u64, "dispatch_queue_get_label (→ empty)"),
            // ── dispatch_set_target_queue ────────────────────────
            ("_dispatch_set_target_queue", shim_dispatch_void2 as *const () as u64, "dispatch_set_target_queue (stub → void)"),
            // ── dispatch_queue_set_specific ──────────────────────
            ("_dispatch_queue_set_specific", shim_dispatch_void4 as *const () as u64, "dispatch_queue_set_specific (stub → void)"),
            // ── dispatch_get_specific ────────────────────────────
            ("_dispatch_get_specific", shim_dispatch_null1 as *const () as u64, "dispatch_get_specific (stub → NULL)"),
        );
        log::debug!("DispatchExtrasStubs: registered 39 dispatch stubs");
    }
}

pub extern "C" fn shim_dispatch_void1(_a: u64) {}
pub extern "C" fn shim_dispatch_void2(_a: u64, _b: u64) {}
pub extern "C" fn shim_dispatch_void3(_a: u64, _b: u64, _c: u64) {}
pub extern "C" fn shim_dispatch_void4(_a: u64, _b: u64, _c: u64, _d: u64) {}
pub extern "C" fn shim_dispatch_zero1(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_dispatch_zero2(_a: u64, _b: u64) -> u64 { 0 }
pub extern "C" fn shim_dispatch_null1(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_dispatch_null3(_a: u64, _b: u64, _c: u64) -> u64 { 0 }
pub extern "C" fn shim_dispatch_block_create(_flags: u64, _block: u64) -> u64 { _block }
pub extern "C" fn shim_dispatch_block_create_qos(_flags: u64, _qos: u64, _rel: i32, _block: u64) -> u64 { _block }
pub extern "C" fn shim_dispatch_first_arg(a: u64, _b: u64) -> u64 { a }

/// Sentinel for `_dispatch_queue_attr_concurrent` data symbol.
static DISPATCH_QUEUE_ATTR_CONCURRENT: [u8; 8] = [0; 8];

/// `void dispatch_apply(size_t iterations, dispatch_queue_t queue,
///                       void (^block)(size_t))`
///
/// Executes the block `iterations` times with indices 0..iterations-1.
/// On macOS this runs concurrently; we run serially (sufficient for ld).
pub extern "C" fn shim_dispatch_apply(iterations: usize, _queue: u64, block: *const u8) {
    if block.is_null() || iterations == 0 {
        return;
    }
    // Block layout: isa(8) + flags(4) + reserved(4) + invoke(8)
    let invoke: extern "C" fn(*const u8, usize) = unsafe {
        *(block.add(16) as *const extern "C" fn(*const u8, usize))
    };
    for i in 0..iterations {
        invoke(block, i);
    }
}

/// `void dispatch_apply_f(size_t iterations, dispatch_queue_t queue,
///                         void *context, void (*work)(void *, size_t))`
pub extern "C" fn shim_dispatch_apply_f(
    iterations: usize,
    _queue: u64,
    context: u64,
    work: Option<extern "C" fn(u64, usize)>,
) {
    if let Some(f) = work {
        for i in 0..iterations {
            f(context, i);
        }
    }
}

/// `dispatch_queue_t dispatch_queue_create(const char *label, dispatch_queue_attr_t attr)`
///
/// Allocates a real queue with a dedicated worker (serial) or pooled
/// execution (concurrent).  Returns a unique handle per call.
static DISPATCH_CREATED_Q: [u8; 8] = [0; 8];

pub extern "C" fn shim_dispatch_queue_create(label: u64, attr: u64) -> u64 {
    let label_str = if label != 0 {
        let cstr = unsafe { std::ffi::CStr::from_ptr(label as *const std::ffi::c_char) };
        cstr.to_str().unwrap_or("")
    } else {
        ""
    };
    let concurrent = attr == DISPATCH_QUEUE_ATTR_CONCURRENT.as_ptr() as u64;
    dispatch_queue::queue_create(label_str, concurrent)
}

// ═══════════════════════════════════════════════════════════════════
//  dispatch_time / dispatch_walltime
// ═══════════════════════════════════════════════════════════════════

/// Mach absolute time constant: start counting from "now".
const DISPATCH_TIME_NOW: u64 = 0;
/// Mach absolute time constant: infinite future.
const DISPATCH_TIME_FOREVER: u64 = !0;

/// `dispatch_time_t dispatch_time(dispatch_time_t when, int64_t delta)`
///
/// Returns a time value relative to `when` offset by `delta` nanoseconds.
/// We map to a simple nanosecond-since-epoch representation.
pub extern "C" fn shim_dispatch_time(when: u64, delta: i64) -> u64 {
    if when == DISPATCH_TIME_FOREVER {
        return DISPATCH_TIME_FOREVER;
    }

    let base = if when == DISPATCH_TIME_NOW {
        // Current mach_absolute_time → approximate with clock_gettime
        nanos_now_monotonic()
    } else {
        when
    };

    if delta >= 0 {
        base.saturating_add(delta as u64)
    } else {
        base.saturating_sub((-delta) as u64)
    }
}

/// `dispatch_time_t dispatch_walltime(const struct timespec *when, int64_t delta)`
///
/// Returns a wall-clock time value.  If `when` is NULL, uses the current time.
pub extern "C" fn shim_dispatch_walltime(when: *const libc::timespec, delta: i64) -> u64 {
    let base = if when.is_null() {
        nanos_now_monotonic()
    } else {
        let ts = unsafe { &*when };
        (ts.tv_sec as u64).saturating_mul(1_000_000_000).saturating_add(ts.tv_nsec as u64)
    };

    if delta >= 0 {
        base.saturating_add(delta as u64)
    } else {
        base.saturating_sub((-delta) as u64)
    }
}

fn nanos_now_monotonic() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    #[cfg(target_os = "linux")]
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts); }
    #[cfg(target_os = "macos")]
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts); }
    (ts.tv_sec as u64).saturating_mul(1_000_000_000).saturating_add(ts.tv_nsec as u64)
}

// ═══════════════════════════════════════════════════════════════════
//  dispatch_after / dispatch_after_f
// ═══════════════════════════════════════════════════════════════════

/// `void dispatch_after(dispatch_time_t when, dispatch_queue_t queue,
///                       dispatch_block_t block)`
///
/// Schedules a block to execute after the specified time on the given queue.
pub extern "C" fn shim_dispatch_after(when: u64, queue: u64, block: *const u8) {
    if block.is_null() {
        return;
    }

    let delay = delay_from_dispatch_time(when);
    let invoke_ptr = unsafe { *(block.add(16) as *const usize) };
    let block_addr = block as usize;

    // Timer thread sleeps, then submits to the queue
    std::thread::spawn(move || {
        if delay > std::time::Duration::ZERO {
            std::thread::sleep(delay);
        }
        dispatch_queue::queue_async(queue, Box::new(move || {
            unsafe {
                let invoke: extern "C" fn(*const u8) = std::mem::transmute(invoke_ptr);
                invoke(block_addr as *const u8);
            }
        }));
    });
}

/// `void dispatch_after_f(dispatch_time_t when, dispatch_queue_t queue,
///                          void *context, void (*work)(void *))`
pub extern "C" fn shim_dispatch_after_f(
    when: u64,
    queue: u64,
    context: u64,
    work: Option<extern "C" fn(u64)>,
) {
    let Some(f) = work else { return; };
    let delay = delay_from_dispatch_time(when);

    std::thread::spawn(move || {
        if delay > std::time::Duration::ZERO {
            std::thread::sleep(delay);
        }
        dispatch_queue::queue_async(queue, Box::new(move || {
            f(context);
        }));
    });
}

/// Convert a dispatch_time_t to a Duration to sleep.
fn delay_from_dispatch_time(when: u64) -> std::time::Duration {
    if when == DISPATCH_TIME_FOREVER {
        // "Forever" — cap at a large but finite duration
        return std::time::Duration::from_secs(86400 * 365);
    }
    if when == DISPATCH_TIME_NOW {
        return std::time::Duration::ZERO;
    }
    let now = nanos_now_monotonic();
    if when > now {
        std::time::Duration::from_nanos(when - now)
    } else {
        std::time::Duration::ZERO
    }
}

// ═══════════════════════════════════════════════════════════════════
//  dispatch_async_f / dispatch_sync_f
// ═══════════════════════════════════════════════════════════════════

/// `void dispatch_async_f(dispatch_queue_t queue, void *context,
///                          void (*work)(void *))`
///
/// Routes through the queue model asynchronously.
pub extern "C" fn shim_dispatch_async_f(
    queue: u64,
    context: u64,
    work: Option<extern "C" fn(u64)>,
) {
    let Some(f) = work else { return; };
    dispatch_queue::queue_async(queue, Box::new(move || {
        f(context);
    }));
}

/// `void dispatch_sync_f(dispatch_queue_t queue, void *context,
///                         void (*work)(void *))`
///
/// Routes through the queue model synchronously.
pub extern "C" fn shim_dispatch_sync_f(
    queue: u64,
    context: u64,
    work: Option<extern "C" fn(u64)>,
) {
    let Some(f) = work else { return; };
    dispatch_queue::queue_sync(queue, Box::new(move || {
        f(context);
    }));
}

// ═══════════════════════════════════════════════════════════════════
//  dispatch_barrier_sync / dispatch_barrier_async
// ═══════════════════════════════════════════════════════════════════

/// `void dispatch_barrier_sync(dispatch_queue_t queue, dispatch_block_t block)`
///
/// Barrier on a concurrent queue ensures exclusive access.
pub extern "C" fn shim_dispatch_barrier_sync(queue: u64, block: *const u8) {
    if block.is_null() {
        return;
    }
    let invoke_ptr = unsafe { *(block.add(16) as *const usize) };
    let block_addr = block as usize;
    dispatch_queue::queue_barrier_sync(queue, Box::new(move || {
        unsafe {
            let invoke: extern "C" fn(*const u8) = std::mem::transmute(invoke_ptr);
            invoke(block_addr as *const u8);
        }
    }));
}

/// `void dispatch_barrier_async(dispatch_queue_t queue, dispatch_block_t block)`
pub extern "C" fn shim_dispatch_barrier_async(queue: u64, block: *const u8) {
    if block.is_null() {
        return;
    }
    let invoke_ptr = unsafe { *(block.add(16) as *const usize) };
    let block_addr = block as usize;
    dispatch_queue::queue_barrier_async(queue, Box::new(move || {
        unsafe {
            let invoke: extern "C" fn(*const u8) = std::mem::transmute(invoke_ptr);
            invoke(block_addr as *const u8);
        }
    }));
}

/// `void dispatch_barrier_async_f(dispatch_queue_t queue, void *context,
///                                  void (*work)(void *))`
pub extern "C" fn shim_dispatch_barrier_async_f(
    queue: u64,
    context: u64,
    work: Option<extern "C" fn(u64)>,
) {
    let Some(f) = work else { return; };
    dispatch_queue::queue_barrier_async(queue, Box::new(move || {
        f(context);
    }));
}

// ═══════════════════════════════════════════════════════════════════
//  dispatch_queue_get_label
// ═══════════════════════════════════════════════════════════════════

/// Empty label string for queues that don't have one.
static EMPTY_LABEL: [u8; 1] = [0];

/// `const char *dispatch_queue_get_label(dispatch_queue_t queue)`
///
/// Returns the label for a queue.  Well-known queues return their real
/// label; user-created queues return an empty string for now.
pub extern "C" fn shim_dispatch_queue_get_label(queue: u64) -> *const u8 {
    let label = dispatch_queue::queue_get_label(queue);
    if label.is_empty() {
        EMPTY_LABEL.as_ptr()
    } else {
        label.as_ptr()
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_time_now_plus_delta() {
        let t = shim_dispatch_time(DISPATCH_TIME_NOW, 1_000_000_000); // 1 second
        // Should be roughly now + 1s in nanoseconds
        assert!(t > 0);
    }

    #[test]
    fn dispatch_time_forever_stays_forever() {
        let t = shim_dispatch_time(DISPATCH_TIME_FOREVER, 1_000_000);
        assert_eq!(t, DISPATCH_TIME_FOREVER);
    }

    #[test]
    fn dispatch_time_negative_delta() {
        let now = nanos_now_monotonic();
        let t = shim_dispatch_time(now, -100);
        assert_eq!(t, now - 100);
    }

    #[test]
    fn dispatch_walltime_null_uses_now() {
        let t = shim_dispatch_walltime(std::ptr::null(), 0);
        assert!(t > 0);
    }

    #[test]
    fn delay_from_now_is_zero() {
        let d = delay_from_dispatch_time(DISPATCH_TIME_NOW);
        assert_eq!(d, std::time::Duration::ZERO);
    }

    #[test]
    fn delay_from_past_is_zero() {
        let d = delay_from_dispatch_time(1); // 1 nanosecond since epoch = far in the past
        assert_eq!(d, std::time::Duration::ZERO);
    }

    #[test]
    fn delay_from_future_is_positive() {
        let future = nanos_now_monotonic() + 1_000_000_000;
        let d = delay_from_dispatch_time(future);
        assert!(d > std::time::Duration::ZERO);
        assert!(d <= std::time::Duration::from_secs(2));
    }

    #[test]
    fn dispatch_sync_f_executes_inline() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        extern "C" fn set_flag(_ctx: u64) {
            CALLED.store(true, Ordering::SeqCst);
        }

        CALLED.store(false, Ordering::SeqCst);
        shim_dispatch_sync_f(0, 0, Some(set_flag));
        assert!(CALLED.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_async_f_runs_on_thread() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        static DONE: AtomicBool = AtomicBool::new(false);

        extern "C" fn set_done(_ctx: u64) {
            DONE.store(true, Ordering::SeqCst);
        }

        DONE.store(false, Ordering::SeqCst);
        shim_dispatch_async_f(0, 0, Some(set_done));

        // Wait up to 1 second for the thread to complete
        for _ in 0..100 {
            if DONE.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(DONE.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_queue_get_label_returns_empty() {
        let label = shim_dispatch_queue_get_label(0);
        assert!(!label.is_null());
        let c = unsafe { *label };
        assert_eq!(c, 0); // empty string (NUL terminator)
    }

    #[test]
    fn dispatch_after_f_fires() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static FIRED: AtomicBool = AtomicBool::new(false);

        extern "C" fn mark_fired(_ctx: u64) {
            FIRED.store(true, Ordering::SeqCst);
        }

        FIRED.store(false, Ordering::SeqCst);
        // Schedule for "now" — should fire immediately
        shim_dispatch_after_f(DISPATCH_TIME_NOW, 0, 0, Some(mark_fired));

        for _ in 0..100 {
            if FIRED.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(FIRED.load(Ordering::SeqCst));
    }

    #[test]
    fn dispatch_extras_install_count() {
        let mut table = ShimTable::new();
        DispatchExtrasStubs::install(&mut table);
        // Should have 39 entries
        assert!(table.len() >= 39, "expected ≥39 dispatch stubs, got {}", table.len());
    }
}
