//! NSCalendar shim: calendar calculations using libc time functions.
//!
//! Object layout: 16 bytes
//!   [0..8):  ISA pointer
//!   [8..16): identifier_ptr (pointer to heap C string, e.g. "gregorian")
//!
//! Supports `+[NSCalendar currentCalendar]` singleton and basic
//! `components:fromDate:` / `dateFromComponents:` / `dateByAddingComponents:toDate:options:`.
//!
//! NSDateComponents layout: 64 bytes
//!   [0..8):   ISA pointer  (reuses NSObject class — lightweight struct)
//!   [8..16):  year (i64)
//!   [16..24): month (i64)
//!   [24..32): day (i64)
//!   [32..40): hour (i64)
//!   [40..48): minute (i64)
//!   [48..56): second (i64)
//!   [56..64): weekday (i64)

use std::ffi::c_char;
use std::sync::OnceLock;

use crate::cfstring;
use crate::nsdate;
use crate::nsobj;

const NSCALENDAR_SIZE: usize = 16;
const NSDATECOMPONENTS_SIZE: usize = 64;

/// CF absolute time epoch: 2001-01-01 00:00:00 UTC
const CF_EPOCH_UNIX: i64 = 978307200;

/// NSCalendarUnit bit flags (matching Apple's definitions).
#[allow(dead_code)]
mod unit {
    pub const ERA: u64 = 1 << 1;
    pub const YEAR: u64 = 1 << 2;
    pub const MONTH: u64 = 1 << 3;
    pub const DAY: u64 = 1 << 4;
    pub const HOUR: u64 = 1 << 5;
    pub const MINUTE: u64 = 1 << 6;
    pub const SECOND: u64 = 1 << 7;
    pub const WEEKDAY: u64 = 1 << 9;
}

static CURRENT_CALENDAR: OnceLock<u64> = OnceLock::new();

fn alloc_nscalendar(identifier: &str) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSCALENDAR_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    let c_id = std::ffi::CString::new(identifier).unwrap_or_default();
    let id_ptr = unsafe {
        let len = c_id.as_bytes_with_nul().len();
        let buf = crate::pal_alloc().malloc(len);
        if !buf.is_null() {
            std::ptr::copy_nonoverlapping(c_id.as_ptr() as *const u8, buf, len);
        }
        buf
    };
    unsafe {
        *obj = nsobj::nscalendar_class_addr();
        *obj.add(1) = id_ptr as u64;
    }
    obj as u64
}

fn alloc_date_components(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    weekday: i64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSDATECOMPONENTS_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    unsafe {
        *obj = nsobj::nsobject_class_addr(); // lightweight — uses NSObject ISA
        *obj.add(1) = year as u64;
        *obj.add(2) = month as u64;
        *obj.add(3) = day as u64;
        *obj.add(4) = hour as u64;
        *obj.add(5) = minute as u64;
        *obj.add(6) = second as u64;
        *obj.add(7) = weekday as u64;
    }
    obj as u64
}

/// Convert CF absolute time to calendar components.
fn cf_time_to_components(cf_time: f64) -> platform_core::clock::CalendarComponents {
    let unix_time = (cf_time as i64) + CF_EPOCH_UNIX;
    crate::platform().clock().localtime(unix_time)
}

/// Convert calendar components to CF absolute time.
fn components_to_cf_time(comps: &mut platform_core::clock::CalendarComponents) -> f64 {
    let unix = crate::platform().clock().mktime(comps);
    (unix - CF_EPOCH_UNIX) as f64
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSCalendar currentCalendar]` — returns the singleton Gregorian calendar.
pub extern "C" fn shim_nscalendar_current_calendar(_cls: u64, _cmd: u64) -> u64 {
    *CURRENT_CALENDAR.get_or_init(|| alloc_nscalendar("gregorian"))
}

/// `+[NSCalendar calendarWithIdentifier:]`
pub extern "C" fn shim_nscalendar_with_identifier(_cls: u64, _cmd: u64, identifier: u64) -> u64 {
    if identifier == 0 {
        return shim_nscalendar_current_calendar(_cls, _cmd);
    }
    let Some(id_str) = cfstring::extract_cfstring_cstr(identifier) else {
        return shim_nscalendar_current_calendar(_cls, _cmd);
    };
    alloc_nscalendar(id_str)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSCalendar calendarIdentifier]` — returns the identifier as NSString.
pub extern "C" fn shim_nscalendar_identifier(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const c_char;
    if id_ptr.is_null() {
        return 0;
    }
    cfstring::shim_cf_string_create_with_cstring(0, id_ptr, 0)
}

/// `-[NSCalendar components:fromDate:]` — extract date components from an NSDate.
///
/// `units` is a bitmask of NSCalendarUnit values.
/// `date` is an NSDate pointer (16-byte layout: isa + f64 time).
pub extern "C" fn shim_nscalendar_components_from_date(
    obj: u64,
    _cmd: u64,
    units: u64,
    date: u64,
) -> u64 {
    if obj == 0 || date == 0 {
        return 0;
    }
    // Read CF absolute time from NSDate (offset 8)
    let cf_time = unsafe {
        let bytes = *((date as *const u64).add(1));
        f64::from_bits(bytes)
    };
    let cc = cf_time_to_components(cf_time);

    let year = if units & unit::YEAR != 0 {
        (1900 + cc.year) as i64
    } else {
        i64::MIN
    };
    let month = if units & unit::MONTH != 0 {
        (1 + cc.mon) as i64
    } else {
        i64::MIN
    };
    let day = if units & unit::DAY != 0 {
        cc.mday as i64
    } else {
        i64::MIN
    };
    let hour = if units & unit::HOUR != 0 {
        cc.hour as i64
    } else {
        i64::MIN
    };
    let minute = if units & unit::MINUTE != 0 {
        cc.min as i64
    } else {
        i64::MIN
    };
    let second = if units & unit::SECOND != 0 {
        cc.sec as i64
    } else {
        i64::MIN
    };
    let weekday = if units & unit::WEEKDAY != 0 {
        // ObjC weekday: 1=Sunday..7=Saturday; wday: 0=Sunday..6=Saturday
        (cc.wday + 1) as i64
    } else {
        i64::MIN
    };

    alloc_date_components(year, month, day, hour, minute, second, weekday)
}

/// `-[NSCalendar component:fromDate:]` — extract a single component.
pub extern "C" fn shim_nscalendar_component_from_date(
    obj: u64,
    _cmd: u64,
    unit_val: u64,
    date: u64,
) -> i64 {
    if obj == 0 || date == 0 {
        return 0;
    }
    let cf_time = unsafe {
        let bytes = *((date as *const u64).add(1));
        f64::from_bits(bytes)
    };
    let cc = cf_time_to_components(cf_time);

    match unit_val {
        u if u == unit::YEAR => (1900 + cc.year) as i64,
        u if u == unit::MONTH => (1 + cc.mon) as i64,
        u if u == unit::DAY => cc.mday as i64,
        u if u == unit::HOUR => cc.hour as i64,
        u if u == unit::MINUTE => cc.min as i64,
        u if u == unit::SECOND => cc.sec as i64,
        u if u == unit::WEEKDAY => (cc.wday + 1) as i64,
        _ => 0,
    }
}

/// `-[NSCalendar dateFromComponents:]` — create an NSDate from date components.
pub extern "C" fn shim_nscalendar_date_from_components(obj: u64, _cmd: u64, comps: u64) -> u64 {
    if obj == 0 || comps == 0 {
        return 0;
    }
    let year = unsafe { *((comps as *const u64).add(1)) as i64 };
    let month = unsafe { *((comps as *const u64).add(2)) as i64 };
    let day = unsafe { *((comps as *const u64).add(3)) as i64 };
    let hour = unsafe { *((comps as *const u64).add(4)) as i64 };
    let minute = unsafe { *((comps as *const u64).add(5)) as i64 };
    let second = unsafe { *((comps as *const u64).add(6)) as i64 };

    let y = if year == i64::MIN { 2001 } else { year };
    let m = if month == i64::MIN { 1 } else { month };
    let d = if day == i64::MIN { 1 } else { day };
    let h = if hour == i64::MIN { 0 } else { hour };
    let mi = if minute == i64::MIN { 0 } else { minute };
    let s = if second == i64::MIN { 0 } else { second };

    let mut cc = platform_core::clock::CalendarComponents {
        year: (y - 1900) as i32,
        mon: (m - 1) as i32,
        mday: d as i32,
        hour: h as i32,
        min: mi as i32,
        sec: s as i32,
        isdst: -1,
        ..Default::default()
    };
    let cf_time = components_to_cf_time(&mut cc);
    nsdate::shim_nsdate_date_with_ti_since_ref(0, 0, cf_time)
}

/// `-[NSCalendar dateByAddingComponents:toDate:options:]` — add components to a date.
pub extern "C" fn shim_nscalendar_date_by_adding_components(
    obj: u64,
    _cmd: u64,
    comps: u64,
    date: u64,
    _options: u64,
) -> u64 {
    if obj == 0 || comps == 0 || date == 0 {
        return 0;
    }
    // Read base date
    let cf_time = unsafe {
        let bytes = *((date as *const u64).add(1));
        f64::from_bits(bytes)
    };
    let mut cc = cf_time_to_components(cf_time);

    // Read delta components
    let dy = unsafe { *((comps as *const u64).add(1)) as i64 };
    let dm = unsafe { *((comps as *const u64).add(2)) as i64 };
    let dd = unsafe { *((comps as *const u64).add(3)) as i64 };
    let dh = unsafe { *((comps as *const u64).add(4)) as i64 };
    let dmi = unsafe { *((comps as *const u64).add(5)) as i64 };
    let ds = unsafe { *((comps as *const u64).add(6)) as i64 };

    if dy != i64::MIN {
        cc.year += dy as i32;
    }
    if dm != i64::MIN {
        cc.mon += dm as i32;
    }
    if dd != i64::MIN {
        cc.mday += dd as i32;
    }
    if dh != i64::MIN {
        cc.hour += dh as i32;
    }
    if dmi != i64::MIN {
        cc.min += dmi as i32;
    }
    if ds != i64::MIN {
        cc.sec += ds as i32;
    }
    cc.isdst = -1;

    let new_cf_time = components_to_cf_time(&mut cc);
    nsdate::shim_nsdate_date_with_ti_since_ref(0, 0, new_cf_time)
}

/// `-[NSCalendar description]` — returns a description string.
pub extern "C" fn shim_nscalendar_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const c_char;
    let id_str = if !id_ptr.is_null() {
        let c = unsafe { std::ffi::CStr::from_ptr(id_ptr) };
        c.to_str().unwrap_or("?")
    } else {
        "?"
    };
    let desc = format!("<NSCalendar: {id_str}>");
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
