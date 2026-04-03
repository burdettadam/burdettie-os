//! NSRunLoop shim: run loop for scheduling timers and input sources.
//!
//! Object layout (singleton per thread — we only support main):
//!
//!   offset  0: isa     (u64 — pointer to NSRunLoop class)
//!   offset  8: running (u64 — 1 while running, 0 otherwise)
//!
//! Total: 16 bytes.
//!
//! Timer firing: `addTimer:forMode:` appends to a global `PENDING_TIMERS`
//! list. Each call to `run` / `runUntilDate:` / `runMode:beforeDate:` drains
//! and fires all currently-due timers, then returns. This is sufficient for
//! the common single-threaded pattern of scheduling work and then spinning
//! the run loop once.

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

fn dispatch_selector_to_target(sel: u64, target: u64, arg: u64) -> bool {
    use objc_runtime::OBJC_RUNTIME;

    if sel == 0 || target == 0 {
        return false;
    }

    let sel_name = unsafe {
        std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };
    if sel_name.is_empty() {
        return false;
    }

    let isa = unsafe { *(target as *const u64) };

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };

    let Some(imp) = rt.msg_lookup(isa, sel_name) else {
        return false;
    };

    let sel_ptr = rt.selector_ptr(sel_name).unwrap_or(sel as usize) as u64;
    drop(guard);

    let func: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
    func(target, sel_ptr, arg);
    true
}

// ── Pending timer queue ────────────────────────────────────────────────────

struct PendingTimer {
    timer: u64,        // NSTimer object address
    fire_at: Instant,  // when to fire
}

static PENDING_TIMERS: OnceLock<Mutex<Vec<PendingTimer>>> = OnceLock::new();

fn pending_timers() -> &'static Mutex<Vec<PendingTimer>> {
    PENDING_TIMERS.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn reset_runtime_state_for_reuse() {
    pending_timers().lock().unwrap_or_else(|e| e.into_inner()).clear();
    if let Some(run_loop) = MAIN_RUN_LOOP.get().copied() {
        unsafe {
            *((run_loop as usize + 8) as *mut u64) = 0;
        }
    }
}

/// Fire all timers whose `fire_at` is ≤ now, keeping repeating timers.
fn fire_due_timers() {
    let now = Instant::now();
    let mut guard = pending_timers().lock().unwrap_or_else(|e| e.into_inner());
    let mut i = 0;
    while i < guard.len() {
        if guard[i].fire_at <= now {
            let timer = guard[i].timer;
            // Read interval and repeats flag from NSTimer layout
            // [0]=isa [1]=interval_bits [2]=block_ptr [3]=flags
            let interval_bits = unsafe { *((timer as *const u64).add(1)) };
            let flags = unsafe { *((timer as *const u64).add(3)) };
            let flag_valid: u64 = 1;
            let flag_repeats: u64 = 2;
            if flags & flag_valid == 0 {
                guard.remove(i);
                continue;
            }
            // Call the timer's fire shim directly
            crate::nstimer::shim_nstimer_fire(timer, 0);
            let repeats = flags & flag_repeats != 0;
            if repeats {
                let interval = f64::from_bits(interval_bits);
                guard[i].fire_at = now + std::time::Duration::from_secs_f64(interval.max(0.001));
                i += 1;
            } else {
                guard.remove(i);
            }
        } else {
            i += 1;
        }
    }
}

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

/// `-[NSRunLoop runUntilDate:]` — fires all due timers, then returns.
pub extern "C" fn shim_nsrunloop_run_until_date(_obj: u64, _cmd: u64, _date: u64) {
    fire_due_timers();
}

/// `-[NSRunLoop runMode:beforeDate:]` — fires all due timers, returns YES.
pub extern "C" fn shim_nsrunloop_run_mode_before_date(_obj: u64, _cmd: u64, _mode: u64, _date: u64) -> u8 {
    fire_due_timers();
    1
}

/// `-[NSRunLoop run]` — fires all due timers and returns.
///
/// In a real run loop, `run` would block indefinitely. Here we fire pending
/// timers once and return so that single-threaded tests using the pattern
/// `[timer scheduledTimer…]; [[NSRunLoop currentRunLoop] run];` proceed.
pub extern "C" fn shim_nsrunloop_run(_obj: u64, _cmd: u64) {
    fire_due_timers();
}

/// `-[NSRunLoop addTimer:forMode:]` — enqueues the timer for firing on the next run.
pub extern "C" fn shim_nsrunloop_add_timer_for_mode(_obj: u64, _cmd: u64, timer: u64, _mode: u64) {
    if timer == 0 { return; }
    // Read the fire interval from the NSTimer layout
    let interval_bits = unsafe { *((timer as *const u64).add(1)) };
    let interval = f64::from_bits(interval_bits);
    let fire_at = Instant::now() + std::time::Duration::from_secs_f64(interval.max(0.0));
    let mut guard = pending_timers().lock().unwrap_or_else(|e| e.into_inner());
    guard.push(PendingTimer { timer, fire_at });
}

/// `-[NSRunLoop currentMode]` — returns NSDefaultRunLoopMode.
pub extern "C" fn shim_nsrunloop_current_mode(_obj: u64, _cmd: u64) -> u64 {
    nsdefault_run_loop_mode_addr()
}

/// `-[NSRunLoop getCFRunLoop]` — returns the run loop object itself (stub).
pub extern "C" fn shim_nsrunloop_get_cf_run_loop(obj: u64, _cmd: u64) -> u64 {
    // In a real implementation this returns the wrapped CFRunLoopRef.
    // We return the NSRunLoop object itself since there's no separate CF layer.
    obj
}

/// `-[NSRunLoop performSelector:target:argument:order:modes:]`
pub extern "C" fn shim_nsrunloop_perform_selector(
    _obj: u64, _cmd: u64, sel: u64, target: u64, arg: u64, _order: u64, _modes: u64,
) -> u64 {
    let _ = dispatch_selector_to_target(sel, target, arg);
    0
}

/// `-[NSRunLoop acceptInputForMode:beforeDate:]`
pub extern "C" fn shim_nsrunloop_accept_input(_obj: u64, _cmd: u64, _mode: u64, _date: u64) -> u64 {
    fire_due_timers();
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static RUN_LOOP_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock_runloop_state() -> std::sync::MutexGuard<'static, ()> {
        RUN_LOOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn current_run_loop_returns_nonzero() {
        let rl = shim_nsrunloop_current_run_loop(0, 0);
        assert_ne!(rl, 0);
    }

    #[test]
    fn main_equals_current() {
        let main = shim_nsrunloop_main_run_loop(0, 0);
        let curr = shim_nsrunloop_current_run_loop(0, 0);
        assert_ne!(main, 0);
        assert_eq!(main, curr);
    }

    #[test]
    fn current_mode_returns_nonzero() {
        let mode = shim_nsrunloop_current_mode(0, 0);
        assert_ne!(mode, 0);
    }

    #[test]
    fn run_nil_is_safe() {
        shim_nsrunloop_run(0, 0);
    }

    #[test]
    fn run_until_date_nil_is_safe() {
        shim_nsrunloop_run_until_date(0, 0, 0);
    }

    #[test]
    fn add_timer_nil_timer_is_safe() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let rl = shim_nsrunloop_current_run_loop(0, 0);
        shim_nsrunloop_add_timer_for_mode(rl, 0, 0, 0);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn run_mode_before_date_returns_yes() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let rl = shim_nsrunloop_current_run_loop(0, 0);
        let result = shim_nsrunloop_run_mode_before_date(rl, 0, 0, 0);
        assert_eq!(result, 1);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn nsrunloop_current_mode_returns_default() {
        let mode = shim_nsrunloop_current_mode(0, 0);
        assert_ne!(mode, 0);
        assert_eq!(mode, nsdefault_run_loop_mode_addr());
    }

    #[test]
    fn nsrunloop_run_mode_processes_timers() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        // Create a non-repeating timer with 0 interval (block=0)
        let t = crate::nstimer::shim_nstimer_timer_with_block(0, 0, 0.0, 0, 0);
        assert_ne!(t, 0);
        assert_eq!(crate::nstimer::shim_nstimer_is_valid(t, 0), 1);

        // Add to run loop manually (timerWithBlock doesn't auto-schedule)
        let rl = shim_nsrunloop_current_run_loop(0, 0);
        shim_nsrunloop_add_timer_for_mode(rl, 0, t, 0);

        // Small sleep so fire_at is in the past
        std::thread::sleep(std::time::Duration::from_millis(5));

        // runMode:beforeDate: should fire the timer
        let result = shim_nsrunloop_run_mode_before_date(rl, 0, 0, 0);
        assert_eq!(result, 1);
        assert_eq!(
            crate::nstimer::shim_nstimer_is_valid(t, 0),
            0,
            "non-repeating timer should auto-invalidate after fire"
        );
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn perform_selector_nil_inputs_are_safe() {
        let rl = shim_nsrunloop_current_run_loop(0, 0);
        assert_eq!(shim_nsrunloop_perform_selector(rl, 0, 0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn nsrunloop_accept_input_processes_timers() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let t = crate::nstimer::shim_nstimer_timer_with_block(0, 0, 0.0, 0, 0);
        assert_ne!(t, 0);
        assert_eq!(crate::nstimer::shim_nstimer_is_valid(t, 0), 1);

        let rl = shim_nsrunloop_current_run_loop(0, 0);
        shim_nsrunloop_add_timer_for_mode(rl, 0, t, 0);

        std::thread::sleep(std::time::Duration::from_millis(5));

        shim_nsrunloop_accept_input(rl, 0, 0, 0);
        assert_eq!(
            crate::nstimer::shim_nstimer_is_valid(t, 0),
            0,
            "acceptInputForMode:beforeDate: should drain due timers"
        );
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn reset_runtime_state_clears_pending_timers() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();

        let t = crate::nstimer::shim_nstimer_timer_with_block(0, 0, 0.0, 0, 0);
        assert_ne!(t, 0);
        assert_eq!(crate::nstimer::shim_nstimer_is_valid(t, 0), 1);

        let rl = shim_nsrunloop_current_run_loop(0, 0);
        shim_nsrunloop_add_timer_for_mode(rl, 0, t, 0);

        reset_runtime_state_for_reuse();
        shim_nsrunloop_accept_input(rl, 0, 0, 0);

        assert_eq!(
            crate::nstimer::shim_nstimer_is_valid(t, 0),
            1,
            "reset_runtime_state_for_reuse should unschedule pending timers without firing them"
        );
        reset_runtime_state_for_reuse();
    }
}

/// `-[NSRunLoop description]`
pub extern "C" fn shim_nsrunloop_description(_obj: u64, _cmd: u64) -> u64 {
    let c = std::ffi::CString::new("<NSRunLoop: kCFRunLoopDefaultMode>").unwrap();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}
