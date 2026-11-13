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
        register_shims!(
            table,
            (
                "_os_log_create",
                shim_os_log_create as *const () as u64,
                "os_log_create (stub → sentinel handle)"
            ),
            (
                "_os_log_type_enabled",
                shim_os_log_type_enabled as *const () as u64,
                "os_log_type_enabled (stub → true)"
            ),
            (
                "__os_log_impl",
                shim_os_log_impl as *const () as u64,
                "_os_log_impl (stub → log::debug)"
            ),
            (
                "__os_log_debug_impl",
                shim_os_log_impl as *const () as u64,
                "_os_log_debug_impl (stub → log::debug)"
            ),
            (
                "__os_log_error_impl",
                shim_os_log_error_impl as *const () as u64,
                "_os_log_error_impl (stub → log::error)"
            ),
            (
                "__os_log_fault_impl",
                shim_os_log_error_impl as *const () as u64,
                "_os_log_fault_impl (stub → log::error)"
            ),
            (
                "__os_log_pack_send",
                shim_os_log_void_stub as *const () as u64,
                "_os_log_pack_send (stub → void)"
            ),
            (
                "__os_log_is_debug_enabled",
                shim_os_log_true_stub as *const () as u64,
                "_os_log_is_debug_enabled (stub → 1)"
            )
        );

        // ── os_log data symbols ─────────────────────────────────
        register_shims!(
            table,
            (
                "__os_log_default",
                &OS_LOG_DEFAULT_SENTINEL as *const u64 as u64,
                "_os_log_default data symbol"
            ),
            (
                "___os_log_disabled",
                &OS_LOG_DISABLED_SENTINEL as *const u64 as u64,
                "__os_log_disabled data symbol"
            )
        );

        // ── os_signpost ─────────────────────────────────────────
        register_shims!(
            table,
            (
                "_os_signpost_interval_begin",
                shim_os_log_void_stub as *const () as u64,
                "os_signpost_interval_begin (stub → void)"
            ),
            (
                "_os_signpost_interval_end",
                shim_os_log_void_stub as *const () as u64,
                "os_signpost_interval_end (stub → void)"
            ),
            (
                "_os_signpost_event_emit",
                shim_os_log_void_stub as *const () as u64,
                "os_signpost_event_emit (stub → void)"
            ),
            (
                "_os_signpost_id_generate",
                shim_os_signpost_id_generate as *const () as u64,
                "os_signpost_id_generate (stub → counter)"
            ),
            (
                "_os_signpost_id_make_with_pointer",
                shim_os_signpost_id_make_with_pointer as *const () as u64,
                "os_signpost_id_make_with_pointer (stub → ptr)"
            )
        );

        // ── os_activity ─────────────────────────────────────────
        register_shims!(
            table,
            (
                "_os_activity_initiate",
                shim_os_activity_initiate as *const () as u64,
                "os_activity_initiate (stub → inline block)"
            ),
            (
                "_os_activity_scope_enter",
                shim_os_log_void2_stub as *const () as u64,
                "os_activity_scope_enter (stub → void)"
            ),
            (
                "_os_activity_scope_leave",
                shim_os_log_void1_stub as *const () as u64,
                "os_activity_scope_leave (stub → void)"
            ),
            (
                "__os_activity_create",
                shim_os_activity_create as *const () as u64,
                "_os_activity_create (stub → sentinel)"
            ),
            (
                "__os_activity_current",
                shim_os_activity_current as *const () as u64,
                "_os_activity_current (stub → sentinel)"
            )
        );

        // ── Private utility used by Xcode tools ─────────────────
        register_shims!(
            table,
            (
                "_os_log_set_default_behavior",
                shim_os_log_void1_stub as *const () as u64,
                "os_log_set_default_behavior (stub → void)"
            )
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
/// subsystem/category/level metadata.  Decodes the Apple binary argument
/// buffer to produce the actual formatted message.
pub extern "C" fn shim_os_log_impl(
    _dso: u64,
    log_obj: u64,
    log_type: u8,
    format: u64,
    buf: u64,
    size: u32,
) {
    let fmt = extract_format_string(format);
    let msg = if buf != 0 && size >= 2 {
        decode_os_log_message(fmt, buf, size)
    } else {
        format_os_log_string(fmt, &[])
    };
    let (subsystem, category) = lookup_log_handle(log_obj);
    let level = burdettie_log_pipeline::LogLevel::from_apple(log_type);

    burdettie_log_pipeline::emit(burdettie_log_pipeline::LogEvent {
        subsystem,
        category,
        level,
        message: msg,
        sequence: 0,
        timestamp_ms: 0,
    });
}

/// _os_log_error_impl — same as _os_log_impl but forces error level.
pub extern "C" fn shim_os_log_error_impl(
    _dso: u64,
    log_obj: u64,
    _type: u8,
    format: u64,
    buf: u64,
    size: u32,
) {
    let fmt = extract_format_string(format);
    let msg = if buf != 0 && size >= 2 {
        decode_os_log_message(fmt, buf, size)
    } else {
        format_os_log_string(fmt, &[])
    };
    let (subsystem, category) = lookup_log_handle(log_obj);

    burdettie_log_pipeline::emit(burdettie_log_pipeline::LogEvent {
        subsystem,
        category,
        level: burdettie_log_pipeline::LogLevel::Error,
        message: msg,
        sequence: 0,
        timestamp_ms: 0,
    });
}

// ── os_log binary argument buffer decoder ───────────────────────
//
// Apple's `_os_log_impl` receives `buf` and `size` containing pre-encoded
// argument values.  The compiler generates this buffer at the call site.
//
// Buffer layout:
//   [summary:1][num_args:1] then for each arg:
//   [type_descriptor:1][size:1][data:size bytes]
//
// Type descriptors (lower 4 bits):
//   0 = scalar (unsigned)
//   1 = scalar (signed)
//   2 = pointer to C string  (8 bytes on 64-bit)
//   3 = pointer to data blob (preceded by length)
//   4 = pointer to ObjC object
//
// Visibility bits (upper 4 bits of type_descriptor):
//   0x00 = default/private
//   0x02 = public
//   0x04 = sensitive
//
// Format specifiers in the format string use the pattern:
//   %{modifier}[flags][width][.precision]type
// where modifier can be `public`, `private`, `sensitive`, or domain-specific.

/// Decode an os_log binary argument buffer and format the message.
fn decode_os_log_message(fmt: &str, buf: u64, size: u32) -> String {
    let size = size as usize;
    if size < 2 {
        return format_os_log_string(fmt, &[]);
    }

    // Read the buffer header
    let buf_ptr = buf as *const u8;
    let _summary = unsafe { *buf_ptr };
    let num_args = unsafe { *buf_ptr.add(1) } as usize;

    // Extract argument values from the buffer
    let mut args: Vec<OsLogArg> = Vec::with_capacity(num_args);
    let mut offset = 2usize;

    for _ in 0..num_args {
        if offset + 2 > size {
            break;
        }
        let type_desc = unsafe { *buf_ptr.add(offset) };
        let arg_size = unsafe { *buf_ptr.add(offset + 1) } as usize;
        offset += 2;

        if offset + arg_size > size {
            break;
        }

        let data_slice = unsafe { std::slice::from_raw_parts(buf_ptr.add(offset), arg_size) };
        let arg_type = type_desc & 0x0F;

        let arg = match arg_type {
            0 | 1 => {
                // Scalar (unsigned or signed integer)
                let val = read_scalar(data_slice, arg_type == 1);
                OsLogArg::Scalar(val)
            }
            2 => {
                // Pointer to C string
                if arg_size >= 8 {
                    let ptr = u64::from_le_bytes(data_slice[..8].try_into().unwrap_or([0; 8]));
                    if ptr != 0 {
                        let s = extract_cstring(ptr);
                        OsLogArg::String(s.to_string())
                    } else {
                        OsLogArg::String("(null)".to_string())
                    }
                } else {
                    OsLogArg::String("(?)".to_string())
                }
            }
            3 => {
                // Data blob pointer (length-prefixed)
                OsLogArg::String(format!("<data:{} bytes>", arg_size))
            }
            4 => {
                // ObjC object pointer
                if arg_size >= 8 {
                    let ptr = u64::from_le_bytes(data_slice[..8].try_into().unwrap_or([0; 8]));
                    OsLogArg::String(format!("<obj:{ptr:#x}>"))
                } else {
                    OsLogArg::String("<obj>".to_string())
                }
            }
            _ => OsLogArg::String(format!("<type:{type_desc:#x}>")),
        };

        args.push(arg);
        offset += arg_size;
    }

    // Now format the string by replacing specifiers with decoded args
    format_os_log_string(fmt, &args)
}

#[derive(Debug)]
enum OsLogArg {
    Scalar(i64),
    String(String),
}

/// Read a scalar value from a little-endian byte slice.
fn read_scalar(data: &[u8], signed: bool) -> i64 {
    match data.len() {
        1 => {
            if signed {
                data[0] as i8 as i64
            } else {
                data[0] as i64
            }
        }
        2 => {
            let v = u16::from_le_bytes(data[..2].try_into().unwrap_or([0; 2]));
            if signed {
                v as i16 as i64
            } else {
                v as i64
            }
        }
        4 => {
            let v = u32::from_le_bytes(data[..4].try_into().unwrap_or([0; 4]));
            if signed {
                v as i32 as i64
            } else {
                v as i64
            }
        }
        8 => {
            let v = u64::from_le_bytes(data[..8].try_into().unwrap_or([0; 8]));
            v as i64
        }
        _ => 0,
    }
}

/// Format an os_log format string by substituting %specifiers with decoded args.
///
/// Handles Apple extensions like `%{public}s`, `%{private}d`, etc.
/// Strips the `{modifier}` part and uses the underlying printf specifier.
fn format_os_log_string(fmt: &str, args: &[OsLogArg]) -> String {
    let mut result = String::with_capacity(fmt.len() + 64);
    let mut arg_idx = 0;
    let bytes = fmt.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'%' {
            result.push(bytes[i] as char);
            i += 1;
            continue;
        }

        // Check for %%
        if i + 1 < len && bytes[i + 1] == b'%' {
            result.push('%');
            i += 2;
            continue;
        }

        i += 1; // skip '%'

        // Skip {modifier} if present (e.g., {public}, {private}, {signpost.description:begin})
        if i < len && bytes[i] == b'{' {
            while i < len && bytes[i] != b'}' {
                i += 1;
            }
            if i < len {
                i += 1; // skip '}'
            }
        }

        // Parse flags: -, +, 0, space, #
        while i < len && matches!(bytes[i], b'-' | b'+' | b'0' | b' ' | b'#') {
            i += 1;
        }

        // Parse width (digits or *)
        if i < len && bytes[i] == b'*' {
            // Width from argument
            if arg_idx < args.len() {
                arg_idx += 1; // consume the width arg
            }
            i += 1;
        } else {
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Parse precision  (.digits or .*)
        if i < len && bytes[i] == b'.' {
            i += 1;
            if i < len && bytes[i] == b'*' {
                if arg_idx < args.len() {
                    arg_idx += 1; // consume precision arg
                }
                i += 1;
            } else {
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
        }

        // Parse length modifier (l, ll, h, hh, z, t, q, L)
        while i < len && matches!(bytes[i], b'l' | b'h' | b'z' | b't' | b'q' | b'L' | b'j') {
            i += 1;
        }

        // Parse conversion specifier
        if i >= len {
            break;
        }

        let spec = bytes[i] as char;
        i += 1;

        let arg = if arg_idx < args.len() {
            let a = &args[arg_idx];
            arg_idx += 1;
            a
        } else {
            result.push_str("<?>");
            continue;
        };

        match spec {
            'd' | 'i' => match arg {
                OsLogArg::Scalar(v) => result.push_str(&v.to_string()),
                OsLogArg::String(s) => result.push_str(s),
            },
            'u' => match arg {
                OsLogArg::Scalar(v) => result.push_str(&(*v as u64).to_string()),
                OsLogArg::String(s) => result.push_str(s),
            },
            'x' => match arg {
                OsLogArg::Scalar(v) => result.push_str(&format!("{:x}", *v as u64)),
                OsLogArg::String(s) => result.push_str(s),
            },
            'X' => match arg {
                OsLogArg::Scalar(v) => result.push_str(&format!("{:X}", *v as u64)),
                OsLogArg::String(s) => result.push_str(s),
            },
            'o' => match arg {
                OsLogArg::Scalar(v) => result.push_str(&format!("{:o}", *v as u64)),
                OsLogArg::String(s) => result.push_str(s),
            },
            'p' => match arg {
                OsLogArg::Scalar(v) => result.push_str(&format!("0x{:x}", *v as u64)),
                OsLogArg::String(s) => result.push_str(s),
            },
            'f' | 'F' | 'e' | 'E' | 'g' | 'G' => match arg {
                OsLogArg::Scalar(v) => {
                    let fval = f64::from_bits(*v as u64);
                    result.push_str(&format!("{fval}"));
                }
                OsLogArg::String(s) => result.push_str(s),
            },
            's' => match arg {
                OsLogArg::String(s) => result.push_str(s),
                OsLogArg::Scalar(v) => {
                    // Might be a string pointer encoded as scalar
                    if *v != 0 {
                        let s = extract_cstring(*v as u64);
                        result.push_str(s);
                    } else {
                        result.push_str("(null)");
                    }
                }
            },
            'c' => match arg {
                OsLogArg::Scalar(v) => {
                    if let Some(c) = char::from_u32(*v as u32) {
                        result.push(c);
                    }
                }
                OsLogArg::String(s) => result.push_str(s),
            },
            '@' => {
                // ObjC object description — just render whatever we have
                match arg {
                    OsLogArg::String(s) => result.push_str(s),
                    OsLogArg::Scalar(v) => result.push_str(&format!("<obj:{v:#x}>")),
                }
            }
            _ => {
                // Unknown specifier — render raw
                match arg {
                    OsLogArg::Scalar(v) => result.push_str(&v.to_string()),
                    OsLogArg::String(s) => result.push_str(s),
                }
            }
        }
    }

    result
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
            if *ptr.add(len) == 0 {
                break;
            }
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
            if *p.add(len) == 0 {
                break;
            }
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
pub extern "C" fn shim_os_log_true_stub() -> i32 {
    1
}

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
            c"com.apple.test1".as_ptr() as u64,
            c"default".as_ptr() as u64,
        );
        let h2 = shim_os_log_create(
            c"com.apple.test2".as_ptr() as u64,
            c"network".as_ptr() as u64,
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
            c"com.apple.pipeline.test".as_ptr() as u64,
            c"unit".as_ptr() as u64,
        );
        shim_os_log_impl(
            0,
            handle,
            0x01, // OS_LOG_TYPE_INFO
            c"pipeline test message".as_ptr() as u64,
            0,
            0,
        );
        let after = burdettie_log_pipeline::event_count();
        assert!(after > before, "pipeline event count should increase");
    }

    #[test]
    fn os_log_type_always_enabled() {
        assert_eq!(shim_os_log_type_enabled(0, 0), 1);
        assert_eq!(shim_os_log_type_enabled(0, 3), 1);
    }

    // ── os_log argument buffer decoder tests ────────────────────

    #[test]
    fn decode_no_args() {
        let msg = decode_os_log_message("hello world", 0, 0);
        assert_eq!(msg, "hello world");
    }

    #[test]
    fn decode_scalar_int() {
        // Buffer: [summary:0][num_args:1][type:0x01(signed)][size:4][value:42_i32_le]
        let buf: Vec<u8> = vec![0x00, 0x01, 0x01, 0x04, 42, 0, 0, 0];
        let msg = decode_os_log_message("count=%d", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "count=42");
    }

    #[test]
    fn decode_unsigned_int() {
        // Buffer: [summary][num_args:1][type:0x00(unsigned)][size:4][value:255_u32_le]
        let buf: Vec<u8> = vec![0x00, 0x01, 0x00, 0x04, 255, 0, 0, 0];
        let msg = decode_os_log_message("val=%u", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "val=255");
    }

    #[test]
    fn decode_hex_format() {
        let buf: Vec<u8> = vec![0x00, 0x01, 0x00, 0x04, 0xAB, 0xCD, 0x00, 0x00];
        let msg = decode_os_log_message("addr=0x%x", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "addr=0xcdab");
    }

    #[test]
    fn decode_string_pointer() {
        let hello = b"hello\0";
        let ptr_bytes = (hello.as_ptr() as u64).to_le_bytes();
        let mut buf: Vec<u8> = vec![0x00, 0x01, 0x02, 0x08];
        buf.extend_from_slice(&ptr_bytes);
        let msg = decode_os_log_message("msg=%{public}s", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "msg=hello");
    }

    #[test]
    fn decode_null_string_pointer() {
        let ptr_bytes = 0u64.to_le_bytes();
        let mut buf: Vec<u8> = vec![0x00, 0x01, 0x02, 0x08];
        buf.extend_from_slice(&ptr_bytes);
        let msg = decode_os_log_message("msg=%s", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "msg=(null)");
    }

    #[test]
    fn decode_multiple_args() {
        // Two args: signed int 7, unsigned int 42
        let buf: Vec<u8> = vec![
            0x00, 0x02, // summary, 2 args
            0x01, 0x04, 7, 0, 0, 0, // signed i32 = 7
            0x00, 0x04, 42, 0, 0, 0, // unsigned u32 = 42
        ];
        let msg = decode_os_log_message("x=%d y=%u", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "x=7 y=42");
    }

    #[test]
    fn decode_mixed_string_and_int() {
        let name = b"world\0";
        let ptr_bytes = (name.as_ptr() as u64).to_le_bytes();
        let mut buf: Vec<u8> = vec![
            0x00, 0x02, // summary, 2 args
            0x02, 0x08, // string pointer, 8 bytes
        ];
        buf.extend_from_slice(&ptr_bytes);
        buf.extend_from_slice(&[0x01, 0x04, 99, 0, 0, 0]); // signed int = 99
        let msg = decode_os_log_message(
            "hello %{public}s code=%d",
            buf.as_ptr() as u64,
            buf.len() as u32,
        );
        assert_eq!(msg, "hello world code=99");
    }

    #[test]
    fn decode_percent_escape() {
        let msg = decode_os_log_message("100%% done", 0, 0);
        assert_eq!(msg, "100% done");
    }

    #[test]
    fn decode_negative_int() {
        // signed i32 = -1
        let buf: Vec<u8> = vec![0x00, 0x01, 0x01, 0x04, 0xFF, 0xFF, 0xFF, 0xFF];
        let msg = decode_os_log_message("err=%d", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "err=-1");
    }

    #[test]
    fn decode_i64_scalar() {
        // signed i64 = -42
        let val_bytes = (-42i64).to_le_bytes();
        let mut buf: Vec<u8> = vec![0x00, 0x01, 0x01, 0x08];
        buf.extend_from_slice(&val_bytes);
        let msg = decode_os_log_message("big=%lld", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "big=-42");
    }

    #[test]
    fn decode_pointer_format() {
        let buf: Vec<u8> = vec![
            0x00, 0x01, 0x00, 0x08, 0x78, 0x56, 0x34, 0x12, 0x00, 0x00, 0x00, 0x00,
        ];
        let msg = decode_os_log_message("ptr=%p", buf.as_ptr() as u64, buf.len() as u32);
        assert_eq!(msg, "ptr=0x12345678");
    }

    #[test]
    fn format_string_strips_apple_modifiers() {
        // Test that {public}, {private} etc. are stripped from the format
        let args = vec![OsLogArg::String("test".to_string())];
        let msg = format_os_log_string("val=%{public}s", &args);
        assert_eq!(msg, "val=test");

        let args2 = vec![OsLogArg::Scalar(42)];
        let msg2 = format_os_log_string("n=%{private}d", &args2);
        assert_eq!(msg2, "n=42");
    }
}
