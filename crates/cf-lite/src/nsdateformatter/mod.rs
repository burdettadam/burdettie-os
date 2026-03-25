//! NSDateFormatter shim: date-to-string and string-to-date conversion.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa         (u64 — pointer to NSDateFormatter class)
//!   offset  8: date_style  (u64 — NSDateFormatterStyle enum value)
//!   offset 16: time_style  (u64 — NSDateFormatterStyle enum value)
//!   offset 24: format_ptr  (u64 — pointer to NSString date format, or 0)
//!
//! Total: 32 bytes per formatter object.
//!
//! NSDateFormatterStyle values:
//!   0 = NSDateFormatterNoStyle
//!   1 = NSDateFormatterShortStyle     (e.g. "1/1/01" / "1:00 AM")
//!   2 = NSDateFormatterMediumStyle    (e.g. "Jan 1, 2001" / "1:00:00 AM")
//!   3 = NSDateFormatterLongStyle      (e.g. "January 1, 2001" / "1:00:00 AM UTC")
//!   4 = NSDateFormatterFullStyle      (e.g. "Monday, January 1, 2001" / ...)

mod datemath;
mod format;
mod parse;

pub use parse::shim_nsdateformatter_date_from_string;

use crate::cfstring;

const DATE_STYLE_OFF: usize = 8;
const TIME_STYLE_OFF: usize = 16;
const FORMAT_OFF: usize = 24;

/// Seconds between Unix epoch (1970) and CF reference date (2001).
const CF_EPOCH_OFFSET: f64 = 978_307_200.0;

const NSDATE_TI_OFF: usize = 8;

// ── Helpers ────────────────────────────────────────────────────────

fn read_date_style(obj: u64) -> u64 {
    unsafe { *((obj as usize + DATE_STYLE_OFF) as *const u64) }
}

fn read_time_style(obj: u64) -> u64 {
    unsafe { *((obj as usize + TIME_STYLE_OFF) as *const u64) }
}

fn read_format_ptr(obj: u64) -> u64 {
    unsafe { *((obj as usize + FORMAT_OFF) as *const u64) }
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSDateFormatter init]` — returns self with default styles (NoStyle).
pub extern "C" fn shim_nsdateformatter_init(obj: u64, _cmd: u64) -> u64 {
    // All offsets are already 0 from calloc (NoStyle = 0)
    obj
}

/// `-[NSDateFormatter setDateFormat:]`
pub extern "C" fn shim_nsdateformatter_set_date_format(obj: u64, _cmd: u64, fmt: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + FORMAT_OFF) as *mut u64) = fmt;
    }
}

/// `-[NSDateFormatter dateFormat]`
pub extern "C" fn shim_nsdateformatter_date_format(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_format_ptr(obj)
}

/// `-[NSDateFormatter setDateStyle:]`
pub extern "C" fn shim_nsdateformatter_set_date_style(obj: u64, _cmd: u64, style: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + DATE_STYLE_OFF) as *mut u64) = style;
    }
}

/// `-[NSDateFormatter dateStyle]`
pub extern "C" fn shim_nsdateformatter_date_style(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_date_style(obj)
}

/// `-[NSDateFormatter setTimeStyle:]`
pub extern "C" fn shim_nsdateformatter_set_time_style(obj: u64, _cmd: u64, style: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + TIME_STYLE_OFF) as *mut u64) = style;
    }
}

/// `-[NSDateFormatter timeStyle]`
pub extern "C" fn shim_nsdateformatter_time_style(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_time_style(obj)
}

/// `-[NSDateFormatter stringFromDate:]` — format an NSDate to a string.
pub extern "C" fn shim_nsdateformatter_string_from_date(obj: u64, _cmd: u64, date: u64) -> u64 {
    if obj == 0 || date == 0 {
        return 0;
    }

    let ti = unsafe { *((date as usize + NSDATE_TI_OFF) as *const f64) };
    let (year, month, day, h, m, s) = datemath::decompose(ti);

    let fmt_ptr = read_format_ptr(obj);
    let formatted = if fmt_ptr != 0 {
        if let Some(fmt_str) = cfstring::extract_cfstring_cstr(fmt_ptr) {
            format::apply_format(fmt_str, year, month, day, h, m, s)
        } else {
            String::new()
        }
    } else {
        format::format_with_styles(
            read_date_style(obj),
            read_time_style(obj),
            year,
            month,
            day,
            h,
            m,
            s,
        )
    };

    if formatted.is_empty() {
        return 0;
    }
    let cstr = std::ffi::CString::new(formatted).unwrap_or_default();
    cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}
