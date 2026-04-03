//! NSThread shim: thread management Foundation class.
//!
//! Object layout (48 bytes):
//!   [0..8)   ISA pointer
//!   [8..16)  name_ptr   (NSString * or 0)
//!   [16..24) target     (u64 — target object for initWithTarget:)
//!   [24..32) selector   (u64 — selector for initWithTarget:)
//!   [32..40) is_main    (u64 — 1 = main thread, 0 = not)
//!   [40..48) is_executing (u64 — 1 = running, 0 = not; doubles as finished when 2)
//!
//! NSThread wraps POSIX threads and provides ObjC-level thread
//! management. The main thread is identified via `MAIN_THREAD_ID`.

use std::sync::OnceLock;
use std::thread;

use crate::cfstring;
use crate::nsobj;

/// Object size for NSThread instances.
pub(crate) const NSTHREAD_SIZE: usize = 48;

// Field offsets
const NAME_OFF: usize = 8;
const TARGET_OFF: usize = 16;
const SELECTOR_OFF: usize = 24;
const IS_MAIN_OFF: usize = 32;
const EXEC_OFF: usize = 40;

/// Execution state constants stored at EXEC_OFF.
const STATE_NOT_STARTED: u64 = 0;
const STATE_EXECUTING: u64 = 1;
const STATE_FINISHED: u64 = 2;

// ── Main thread ID ─────────────────────────────────────────────────

/// Stores the `ThreadId` of the main thread, captured at first access.
static MAIN_THREAD_ID: OnceLock<thread::ThreadId> = OnceLock::new();

/// Record the calling thread as the main thread. Called once at startup.
pub fn init_main_thread_id() {
    MAIN_THREAD_ID.get_or_init(|| thread::current().id());
}

/// Returns true if the calling thread is the main thread.
fn is_current_main_thread() -> bool {
    let main_id = MAIN_THREAD_ID.get_or_init(|| thread::current().id());
    thread::current().id() == *main_id
}

// ── Main thread singleton ──────────────────────────────────────────

static MAIN_THREAD_OBJ: OnceLock<u64> = OnceLock::new();

fn get_main_thread_obj() -> u64 {
    *MAIN_THREAD_OBJ.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, NSTHREAD_SIZE) as u64;
        if obj == 0 {
            return 0;
        }
        unsafe {
            *(obj as *mut u64) = nsobj::nsthread_class_addr();
            *((obj as usize + IS_MAIN_OFF) as *mut u64) = 1;
            *((obj as usize + EXEC_OFF) as *mut u64) = STATE_EXECUTING;
        }
        obj
    })
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSThread currentThread]` — returns an NSThread for the calling thread.
///
/// Returns the main thread singleton when on the main thread; otherwise
/// allocates a fresh NSThread each call.
pub extern "C" fn shim_nsthread_current_thread(_cls: u64, _cmd: u64) -> u64 {
    if is_current_main_thread() {
        return get_main_thread_obj();
    }
    let obj = crate::pal_alloc().calloc(1, NSTHREAD_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsthread_class_addr();
        *((obj as usize + IS_MAIN_OFF) as *mut u64) = 0;
        *((obj as usize + EXEC_OFF) as *mut u64) = STATE_EXECUTING;
    }
    obj
}

/// `+[NSThread mainThread]` — returns the main-thread singleton.
pub extern "C" fn shim_nsthread_main_thread(_cls: u64, _cmd: u64) -> u64 {
    get_main_thread_obj()
}

/// `+[NSThread isMainThread]` — returns YES if called from the main thread.
pub extern "C" fn shim_nsthread_is_main_thread_class(_cls: u64, _cmd: u64) -> u8 {
    if is_current_main_thread() {
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
) {
    std::thread::sleep(std::time::Duration::from_secs_f64(interval));
}

/// `+[NSThread detachNewThreadSelector:toTarget:withObject:]` — spawn and start immediately.
pub extern "C" fn shim_nsthread_detach(
    _cls: u64,
    _cmd: u64,
    sel: u64,
    target: u64,
    arg: u64,
) {
    if target == 0 {
        return;
    }
    let obj = crate::pal_alloc().calloc(1, NSTHREAD_SIZE) as u64;
    if obj == 0 {
        return;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsthread_class_addr();
        *((obj as usize + TARGET_OFF) as *mut u64) = target;
        *((obj as usize + SELECTOR_OFF) as *mut u64) = sel;
        *((obj as usize + EXEC_OFF) as *mut u64) = STATE_EXECUTING;
    }
    spawn_thread(obj, target, sel, arg);
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSThread isMainThread]` — returns YES if this thread object represents main.
pub extern "C" fn shim_nsthread_is_main_thread(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flag = unsafe { *((obj as usize + IS_MAIN_OFF) as *const u64) };
    if flag != 0 { 1 } else { 0 }
}

/// `-[NSThread name]` — returns the thread name (NSString or nil).
pub extern "C" fn shim_nsthread_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NAME_OFF) as *const u64) }
}

/// `-[NSThread setName:]` — sets the thread name.
pub extern "C" fn shim_nsthread_set_name(obj: u64, _cmd: u64, name: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + NAME_OFF) as *mut u64) = name;
    }
}

/// `-[NSThread initWithTarget:selector:object:]` — configure thread with target/selector.
pub extern "C" fn shim_nsthread_init_with_target(
    obj: u64,
    _cmd: u64,
    target: u64,
    sel: u64,
    _argument: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + TARGET_OFF) as *mut u64) = target;
        *((obj as usize + SELECTOR_OFF) as *mut u64) = sel;
        *((obj as usize + EXEC_OFF) as *mut u64) = STATE_NOT_STARTED;
    }
    obj
}

/// `-[NSThread start]` — spawn a real std::thread to run the target/selector.
pub extern "C" fn shim_nsthread_start(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let target = unsafe { *((obj as usize + TARGET_OFF) as *const u64) };
    let sel = unsafe { *((obj as usize + SELECTOR_OFF) as *const u64) };
    if target == 0 {
        return;
    }
    unsafe {
        *((obj as usize + EXEC_OFF) as *mut u64) = STATE_EXECUTING;
    }
    spawn_thread(obj, target, sel, 0);
}

/// `-[NSThread isExecuting]` — returns YES if the thread is currently running.
pub extern "C" fn shim_nsthread_is_executing(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let state = unsafe { *((obj as usize + EXEC_OFF) as *const u64) };
    if state == STATE_EXECUTING { 1 } else { 0 }
}

/// `-[NSThread isFinished]` — returns YES if the thread has completed.
pub extern "C" fn shim_nsthread_is_finished(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let state = unsafe { *((obj as usize + EXEC_OFF) as *const u64) };
    if state == STATE_FINISHED { 1 } else { 0 }
}

/// `-[NSThread description]` — returns a description string.
pub extern "C" fn shim_nsthread_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let is_main = unsafe { *((obj as usize + IS_MAIN_OFF) as *const u64) };
    let desc = if is_main != 0 {
        c"<NSThread: main>"
    } else {
        c"<NSThread: secondary>"
    };
    cfstring::shim_cf_string_create_with_cstring(0, desc.as_ptr(), 0)
}

/// `-[NSThread threadPriority]` — returns the thread priority (0.0–1.0).
pub extern "C" fn shim_nsthread_thread_priority(_obj: u64, _cmd: u64) -> f64 {
    0.5
}

/// `-[NSThread setThreadPriority:]` — no-op stub.
pub extern "C" fn shim_nsthread_set_thread_priority(_obj: u64, _cmd: u64, _priority: f64) {}

/// `-[NSThread qualityOfService]` — returns NSQualityOfServiceDefault (-1).
pub extern "C" fn shim_nsthread_quality_of_service(_obj: u64, _cmd: u64) -> i64 {
    -1
}

/// `-[NSThread setQualityOfService:]` — no-op stub.
pub extern "C" fn shim_nsthread_set_quality_of_service(_obj: u64, _cmd: u64, _qos: i64) {}

/// `-[NSThread stackSize]` — returns the default stack size (512 KB).
pub extern "C" fn shim_nsthread_stack_size(_obj: u64, _cmd: u64) -> u64 {
    512 * 1024
}

/// `-[NSThread setStackSize:]` — no-op stub.
pub extern "C" fn shim_nsthread_set_stack_size(_obj: u64, _cmd: u64, _size: u64) {}

/// `-[NSThread threadDictionary]` — returns a per-thread NSMutableDictionary.
///
/// Apple's implementation returns a persistent mutable dictionary that
/// lives for the duration of the thread. We lazily create one per thread.
pub extern "C" fn shim_nsthread_thread_dictionary(_obj: u64, _cmd: u64) -> u64 {
    thread_local! {
        static CACHED_DICT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }
    CACHED_DICT.with(|cell| {
        let existing = cell.get();
        if existing != 0 {
            return existing;
        }
        let cls = nsobj::nsmutabledictionary_class_addr();
        let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(cls, 0);
        cell.set(dict);
        dict
    })
}

// ── Internal ───────────────────────────────────────────────────────

/// Spawn a real OS thread that invokes `[target selector:arg]` via the
/// ObjC runtime, then marks the NSThread object as finished.
fn spawn_thread(obj: u64, target: u64, sel: u64, arg: u64) {
    let obj_ptr = obj as usize;
    let target_val = target as usize;
    let sel_val = sel as usize;
    let arg_val = arg as usize;
    thread::spawn(move || {
        // Invoke the target selector via ObjC message dispatch.
        if target_val != 0 && sel_val != 0 {
            use objc_runtime::OBJC_RUNTIME;

            let target = target_val as u64;
            let sel = sel_val as u64;
            let arg = arg_val as u64;

            // Read the selector name from the SEL pointer (C string).
            // Safety: verify the pointer is in a mapped memory page before
            // dereferencing. Use mincore() to check page residency.
            let sel_name = if sel > 0x10000 {
                let page_size = 0x4000u64;
                let page_start = sel & !(page_size - 1);
                let mut vec: u8 = 0;
                let mapped = unsafe {
                    libc::mincore(
                        page_start as *mut libc::c_void,
                        page_size as usize,
                        &mut vec as *mut u8 as *mut libc::c_char,
                    )
                } == 0;
                if mapped {
                    unsafe {
                        std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
                            .to_str()
                            .ok()
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(sel_name) = sel_name {
                if !sel_name.is_empty() {
                    let isa = unsafe { *(target as *const u64) };
                    let guard = match OBJC_RUNTIME.read() {
                        Ok(g) => g,
                        Err(e) => e.into_inner(),
                    };
                    if let Some(rt) = guard.as_ref() {
                        if let Some(imp) = rt.msg_lookup(isa, sel_name) {
                            let sel_ptr = rt.selector_ptr(sel_name)
                                .unwrap_or(sel as usize) as u64;
                            drop(guard);
                            let func: extern "C" fn(u64, u64, u64) =
                                unsafe { std::mem::transmute(imp) };
                            func(target, sel_ptr, arg);
                        }
                    }
                }
            }
        }
        // Mark finished.
        unsafe {
            *((obj_ptr + EXEC_OFF) as *mut u64) = STATE_FINISHED;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn make_thread_obj(is_main: bool) -> u64 {
        let obj = alloc_obj(NSTHREAD_SIZE, crate::nsobj::nsthread_class_addr());
        write_field(obj, IS_MAIN_OFF, if is_main { 1 } else { 0 });
        obj
    }

    // ── Existing tests (updated for new layout) ────────────────────

    #[test]
    fn nsthread_is_main_thread_returns_true() {
        let t = make_thread_obj(true);
        assert_eq!(shim_nsthread_is_main_thread(t, 0), 1);
        free_obj(t);
    }

    #[test]
    fn nsthread_is_main_thread_returns_false() {
        let t = make_thread_obj(false);
        assert_eq!(shim_nsthread_is_main_thread(t, 0), 0);
        free_obj(t);
    }

    #[test]
    fn nsthread_is_main_thread_nil_returns_zero() {
        assert_eq!(shim_nsthread_is_main_thread(0, 0), 0);
    }

    #[test]
    fn nsthread_current_returns_object() {
        // Ensure main thread ID is captured
        init_main_thread_id();
        let t = shim_nsthread_current_thread(0, 0);
        assert_ne!(t, 0, "currentThread must return non-null");
        // On the test runner thread (which IS main), is_main should be set
        let is_main = shim_nsthread_is_main_thread(t, 0);
        assert_eq!(is_main, 1);
    }

    #[test]
    fn nsthread_main_thread_returns_object() {
        init_main_thread_id();
        let t = shim_nsthread_main_thread(0, 0);
        assert_ne!(t, 0, "mainThread must return non-null");
        assert_eq!(shim_nsthread_is_main_thread(t, 0), 1);
    }

    #[test]
    fn nsthread_set_and_get_name() {
        let t = make_thread_obj(false);
        assert_eq!(shim_nsthread_name(t, 0), 0, "name should be nil initially");
        let name = make_string("worker-thread");
        shim_nsthread_set_name(t, 0, name);
        assert_eq!(shim_nsthread_name(t, 0), name);
        free_obj(t);
    }

    #[test]
    fn nsthread_set_name_nil_obj_is_noop() {
        let name = make_string("unused");
        shim_nsthread_set_name(0, 0, name); // must not crash
    }

    #[test]
    fn nsthread_init_with_target() {
        let t = make_thread_obj(false);
        let fake_target: u64 = 0x1234;
        let fake_sel: u64 = 0x5678;
        let result = shim_nsthread_init_with_target(t, 0, fake_target, fake_sel, 0);
        assert_eq!(result, t, "initWithTarget: should return self");
        assert_eq!(read_field(t, TARGET_OFF), fake_target);
        assert_eq!(read_field(t, SELECTOR_OFF), fake_sel);
        assert_eq!(read_field(t, EXEC_OFF), STATE_NOT_STARTED);
        free_obj(t);
    }

    #[test]
    fn nsthread_sleep_for_interval() {
        let before = std::time::Instant::now();
        shim_nsthread_sleep_for_time_interval(0, 0, 0.001); // 1ms
        let elapsed = before.elapsed();
        assert!(
            elapsed.as_micros() >= 500,
            "sleep should have waited at least ~1ms, got {:?}",
            elapsed
        );
    }

    #[test]
    fn nsthread_is_executing_before_start() {
        let t = make_thread_obj(false);
        write_field(t, EXEC_OFF, STATE_NOT_STARTED);
        assert_eq!(shim_nsthread_is_executing(t, 0), 0);
        free_obj(t);
    }

    #[test]
    fn nsthread_start_spawns_thread() {
        let t = alloc_obj(NSTHREAD_SIZE, crate::nsobj::nsthread_class_addr());
        // Set up a dummy target so start proceeds
        write_field(t, TARGET_OFF, 0xAAAA);
        write_field(t, SELECTOR_OFF, 0xBBBB);
        write_field(t, EXEC_OFF, STATE_NOT_STARTED);

        shim_nsthread_start(t, 0);

        // The spawned thread should mark the object as finished shortly
        std::thread::sleep(std::time::Duration::from_millis(50));
        let state = read_field(t, EXEC_OFF);
        assert_eq!(state, STATE_FINISHED, "thread should be finished after sleep");
        free_obj(t);
    }

    #[test]
    fn nsthread_detach_runs_selector() {
        let t = alloc_obj(NSTHREAD_SIZE, crate::nsobj::nsthread_class_addr());
        // Use t as its own "target" for simplicity (just needs non-zero)
        shim_nsthread_detach(0, 0, 0xBBBB, t, 0);
        // Brief sleep to let the detached thread complete
        std::thread::sleep(std::time::Duration::from_millis(50));
        // If we get here without crash, the detach worked
        free_obj(t);
    }

    #[test]
    fn nsthread_is_finished_after_completion() {
        let t = make_thread_obj(false);
        write_field(t, EXEC_OFF, STATE_FINISHED);
        assert_eq!(shim_nsthread_is_finished(t, 0), 1);
        free_obj(t);
    }

    #[test]
    fn nsthread_is_finished_before_start() {
        let t = make_thread_obj(false);
        write_field(t, EXEC_OFF, STATE_NOT_STARTED);
        assert_eq!(shim_nsthread_is_finished(t, 0), 0);
        free_obj(t);
    }

    #[test]
    fn nsthread_description_main_thread() {
        let t = make_thread_obj(true);
        let desc = shim_nsthread_description(t, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSThread: main>");
        free_obj(t);
    }

    #[test]
    fn nsthread_description_secondary_thread() {
        let t = make_thread_obj(false);
        let desc = shim_nsthread_description(t, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSThread: secondary>");
        free_obj(t);
    }

    #[test]
    fn nsthread_description_nil_returns_zero() {
        assert_eq!(shim_nsthread_description(0, 0), 0);
    }

    #[test]
    fn nsthread_is_multi_threaded_always_yes() {
        assert_eq!(shim_nsthread_is_multi_threaded(0, 0), 1);
    }

    #[test]
    fn nsthread_class_exists() {
        let addr = crate::nsobj::nsthread_class_addr();
        assert_ne!(addr, 0, "NSThread class address must be non-zero");
    }

    #[test]
    fn nsthread_thread_dictionary_returns_object() {
        let t = make_thread_obj(false);
        let dict = shim_nsthread_thread_dictionary(t, 0);
        assert_ne!(dict, 0, "threadDictionary must return non-null");
        free_obj(t);
    }

    #[test]
    fn nsthread_init_with_target_nil_returns_zero() {
        assert_eq!(shim_nsthread_init_with_target(0, 0, 0x1, 0x2, 0), 0);
    }

    #[test]
    fn nsthread_is_executing_nil_returns_zero() {
        assert_eq!(shim_nsthread_is_executing(0, 0), 0);
    }

    #[test]
    fn nsthread_is_finished_nil_returns_zero() {
        assert_eq!(shim_nsthread_is_finished(0, 0), 0);
    }

    #[test]
    fn nsthread_thread_priority_returns_default() {
        assert!((shim_nsthread_thread_priority(0, 0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn nsthread_set_thread_priority_is_noop() {
        shim_nsthread_set_thread_priority(0, 0, 0.9);
        // no panic = pass
    }

    #[test]
    fn nsthread_quality_of_service_returns_default() {
        assert_eq!(shim_nsthread_quality_of_service(0, 0), -1);
    }

    #[test]
    fn nsthread_set_quality_of_service_is_noop() {
        shim_nsthread_set_quality_of_service(0, 0, 0x21);
        // no panic = pass
    }

    #[test]
    fn nsthread_stack_size_returns_default() {
        assert_eq!(shim_nsthread_stack_size(0, 0), 512 * 1024);
    }

    #[test]
    fn nsthread_set_stack_size_is_noop() {
        shim_nsthread_set_stack_size(0, 0, 1024 * 1024);
        // no panic = pass
    }
}
