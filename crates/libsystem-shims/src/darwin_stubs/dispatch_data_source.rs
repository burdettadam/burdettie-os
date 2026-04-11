//! `dispatch_data` (immutable scatter-gather buffer) and `dispatch_source`
//! (event-driven monitoring) implementations.
//!
//! `dispatch_data` wraps an `Arc<Vec<u8>>` for each handle.
//! `dispatch_source` supports timer sources with a background firing thread;
//! non-timer source types are accepted but behave as no-ops on resume.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::table::ShimTable;

// ── Public helpers for dispatch_io ─────────────────────────────

/// Create a dispatch_data_t from a byte slice (used by dispatch_io.rs).
pub(crate) fn create_data_from_bytes(bytes: &[u8]) -> u64 {
    let handle = alloc_handle();
    data_registry()
        .lock()
        .unwrap()
        .insert(handle, Arc::new(bytes.to_vec()));
    handle
}

/// Get a copy of the bytes stored in a dispatch_data_t (used by dispatch_io.rs).
pub(crate) fn get_data_bytes(data: u64) -> Option<Vec<u8>> {
    let reg = data_registry().lock().unwrap();
    reg.get(&data).map(|v| v.as_ref().clone())
}

// ---------------------------------------------------------------------------
// Handle allocator
// ---------------------------------------------------------------------------

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0x00DE_0000_0001_0000);

fn alloc_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// dispatch_data registry
// ---------------------------------------------------------------------------

fn data_registry() -> &'static Mutex<HashMap<u64, Arc<Vec<u8>>>> {
    use std::sync::OnceLock;
    static R: OnceLock<Mutex<HashMap<u64, Arc<Vec<u8>>>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

// ---------------------------------------------------------------------------
// dispatch_data_empty sentinel
// ---------------------------------------------------------------------------

static DISPATCH_DATA_EMPTY: u64 = 0;

// ---------------------------------------------------------------------------
// dispatch_source internals
// ---------------------------------------------------------------------------

struct TimerConfig {
    _start: u64,
    interval: u64,
    _leeway: u64,
}

struct Source {
    source_type: u64,
    _handle: u64,
    _mask: u64,
    _queue: u64,
    event_handler: Mutex<Option<u64>>,
    cancel_handler: Mutex<Option<u64>>,
    timer: Mutex<Option<TimerConfig>>,
    cancelled: AtomicBool,
    suspended: AtomicBool,
    /// Accumulated data for DATA_ADD/DATA_OR sources.
    pending_data: AtomicU64,
}

fn source_registry() -> &'static Mutex<HashMap<u64, Arc<Source>>> {
    use std::sync::OnceLock;
    static R: OnceLock<Mutex<HashMap<u64, Arc<Source>>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

// ---------------------------------------------------------------------------
// Source type sentinels — unique addresses used as type identifiers
// ---------------------------------------------------------------------------

static SOURCE_TYPE_TIMER: u8 = 1;
static SOURCE_TYPE_READ: u8 = 2;
static SOURCE_TYPE_WRITE: u8 = 3;
static SOURCE_TYPE_SIGNAL: u8 = 4;
static SOURCE_TYPE_PROC: u8 = 5;
static SOURCE_TYPE_DATA_ADD: u8 = 6;
static SOURCE_TYPE_DATA_OR: u8 = 7;
static SOURCE_TYPE_VNODE: u8 = 8;
static SOURCE_TYPE_MACH_SEND: u8 = 9;
static SOURCE_TYPE_MACH_RECV: u8 = 10;
static SOURCE_TYPE_MEMORYPRESSURE: u8 = 11;
static SOURCE_TYPE_DATA_REPLACE: u8 = 12;

// ---------------------------------------------------------------------------
// Block invocation helper
// ---------------------------------------------------------------------------

/// Invoke a GCD block.  Block layout: `isa(8) + flags(4) + reserved(4) + invoke(8)`.
unsafe fn invoke_block(block: u64) {
    if block == 0 {
        return;
    }
    let block_ptr = block as *const u8;
    let invoke_ptr = unsafe { *(block_ptr.add(16) as *const usize) };
    let invoke: extern "C" fn(*const u8) = unsafe { std::mem::transmute(invoke_ptr) };
    invoke(block_ptr);
}

// ---------------------------------------------------------------------------
// dispatch_data shims
// ---------------------------------------------------------------------------

extern "C" fn shim_dispatch_data_create(
    buffer: u64,
    size: usize,
    _queue: u64,
    _destructor: u64,
) -> u64 {
    let bytes = if buffer == 0 || size == 0 {
        Vec::new()
    } else {
        let src = buffer as *const u8;
        // SAFETY: caller guarantees `buffer` points to at least `size` bytes.
        unsafe { std::slice::from_raw_parts(src, size) }.to_vec()
    };
    let handle = alloc_handle();
    data_registry()
        .lock()
        .unwrap()
        .insert(handle, Arc::new(bytes));
    log::trace!("dispatch_data_create({size} bytes) -> 0x{handle:x}");
    handle
}

extern "C" fn shim_dispatch_data_get_size(data: u64) -> usize {
    let reg = data_registry().lock().unwrap();
    match reg.get(&data) {
        Some(v) => v.len(),
        None => 0,
    }
}

extern "C" fn shim_dispatch_data_create_map(
    data: u64,
    buffer_out: u64,
    size_out: u64,
) -> u64 {
    let reg = data_registry().lock().unwrap();
    let arc = match reg.get(&data) {
        Some(v) => Arc::clone(v),
        None => {
            // Write nulls and return empty handle.
            if buffer_out != 0 {
                unsafe { *(buffer_out as *mut u64) = 0 };
            }
            if size_out != 0 {
                unsafe { *(size_out as *mut usize) = 0 };
            }
            return 0;
        }
    };
    drop(reg);

    // Write pointer and size to output parameters.
    if buffer_out != 0 {
        unsafe { *(buffer_out as *mut u64) = arc.as_ptr() as u64 };
    }
    if size_out != 0 {
        unsafe { *(size_out as *mut usize) = arc.len() };
    }

    // Register a new handle sharing the same data.
    let new_handle = alloc_handle();
    data_registry()
        .lock()
        .unwrap()
        .insert(new_handle, arc);
    new_handle
}

extern "C" fn shim_dispatch_data_apply(data: u64, applier: u64) -> u8 {
    if applier == 0 {
        return 0;
    }
    let reg = data_registry().lock().unwrap();
    let arc = match reg.get(&data) {
        Some(v) => Arc::clone(v),
        None => return 0,
    };
    drop(reg);

    // The applier block signature is:
    //   bool (^)(dispatch_data_t region, size_t offset, const void *buffer, size_t size)
    // We invoke it once with the entire contiguous buffer.
    // Block invoke at offset +16.
    let block_ptr = applier as *const u8;
    let invoke_ptr = unsafe { *(block_ptr.add(16) as *const usize) };
    let invoke: extern "C" fn(*const u8, u64, usize, u64, usize) -> u8 =
        unsafe { std::mem::transmute(invoke_ptr) };
    invoke(block_ptr, data, 0, arc.as_ptr() as u64, arc.len());
    1
}

extern "C" fn shim_dispatch_data_create_concat(data1: u64, data2: u64) -> u64 {
    let reg = data_registry().lock().unwrap();
    let v1 = reg.get(&data1).map(|v| v.as_ref().clone()).unwrap_or_default();
    let v2 = reg.get(&data2).map(|v| v.as_ref().clone()).unwrap_or_default();
    drop(reg);

    let mut combined = v1;
    combined.extend_from_slice(&v2);

    let handle = alloc_handle();
    data_registry()
        .lock()
        .unwrap()
        .insert(handle, Arc::new(combined));
    handle
}

extern "C" fn shim_dispatch_data_create_subrange(
    data: u64,
    offset: usize,
    length: usize,
) -> u64 {
    let reg = data_registry().lock().unwrap();
    let sub = match reg.get(&data) {
        Some(v) => {
            let end = (offset + length).min(v.len());
            let start = offset.min(v.len());
            v[start..end].to_vec()
        }
        None => Vec::new(),
    };
    drop(reg);

    let handle = alloc_handle();
    data_registry()
        .lock()
        .unwrap()
        .insert(handle, Arc::new(sub));
    handle
}

// ---------------------------------------------------------------------------
// dispatch_source shims
// ---------------------------------------------------------------------------

extern "C" fn shim_dispatch_source_create(
    source_type: u64,
    handle: u64,
    mask: u64,
    queue: u64,
) -> u64 {
    let src = Arc::new(Source {
        source_type,
        _handle: handle,
        _mask: mask,
        _queue: queue,
        event_handler: Mutex::new(None),
        cancel_handler: Mutex::new(None),
        timer: Mutex::new(None),
        cancelled: AtomicBool::new(false),
        suspended: AtomicBool::new(true), // sources start suspended
        pending_data: AtomicU64::new(0),
    });
    let h = alloc_handle();
    source_registry().lock().unwrap().insert(h, src);
    log::trace!("dispatch_source_create(type=0x{source_type:x}) -> 0x{h:x}");
    h
}

extern "C" fn shim_dispatch_source_set_event_handler(source: u64, handler: u64) {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&source) {
        *src.event_handler.lock().unwrap() = if handler != 0 { Some(handler) } else { None };
    }
}

extern "C" fn shim_dispatch_source_set_cancel_handler(source: u64, handler: u64) {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&source) {
        *src.cancel_handler.lock().unwrap() = if handler != 0 { Some(handler) } else { None };
    }
}

extern "C" fn shim_dispatch_source_set_timer(
    source: u64,
    start: u64,
    interval: u64,
    leeway: u64,
) {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&source) {
        *src.timer.lock().unwrap() = Some(TimerConfig {
            _start: start,
            interval,
            _leeway: leeway,
        });
    }
}

extern "C" fn shim_dispatch_source_cancel(source: u64) {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&source) {
        src.cancelled.store(true, Ordering::Release);
        let cancel_block = src.cancel_handler.lock().unwrap().take();
        if let Some(block) = cancel_block {
            // SAFETY: caller guaranteed valid block layout.
            unsafe { invoke_block(block) };
        }
    }
}

extern "C" fn shim_dispatch_resume(object: u64) {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&object) {
        src.suspended.store(false, Ordering::Release);

        let is_timer = src.source_type == &SOURCE_TYPE_TIMER as *const u8 as u64;
        let is_read = src.source_type == &SOURCE_TYPE_READ as *const u8 as u64;
        let is_write = src.source_type == &SOURCE_TYPE_WRITE as *const u8 as u64;
        let is_signal = src.source_type == &SOURCE_TYPE_SIGNAL as *const u8 as u64;

        // Timer sources: periodic background thread.
        if is_timer {
            let timer_interval = src
                .timer
                .lock()
                .unwrap()
                .as_ref()
                .map(|t| t.interval)
                .unwrap_or(0);

            if timer_interval > 0 && !src.cancelled.load(Ordering::Acquire) {
                let src_arc = Arc::clone(src);
                std::thread::spawn(move || {
                    let dur = std::time::Duration::from_nanos(timer_interval);
                    loop {
                        std::thread::sleep(dur);
                        if src_arc.cancelled.load(Ordering::Acquire)
                            || src_arc.suspended.load(Ordering::Acquire)
                        {
                            break;
                        }
                        if let Some(handler) = *src_arc.event_handler.lock().unwrap() {
                            unsafe { invoke_block(handler) };
                        }
                    }
                });
            }
        }

        // READ sources: poll the fd for readability.
        if is_read && !src.cancelled.load(Ordering::Acquire) {
            let src_arc = Arc::clone(src);
            let fd = src._handle as i32;
            std::thread::Builder::new()
                .name(format!("dispatch-source-read-{fd}"))
                .spawn(move || {
                    source_poll_fd(src_arc, fd, libc::POLLIN as i16);
                })
                .ok();
        }

        // WRITE sources: poll the fd for writability.
        if is_write && !src.cancelled.load(Ordering::Acquire) {
            let src_arc = Arc::clone(src);
            let fd = src._handle as i32;
            std::thread::Builder::new()
                .name(format!("dispatch-source-write-{fd}"))
                .spawn(move || {
                    source_poll_fd(src_arc, fd, libc::POLLOUT as i16);
                })
                .ok();
        }

        // SIGNAL sources: use signalfd on Linux, poll on macOS.
        if is_signal && !src.cancelled.load(Ordering::Acquire) {
            let src_arc = Arc::clone(src);
            let signum = src._handle as i32;
            std::thread::Builder::new()
                .name(format!("dispatch-source-signal-{signum}"))
                .spawn(move || {
                    source_wait_signal(src_arc, signum);
                })
                .ok();
        }
    }
}

/// Poll a file descriptor for the specified events (POLLIN or POLLOUT).
/// Fires the event handler each time the fd becomes ready.
fn source_poll_fd(src: Arc<Source>, fd: i32, events: i16) {
    loop {
        if src.cancelled.load(Ordering::Acquire) || src.suspended.load(Ordering::Acquire) {
            break;
        }

        let mut pfd = libc::pollfd {
            fd,
            events,
            revents: 0,
        };

        // Poll with 100ms timeout to allow cancellation checks.
        let ret = unsafe { libc::poll(&mut pfd, 1, 100) };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break; // fd went bad
        }

        if ret == 0 {
            continue; // timeout, check cancel again
        }

        if pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) as i16 != 0 {
            break;
        }

        if pfd.revents & events != 0 {
            // Store estimated readable/writable bytes.
            if events == libc::POLLIN as i16 {
                let mut available: i32 = 0;
                unsafe { libc::ioctl(fd, libc::FIONREAD, &mut available) };
                src.pending_data.store(available.max(0) as u64, Ordering::Release);
            }

            if let Some(handler) = *src.event_handler.lock().unwrap() {
                unsafe { invoke_block(handler) };
            }
        }
    }
}

/// Wait for a signal using sigtimedwait (avoids global signal handler conflicts).
fn source_wait_signal(src: Arc<Source>, signum: i32) {
    // Block the signal on this thread so sigtimedwait can catch it.
    #[cfg(target_os = "linux")]
    {
        unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, signum);
            libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());

            let timeout = libc::timespec {
                tv_sec: 0,
                tv_nsec: 100_000_000, // 100ms
            };

            loop {
                if src.cancelled.load(Ordering::Acquire) || src.suspended.load(Ordering::Acquire) {
                    break;
                }

                let ret = libc::sigtimedwait(&set, std::ptr::null_mut(), &timeout);
                if ret == signum {
                    src.pending_data.fetch_add(1, Ordering::Release);
                    if let Some(handler) = *src.event_handler.lock().unwrap() {
                        invoke_block(handler);
                    }
                }
                // ret == -1 with EAGAIN means timeout — loop and check cancellation
            }
        }
    }

    // On macOS, use kqueue-based signal monitoring (placeholder).
    #[cfg(not(target_os = "linux"))]
    {
        let _ = signum;
        while !src.cancelled.load(Ordering::Acquire) && !src.suspended.load(Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

extern "C" fn shim_dispatch_suspend(object: u64) {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&object) {
        src.suspended.store(true, Ordering::Release);
    }
}

extern "C" fn shim_dispatch_set_target_queue(_object: u64, _queue: u64) {
    // No-op — our implementation does not use real queues.
}

/// `size_t dispatch_source_get_data(dispatch_source_t source)`
/// Returns the accumulated data for data sources, readable bytes for READ, etc.
extern "C" fn shim_dispatch_source_get_data(source: u64) -> u64 {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&source) {
        // For DATA_ADD: return and atomically clear the pending data.
        let is_data_add = src.source_type == &SOURCE_TYPE_DATA_ADD as *const u8 as u64;
        let is_data_or = src.source_type == &SOURCE_TYPE_DATA_OR as *const u8 as u64;
        let is_data_replace = src.source_type == &SOURCE_TYPE_DATA_REPLACE as *const u8 as u64;
        if is_data_add || is_data_or || is_data_replace {
            return src.pending_data.swap(0, Ordering::AcqRel);
        }
        // For READ/WRITE/SIGNAL: return the current pending data.
        src.pending_data.load(Ordering::Acquire)
    } else {
        0
    }
}

/// `uintptr_t dispatch_source_get_handle(dispatch_source_t source)`
/// Returns the underlying handle (fd, signal number, pid, etc.).
extern "C" fn shim_dispatch_source_get_handle(source: u64) -> u64 {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&source) {
        src._handle
    } else {
        0
    }
}

/// `unsigned long dispatch_source_get_mask(dispatch_source_t source)`
extern "C" fn shim_dispatch_source_get_mask(source: u64) -> u64 {
    let reg = source_registry().lock().unwrap();
    if let Some(src) = reg.get(&source) {
        src._mask
    } else {
        0
    }
}

/// `void dispatch_source_merge_data(dispatch_source_t source, unsigned long value)`
/// For DATA_ADD, adds value to pending_data. For DATA_OR, ORs it in.
/// For DATA_REPLACE, replaces it. Then fires the event handler.
extern "C" fn shim_dispatch_source_merge_data(source: u64, value: u64) {
    let handler = {
        let reg = source_registry().lock().unwrap();
        if let Some(src) = reg.get(&source) {
            let is_data_add = src.source_type == &SOURCE_TYPE_DATA_ADD as *const u8 as u64;
            let is_data_or = src.source_type == &SOURCE_TYPE_DATA_OR as *const u8 as u64;
            let is_data_replace = src.source_type == &SOURCE_TYPE_DATA_REPLACE as *const u8 as u64;

            if is_data_add {
                src.pending_data.fetch_add(value, Ordering::AcqRel);
            } else if is_data_or {
                src.pending_data.fetch_or(value, Ordering::AcqRel);
            } else if is_data_replace {
                src.pending_data.store(value, Ordering::Release);
            }

            if !src.suspended.load(Ordering::Acquire) && !src.cancelled.load(Ordering::Acquire) {
                *src.event_handler.lock().unwrap()
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(handler) = handler {
        unsafe { invoke_block(handler) };
    }
}

/// `void dispatch_source_set_event_handler_f(dispatch_source_t source,
///     dispatch_function_t handler)`
extern "C" fn shim_dispatch_source_set_event_handler_f(_source: u64, _handler: u64) {
    // Function-pointer event handlers — store as a synthetic block.
    // For now, log and no-op since callers typically use block variant.
    log::trace!("dispatch_source_set_event_handler_f: stub");
}

/// `void dispatch_source_set_cancel_handler_f(dispatch_source_t source,
///     dispatch_function_t handler)`
extern "C" fn shim_dispatch_source_set_cancel_handler_f(_source: u64, _handler: u64) {
    log::trace!("dispatch_source_set_cancel_handler_f: stub");
}

/// `void dispatch_source_set_registration_handler(dispatch_source_t source,
///     dispatch_block_t handler)`
extern "C" fn shim_dispatch_source_set_registration_handler(_source: u64, _handler: u64) {
    // Registration handlers fire when a source is resumed for the first time.
    // For now, stub — fire inline if the handler is provided.
    if _handler != 0 {
        unsafe { invoke_block(_handler) };
    }
}

extern "C" fn shim_dispatch_release(object: u64) {
    // Try data registry first, then source registry.
    if data_registry().lock().unwrap().remove(&object).is_some() {
        return;
    }
    source_registry().lock().unwrap().remove(&object);
}

extern "C" fn shim_dispatch_retain(_object: u64) {
    // No-op — our registry holds the Arc.
}

// ---------------------------------------------------------------------------
// Installation
// ---------------------------------------------------------------------------

pub struct DispatchDataSourceStubs;

impl DispatchDataSourceStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            // dispatch_data
            ("_dispatch_data_create", shim_dispatch_data_create as *const () as u64, "dispatch_data_create (real)"),
            ("_dispatch_data_get_size", shim_dispatch_data_get_size as *const () as u64, "dispatch_data_get_size (real)"),
            ("_dispatch_data_create_map", shim_dispatch_data_create_map as *const () as u64, "dispatch_data_create_map (real)"),
            ("_dispatch_data_apply", shim_dispatch_data_apply as *const () as u64, "dispatch_data_apply (real)"),
            ("_dispatch_data_create_concat", shim_dispatch_data_create_concat as *const () as u64, "dispatch_data_create_concat (real)"),
            ("_dispatch_data_create_subrange", shim_dispatch_data_create_subrange as *const () as u64, "dispatch_data_create_subrange (real)"),
            ("__dispatch_data_empty", &DISPATCH_DATA_EMPTY as *const u64 as u64, "dispatch_data_empty data symbol"),
            // dispatch_source
            ("_dispatch_source_create", shim_dispatch_source_create as *const () as u64, "dispatch_source_create (real)"),
            ("_dispatch_source_set_event_handler", shim_dispatch_source_set_event_handler as *const () as u64, "dispatch_source_set_event_handler (real)"),
            ("_dispatch_source_set_cancel_handler", shim_dispatch_source_set_cancel_handler as *const () as u64, "dispatch_source_set_cancel_handler (real)"),
            ("_dispatch_source_set_timer", shim_dispatch_source_set_timer as *const () as u64, "dispatch_source_set_timer (real)"),
            ("_dispatch_source_cancel", shim_dispatch_source_cancel as *const () as u64, "dispatch_source_cancel (real)"),
            ("_dispatch_resume", shim_dispatch_resume as *const () as u64, "dispatch_resume (real)"),
            ("_dispatch_suspend", shim_dispatch_suspend as *const () as u64, "dispatch_suspend (real)"),
            ("_dispatch_set_target_queue", shim_dispatch_set_target_queue as *const () as u64, "dispatch_set_target_queue (stub → void)"),
            ("_dispatch_release", shim_dispatch_release as *const () as u64, "dispatch_release (real)"),
            ("_dispatch_retain", shim_dispatch_retain as *const () as u64, "dispatch_retain (stub → no-op)"),
            // dispatch_source data/handle/mask accessors
            ("_dispatch_source_get_data", shim_dispatch_source_get_data as *const () as u64, "dispatch_source_get_data (real)"),
            ("_dispatch_source_get_handle", shim_dispatch_source_get_handle as *const () as u64, "dispatch_source_get_handle (real)"),
            ("_dispatch_source_get_mask", shim_dispatch_source_get_mask as *const () as u64, "dispatch_source_get_mask (real)"),
            ("_dispatch_source_merge_data", shim_dispatch_source_merge_data as *const () as u64, "dispatch_source_merge_data (real)"),
            ("_dispatch_source_set_event_handler_f", shim_dispatch_source_set_event_handler_f as *const () as u64, "dispatch_source_set_event_handler_f (stub)"),
            ("_dispatch_source_set_cancel_handler_f", shim_dispatch_source_set_cancel_handler_f as *const () as u64, "dispatch_source_set_cancel_handler_f (stub)"),
            ("_dispatch_source_set_registration_handler", shim_dispatch_source_set_registration_handler as *const () as u64, "dispatch_source_set_registration_handler (stub)"),
            // dispatch_source type sentinels
            ("__dispatch_source_type_timer", &SOURCE_TYPE_TIMER as *const u8 as u64, "dispatch_source_type_timer sentinel"),
            ("__dispatch_source_type_read", &SOURCE_TYPE_READ as *const u8 as u64, "dispatch_source_type_read sentinel"),
            ("__dispatch_source_type_write", &SOURCE_TYPE_WRITE as *const u8 as u64, "dispatch_source_type_write sentinel"),
            ("__dispatch_source_type_signal", &SOURCE_TYPE_SIGNAL as *const u8 as u64, "dispatch_source_type_signal sentinel"),
            ("__dispatch_source_type_proc", &SOURCE_TYPE_PROC as *const u8 as u64, "dispatch_source_type_proc sentinel"),
            ("__dispatch_source_type_data_add", &SOURCE_TYPE_DATA_ADD as *const u8 as u64, "dispatch_source_type_data_add sentinel"),
            ("__dispatch_source_type_data_or", &SOURCE_TYPE_DATA_OR as *const u8 as u64, "dispatch_source_type_data_or sentinel"),
            ("__dispatch_source_type_data_replace", &SOURCE_TYPE_DATA_REPLACE as *const u8 as u64, "dispatch_source_type_data_replace sentinel"),
            ("__dispatch_source_type_vnode", &SOURCE_TYPE_VNODE as *const u8 as u64, "dispatch_source_type_vnode sentinel"),
            ("__dispatch_source_type_mach_send", &SOURCE_TYPE_MACH_SEND as *const u8 as u64, "dispatch_source_type_mach_send sentinel"),
            ("__dispatch_source_type_mach_recv", &SOURCE_TYPE_MACH_RECV as *const u8 as u64, "dispatch_source_type_mach_recv sentinel"),
            ("__dispatch_source_type_memorypressure", &SOURCE_TYPE_MEMORYPRESSURE as *const u8 as u64, "dispatch_source_type_memorypressure sentinel"),
        );
        log::debug!("DispatchDataSourceStubs: registered 36 stubs");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_create_and_get_size() {
        let buf: [u8; 5] = [1, 2, 3, 4, 5];
        let h = shim_dispatch_data_create(
            buf.as_ptr() as u64,
            buf.len(),
            0,
            0,
        );
        assert_ne!(h, 0);
        assert_eq!(shim_dispatch_data_get_size(h), 5);
    }

    #[test]
    fn test_data_empty_size_zero() {
        // Null buffer.
        let h1 = shim_dispatch_data_create(0, 0, 0, 0);
        assert_eq!(shim_dispatch_data_get_size(h1), 0);

        // Non-null buffer but zero size.
        let buf = [42u8];
        let h2 = shim_dispatch_data_create(buf.as_ptr() as u64, 0, 0, 0);
        assert_eq!(shim_dispatch_data_get_size(h2), 0);
    }

    #[test]
    fn test_data_create_map() {
        let buf: [u8; 3] = [10, 20, 30];
        let h = shim_dispatch_data_create(buf.as_ptr() as u64, buf.len(), 0, 0);

        let mut buffer_out: u64 = 0;
        let mut size_out: usize = 0;
        let mapped = shim_dispatch_data_create_map(
            h,
            &mut buffer_out as *mut u64 as u64,
            &mut size_out as *mut usize as u64,
        );
        assert_ne!(mapped, 0);
        assert_ne!(buffer_out, 0);
        assert_eq!(size_out, 3);
    }

    #[test]
    fn test_data_concat() {
        let a: [u8; 2] = [1, 2];
        let b: [u8; 3] = [3, 4, 5];
        let ha = shim_dispatch_data_create(a.as_ptr() as u64, a.len(), 0, 0);
        let hb = shim_dispatch_data_create(b.as_ptr() as u64, b.len(), 0, 0);
        let hc = shim_dispatch_data_create_concat(ha, hb);
        assert_eq!(shim_dispatch_data_get_size(hc), 5);
    }

    #[test]
    fn test_data_subrange() {
        let buf: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let h = shim_dispatch_data_create(buf.as_ptr() as u64, buf.len(), 0, 0);
        let sub = shim_dispatch_data_create_subrange(h, 3, 4);
        assert_eq!(shim_dispatch_data_get_size(sub), 4);
    }

    #[test]
    fn test_source_create() {
        let timer_type = &SOURCE_TYPE_TIMER as *const u8 as u64;
        let h = shim_dispatch_source_create(timer_type, 0, 0, 0);
        assert_ne!(h, 0);
    }

    #[test]
    fn test_source_timer_config() {
        let timer_type = &SOURCE_TYPE_TIMER as *const u8 as u64;
        let h = shim_dispatch_source_create(timer_type, 0, 0, 0);
        // Should not panic.
        shim_dispatch_source_set_timer(h, 0, 1_000_000_000, 0);
    }

    #[test]
    fn test_source_suspend_resume() {
        let read_type = &SOURCE_TYPE_READ as *const u8 as u64;
        let h = shim_dispatch_source_create(read_type, 0, 0, 0);
        // Source starts suspended; resume then suspend again.
        shim_dispatch_resume(h);
        shim_dispatch_suspend(h);
    }

    #[test]
    fn test_dispatch_release() {
        let buf: [u8; 4] = [0xAA; 4];
        let h = shim_dispatch_data_create(buf.as_ptr() as u64, buf.len(), 0, 0);
        assert_eq!(shim_dispatch_data_get_size(h), 4);
        shim_dispatch_release(h);
        // After release, size should be 0 (handle not found).
        assert_eq!(shim_dispatch_data_get_size(h), 0);
    }

    #[test]
    fn test_source_cancel() {
        let timer_type = &SOURCE_TYPE_TIMER as *const u8 as u64;
        let h = shim_dispatch_source_create(timer_type, 0, 0, 0);
        shim_dispatch_source_cancel(h);
        // Verify the source is in the registry and cancelled.
        let reg = source_registry().lock().unwrap();
        if let Some(src) = reg.get(&h) {
            assert!(src.cancelled.load(Ordering::Acquire));
        } else {
            panic!("source should still be in registry after cancel");
        }
    }
}
