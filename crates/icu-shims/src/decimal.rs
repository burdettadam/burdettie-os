//! Real number formatting shims backed by `icu_decimal`.
//!
//! Replaces the `unum_*` stub returns with actual ICU4X decimal formatting.
//! The C-level `unum_*` API uses opaque `UNumberFormat*` handles; we map
//! these to an internal table of formatter configurations.
//!
//! Because ICU4X `DecimalFormatter` is not `Send`, we create formatters
//! on-demand per format call rather than storing them in a global table.

use std::collections::HashMap;
use std::sync::Mutex;

use icu_decimal::DecimalFormatter;
use icu_locale::locale;
use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

/// U_ZERO_ERROR
const U_ZERO_ERROR: i32 = 0;

// ── Handle table ───────────────────────────────────────────────

static NEXT_HANDLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// We store only the style, not the formatter itself (not Send).
struct FormatterConfig {
    /// ICU style (0=pattern, 1=decimal, 2=currency, 3=percent, etc.)
    style: i32,
}

static CONFIGS: Mutex<Option<HashMap<u64, FormatterConfig>>> = Mutex::new(None);

fn with_table<R>(f: impl FnOnce(&mut HashMap<u64, FormatterConfig>) -> R) -> R {
    let mut guard = CONFIGS.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    f(table)
}

// ── Registration ───────────────────────────────────────────────

pub fn install(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_unum_open",
            shim_unum_open as *const () as u64,
            "unum_open (icu4x → real formatter)"
        ),
        (
            "_unum_close",
            shim_unum_close as *const () as u64,
            "unum_close (icu4x)"
        ),
        (
            "_unum_formatInt64",
            shim_unum_format_int64 as *const () as u64,
            "unum_formatInt64 (icu4x → real)"
        ),
        (
            "_unum_formatDouble",
            shim_unum_format_double as *const () as u64,
            "unum_formatDouble (icu4x → real)"
        ),
        (
            "_unum_parseToNumber",
            shim_unum_parse_to_number as *const () as u64,
            "unum_parseToNumber (real parser)"
        ),
        (
            "_unum_parse",
            shim_unum_parse as *const () as u64,
            "unum_parse → int32_t"
        ),
        (
            "_unum_parseDouble",
            shim_unum_parse_double as *const () as u64,
            "unum_parseDouble → double"
        )
    );
}

/// Create a decimal formatter (thread-local, no Send required).
fn make_formatter() -> Option<DecimalFormatter> {
    DecimalFormatter::try_new(locale!("en").into(), Default::default()).ok()
}

// ── Shim implementations ──────────────────────────────────────

/// `unum_open(style, pattern, patternLen, locale, parseErr, status) → UNumberFormat*`
pub extern "C" fn shim_unum_open(
    style: i32,
    _pattern: u64,
    _pattern_len: i32,
    _locale: u64,
    _parse_err: u64,
    status: *mut i32,
) -> u64 {
    let handle = NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    with_table(|t| t.insert(handle, FormatterConfig { style }));

    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    handle
}

/// `unum_close(fmt)` — releases a number formatter handle.
pub extern "C" fn shim_unum_close(handle: u64) {
    with_table(|t| t.remove(&handle));
}

/// `unum_formatInt64(fmt, number, result, resultLength, pos, status) → int32_t`
pub extern "C" fn shim_unum_format_int64(
    _handle: u64,
    number: i64,
    result: *mut u16,
    result_len: i32,
    _pos: u64,
    status: *mut i32,
) -> i32 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }

    let text = if let Some(fmt) = make_formatter() {
        let dec = icu_decimal::input::Decimal::from(number);
        fmt.format(&dec).to_string()
    } else {
        format!("{number}")
    };

    write_utf16(&text, result, result_len)
}

/// `unum_formatDouble(fmt, number, result, resultLength, pos, status) → int32_t`
pub extern "C" fn shim_unum_format_double(
    handle: u64,
    number: f64,
    result: *mut u16,
    result_len: i32,
    _pos: u64,
    status: *mut i32,
) -> i32 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }

    let style = with_table(|t| t.get(&handle).map(|c| c.style).unwrap_or(1));

    let text = if let Some(fmt) = make_formatter() {
        // For percent style, multiply by 100
        let n = if style == 3 { number * 100.0 } else { number };
        let s = format!("{n}");
        if let Ok(dec) = s.parse::<icu_decimal::input::Decimal>() {
            let formatted = fmt.format(&dec).to_string();
            if style == 3 {
                format!("{formatted}%")
            } else {
                formatted
            }
        } else {
            s
        }
    } else {
        format!("{number}")
    };

    write_utf16(&text, result, result_len)
}

/// `unum_parseToNumber(fmt, text, textLen, parsePos, result, status)`
///
/// Parses UTF-16 text to a number and stores the result. Handles:
///   - integers: "12,345" → 12345
///   - decimals: "3.14" → 3.14
///   - percent: "50%" → 0.5
///   - negative: "-42" → -42
///   - group separators (stripped)
pub extern "C" fn shim_unum_parse_to_number(
    handle: u64,
    text: *const u16,
    text_len: i32,
    parse_pos: *mut i32,
    result: u64,
    status: *mut i32,
) {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }

    let s = read_utf16(text, text_len);
    let style = with_table(|t| t.get(&handle).map(|c| c.style).unwrap_or(1));
    let parsed = parse_number_string(&s, style);

    // Store the parsed value in the result handle (a pointer to f64).
    if result != 0 {
        unsafe {
            *(result as *mut f64) = parsed;
        }
    }

    if !parse_pos.is_null() {
        unsafe {
            *parse_pos = s.trim().len() as i32;
        }
    }
}

/// `unum_parse(fmt, text, textLen, parsePos, status) → int32_t`
///
/// Parses text to a 32-bit integer.
pub extern "C" fn shim_unum_parse(
    handle: u64,
    text: *const u16,
    text_len: i32,
    parse_pos: *mut i32,
    status: *mut i32,
) -> i32 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }

    let s = read_utf16(text, text_len);
    let style = with_table(|t| t.get(&handle).map(|c| c.style).unwrap_or(1));
    let parsed = parse_number_string(&s, style);

    if !parse_pos.is_null() {
        unsafe {
            *parse_pos = s.trim().len() as i32;
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    {
        parsed as i32
    }
}

/// `unum_parseDouble(fmt, text, textLen, parsePos, status) → double`
///
/// Parses text to a double.
pub extern "C" fn shim_unum_parse_double(
    handle: u64,
    text: *const u16,
    text_len: i32,
    parse_pos: *mut i32,
    status: *mut i32,
) -> f64 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }

    let s = read_utf16(text, text_len);
    let style = with_table(|t| t.get(&handle).map(|c| c.style).unwrap_or(1));
    let parsed = parse_number_string(&s, style);

    if !parse_pos.is_null() {
        unsafe {
            *parse_pos = s.trim().len() as i32;
        }
    }

    parsed
}

/// Read a UTF-16 buffer into a Rust String.
fn read_utf16(ptr: *const u16, len: i32) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let slice = if len < 0 {
        let mut n = 0usize;
        unsafe {
            while *ptr.add(n) != 0 {
                n += 1;
            }
        }
        unsafe { std::slice::from_raw_parts(ptr, n) }
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len as usize) }
    };

    String::from_utf16_lossy(slice)
}

/// Parse a numeric string, handling group separators, percent, and sign.
fn parse_number_string(s: &str, style: i32) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }

    // Check for percent suffix
    let (s, is_percent) = if let Some(number) = s.strip_suffix('%') {
        (number, true)
    } else {
        (s, style == 3) // percent style (3) implies division
    };

    // Strip group separators (comma, thin space, non-breaking space)
    let cleaned: String = s
        .chars()
        .filter(|c| *c != ',' && *c != '\u{00A0}' && *c != '\u{202F}' && *c != ' ')
        .collect();

    let value = cleaned.parse::<f64>().unwrap_or(0.0);

    if is_percent {
        value / 100.0
    } else {
        value
    }
}

// ── Helpers ────────────────────────────────────────────────────

fn write_utf16(s: &str, buf: *mut u16, buf_len: i32) -> i32 {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let copy_len = utf16.len().min(buf_len.max(0) as usize);

    if !buf.is_null() && copy_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), buf, copy_len);
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    {
        utf16.len() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unum_open_returns_handle() {
        let mut status = 0i32;
        let h = shim_unum_open(1, 0, 0, 0, 0, &mut status);
        assert_ne!(h, 0);
        assert_eq!(status, 0);
        shim_unum_close(h);
    }

    #[test]
    fn unum_format_int64_basic() {
        let mut status = 0i32;
        let h = shim_unum_open(1, 0, 0, 0, 0, &mut status);

        let mut buf = [0u16; 32];
        let len = shim_unum_format_int64(h, 12345, buf.as_mut_ptr(), 32, 0, &mut status);
        assert!(len > 0);
        assert_eq!(status, 0);

        let text = String::from_utf16_lossy(&buf[..len as usize]);
        assert!(
            text.contains("12") && text.contains("345"),
            "formatted 12345 should contain digits: {text}"
        );

        shim_unum_close(h);
    }

    #[test]
    fn unum_format_double_basic() {
        let mut status = 0i32;
        let h = shim_unum_open(1, 0, 0, 0, 0, &mut status);

        let mut buf = [0u16; 32];
        let len = shim_unum_format_double(h, 3.25, buf.as_mut_ptr(), 32, 0, &mut status);
        assert!(len > 0);

        let text = String::from_utf16_lossy(&buf[..len as usize]);
        assert!(
            text.contains("3"),
            "formatted 3.25 should contain '3': {text}"
        );

        shim_unum_close(h);
    }

    #[test]
    fn unum_close_invalid_handle() {
        shim_unum_close(9999);
    }

    // ── Number parsing tests ──────────────────────────────────

    #[test]
    fn parse_integer() {
        assert_eq!(parse_number_string("12345", 1), 12345.0);
    }

    #[test]
    fn parse_integer_with_groups() {
        assert_eq!(parse_number_string("12,345", 1), 12345.0);
    }

    #[test]
    fn parse_negative() {
        assert_eq!(parse_number_string("-42", 1), -42.0);
    }

    #[test]
    fn parse_decimal() {
        assert_eq!(parse_number_string("3.25", 1), 3.25);
    }

    #[test]
    fn parse_percent_suffix() {
        assert_eq!(parse_number_string("50%", 1), 0.5);
    }

    #[test]
    fn parse_percent_style() {
        // style 3 = percent: "50" → 0.5
        assert_eq!(parse_number_string("50", 3), 0.5);
    }

    #[test]
    fn parse_empty_string() {
        assert_eq!(parse_number_string("", 1), 0.0);
    }

    #[test]
    fn parse_large_number_with_groups() {
        assert_eq!(parse_number_string("1,234,567,890", 1), 1_234_567_890.0);
    }

    #[test]
    fn unum_parse_via_shim() {
        let mut status = 0i32;
        let h = shim_unum_open(1, 0, 0, 0, 0, &mut status);

        let input = "12,345";
        let utf16: Vec<u16> = input.encode_utf16().collect();
        let mut pos = 0i32;

        let val = shim_unum_parse(h, utf16.as_ptr(), utf16.len() as i32, &mut pos, &mut status);
        assert_eq!(status, 0);
        assert_eq!(val, 12345);

        shim_unum_close(h);
    }

    #[test]
    fn unum_parse_double_via_shim() {
        let mut status = 0i32;
        let h = shim_unum_open(1, 0, 0, 0, 0, &mut status);

        let input = "3.25";
        let utf16: Vec<u16> = input.encode_utf16().collect();
        let mut pos = 0i32;

        let val =
            shim_unum_parse_double(h, utf16.as_ptr(), utf16.len() as i32, &mut pos, &mut status);
        assert_eq!(status, 0);
        assert!((val - 3.25).abs() < 0.001);

        shim_unum_close(h);
    }

    #[test]
    fn unum_parse_to_number_stores_result() {
        let mut status = 0i32;
        let h = shim_unum_open(1, 0, 0, 0, 0, &mut status);

        let input = "42";
        let utf16: Vec<u16> = input.encode_utf16().collect();
        let mut pos = 0i32;
        let mut result: f64 = 0.0;

        shim_unum_parse_to_number(
            h,
            utf16.as_ptr(),
            utf16.len() as i32,
            &mut pos,
            &mut result as *mut f64 as u64,
            &mut status,
        );
        assert_eq!(status, 0);
        assert_eq!(result, 42.0);

        shim_unum_close(h);
    }
}
