//! Apple os_log / os_signpost / os_activity stubs.
//!
//! Routes os_log output to the `log` crate (which maps to env_logger/stderr).
//! os_signpost and os_activity are no-ops since there's no Instruments
//! integration on Linux.

use std::sync::atomic::{AtomicU64, Ordering};

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

impl OsLogStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── Core os_log ─────────────────────────────────────────
        register_shims!(table,
            ("_os_log_create", shim_os_log_create as u64, "os_log_create (stub → sentinel handle)"),
            ("_os_log_type_enabled", shim_os_log_type_enabled as u64, "os_log_type_enabled (stub → true)"),
            ("__os_log_impl", shim_os_log_impl as u64, "_os_log_impl (stub → log::debug)"),
            ("__os_log_debug_impl", shim_os_log_impl as u64, "_os_log_debug_impl (stub → log::debug)"),
            ("__os_log_error_impl", shim_os_log_error_impl as u64, "_os_log_error_impl (stub → log::error)"),
            ("__os_log_fault_impl", shim_os_log_error_impl as u64, "_os_log_fault_impl (stub → log::error)"),
            ("__os_log_pack_send", shim_os_log_void_stub as u64, "_os_log_pack_send (stub → void)"),
            ("__os_log_is_debug_enabled", shim_os_log_true_stub as u64, "_os_log_is_debug_enabled (stub → 1)")
        );

        // ── os_log data symbols ─────────────────────────────────
        register_shims!(table,
            ("__os_log_default", &OS_LOG_DEFAULT_SENTINEL as *const u64 as u64, "_os_log_default data symbol"),
            ("___os_log_disabled", &OS_LOG_DISABLED_SENTINEL as *const u64 as u64, "__os_log_disabled data symbol")
        );

        // ── os_signpost ─────────────────────────────────────────
        register_shims!(table,
            ("_os_signpost_interval_begin", shim_os_log_void_stub as u64, "os_signpost_interval_begin (stub → void)"),
            ("_os_signpost_interval_end", shim_os_log_void_stub as u64, "os_signpost_interval_end (stub → void)"),
            ("_os_signpost_event_emit", shim_os_log_void_stub as u64, "os_signpost_event_emit (stub → void)"),
            ("_os_signpost_id_generate", shim_os_signpost_id_generate as u64, "os_signpost_id_generate (stub → counter)"),
            ("_os_signpost_id_make_with_pointer", shim_os_signpost_id_make_with_pointer as u64, "os_signpost_id_make_with_pointer (stub → ptr)")
        );

        // ── os_activity ─────────────────────────────────────────
        register_shims!(table,
            ("_os_activity_initiate", shim_os_activity_initiate as u64, "os_activity_initiate (stub → inline block)"),
            ("_os_activity_scope_enter", shim_os_log_void2_stub as u64, "os_activity_scope_enter (stub → void)"),
            ("_os_activity_scope_leave", shim_os_log_void1_stub as u64, "os_activity_scope_leave (stub → void)"),
            ("__os_activity_create", shim_os_activity_create as u64, "_os_activity_create (stub → sentinel)"),
            ("__os_activity_current", shim_os_activity_current as u64, "_os_activity_current (stub → sentinel)")
        );

        // ── Private utility used by Xcode tools ─────────────────
        register_shims!(table,
            ("_os_log_set_default_behavior", shim_os_log_void1_stub as u64, "os_log_set_default_behavior (stub → void)")
        );

        log::debug!("OsLogStubs: registered 22 os_log/signpost/activity stubs");
    }
}

// ── Core os_log ─────────────────────────────────────────────────

/// os_log_create(subsystem, category) → os_log_t
pub extern "C" fn shim_os_log_create(_subsystem: u64, _category: u64) -> u64 {
    &OS_LOG_DEFAULT_SENTINEL as *const u64 as u64
}

/// os_log_type_enabled(log, type) → bool
pub extern "C" fn shim_os_log_type_enabled(_log: u64, _type: u8) -> i32 {
    1 // always enabled
}

/// _os_log_impl(dso, log, type, format, buf, size)
/// The real implementation formats the log message from a packed buffer.
/// We extract the format string (a C string pointer) and emit it via the
/// `log` crate so that diagnostic text from the guest binary is visible.
pub extern "C" fn shim_os_log_impl(
    _dso: u64, _log: u64, _type: u8, format: u64, _buf: u64, _size: u32,
) {
    let msg = extract_format_string(format);
    log::debug!("os_log: {}", msg);
}

/// _os_log_error_impl — same as _os_log_impl but at error level.
pub extern "C" fn shim_os_log_error_impl(
    _dso: u64, _log: u64, _type: u8, format: u64, _buf: u64, _size: u32,
) {
    let msg = extract_format_string(format);
    log::error!("os_log: {}", msg);
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
    fn os_log_type_always_enabled() {
        assert_eq!(shim_os_log_type_enabled(0, 0), 1);
        assert_eq!(shim_os_log_type_enabled(0, 3), 1);
    }
}
