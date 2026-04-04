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

        // For timer sources: if interval > 0 and not cancelled, spawn a
        // background thread that periodically fires the event handler.
        let timer_interval = src
            .timer
            .lock()
            .unwrap()
            .as_ref()
            .map(|t| t.interval)
            .unwrap_or(0);

        let is_timer = src.source_type == &SOURCE_TYPE_TIMER as *const u8 as u64;
        if is_timer && timer_interval > 0 && !src.cancelled.load(Ordering::Acquire) {
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
                        // SAFETY: caller guaranteed valid block layout.
                        unsafe { invoke_block(handler) };
                    }
                }
            });
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
            // dispatch_source type sentinels
            ("__dispatch_source_type_timer", &SOURCE_TYPE_TIMER as *const u8 as u64, "dispatch_source_type_timer sentinel"),
            ("__dispatch_source_type_read", &SOURCE_TYPE_READ as *const u8 as u64, "dispatch_source_type_read sentinel"),
            ("__dispatch_source_type_write", &SOURCE_TYPE_WRITE as *const u8 as u64, "dispatch_source_type_write sentinel"),
            ("__dispatch_source_type_signal", &SOURCE_TYPE_SIGNAL as *const u8 as u64, "dispatch_source_type_signal sentinel"),
            ("__dispatch_source_type_proc", &SOURCE_TYPE_PROC as *const u8 as u64, "dispatch_source_type_proc sentinel"),
        );
        log::debug!("DispatchDataSourceStubs: registered 22 stubs");
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
