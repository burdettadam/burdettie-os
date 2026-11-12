//! Real date/time formatting shims backed by `icu_datetime`.
//!
//! Replaces the `udat_*` stub returns with actual ICU4X date-time formatting.
//! The C-level `udat_*` API uses opaque `UDateFormat*` handles; we map these
//! to an internal config table and create formatters on-demand (since ICU4X
//! `DateTimeFormatter` is not `Send`).

use std::collections::HashMap;
use std::sync::Mutex;

use icu_datetime::fieldsets;
use icu_datetime::DateTimeFormatter;
use icu_locale::locale;
use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

/// U_ZERO_ERROR
const U_ZERO_ERROR: i32 = 0;

// ── Handle table ───────────────────────────────────────────────

static NEXT_HANDLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// We store only the style config, not the formatter (not Send).
struct FormatterConfig {
    _time_style: i32,
    _date_style: i32,
}

static CONFIGS: Mutex<Option<HashMap<u64, FormatterConfig>>> = Mutex::new(None);

fn with_table<R>(f: impl FnOnce(&mut HashMap<u64, FormatterConfig>) -> R) -> R {
    let mut guard = CONFIGS.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    f(table)
}

// ── ICU style constants ────────────────────────────────────────

/// Map ICU `UDateFormatStyle` (0–4) to ICU4X length.
///
/// ICU C values:
///   UDAT_FULL    = 0
///   UDAT_LONG    = 1
///   UDAT_MEDIUM  = 2
///   UDAT_SHORT   = 3
///   UDAT_NONE    = -1
fn _style_from_icu(_style: i32) -> fieldsets::YMDT {
    fieldsets::YMDT::medium()
}

/// Create a date-time formatter on demand (not Send, can't store in static).
fn make_formatter() -> Option<DateTimeFormatter<fieldsets::YMDT>> {
    DateTimeFormatter::try_new(locale!("en").into(), fieldsets::YMDT::medium()).ok()
}

// ── Registration ───────────────────────────────────────────────

pub fn install(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_udat_open",
            shim_udat_open as *const () as u64,
            "udat_open (icu4x → real date formatter)"
        ),
        (
            "_udat_close",
            shim_udat_close as *const () as u64,
            "udat_close (icu4x)"
        ),
        (
            "_udat_format",
            shim_udat_format as *const () as u64,
            "udat_format (icu4x → real formatting)"
        ),
        (
            "_udat_parse",
            shim_udat_parse as *const () as u64,
            "udat_parse (ISO 8601 + ICU medium format parser)"
        )
    );
}

// ── Shim implementations ──────────────────────────────────────

/// `udat_open(timeStyle, dateStyle, locale, tzID, tzIDLen, pattern, patternLen, status)`
///
/// Creates a date formatter. We store the style config and create
/// the actual ICU4X formatter on demand per format call.
pub extern "C" fn shim_udat_open(
    time_style: i32,
    date_style: i32,
    _locale: u64,
    _tz_id: u64,
    _tz_id_len: i32,
    _pattern: u64,
    _pattern_len: i32,
    status: *mut i32,
) -> u64 {
    let handle = NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    with_table(|t| {
        t.insert(
            handle,
            FormatterConfig {
                _time_style: time_style,
                _date_style: date_style,
            },
        )
    });

    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    handle
}

/// `udat_close(fmt)` — releases a date formatter handle.
pub extern "C" fn shim_udat_close(handle: u64) {
    with_table(|t| t.remove(&handle));
}

/// `udat_format(fmt, dateToFormat, result, resultLength, position, status) → int32_t`
///
/// Formats a UDate (milliseconds since Unix epoch) into a UTF-16 buffer.
/// Returns the number of UChars written.
pub extern "C" fn shim_udat_format(
    handle: u64,
    date_ms: f64,
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

    // Convert milliseconds-since-epoch to a readable date string
    // using ICU4X's DateTimeFormatter (created on demand — not Send).
    let secs = (date_ms / 1000.0) as i64;

    // Build an ISO datetime from the timestamp
    let formatted = {
        // Check handle is valid
        let valid = with_table(|t| t.contains_key(&handle));
        if !valid {
            None
        } else if let Some(fmt) = make_formatter() {
            let total_days = secs / 86400;
            let day_secs = (secs % 86400 + 86400) % 86400;
            let hour = (day_secs / 3600) as u8;
            let minute = ((day_secs % 3600) / 60) as u8;
            let second = (day_secs % 60) as u8;

            let (year, month, day) = days_to_ymd(total_days);

            let Ok(date) = icu_datetime::input::Date::try_new_iso(year, month, day) else {
                return 0;
            };
            let Ok(time) = icu_datetime::input::Time::try_new(hour, minute, second, 0) else {
                return 0;
            };
            let dt = icu_datetime::input::DateTime { date, time };

            Some(fmt.format(&dt).to_string())
        } else {
            None
        }
    };

    let text = formatted.unwrap_or_default();
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let copy_len = utf16.len().min(result_len.max(0) as usize);

    if !result.is_null() && copy_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), result, copy_len);
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    {
        utf16.len() as i32
    }
}

/// `udat_parse(fmt, text, textLength, parsePos, status) → UDate`
///
/// Parses a date string (UTF-16) and returns milliseconds since Unix epoch.
/// Supports:
///   - ISO 8601: "2024-01-15T10:30:00" or "2024-01-15 10:30:00"
///   - Simple date: "2024-01-15"
///   - ICU medium format: "Jan 15, 2024, 10:30:00 AM" (as produced by `udat_format`)
///   - Numeric timestamp (milliseconds)
pub extern "C" fn shim_udat_parse(
    _handle: u64,
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
    if s.is_empty() {
        return 0.0;
    }

    let result = parse_date_string(&s);

    // Update parse position to end of string on success, 0 on failure.
    if !parse_pos.is_null() {
        unsafe {
            *parse_pos = if result != 0.0 { s.len() as i32 } else { 0 };
        }
    }

    result
}

/// Read a UTF-16 buffer into a Rust String.
fn read_utf16(ptr: *const u16, len: i32) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let slice = if len < 0 {
        // Null-terminated
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

/// Try to parse a date string into milliseconds since Unix epoch.
fn parse_date_string(s: &str) -> f64 {
    let s = s.trim();

    // Try numeric timestamp first (plain number = milliseconds since epoch).
    if let Ok(ms) = s.parse::<f64>() {
        return ms;
    }

    // Try ISO 8601: "YYYY-MM-DDTHH:MM:SS" or "YYYY-MM-DD HH:MM:SS"
    if let Some(ms) = parse_iso8601(s) {
        return ms;
    }

    // Try simple date: "YYYY-MM-DD"
    if let Some(ms) = parse_ymd(s) {
        return ms;
    }

    // Try ICU medium format: "Mon DD, YYYY, HH:MM:SS AM/PM" or "Mon DD, YYYY"
    if let Some(ms) = parse_icu_medium(s) {
        return ms;
    }

    0.0
}

/// Parse "YYYY-MM-DD" optionally followed by "T" or " " then "HH:MM:SS".
fn parse_iso8601(s: &str) -> Option<f64> {
    // At minimum "YYYY-MM-DD" = 10 chars
    if s.len() < 10 {
        return None;
    }

    let date_part = &s[..10];
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let (hour, minute, second) = if s.len() > 10 {
        let sep = s.as_bytes()[10];
        if sep == b'T' || sep == b' ' {
            parse_hms(&s[11..])?
        } else {
            (0, 0, 0)
        }
    } else {
        (0, 0, 0)
    };

    Some(ymd_hms_to_ms(year, month, day, hour, minute, second))
}

/// Parse "HH:MM:SS" from the start of s, returning (h, m, s).
fn parse_hms(s: &str) -> Option<(u32, u32, u32)> {
    // Take first 8 chars at most ("HH:MM:SS"); handle shorter forms
    let time_str = if s.len() >= 8 { &s[..8] } else { s };
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    let h: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let sec: u32 = if parts.len() >= 3 {
        // Handle fractional seconds by taking only integer part
        parts[2].split('.').next()?.parse().ok()?
    } else {
        0
    };
    if h > 23 || m > 59 || sec > 59 {
        return None;
    }
    Some((h, m, sec))
}

/// Parse simple "YYYY-MM-DD".
fn parse_ymd(s: &str) -> Option<f64> {
    if s.len() != 10 {
        return None;
    }
    parse_iso8601(s)
}

/// Month name abbreviations for ICU medium format parsing.
fn month_from_abbrev(s: &str) -> Option<u32> {
    match s {
        "Jan" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "May" => Some(5),
        "Jun" => Some(6),
        "Jul" => Some(7),
        "Aug" => Some(8),
        "Sep" => Some(9),
        "Oct" => Some(10),
        "Nov" => Some(11),
        "Dec" => Some(12),
        _ => None,
    }
}

/// Parse ICU medium format like "Jan 15, 2024, 10:30:00 AM" or "Jan 1, 2024".
fn parse_icu_medium(s: &str) -> Option<f64> {
    // Split on whitespace and commas for flexible parsing.
    let tokens: Vec<&str> = s
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.len() < 3 {
        return None;
    }

    // Find month abbreviation
    let (month, month_idx) = tokens
        .iter()
        .enumerate()
        .find_map(|(i, t)| month_from_abbrev(t).map(|m| (m, i)))?;

    // Day and year should be adjacent to month
    let day_str = tokens.get(month_idx + 1)?;
    let day: u32 = day_str.parse().ok()?;
    let year_str = tokens.get(month_idx + 2)?;
    let year: i32 = year_str.parse().ok()?;

    // Optional time component
    let (hour, minute, second) = if let Some(time_tok) = tokens.get(month_idx + 3) {
        if time_tok.contains(':') {
            let (h, m, sec) = parse_hms(time_tok)?;
            // Check for AM/PM
            let (h, _, _) = if let Some(ampm) = tokens.get(month_idx + 4) {
                match ampm.to_uppercase().as_str() {
                    "AM" => {
                        let h = if h == 12 { 0 } else { h };
                        (h, m, sec)
                    }
                    "PM" => {
                        let h = if h == 12 { 12 } else { h + 12 };
                        (h, m, sec)
                    }
                    _ => (h, m, sec),
                }
            } else {
                (h, m, sec)
            };
            (h, m, sec)
        } else {
            (0, 0, 0)
        }
    } else {
        (0, 0, 0)
    };

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    Some(ymd_hms_to_ms(year, month, day, hour, minute, second))
}

/// Convert year/month/day/hour/minute/second to milliseconds since Unix epoch (UTC).
fn ymd_hms_to_ms(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> f64 {
    // Reverse of days_to_ymd using the Hinnant algorithm.
    let y = if month <= 2 {
        year as i64 - 1
    } else {
        year as i64
    };
    let m = if month <= 2 {
        month as i64 + 9
    } else {
        month as i64 - 3
    };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * m as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe as i64 - 719_468;

    let total_secs = days * 86400 + hour as i64 * 3600 + min as i64 * 60 + sec as i64;
    total_secs as f64 * 1000.0
}

// ── Calendar helpers ───────────────────────────────────────────

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
/// Uses the proleptic Gregorian calendar civil algorithm.
fn days_to_ymd(days: i64) -> (i32, u8, u8) {
    // Algorithm from https://howardhinnant.github.io/date_algorithms.html
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    (y as i32, m as u8, d as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udat_open_returns_handle() {
        let mut status = 0i32;
        let h = shim_udat_open(2, 2, 0, 0, 0, 0, 0, &mut status);
        assert_ne!(h, 0);
        assert_eq!(status, 0);
        shim_udat_close(h);
    }

    #[test]
    fn udat_format_epoch() {
        let mut status = 0i32;
        let h = shim_udat_open(2, 2, 0, 0, 0, 0, 0, &mut status);
        assert_ne!(h, 0);

        let mut buf = [0u16; 64];
        let len = shim_udat_format(h, 0.0, buf.as_mut_ptr(), 64, 0, &mut status);
        assert!(len > 0, "formatted string should be non-empty");
        assert_eq!(status, 0);

        let text = String::from_utf16_lossy(&buf[..len as usize]);
        // Should contain "1970" (epoch year) and "Jan" or "January" or "1"
        assert!(
            text.contains("1970"),
            "formatted epoch should mention 1970: {text}"
        );

        shim_udat_close(h);
    }

    #[test]
    fn udat_close_invalid_handle() {
        // Should not panic
        shim_udat_close(9999);
    }

    #[test]
    fn days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known_date() {
        // 2024-01-01 = 19723 days after epoch
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
    }

    // ── Date parsing tests ─────────────────────────────────────

    #[test]
    fn parse_iso8601_full() {
        // 2024-01-15T10:30:00 UTC
        let ms = parse_date_string("2024-01-15T10:30:00");
        let expected = ymd_hms_to_ms(2024, 1, 15, 10, 30, 0);
        assert_eq!(ms, expected);
    }

    #[test]
    fn parse_iso8601_with_space() {
        let ms = parse_date_string("2024-01-15 10:30:00");
        let expected = ymd_hms_to_ms(2024, 1, 15, 10, 30, 0);
        assert_eq!(ms, expected);
    }

    #[test]
    fn parse_simple_date() {
        let ms = parse_date_string("2024-01-15");
        let expected = ymd_hms_to_ms(2024, 1, 15, 0, 0, 0);
        assert_eq!(ms, expected);
    }

    #[test]
    fn parse_epoch_date() {
        let ms = parse_date_string("1970-01-01");
        assert_eq!(ms, 0.0);
    }

    #[test]
    fn parse_numeric_timestamp() {
        let ms = parse_date_string("1705312200000");
        assert_eq!(ms, 1_705_312_200_000.0);
    }

    #[test]
    fn parse_icu_medium_format() {
        // "Jan 15, 2024" as produced by ICU medium date format
        let ms = parse_date_string("Jan 15, 2024");
        let expected = ymd_hms_to_ms(2024, 1, 15, 0, 0, 0);
        assert_eq!(ms, expected);
    }

    #[test]
    fn parse_icu_medium_with_time() {
        let ms = parse_date_string("Jan 15, 2024, 2:30:00 PM");
        let expected = ymd_hms_to_ms(2024, 1, 15, 14, 30, 0);
        assert_eq!(ms, expected);
    }

    #[test]
    fn parse_icu_medium_am() {
        let ms = parse_date_string("Dec 25, 2023, 9:00:00 AM");
        let expected = ymd_hms_to_ms(2023, 12, 25, 9, 0, 0);
        assert_eq!(ms, expected);
    }

    #[test]
    fn parse_empty_string_returns_zero() {
        assert_eq!(parse_date_string(""), 0.0);
    }

    #[test]
    fn parse_invalid_returns_zero() {
        assert_eq!(parse_date_string("not a date"), 0.0);
    }

    #[test]
    fn ymd_hms_roundtrip() {
        // Format then parse should produce the same ms
        let original_ms = 1_705_312_200_000.0; // some timestamp
        let secs = (original_ms / 1000.0) as i64;
        let total_days = secs / 86400;
        let day_secs = (secs % 86400 + 86400) % 86400;
        let (y, m, d) = days_to_ymd(total_days);
        let h = (day_secs / 3600) as u32;
        let min = ((day_secs % 3600) / 60) as u32;
        let sec = (day_secs % 60) as u32;
        let roundtrip = ymd_hms_to_ms(y, m as u32, d as u32, h, min, sec);
        assert_eq!(roundtrip, original_ms);
    }

    #[test]
    fn udat_parse_via_shim() {
        let mut status = 0i32;
        let h = shim_udat_open(2, 2, 0, 0, 0, 0, 0, &mut status);

        let input = "2024-01-15T10:30:00";
        let utf16: Vec<u16> = input.encode_utf16().collect();
        let mut pos = 0i32;

        let ms = shim_udat_parse(h, utf16.as_ptr(), utf16.len() as i32, &mut pos, &mut status);
        assert_eq!(status, 0);
        assert!(ms != 0.0, "parsed date should not be epoch");
        assert_eq!(pos, utf16.len() as i32);

        let expected = ymd_hms_to_ms(2024, 1, 15, 10, 30, 0);
        assert_eq!(ms, expected);

        shim_udat_close(h);
    }
}
