//! Apple os_log / os_signpost / os_activity stubs.
//!
//! Routes os_log output through the BurdettieOS structured log pipeline
//! (`burdettie-log-pipeline`) which captures subsystem, category, level, and
//! message.  Falls back to the `log` crate for signpost and activity stubs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::table::ShimTable;

pub struct OsLogStubs;

// ── Data sentinels ──────────────────────────────────────────────

/// Fake os_log_t handle for `os_log_default` / `OS_LOG_DEFAULT`.
/// Non-NULL so callers doing `if (log)` don't skip logging.
static OS_LOG_DEFAULT_SENTINEL: u64 = 0x4F53_4C47_0001;

/// Fake handle representing a disabled log.
static OS_LOG_DISABLED_SENTINEL: u64 = 0x4F53_4C47_0000;

/// Counter for os_signpost_id_generate
static SIGNPOST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Fake os_activity handle
static OS_ACTIVITY_SENTINEL: u64 = 0x4F53_4143_0001;

// ── Log handle registry ─────────────────────────────────────────
//
// os_log_create(subsystem, category) returns a unique handle.
// We track handle → (subsystem, category) so that _os_log_impl can
// include structured metadata in each event.

/// Counter for generating unique log handles.
static LOG_HANDLE_COUNTER: AtomicU64 = AtomicU64::new(0x4F53_4C47_1000);

/// Map from log handle → (subsystem, category).
static LOG_HANDLE_INFO: Mutex<Option<HashMap<u64, (String, String)>>> = Mutex::new(None);

fn register_log_handle(subsystem: &str, category: &str) -> u64 {
    let handle = LOG_HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut guard = LOG_HANDLE_INFO.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(handle, (subsystem.to_string(), category.to_string()));
    handle
}

fn lookup_log_handle(handle: u64) -> (String, String) {
    let guard = LOG_HANDLE_INFO.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|map| map.get(&handle).cloned())
        .unwrap_or_else(|| ("system".into(), "default".into()))
}

impl OsLogStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── Core os_log ─────────────────────────────────────────
        register_shims!(table,
            ("_os_log_create", shim_os_log_create as *const () as u64, "os_log_create (stub → sentinel handle)"),
            ("_os_log_type_enabled", shim_os_log_type_enabled as *const () as u64, "os_log_type_enabled (stub → true)"),
            ("__os_log_impl", shim_os_log_impl as *const () as u64, "_os_log_impl (stub → log::debug)"),
            ("__os_log_debug_impl", shim_os_log_impl as *const () as u64, "_os_log_debug_impl (stub → log::debug)"),
            ("__os_log_error_impl", shim_os_log_error_impl as *const () as u64, "_os_log_error_impl (stub → log::error)"),
            ("__os_log_fault_impl", shim_os_log_error_impl as *const () as u64, "_os_log_fault_impl (stub → log::error)"),
            ("__os_log_pack_send", shim_os_log_void_stub as *const () as u64, "_os_log_pack_send (stub → void)"),
            ("__os_log_is_debug_enabled", shim_os_log_true_stub as *const () as u64, "_os_log_is_debug_enabled (stub → 1)")
        );

        // ── os_log data symbols ─────────────────────────────────
        register_shims!(table,
            ("__os_log_default", &OS_LOG_DEFAULT_SENTINEL as *const u64 as u64, "_os_log_default data symbol"),
            ("___os_log_disabled", &OS_LOG_DISABLED_SENTINEL as *const u64 as u64, "__os_log_disabled data symbol")
        );

        // ── os_signpost ─────────────────────────────────────────
        register_shims!(table,
            ("_os_signpost_interval_begin", shim_os_log_void_stub as *const () as u64, "os_signpost_interval_begin (stub → void)"),
            ("_os_signpost_interval_end", shim_os_log_void_stub as *const () as u64, "os_signpost_interval_end (stub → void)"),
            ("_os_signpost_event_emit", shim_os_log_void_stub as *const () as u64, "os_signpost_event_emit (stub → void)"),
            ("_os_signpost_id_generate", shim_os_signpost_id_generate as *const () as u64, "os_signpost_id_generate (stub → counter)"),
            ("_os_signpost_id_make_with_pointer", shim_os_signpost_id_make_with_pointer as *const () as u64, "os_signpost_id_make_with_pointer (stub → ptr)")
        );

        // ── os_activity ─────────────────────────────────────────
        register_shims!(table,
            ("_os_activity_initiate", shim_os_activity_initiate as *const () as u64, "os_activity_initiate (stub → inline block)"),
            ("_os_activity_scope_enter", shim_os_log_void2_stub as *const () as u64, "os_activity_scope_enter (stub → void)"),
            ("_os_activity_scope_leave", shim_os_log_void1_stub as *const () as u64, "os_activity_scope_leave (stub → void)"),
            ("__os_activity_create", shim_os_activity_create as *const () as u64, "_os_activity_create (stub → sentinel)"),
            ("__os_activity_current", shim_os_activity_current as *const () as u64, "_os_activity_current (stub → sentinel)")
        );

        // ── Private utility used by Xcode tools ─────────────────
        register_shims!(table,
            ("_os_log_set_default_behavior", shim_os_log_void1_stub as *const () as u64, "os_log_set_default_behavior (stub → void)")
        );

        log::debug!("OsLogStubs: registered 22 os_log/signpost/activity stubs");
    }
}

// ── Core os_log ─────────────────────────────────────────────────

/// os_log_create(subsystem, category) → os_log_t
///
/// Returns a unique handle that tracks the subsystem+category pair.
pub extern "C" fn shim_os_log_create(subsystem: u64, category: u64) -> u64 {
    let sub_str = extract_cstring(subsystem);
    let cat_str = extract_cstring(category);
    let handle = register_log_handle(sub_str, cat_str);
    log::debug!("os_log_create({sub_str}, {cat_str}) → {handle:#x}");
    handle
}

/// os_log_type_enabled(log, type) → bool
pub extern "C" fn shim_os_log_type_enabled(_log: u64, _type: u8) -> i32 {
    1 // always enabled
}

/// _os_log_impl(dso, log, type, format, buf, size)
///
/// Routes through the BurdettieOS structured log pipeline with full
/// subsystem/category/level metadata.
pub extern "C" fn shim_os_log_impl(
    _dso: u64, log_obj: u64, log_type: u8, format: u64, _buf: u64, _size: u32,
) {
    let msg = extract_format_string(format);
    let (subsystem, category) = lookup_log_handle(log_obj);
    let level = burdettie_log_pipeline::LogLevel::from_apple(log_type);

    burdettie_log_pipeline::emit(burdettie_log_pipeline::LogEvent {
        subsystem,
        category,
        level,
        message: msg.to_string(),
        sequence: 0,     // filled by collector
        timestamp_ms: 0, // filled by collector
    });
}

/// _os_log_error_impl — same as _os_log_impl but forces error level.
pub extern "C" fn shim_os_log_error_impl(
    _dso: u64, log_obj: u64, _type: u8, format: u64, _buf: u64, _size: u32,
) {
    let msg = extract_format_string(format);
    let (subsystem, category) = lookup_log_handle(log_obj);

    burdettie_log_pipeline::emit(burdettie_log_pipeline::LogEvent {
        subsystem,
        category,
        level: burdettie_log_pipeline::LogLevel::Error,
        message: msg.to_string(),
        sequence: 0,
        timestamp_ms: 0,
    });
}

/// Extract the C format string from a `_os_log_impl` call.
/// The `format` argument is a pointer to a NUL-terminated C string.
/// We read it safely, capping at 512 bytes to avoid runaway reads.
fn extract_format_string(format: u64) -> &'static str {
    if format == 0 {
        return "(null)";
    }
    let ptr = format as *const u8;
    // Safety: format is a pointer into the guest's address space which
    // is mapped into our process.  We bound the read to 512 bytes.
    let mut len = 0usize;
    unsafe {
        while len < 512 {
            if *ptr.add(len) == 0 { break; }
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8(slice).unwrap_or("(invalid utf8)")
    }
}

/// Extract a C string from a pointer, returning a `&str`.
/// Capped at 256 bytes for subsystem/category strings.
fn extract_cstring(ptr: u64) -> &'static str {
    if ptr == 0 {
        return "";
    }
    let p = ptr as *const u8;
    let mut len = 0usize;
    unsafe {
        while len < 256 {
            if *p.add(len) == 0 { break; }
            len += 1;
        }
        let slice = std::slice::from_raw_parts(p, len);
        std::str::from_utf8(slice).unwrap_or("(invalid)")
    }
}

// ── Generic stubs ───────────────────────────────────────────────

pub extern "C" fn shim_os_log_void_stub() {}
pub extern "C" fn shim_os_log_void1_stub(_a: u64) {}
pub extern "C" fn shim_os_log_void2_stub(_a: u64, _b: u64) {}
pub extern "C" fn shim_os_log_true_stub() -> i32 { 1 }

// ── os_signpost ─────────────────────────────────────────────────

pub extern "C" fn shim_os_signpost_id_generate(_log: u64) -> u64 {
    SIGNPOST_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub extern "C" fn shim_os_signpost_id_make_with_pointer(_log: u64, ptr: u64) -> u64 {
    ptr
}

// ── os_activity ─────────────────────────────────────────────────

/// os_activity_initiate(description, flags, block)
/// Execute the block inline (activity tracking is a no-op).
pub extern "C" fn shim_os_activity_initiate(_desc: u64, _flags: u32, block: u64) {
    if block != 0 {
        // Block layout: isa(8) + flags(4) + reserved(4) + invoke(8)
        unsafe {
            let invoke_ptr = *((block + 16) as *const u64);
            if invoke_ptr != 0 {
                let invoke: extern "C" fn(u64) = std::mem::transmute(invoke_ptr);
                invoke(block);
            }
        }
    }
}

/// _os_activity_create(description, parent, flags) → os_activity_t
pub extern "C" fn shim_os_activity_create(_desc: u64, _parent: u64, _flags: u32) -> u64 {
    &OS_ACTIVITY_SENTINEL as *const u64 as u64
}

/// _os_activity_current() → os_activity_t
pub extern "C" fn shim_os_activity_current() -> u64 {
    &OS_ACTIVITY_SENTINEL as *const u64 as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::ShimTable;

    #[test]
    fn os_log_stubs_registered() {
        let mut table = ShimTable::new();
        OsLogStubs::install(&mut table);
        assert!(table.resolve("_os_log_create").is_some());
        assert!(table.resolve("__os_log_impl").is_some());
        assert!(table.resolve("__os_log_default").is_some());
        assert!(table.resolve("_os_signpost_id_generate").is_some());
        assert!(table.resolve("__os_activity_create").is_some());
    }

    #[test]
    fn os_log_default_is_non_null() {
        let addr = &OS_LOG_DEFAULT_SENTINEL as *const u64 as u64;
        assert_ne!(addr, 0);
    }

    #[test]
    fn signpost_ids_are_unique() {
        let id1 = shim_os_signpost_id_generate(0);
        let id2 = shim_os_signpost_id_generate(0);
        assert_ne!(id1, id2);
    }

    #[test]
    fn os_log_create_returns_unique_handles() {
        let h1 = shim_os_log_create(
            b"com.apple.test1\0".as_ptr() as u64,
            b"default\0".as_ptr() as u64,
        );
        let h2 = shim_os_log_create(
            b"com.apple.test2\0".as_ptr() as u64,
            b"network\0".as_ptr() as u64,
        );
        assert_ne!(h1, h2);

        // Verify the handle maps to the correct subsystem
        let (sub, cat) = lookup_log_handle(h1);
        assert_eq!(sub, "com.apple.test1");
        assert_eq!(cat, "default");
    }

    #[test]
    fn os_log_impl_emits_to_pipeline() {
        // Initialize the pipeline
        burdettie_log_pipeline::init(burdettie_log_pipeline::OutputFormat::Json);

        let before = burdettie_log_pipeline::event_count();
        let handle = shim_os_log_create(
            b"com.apple.pipeline.test\0".as_ptr() as u64,
            b"unit\0".as_ptr() as u64,
        );
        shim_os_log_impl(
            0, handle, 0x01, // OS_LOG_TYPE_INFO
            b"pipeline test message\0".as_ptr() as u64, 0, 0,
        );
        let after = burdettie_log_pipeline::event_count();
        assert!(after > before, "pipeline event count should increase");
    }

    #[test]
    fn os_log_type_always_enabled() {
        assert_eq!(shim_os_log_type_enabled(0, 0), 1);
        assert_eq!(shim_os_log_type_enabled(0, 3), 1);
    }
}
