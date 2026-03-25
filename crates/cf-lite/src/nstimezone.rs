//! NSTimeZone shim: time zone singletons and basic queries.
//!
//! Object layout: 24 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  name_ptr (pointer to heap C string, e.g. "America/New_York")
//!   [16..24): seconds_from_gmt (i64)
//!
//! Singletons: systemTimeZone, localTimeZone (alias for system).

use std::sync::OnceLock;

use crate::cfstring;
use crate::nsobj;

const NSTIMEZONE_SIZE: usize = 24;

static SYSTEM_TIMEZONE: OnceLock<u64> = OnceLock::new();

/// Get the system timezone name and offset.
fn detect_system_timezone() -> (String, i64) {
    // Try TZ environment variable first
    if let Ok(tz) = std::env::var("TZ") {
        if !tz.is_empty() {
            return (tz, 0);
        }
    }

    let clock = crate::platform().clock();
    let offset = clock.utc_offset_sec() as i64;
    let name = clock.timezone_name();

    if name.is_empty() {
        let fallback = if offset == 0 { "UTC" } else { "GMT" };
        (fallback.to_string(), offset)
    } else {
        (name, offset)
    }
}

fn alloc_nstimezone(name: &str, offset: i64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSTIMEZONE_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    let c_name = std::ffi::CString::new(name).unwrap_or_default();
    let name_ptr = unsafe {
        let len = c_name.as_bytes_with_nul().len();
        let buf = crate::pal_alloc().malloc(len);
        if !buf.is_null() {
            std::ptr::copy_nonoverlapping(c_name.as_ptr() as *const u8, buf, len);
        }
        buf
    };
    unsafe {
        *obj = nsobj::nstimezone_class_addr();
        *obj.add(1) = name_ptr as u64;
        *obj.add(2) = offset as u64;
    }
    obj as u64
}

// ── Abbreviation → offset table ────────────────────────────────────

fn abbrev_to_offset(abbr: &str) -> i64 {
    match abbr {
        "UTC" | "GMT" => 0,
        "EST" => -18000,
        "EDT" => -14400,
        "CST" => -21600,
        "CDT" => -18000,
        "MST" => -25200,
        "MDT" => -21600,
        "PST" => -28800,
        "PDT" => -25200,
        "JST" => 32400,
        "CET" => 3600,
        "CEST" => 7200,
        "IST" => 19800,
        "AEST" => 36000,
        "AEDT" => 39600,
        _ => 0,
    }
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSTimeZone systemTimeZone]` — returns the singleton system timezone.
pub extern "C" fn shim_nstimezone_system_timezone(_cls: u64, _cmd: u64) -> u64 {
    *SYSTEM_TIMEZONE.get_or_init(|| {
        let (name, offset) = detect_system_timezone();
        alloc_nstimezone(&name, offset)
    })
}

/// `+[NSTimeZone localTimeZone]` — alias for systemTimeZone.
pub extern "C" fn shim_nstimezone_local_timezone(_cls: u64, _cmd: u64) -> u64 {
    shim_nstimezone_system_timezone(_cls, _cmd)
}

/// `+[NSTimeZone timeZoneWithName:]` — create/return a timezone by name.
pub extern "C" fn shim_nstimezone_with_name(_cls: u64, _cmd: u64, name: u64) -> u64 {
    if name == 0 {
        return 0;
    }
    let Some(name_str) = cfstring::extract_cfstring_cstr(name) else {
        return 0;
    };
    // Use the system offset if it matches, otherwise assume 0
    let offset = if name_str == "UTC" || name_str == "GMT" {
        0
    } else {
        // For known zones with abbreviations, try detecting
        abbrev_to_offset(name_str)
    };
    alloc_nstimezone(name_str, offset)
}

/// `+[NSTimeZone timeZoneWithAbbreviation:]` — create by abbreviation.
pub extern "C" fn shim_nstimezone_with_abbreviation(_cls: u64, _cmd: u64, abbr: u64) -> u64 {
    if abbr == 0 {
        return 0;
    }
    let Some(abbr_str) = cfstring::extract_cfstring_cstr(abbr) else {
        return 0;
    };
    let offset = abbrev_to_offset(abbr_str);
    alloc_nstimezone(abbr_str, offset)
}

/// `+[NSTimeZone timeZoneForSecondsFromGMT:]` — create by offset.
pub extern "C" fn shim_nstimezone_for_seconds_from_gmt(_cls: u64, _cmd: u64, seconds: i64) -> u64 {
    let name = if seconds == 0 {
        "GMT".to_string()
    } else {
        let h = seconds / 3600;
        let m = (seconds.abs() % 3600) / 60;
        if m == 0 {
            format!("GMT{h:+}")
        } else {
            format!("GMT{h:+}:{m:02}")
        }
    };
    alloc_nstimezone(&name, seconds)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSTimeZone name]` — returns the timezone name as NSString.
pub extern "C" fn shim_nstimezone_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let name_ptr = unsafe { *((obj as *const u64).add(1)) } as *const i8;
    if name_ptr.is_null() {
        return 0;
    }
    cfstring::shim_cf_string_create_with_cstring(0, name_ptr, 0)
}

/// `-[NSTimeZone abbreviation]` — returns the abbreviation; for our shim, same as name.
pub extern "C" fn shim_nstimezone_abbreviation(obj: u64, _cmd: u64) -> u64 {
    shim_nstimezone_name(obj, _cmd)
}

/// `-[NSTimeZone secondsFromGMT]` — returns the offset in seconds.
pub extern "C" fn shim_nstimezone_seconds_from_gmt(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(2)) as i64 }
}

/// `-[NSTimeZone description]` — returns description string.
pub extern "C" fn shim_nstimezone_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let name_ptr = unsafe { *((obj as *const u64).add(1)) } as *const i8;
    let name_str = if !name_ptr.is_null() {
        let c = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        c.to_str().unwrap_or("?")
    } else {
        "?"
    };
    let offset = unsafe { *((obj as *const u64).add(2)) as i64 };
    let desc = format!("{name_str} (GMT{:+})", offset / 3600);
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
