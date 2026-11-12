//! NSISO8601DateFormatter shim: ISO 8601 date formatting.
//!
//! Object layout (24 bytes):
//!   offset  0: isa            (u64)
//!   offset  8: format_options (u64 — bitmask)
//!   offset 16: timezone_ptr   (u64 — NSTimeZone, 0 = UTC)

use crate::cfstring;
use crate::nsobj;

const OPTIONS_OFF: usize = 8;
const TZ_OFF: usize = 16;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn alloc_formatter() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsiso8601dateformatter_class_addr();
        // Default: full ISO 8601 options (year+month+day+time+timezone+colons+dashes)
        *((obj as usize + OPTIONS_OFF) as *mut u64) = 0xFFFF;
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSISO8601DateFormatter alloc]`
pub extern "C" fn shim_iso8601_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_formatter()
}

/// `-[NSISO8601DateFormatter init]`
pub extern "C" fn shim_iso8601_init(obj: u64, _cmd: u64) -> u64 {
    obj // already initialized via alloc
}

/// `+[NSISO8601DateFormatter stringFromDate:timeZone:formatOptions:]`
pub extern "C" fn shim_iso8601_class_string_from_date(
    _cls: u64,
    _cmd: u64,
    date: u64,
    _tz: u64,
    _options: u64,
) -> u64 {
    format_date_iso(date)
}

fn format_date_iso(date: u64) -> u64 {
    if date == 0 {
        return 0;
    }
    let ti = crate::nsdate::shim_nsdate_ti_since_ref(date, 0);
    // NSDate reference: 2001-01-01 00:00:00 UTC
    // Unix epoch offset: 978307200
    let unix = ti + 978_307_200.0;
    let secs = unix as i64;
    // Simple UTC formatting (no chrono dependency)
    let days_since_epoch = secs / 86400;
    let time_of_day = (secs % 86400 + 86400) % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm)
    let z = days_since_epoch + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if mo <= 2 { y + 1 } else { y };

    let iso = format!("{yr:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z");
    make_nsstring(&iso)
}

/// Parse an ISO 8601 string → NSDate.
fn parse_iso8601(s: &str) -> u64 {
    // Parse: YYYY-MM-DDTHH:MM:SSZ or YYYY-MM-DDTHH:MM:SS+00:00
    let clean = s.trim();
    let date_part;
    let time_part;

    if let Some(t_pos) = clean.find('T') {
        date_part = &clean[..t_pos];
        time_part = &clean[t_pos + 1..];
    } else if clean.len() == 10 {
        // Date only
        date_part = clean;
        time_part = "00:00:00Z";
    } else {
        return 0;
    }

    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return 0;
    }
    let Ok(yr) = parts[0].parse::<i64>() else {
        return 0;
    };
    let Ok(mo) = parts[1].parse::<u32>() else {
        return 0;
    };
    let Ok(d) = parts[2].parse::<u32>() else {
        return 0;
    };

    // Strip timezone suffix for time parsing
    let time_clean = time_part
        .trim_end_matches('Z')
        .split('+')
        .next()
        .unwrap_or("")
        .split('-')
        .next()
        .unwrap_or("");
    let tparts: Vec<&str> = time_clean.split(':').collect();
    let h: i64 = tparts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let m: i64 = tparts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let sec: i64 = tparts
        .get(2)
        .and_then(|s| s.split('.').next()?.parse().ok())
        .unwrap_or(0);

    // Convert to days since epoch (Howard Hinnant's algorithm, inverse)
    let y = if mo <= 2 { yr - 1 } else { yr };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let adjusted_mo = if mo > 2 { mo - 3 } else { mo + 9 } as u64;
    let doy = (153 * adjusted_mo + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe as i64 - 719468;

    let unix = days * 86400 + h * 3600 + m * 60 + sec;
    let reference = unix as f64 - 978_307_200.0;

    // Create NSDate with timeIntervalSinceReferenceDate
    crate::nsdate::shim_nsdate_date_with_ti_since_ref(0, 0, reference)
}

// ── Instance methods ───────────────────────────────────────────

/// `-[NSISO8601DateFormatter stringFromDate:]`
pub extern "C" fn shim_iso8601_string_from_date(_obj: u64, _cmd: u64, date: u64) -> u64 {
    format_date_iso(date)
}

/// `-[NSISO8601DateFormatter dateFromString:]`
pub extern "C" fn shim_iso8601_date_from_string(_obj: u64, _cmd: u64, string: u64) -> u64 {
    if string == 0 {
        return 0;
    }
    let Some(s) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };
    parse_iso8601(&s)
}

/// `-[NSISO8601DateFormatter setFormatOptions:]`
pub extern "C" fn shim_iso8601_set_format_options(obj: u64, _cmd: u64, options: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + OPTIONS_OFF) as *mut u64) = options;
    }
    0
}

/// `-[NSISO8601DateFormatter formatOptions]`
pub extern "C" fn shim_iso8601_format_options(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + OPTIONS_OFF) as *const u64) }
}

/// `-[NSISO8601DateFormatter setTimeZone:]`
pub extern "C" fn shim_iso8601_set_timezone(obj: u64, _cmd: u64, tz: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + TZ_OFF) as *mut u64) = tz;
    }
    0
}

/// `-[NSISO8601DateFormatter timeZone]`
pub extern "C" fn shim_iso8601_timezone(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + TZ_OFF) as *const u64) }
}

pub extern "C" fn shim_iso8601_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    make_nsstring("<NSISO8601DateFormatter>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn alloc_and_init() {
        let obj = shim_iso8601_alloc(0, 0);
        assert_ne!(obj, 0);
        let obj = shim_iso8601_init(obj, 0);
        assert_ne!(obj, 0);
        free_obj(obj);
    }

    #[test]
    fn format_options_roundtrip() {
        let obj = shim_iso8601_alloc(0, 0);
        shim_iso8601_set_format_options(obj, 0, 0x1F);
        assert_eq!(shim_iso8601_format_options(obj, 0), 0x1F);
        free_obj(obj);
    }

    #[test]
    fn string_from_date_cf_epoch() {
        // CF reference date (ti=0) → 2001-01-01T00:00:00Z
        let date = crate::nsdate::alloc_nsdate(0.0);
        let obj = shim_iso8601_alloc(0, 0);
        let s = shim_iso8601_string_from_date(obj, 0, date);
        assert_ne!(s, 0);
        let sv = read_string(s);
        assert!(
            sv.starts_with("2001-01-01"),
            "expected 2001-01-01 prefix in '{sv}'"
        );
        free_obj(date);
        free_obj(obj);
        free_obj(s);
    }

    #[test]
    fn date_from_string_roundtrip() {
        let date = crate::nsdate::alloc_nsdate(0.0);
        let obj = shim_iso8601_alloc(0, 0);
        let s = shim_iso8601_string_from_date(obj, 0, date);
        let date2 = shim_iso8601_date_from_string(obj, 0, s);
        assert_ne!(date2, 0);
        let ti = crate::nsdate::shim_nsdate_ti_since_ref(date2, 0);
        assert!(
            ti.abs() < 2.0,
            "expected ~0 CF time after round-trip, got {ti}"
        );
        free_obj(date);
        free_obj(date2);
        free_obj(obj);
        free_obj(s);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_iso8601_string_from_date(0, 0, 0), 0);
        assert_eq!(shim_iso8601_date_from_string(0, 0, 0), 0);
        assert_eq!(shim_iso8601_description(0, 0), 0);
        assert_eq!(shim_iso8601_format_options(0, 0), 0);
    }
}
