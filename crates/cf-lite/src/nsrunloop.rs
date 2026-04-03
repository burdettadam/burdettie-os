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

// ── CFRunLoopSource ────────────────────────────────────────────────────────
//
// A version-0 source stores the CFRunLoopSourceContext fields we care about:
// - `info: u64` — opaque pointer passed to callbacks
// - `perform: u64` — function pointer: `void (*perform)(void *info)`
// - `signaled: bool` — whether CFRunLoopSourceSignal was called
// - `valid: bool`

struct RunLoopSource {
    /// Opaque ID for the allocated source object.
    addr: u64,
    /// `void *info` from CFRunLoopSourceContext.
    info: u64,
    /// `void (*perform)(void *info)` — the source's perform callback.
    perform: u64,
    /// Whether the source has been signaled (ready to fire).
    signaled: bool,
    /// Whether the source is still valid.
    valid: bool,
}

static SOURCES: OnceLock<Mutex<Vec<RunLoopSource>>> = OnceLock::new();

fn sources() -> &'static Mutex<Vec<RunLoopSource>> {
    SOURCES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Allocate a new CFRunLoopSource from a context pointer.
///
/// `CFRunLoopSourceContext` (version 0) layout:
/// ```text
/// [0]  CFIndex version         (8 bytes)
/// [1]  void *info              (8 bytes)
/// [2]  retain callback         (8 bytes)
/// [3]  release callback        (8 bytes)
/// [4]  copyDescription         (8 bytes)
/// [5]  equal callback          (8 bytes)
/// [6]  hash callback           (8 bytes)
/// [7]  schedule callback       (8 bytes)
/// [8]  cancel callback         (8 bytes)
/// [9]  perform callback        (8 bytes)
/// ```
pub fn create_source(ctx: u64) -> u64 {
    let (info, perform) = if ctx != 0 {
        unsafe {
            let base = ctx as *const u64;
            let info = *base.add(1);
            let perform = *base.add(9);
            (info, perform)
        }
    } else {
        (0, 0)
    };
    // Allocate a small object as the source "handle"
    let addr = crate::pal_alloc().calloc(1, 8) as u64;
    if addr == 0 { return 0; }
    let mut guard = sources().lock().unwrap_or_else(|e| e.into_inner());
    guard.push(RunLoopSource { addr, info, perform, signaled: false, valid: true });
    addr
}

pub fn signal_source(source: u64) {
    let mut guard = sources().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = guard.iter_mut().find(|s| s.addr == source && s.valid) {
        s.signaled = true;
    }
}

pub fn invalidate_source(source: u64) {
    let mut guard = sources().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = guard.iter_mut().find(|s| s.addr == source) {
        s.valid = false;
        s.signaled = false;
    }
}

pub fn source_is_valid(source: u64) -> u8 {
    let guard = sources().lock().unwrap_or_else(|e| e.into_inner());
    if guard.iter().any(|s| s.addr == source && s.valid) { 1 } else { 0 }
}

pub fn add_source(source: u64) {
    // Source is already tracked in the global SOURCES Vec.
    // Mark it as valid in case it was re-added after removal.
    let guard = sources().lock().unwrap_or_else(|e| e.into_inner());
    let _ = guard.iter().find(|s| s.addr == source);
    // No-op beyond ensuring it exists — fire_sources handles dispatch.
}

pub fn remove_source(source: u64) {
    let mut guard = sources().lock().unwrap_or_else(|e| e.into_inner());
    guard.retain(|s| s.addr != source);
}

/// Fire all signaled sources. Called from run loop dispatch.
fn fire_signaled_sources() {
    // Collect sources to fire while holding the lock, then fire without lock.
    let to_fire: Vec<(u64, u64)> = {
        let mut guard = sources().lock().unwrap_or_else(|e| e.into_inner());
        let mut result = Vec::new();
        for s in guard.iter_mut() {
            if s.valid && s.signaled {
                s.signaled = false;
                if s.perform != 0 {
                    result.push((s.perform, s.info));
                }
            }
        }
        result
    };
    for (perform, info) in to_fire {
        let f: extern "C" fn(u64) = unsafe { std::mem::transmute(perform) };
        f(info);
    }
}

// ── CFRunLoopObserver ──────────────────────────────────────────────────────
//
// Core Foundation run loop activity flags:
//   kCFRunLoopEntry       = 1 << 0
//   kCFRunLoopBeforeTimers = 1 << 1
//   kCFRunLoopBeforeSources = 1 << 2
//   kCFRunLoopBeforeWaiting = 1 << 5
//   kCFRunLoopAfterWaiting  = 1 << 6
//   kCFRunLoopExit         = 1 << 7
//   kCFRunLoopAllActivities = 0x0FFFFFFF

pub const CF_RUN_LOOP_ENTRY: u64 = 1 << 0;
pub const CF_RUN_LOOP_BEFORE_TIMERS: u64 = 1 << 1;
pub const CF_RUN_LOOP_BEFORE_SOURCES: u64 = 1 << 2;
pub const CF_RUN_LOOP_BEFORE_WAITING: u64 = 1 << 5;
pub const CF_RUN_LOOP_AFTER_WAITING: u64 = 1 << 6;
pub const CF_RUN_LOOP_EXIT: u64 = 1 << 7;

struct RunLoopObserver {
    addr: u64,
    /// Bitmask of activity flags this observer monitors.
    activities: u64,
    /// Whether the observer fires on every matching activity (true) or once (false).
    repeats: bool,
    /// `void (*callback)(CFRunLoopObserverRef observer, CFRunLoopActivity activity, void *info)`
    callback: u64,
    /// `void *info` from observer context.
    info: u64,
    valid: bool,
}

static OBSERVERS: OnceLock<Mutex<Vec<RunLoopObserver>>> = OnceLock::new();

fn observers() -> &'static Mutex<Vec<RunLoopObserver>> {
    OBSERVERS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn create_observer(activities: u64, repeats: bool, callback: u64, ctx: u64) -> u64 {
    let info = if ctx != 0 {
        // CFRunLoopObserverContext: version(8), info(8), retain(8), release(8), copyDescription(8)
        unsafe { *(ctx as *const u64).add(1) }
    } else {
        0
    };
    let addr = crate::pal_alloc().calloc(1, 8) as u64;
    if addr == 0 { return 0; }
    let mut guard = observers().lock().unwrap_or_else(|e| e.into_inner());
    guard.push(RunLoopObserver { addr, activities, repeats, callback, info, valid: true });
    addr
}

pub fn add_observer(observer: u64) {
    // Observer is already tracked in the global OBSERVERS Vec.
    // No-op — just ensure the observer exists.
    let _guard = observers().lock().unwrap_or_else(|e| e.into_inner());
    let _ = observer;
}

pub fn remove_observer(observer: u64) {
    let mut guard = observers().lock().unwrap_or_else(|e| e.into_inner());
    guard.retain(|o| o.addr != observer);
}

pub fn invalidate_observer(observer: u64) {
    let mut guard = observers().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(o) = guard.iter_mut().find(|o| o.addr == observer) {
        o.valid = false;
    }
}

/// Notify all observers watching for the given activity.
fn notify_observers(activity: u64) {
    let to_call: Vec<(u64, u64, u64, bool)> = {
        let guard = observers().lock().unwrap_or_else(|e| e.into_inner());
        guard.iter()
            .filter(|o| o.valid && (o.activities & activity) != 0 && o.callback != 0)
            .map(|o| (o.callback, o.addr, o.info, o.repeats))
            .collect()
    };
    for (callback, addr, info, repeats) in &to_call {
        let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(*callback) };
        f(*addr, activity, *info);
        if !repeats {
            invalidate_observer(*addr);
        }
    }
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
    sources().lock().unwrap_or_else(|e| e.into_inner()).clear();
    observers().lock().unwrap_or_else(|e| e.into_inner()).clear();
    if let Some(run_loop) = MAIN_RUN_LOOP.get().copied() {
        unsafe {
            *((run_loop as usize + 8) as *mut u64) = 0;
        }
    }
}

/// Execute one full run-loop cycle: notify observers, fire sources, fire timers.
fn run_loop_cycle() {
    notify_observers(CF_RUN_LOOP_BEFORE_TIMERS);
    notify_observers(CF_RUN_LOOP_BEFORE_SOURCES);
    fire_signaled_sources();
    fire_due_timers();
    // Flush pending Auto Layout passes before waiting — mirrors Apple's
    // CATransaction::commit() / layout flush at kCFRunLoopBeforeWaiting.
    crate::layout_solver::flush_pending_layouts();
    notify_observers(CF_RUN_LOOP_BEFORE_WAITING);
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

/// `-[NSRunLoop runUntilDate:]` — runs one cycle (observers + sources + timers).
pub extern "C" fn shim_nsrunloop_run_until_date(_obj: u64, _cmd: u64, _date: u64) {
    run_loop_cycle();
}

/// `-[NSRunLoop runMode:beforeDate:]` — runs one cycle, returns YES.
pub extern "C" fn shim_nsrunloop_run_mode_before_date(_obj: u64, _cmd: u64, _mode: u64, _date: u64) -> u8 {
    run_loop_cycle();
    1
}

/// `-[NSRunLoop run]` — runs one cycle (observers + sources + timers).
///
/// In a real run loop, `run` would block indefinitely. Here we run one
/// cycle and return so that single-threaded tests using the pattern
/// `[timer scheduledTimer…]; [[NSRunLoop currentRunLoop] run];` proceed.
pub extern "C" fn shim_nsrunloop_run(_obj: u64, _cmd: u64) {
    notify_observers(CF_RUN_LOOP_ENTRY);
    run_loop_cycle();
    notify_observers(CF_RUN_LOOP_EXIT);
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
    run_loop_cycle();
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

    // ── CFRunLoopSource tests ──────────────────────────────────────────

    #[test]
    fn create_source_returns_nonzero() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let src = create_source(0);
        assert_ne!(src, 0);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn source_is_valid_after_create() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let src = create_source(0);
        assert_eq!(source_is_valid(src), 1);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn source_invalidate() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let src = create_source(0);
        assert_eq!(source_is_valid(src), 1);
        invalidate_source(src);
        assert_eq!(source_is_valid(src), 0);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn source_signal_and_check() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let src = create_source(0);
        signal_source(src);
        // fire_signaled_sources should not crash even with perform == 0
        fire_signaled_sources();
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn remove_source_makes_it_gone() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let src = create_source(0);
        assert_eq!(source_is_valid(src), 1);
        remove_source(src);
        assert_eq!(source_is_valid(src), 0);
        reset_runtime_state_for_reuse();
    }

    // ── CFRunLoopObserver tests ────────────────────────────────────────

    #[test]
    fn create_observer_returns_nonzero() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let obs = create_observer(CF_RUN_LOOP_ENTRY, true, 0, 0);
        assert_ne!(obs, 0);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn observer_invalidate() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let obs = create_observer(CF_RUN_LOOP_BEFORE_TIMERS, true, 0, 0);
        assert_ne!(obs, 0);
        invalidate_observer(obs);
        // After invalidation the observer should not fire (callback == 0 anyway,
        // but verify the valid flag is cleared by checking the internal state).
        let guard = observers().lock().unwrap();
        let o = guard.iter().find(|o| o.addr == obs);
        assert!(o.is_some(), "observer should still be in the Vec");
        assert!(!o.unwrap().valid, "observer should be invalid");
        drop(guard);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn remove_observer_removes() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        let obs = create_observer(CF_RUN_LOOP_EXIT, false, 0, 0);
        assert_ne!(obs, 0);
        remove_observer(obs);
        let guard = observers().lock().unwrap();
        assert!(guard.iter().find(|o| o.addr == obs).is_none(), "observer should be removed");
        drop(guard);
        reset_runtime_state_for_reuse();
    }

    // ── Nil safety tests ───────────────────────────────────────────────

    #[test]
    fn nil_source_operations_safe() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        signal_source(0);
        invalidate_source(0);
        assert_eq!(source_is_valid(0), 0);
        add_source(0);
        remove_source(0);
        reset_runtime_state_for_reuse();
    }

    #[test]
    fn nil_observer_safe() {
        let _guard = lock_runloop_state();
        reset_runtime_state_for_reuse();
        invalidate_observer(0);
        remove_observer(0);
        add_observer(0);
        reset_runtime_state_for_reuse();
    }
}

/// `-[NSRunLoop description]`
pub extern "C" fn shim_nsrunloop_description(_obj: u64, _cmd: u64) -> u64 {
    let c = std::ffi::CString::new("<NSRunLoop: kCFRunLoopDefaultMode>").unwrap();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}
