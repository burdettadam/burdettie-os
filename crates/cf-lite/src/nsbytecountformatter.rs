//! NSByteCountFormatter shim: human-readable byte size formatting.
//!
//! Object layout (24 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: count_style  (u64 — 0=file, 1=memory, 2=decimal, 3=binary)
//!   offset 16: allowed_units(u64 — bitmask, 0=all)

use crate::cfstring;
use crate::nsobj;

const COUNT_STYLE_OFF: usize = 8;
const ALLOWED_UNITS_OFF: usize = 16;

fn alloc_formatter() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nsbytecountformatter_class_addr(); }
    obj
}

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn format_bytes(count: i64, is_binary: bool) -> String {
    let abs = (count as f64).abs();
    let (divisor, units): (f64, &[&str]) = if is_binary {
        (1024.0, &["bytes", "KiB", "MiB", "GiB", "TiB", "PiB"])
    } else {
        (1000.0, &["bytes", "KB", "MB", "GB", "TB", "PB"])
    };

    if abs < divisor {
        return format!("{count} bytes");
    }

    let mut val = abs;
    let mut idx = 0usize;
    while val >= divisor && idx + 1 < units.len() {
        val /= divisor;
        idx += 1;
    }
    if count < 0 { val = -val; }

    if val.fract().abs() < 0.05 {
        format!("{:.0} {}", val, units[idx])
    } else {
        format!("{:.1} {}", val, units[idx])
    }
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSByteCountFormatter stringFromByteCount:countStyle:]`
pub extern "C" fn shim_bytecountformatter_string_from(
    _cls: u64, _cmd: u64, byte_count: i64, count_style: u64,
) -> u64 {
    let is_binary = count_style == 3; // NSByteCountFormatterCountStyleBinary
    make_nsstring(&format_bytes(byte_count, is_binary))
}

/// `+[NSByteCountFormatter alloc]`
pub extern "C" fn shim_bytecountformatter_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_formatter()
}

// ── Instance methods ───────────────────────────────────────────

/// `-[NSByteCountFormatter init]`
pub extern "C" fn shim_bytecountformatter_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSByteCountFormatter stringFromByteCount:]`
pub extern "C" fn shim_bytecountformatter_string_from_instance(
    obj: u64, _cmd: u64, byte_count: i64,
) -> u64 {
    if obj == 0 { return 0; }
    let style = unsafe { *((obj as usize + COUNT_STYLE_OFF) as *const u64) };
    let is_binary = style == 3;
    make_nsstring(&format_bytes(byte_count, is_binary))
}

/// `-[NSByteCountFormatter setCountStyle:]`
pub extern "C" fn shim_bytecountformatter_set_count_style(
    obj: u64, _cmd: u64, style: u64,
) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + COUNT_STYLE_OFF) as *mut u64) = style; }
    0
}

/// `-[NSByteCountFormatter countStyle]`
pub extern "C" fn shim_bytecountformatter_count_style(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + COUNT_STYLE_OFF) as *const u64) }
}

/// `-[NSByteCountFormatter setAllowedUnits:]`
pub extern "C" fn shim_bytecountformatter_set_allowed_units(
    obj: u64, _cmd: u64, units: u64,
) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ALLOWED_UNITS_OFF) as *mut u64) = units; }
    0
}

/// `-[NSByteCountFormatter allowedUnits]`
pub extern "C" fn shim_bytecountformatter_allowed_units(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ALLOWED_UNITS_OFF) as *const u64) }
}

pub extern "C" fn shim_bytecountformatter_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_nsstring("<NSByteCountFormatter>")
}
