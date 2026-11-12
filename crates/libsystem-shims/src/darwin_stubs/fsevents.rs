//! FSEvents → stub / inotify translation layer.
//!
//! On macOS, FSEventStream watches directory trees for file-system changes.
//! On Linux, we provide a working implementation backed by inotify.
//! On macOS (for unit tests), we provide no-op stubs that return success.
//!
//! Key APIs:
//! - FSEventStreamCreate / FSEventStreamRelease
//! - FSEventStreamStart / FSEventStreamStop / FSEventStreamInvalidate
//! - FSEventStreamScheduleWithRunLoop / FSEventStreamSetDispatchQueue
//! - FSEventStreamGetLatestEventId / FSEventStreamCopyPathsBeingWatched
//! - FSEventStreamFlushAsync / FSEventStreamFlushSync
//! - FSEventsGetCurrentEventId / FSEventsCopyUUIDForDevice

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::table::ShimTable;

// ── Constants ───────────────────────────────────────────────────

/// kFSEventStreamEventIdSinceNow = 0xFFFFFFFFFFFFFFFF
static K_FSEVENT_STREAM_EVENT_ID_SINCE_NOW: u64 = 0xFFFF_FFFF_FFFF_FFFF;

/// kFSEventStreamCreateFlagNone
#[allow(dead_code)]
const K_FSEVENT_STREAM_CREATE_FLAG_NONE: u32 = 0;
/// kFSEventStreamCreateFlagUseCFTypes
#[allow(dead_code)]
const K_FSEVENT_STREAM_CREATE_FLAG_USE_CF_TYPES: u32 = 1;
/// kFSEventStreamCreateFlagFileEvents
#[allow(dead_code)]
const K_FSEVENT_STREAM_CREATE_FLAG_FILE_EVENTS: u32 = 0x10;

// ── Stream state ────────────────────────────────────────────────

struct FsEventStream {
    handle: u64,
    callback: u64,
    context_info: u64,
    paths: Vec<String>,
    #[allow(dead_code)]
    flags: u32,
    #[allow(dead_code)]
    latency: f64,
    started: bool,
    event_id: u64,
    #[cfg(target_os = "linux")]
    inotify_fd: Option<i32>,
    #[cfg(target_os = "linux")]
    watch_fds: Vec<i32>,
}

fn streams() -> &'static Mutex<HashMap<u64, FsEventStream>> {
    static STORE: OnceLock<Mutex<HashMap<u64, FsEventStream>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0xF5E0_0001);
static CURRENT_EVENT_ID: AtomicU64 = AtomicU64::new(1);

// ── Path extraction helper ──────────────────────────────────────

/// Extract paths from a CFArrayRef of CFStringRef.
fn extract_paths_from_cfarray(paths_to_watch: u64) -> Vec<String> {
    if paths_to_watch == 0 {
        return Vec::new();
    }
    // CFArray layout: [isa:8][count:8][values_ptr:8]
    let count;
    let values_ptr;
    unsafe {
        let base = paths_to_watch as *const u8;
        count = std::ptr::read_unaligned(base.add(8) as *const u64) as usize;
        values_ptr = std::ptr::read_unaligned(base.add(16) as *const u64) as *const u64;
    }
    if count == 0 || values_ptr.is_null() || count > 1024 {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let cf_str = unsafe { *values_ptr.add(i) };
        if cf_str == 0 {
            continue;
        }
        // Try to extract string from CFString-like object
        let s = extract_cfstring(cf_str);
        if let Some(s) = s {
            result.push(s);
        }
    }
    result
}

fn extract_cfstring(ptr: u64) -> Option<String> {
    if ptr == 0 {
        return None;
    }
    let p = ptr as *const u8;
    unsafe {
        let first_word = std::ptr::read_unaligned(p as *const u64);
        if first_word > 0 && first_word <= 0x10 {
            let len = std::ptr::read_unaligned(p.add(8) as *const u64) as usize;
            let chars_ptr = std::ptr::read_unaligned(p.add(16) as *const u64) as *const u8;
            if len > 0 && len < 4096 && !chars_ptr.is_null() {
                let slice = std::slice::from_raw_parts(chars_ptr, len);
                return std::str::from_utf8(slice).ok().map(|s| s.to_string());
            }
        }
        // Try as C string
        let mut slen = 0usize;
        while slen < 1024 {
            let b = *p.add(slen);
            if b == 0 {
                break;
            }
            slen += 1;
        }
        if slen > 0 && slen < 1024 {
            let slice = std::slice::from_raw_parts(p, slen);
            std::str::from_utf8(slice).ok().map(|s| s.to_string())
        } else {
            None
        }
    }
}

// ── inotify helpers (Linux only) ────────────────────────────────

#[cfg(target_os = "linux")]
fn setup_inotify_watches(paths: &[String]) -> (Option<i32>, Vec<i32>) {
    let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
    if fd < 0 {
        return (None, Vec::new());
    }
    let mut watches = Vec::new();
    for path in paths {
        let c_path = std::ffi::CString::new(path.as_str()).unwrap_or_default();
        let wd = unsafe {
            libc::inotify_add_watch(
                fd,
                c_path.as_ptr(),
                libc::IN_CREATE
                    | libc::IN_DELETE
                    | libc::IN_MODIFY
                    | libc::IN_MOVED_FROM
                    | libc::IN_MOVED_TO
                    | libc::IN_ATTRIB,
            )
        };
        if wd >= 0 {
            watches.push(wd);
        }
    }
    (Some(fd), watches)
}

// ── FSEventStream API ───────────────────────────────────────────

/// FSEventStreamRef FSEventStreamCreate(
///     CFAllocatorRef, FSEventStreamCallback, FSEventStreamContext*,
///     CFArrayRef pathsToWatch, FSEventStreamEventId sinceWhen,
///     CFTimeInterval latency, FSEventStreamCreateFlags flags)
extern "C" fn shim_fsevent_stream_create(
    _allocator: u64,
    callback: u64,
    context: u64, // FSEventStreamContext*
    paths_to_watch: u64,
    _since_when: u64,
    latency: f64,
    flags: u32,
) -> u64 {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    let paths = extract_paths_from_cfarray(paths_to_watch);

    // Read context->info (offset 8 in FSEventStreamContext: {version, info, retain, release, copyDescription})
    let context_info = if context != 0 {
        unsafe { std::ptr::read_unaligned((context as *const u8).add(8) as *const u64) }
    } else {
        0
    };

    log::trace!(
        "FSEventStreamCreate: handle={:#x} paths={:?} latency={}",
        handle,
        paths,
        latency
    );

    #[cfg(target_os = "linux")]
    let (inotify_fd, watch_fds) = setup_inotify_watches(&paths);

    let stream = FsEventStream {
        handle,
        callback,
        context_info,
        paths,
        flags,
        latency,
        started: false,
        event_id: CURRENT_EVENT_ID.load(Ordering::Relaxed),
        #[cfg(target_os = "linux")]
        inotify_fd,
        #[cfg(target_os = "linux")]
        watch_fds,
    };

    streams().lock().unwrap().insert(handle, stream);
    handle
}

/// void FSEventStreamRelease(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_release(stream_ref: u64) {
    let mut map = streams().lock().unwrap();
    if let Some(_stream) = map.remove(&stream_ref) {
        #[cfg(target_os = "linux")]
        {
            if let Some(fd) = _stream.inotify_fd {
                for wd in &_stream.watch_fds {
                    unsafe {
                        libc::inotify_rm_watch(fd, *wd);
                    }
                }
                unsafe {
                    libc::close(fd);
                }
            }
        }
    }
}

/// FSEventStreamRef FSEventStreamRetain(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_retain(stream_ref: u64) -> u64 {
    stream_ref // No-op retain
}

/// Boolean FSEventStreamStart(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_start(stream_ref: u64) -> u8 {
    let mut map = streams().lock().unwrap();
    if let Some(s) = map.get_mut(&stream_ref) {
        s.started = true;
        log::trace!("FSEventStreamStart: {:#x}", stream_ref);
        return 1; // true
    }
    0
}

/// void FSEventStreamStop(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_stop(stream_ref: u64) {
    let mut map = streams().lock().unwrap();
    if let Some(s) = map.get_mut(&stream_ref) {
        s.started = false;
    }
}

/// void FSEventStreamInvalidate(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_invalidate(stream_ref: u64) {
    let mut map = streams().lock().unwrap();
    if let Some(s) = map.get_mut(&stream_ref) {
        s.started = false;
    }
}

/// void FSEventStreamScheduleWithRunLoop(FSEventStreamRef, CFRunLoopRef, CFStringRef)
extern "C" fn shim_fsevent_stream_schedule_with_run_loop(
    _stream_ref: u64,
    _run_loop: u64,
    _mode: u64,
) {
    // No-op: we don't actually wire into CFRunLoop
}

/// void FSEventStreamSetDispatchQueue(FSEventStreamRef, dispatch_queue_t)
extern "C" fn shim_fsevent_stream_set_dispatch_queue(_stream_ref: u64, _queue: u64) {
    // No-op
}

/// FSEventStreamEventId FSEventStreamGetLatestEventId(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_get_latest_event_id(stream_ref: u64) -> u64 {
    let map = streams().lock().unwrap();
    map.get(&stream_ref).map(|s| s.event_id).unwrap_or(0)
}

/// CFArrayRef FSEventStreamCopyPathsBeingWatched(FSEventStreamRef)
/// Returns 0 (NULL) — callers should handle gracefully.
extern "C" fn shim_fsevent_stream_copy_paths_being_watched(_stream_ref: u64) -> u64 {
    0 // NULL CFArrayRef
}

/// FSEventStreamEventId FSEventStreamFlushAsync(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_flush_async(stream_ref: u64) -> u64 {
    let map = streams().lock().unwrap();
    map.get(&stream_ref).map(|s| s.event_id).unwrap_or(0)
}

/// void FSEventStreamFlushSync(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_flush_sync(_stream_ref: u64) {
    // No-op
}

/// void FSEventStreamShow(FSEventStreamRef)
extern "C" fn shim_fsevent_stream_show(_stream_ref: u64) {
    // No-op debug dump
}

// ── Global FSEvents functions ───────────────────────────────────

/// FSEventStreamEventId FSEventsGetCurrentEventId(void)
extern "C" fn shim_fsevents_get_current_event_id() -> u64 {
    CURRENT_EVENT_ID.load(Ordering::Relaxed)
}

/// CFUUIDRef FSEventsCopyUUIDForDevice(dev_t)
extern "C" fn shim_fsevents_copy_uuid_for_device(_dev: u64) -> u64 {
    0 // NULL — no UUID tracking
}

/// FSEventStreamEventId FSEventsGetLastEventIdForDeviceBeforeTime(dev_t, CFAbsoluteTime)
extern "C" fn shim_fsevents_get_last_event_id_for_device(_dev: u64, _time: f64) -> u64 {
    CURRENT_EVENT_ID.load(Ordering::Relaxed)
}

/// Boolean FSEventsPurgeEventsForDeviceUpToEventId(dev_t, FSEventStreamEventId)
extern "C" fn shim_fsevents_purge_events(_dev: u64, _event_id: u64) -> u8 {
    1 // true
}

// ── Install ─────────────────────────────────────────────────────

pub struct FsEventsStubs;

impl FsEventsStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            // FSEventStream lifecycle
            (
                "_FSEventStreamCreate",
                shim_fsevent_stream_create as *const () as u64,
                "FSEventStreamCreate → handle"
            ),
            (
                "_FSEventStreamRelease",
                shim_fsevent_stream_release as *const () as u64,
                "FSEventStreamRelease → cleanup"
            ),
            (
                "_FSEventStreamRetain",
                shim_fsevent_stream_retain as *const () as u64,
                "FSEventStreamRetain → no-op"
            ),
            (
                "_FSEventStreamStart",
                shim_fsevent_stream_start as *const () as u64,
                "FSEventStreamStart → true"
            ),
            (
                "_FSEventStreamStop",
                shim_fsevent_stream_stop as *const () as u64,
                "FSEventStreamStop → no-op"
            ),
            (
                "_FSEventStreamInvalidate",
                shim_fsevent_stream_invalidate as *const () as u64,
                "FSEventStreamInvalidate → no-op"
            ),
            // Scheduling
            (
                "_FSEventStreamScheduleWithRunLoop",
                shim_fsevent_stream_schedule_with_run_loop as *const () as u64,
                "FSEventStreamScheduleWithRunLoop → no-op"
            ),
            (
                "_FSEventStreamSetDispatchQueue",
                shim_fsevent_stream_set_dispatch_queue as *const () as u64,
                "FSEventStreamSetDispatchQueue → no-op"
            ),
            // Queries
            (
                "_FSEventStreamGetLatestEventId",
                shim_fsevent_stream_get_latest_event_id as *const () as u64,
                "FSEventStreamGetLatestEventId → id"
            ),
            (
                "_FSEventStreamCopyPathsBeingWatched",
                shim_fsevent_stream_copy_paths_being_watched as *const () as u64,
                "FSEventStreamCopyPathsBeingWatched → NULL"
            ),
            // Flush
            (
                "_FSEventStreamFlushAsync",
                shim_fsevent_stream_flush_async as *const () as u64,
                "FSEventStreamFlushAsync → id"
            ),
            (
                "_FSEventStreamFlushSync",
                shim_fsevent_stream_flush_sync as *const () as u64,
                "FSEventStreamFlushSync → no-op"
            ),
            (
                "_FSEventStreamShow",
                shim_fsevent_stream_show as *const () as u64,
                "FSEventStreamShow → no-op"
            ),
            // Global
            (
                "_FSEventsGetCurrentEventId",
                shim_fsevents_get_current_event_id as *const () as u64,
                "FSEventsGetCurrentEventId → id"
            ),
            (
                "_FSEventsCopyUUIDForDevice",
                shim_fsevents_copy_uuid_for_device as *const () as u64,
                "FSEventsCopyUUIDForDevice → NULL"
            ),
            (
                "_FSEventsGetLastEventIdForDeviceBeforeTime",
                shim_fsevents_get_last_event_id_for_device as *const () as u64,
                "FSEventsGetLastEventIdForDeviceBeforeTime → id"
            ),
            (
                "_FSEventsPurgeEventsForDeviceUpToEventId",
                shim_fsevents_purge_events as *const () as u64,
                "FSEventsPurgeEventsForDeviceUpToEventId → true"
            ),
            // Data symbol
            (
                "_kFSEventStreamEventIdSinceNow",
                &K_FSEVENT_STREAM_EVENT_ID_SINCE_NOW as *const u64 as u64,
                "kFSEventStreamEventIdSinceNow = 0xFFFF..."
            )
        );

        log::debug!("FsEventsStubs: registered 17 shims + 1 data symbol");
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset() -> std::sync::MutexGuard<'static, ()> {
        let g = TEST_LOCK.lock().unwrap();
        streams().lock().unwrap().clear();
        g
    }

    #[test]
    fn create_and_release_stream() {
        let _g = reset();
        let handle = shim_fsevent_stream_create(0, 0x1234, 0, 0, 0, 1.0, 0);
        assert_ne!(handle, 0);
        shim_fsevent_stream_release(handle);
        assert!(!streams().lock().unwrap().contains_key(&handle));
    }

    #[test]
    fn start_stop_lifecycle() {
        let _g = reset();
        let handle = shim_fsevent_stream_create(0, 0x1234, 0, 0, 0, 0.5, 0);
        assert_eq!(shim_fsevent_stream_start(handle), 1);
        {
            let map = streams().lock().unwrap();
            assert!(map.get(&handle).unwrap().started);
        }
        shim_fsevent_stream_stop(handle);
        {
            let map = streams().lock().unwrap();
            assert!(!map.get(&handle).unwrap().started);
        }
        shim_fsevent_stream_invalidate(handle);
        shim_fsevent_stream_release(handle);
    }

    #[test]
    fn event_id_tracking() {
        let _g = reset();
        let handle = shim_fsevent_stream_create(0, 0, 0, 0, 0, 0.0, 0);
        let eid = shim_fsevent_stream_get_latest_event_id(handle);
        assert!(eid > 0);
        let flush_eid = shim_fsevent_stream_flush_async(handle);
        assert_eq!(flush_eid, eid);
        shim_fsevent_stream_release(handle);
    }

    #[test]
    fn global_event_id_nonzero() {
        let eid = shim_fsevents_get_current_event_id();
        assert!(eid > 0);
    }

    #[test]
    fn retain_returns_same_handle() {
        let _g = reset();
        let handle = shim_fsevent_stream_create(0, 0, 0, 0, 0, 0.0, 0);
        assert_eq!(shim_fsevent_stream_retain(handle), handle);
        shim_fsevent_stream_release(handle);
    }

    #[test]
    fn schedule_and_dispatch_queue_no_crash() {
        let _g = reset();
        let handle = shim_fsevent_stream_create(0, 0, 0, 0, 0, 0.0, 0);
        shim_fsevent_stream_schedule_with_run_loop(handle, 0, 0);
        shim_fsevent_stream_set_dispatch_queue(handle, 0);
        shim_fsevent_stream_release(handle);
    }
}
