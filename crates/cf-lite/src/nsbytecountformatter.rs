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
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsbytecountformatter_class_addr();
    }
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
    if count < 0 {
        val = -val;
    }

    if val.fract().abs() < 0.05 {
        format!("{:.0} {}", val, units[idx])
    } else {
        format!("{:.1} {}", val, units[idx])
    }
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSByteCountFormatter stringFromByteCount:countStyle:]`
pub extern "C" fn shim_bytecountformatter_string_from(
    _cls: u64,
    _cmd: u64,
    byte_count: i64,
    count_style: u64,
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
    obj: u64,
    _cmd: u64,
    byte_count: i64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let style = unsafe { *((obj as usize + COUNT_STYLE_OFF) as *const u64) };
    let is_binary = style == 3;
    make_nsstring(&format_bytes(byte_count, is_binary))
}

/// `-[NSByteCountFormatter setCountStyle:]`
pub extern "C" fn shim_bytecountformatter_set_count_style(obj: u64, _cmd: u64, style: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + COUNT_STYLE_OFF) as *mut u64) = style;
    }
    0
}

/// `-[NSByteCountFormatter countStyle]`
pub extern "C" fn shim_bytecountformatter_count_style(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + COUNT_STYLE_OFF) as *const u64) }
}

/// `-[NSByteCountFormatter setAllowedUnits:]`
pub extern "C" fn shim_bytecountformatter_set_allowed_units(
    obj: u64,
    _cmd: u64,
    units: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + ALLOWED_UNITS_OFF) as *mut u64) = units;
    }
    0
}

/// `-[NSByteCountFormatter allowedUnits]`
pub extern "C" fn shim_bytecountformatter_allowed_units(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + ALLOWED_UNITS_OFF) as *const u64) }
}

pub extern "C" fn shim_bytecountformatter_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    make_nsstring("<NSByteCountFormatter>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── Helper: read the string result and free it ──────────────

    fn read_and_free(s: u64) -> String {
        assert_ne!(s, 0, "expected non-nil string");
        let out = read_string(s);
        free_obj(s);
        out
    }

    // ── alloc / init ────────────────────────────────────────────

    #[test]
    fn alloc_returns_valid_object() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        assert_ne!(obj, 0);
        free_obj(obj);
    }

    #[test]
    fn init_returns_same_object() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        let obj2 = shim_bytecountformatter_init(obj, 0);
        assert_eq!(obj, obj2);
        free_obj(obj);
    }

    // ── Class method: stringFromByteCount:countStyle: ───────────

    #[test]
    fn class_string_from_zero_bytes() {
        let s = shim_bytecountformatter_string_from(0, 0, 0, 0);
        assert_eq!(read_and_free(s), "0 bytes");
    }

    #[test]
    fn class_string_from_small_bytes_decimal() {
        let s = shim_bytecountformatter_string_from(0, 0, 999, 0);
        assert_eq!(read_and_free(s), "999 bytes");
    }

    #[test]
    fn class_string_from_1kb_decimal() {
        // count_style 2 = decimal (uses 1000-based)
        let s = shim_bytecountformatter_string_from(0, 0, 1000, 2);
        assert_eq!(read_and_free(s), "1 KB");
    }

    #[test]
    fn class_string_from_1024_decimal() {
        // 1024 in decimal mode → 1.0 KB
        let s = shim_bytecountformatter_string_from(0, 0, 1024, 0);
        assert_eq!(read_and_free(s), "1 KB");
    }

    #[test]
    fn class_string_from_1024_binary() {
        // count_style 3 = binary (uses 1024-based)
        let s = shim_bytecountformatter_string_from(0, 0, 1024, 3);
        assert_eq!(read_and_free(s), "1 KiB");
    }

    #[test]
    fn class_string_from_1mb_decimal() {
        let s = shim_bytecountformatter_string_from(0, 0, 1_000_000, 0);
        assert_eq!(read_and_free(s), "1 MB");
    }

    #[test]
    fn class_string_from_1mib_binary() {
        let s = shim_bytecountformatter_string_from(0, 0, 1_048_576, 3);
        assert_eq!(read_and_free(s), "1 MiB");
    }

    #[test]
    fn class_string_from_1gb_decimal() {
        let s = shim_bytecountformatter_string_from(0, 0, 1_000_000_000, 0);
        assert_eq!(read_and_free(s), "1 GB");
    }

    #[test]
    fn class_string_from_1gib_binary() {
        let s = shim_bytecountformatter_string_from(0, 0, 1_073_741_824, 3);
        assert_eq!(read_and_free(s), "1 GiB");
    }

    #[test]
    fn class_string_from_fractional_value() {
        // 1536 bytes decimal: 1536/1000 = 1.536 → "1.5 KB"
        let s = shim_bytecountformatter_string_from(0, 0, 1536, 0);
        let text = read_and_free(s);
        assert!(text.contains("KB"), "expected KB in '{text}'");
    }

    #[test]
    fn class_string_from_negative_count() {
        let s = shim_bytecountformatter_string_from(0, 0, -1024, 0);
        let text = read_and_free(s);
        assert!(text.contains("KB"), "expected KB unit in '{text}'");
        assert!(text.contains("-"), "expected negative sign in '{text}'");
    }

    #[test]
    fn class_string_from_very_large_count() {
        // 1 TB decimal = 1_000_000_000_000
        let s = shim_bytecountformatter_string_from(0, 0, 1_000_000_000_000, 0);
        assert_eq!(read_and_free(s), "1 TB");
    }

    // ── Instance method: stringFromByteCount: ───────────────────

    #[test]
    fn instance_string_from_nil_returns_zero() {
        let r = shim_bytecountformatter_string_from_instance(0, 0, 1024);
        assert_eq!(r, 0);
    }

    #[test]
    fn instance_string_from_default_style() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        // default count_style is 0 (file/decimal)
        let s = shim_bytecountformatter_string_from_instance(obj, 0, 1_000_000);
        assert_eq!(read_and_free(s), "1 MB");
        free_obj(obj);
    }

    #[test]
    fn instance_string_from_binary_style() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        shim_bytecountformatter_set_count_style(obj, 0, 3); // binary
        let s = shim_bytecountformatter_string_from_instance(obj, 0, 1_048_576);
        assert_eq!(read_and_free(s), "1 MiB");
        free_obj(obj);
    }

    #[test]
    fn instance_string_from_zero() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        let s = shim_bytecountformatter_string_from_instance(obj, 0, 0);
        assert_eq!(read_and_free(s), "0 bytes");
        free_obj(obj);
    }

    // ── countStyle getter / setter ──────────────────────────────

    #[test]
    fn count_style_default_is_zero() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        assert_eq!(shim_bytecountformatter_count_style(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn count_style_set_and_get() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        shim_bytecountformatter_set_count_style(obj, 0, 3);
        assert_eq!(shim_bytecountformatter_count_style(obj, 0), 3);
        shim_bytecountformatter_set_count_style(obj, 0, 1);
        assert_eq!(shim_bytecountformatter_count_style(obj, 0), 1);
        free_obj(obj);
    }

    #[test]
    fn count_style_nil_safety() {
        assert_eq!(shim_bytecountformatter_count_style(0, 0), 0);
        assert_eq!(shim_bytecountformatter_set_count_style(0, 0, 3), 0);
    }

    // ── allowedUnits getter / setter ────────────────────────────

    #[test]
    fn allowed_units_default_is_zero() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        assert_eq!(shim_bytecountformatter_allowed_units(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn allowed_units_set_and_get() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        shim_bytecountformatter_set_allowed_units(obj, 0, 0xFF);
        assert_eq!(shim_bytecountformatter_allowed_units(obj, 0), 0xFF);
        free_obj(obj);
    }

    #[test]
    fn allowed_units_nil_safety() {
        assert_eq!(shim_bytecountformatter_allowed_units(0, 0), 0);
        assert_eq!(shim_bytecountformatter_set_allowed_units(0, 0, 0xFF), 0);
    }

    // ── description ─────────────────────────────────────────────

    #[test]
    fn description_returns_valid_string() {
        let obj = shim_bytecountformatter_alloc(0, 0);
        let s = shim_bytecountformatter_description(obj, 0);
        assert_eq!(read_and_free(s), "<NSByteCountFormatter>");
        free_obj(obj);
    }

    #[test]
    fn description_nil_returns_zero() {
        assert_eq!(shim_bytecountformatter_description(0, 0), 0);
    }

    // ── format_bytes internal logic ─────────────────────────────

    #[test]
    fn format_bytes_boundary_decimal() {
        assert_eq!(format_bytes(999, false), "999 bytes");
        assert_eq!(format_bytes(1000, false), "1 KB");
    }

    #[test]
    fn format_bytes_boundary_binary() {
        assert_eq!(format_bytes(1023, true), "1023 bytes");
        assert_eq!(format_bytes(1024, true), "1 KiB");
    }

    #[test]
    fn format_bytes_negative() {
        let s = format_bytes(-2048, false);
        assert!(s.contains("-"), "expected negative: {s}");
    }
}
