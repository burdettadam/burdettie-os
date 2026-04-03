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

/// `-[NSDateFormatter locale]` — returns current locale.
pub extern "C" fn shim_nsdateformatter_locale(_obj: u64, _cmd: u64) -> u64 {
    if _obj == 0 {
        return 0;
    }
    crate::nslocale::shim_nslocale_current_locale(0, 0)
}

/// `-[NSDateFormatter setLocale:]` — no-op.
pub extern "C" fn shim_nsdateformatter_set_locale(_obj: u64, _cmd: u64, _locale: u64) {
    // no-op
}

/// `-[NSDateFormatter calendar]` — returns current calendar.
pub extern "C" fn shim_nsdateformatter_calendar(_obj: u64, _cmd: u64) -> u64 {
    if _obj == 0 {
        return 0;
    }
    crate::nscalendar::shim_nscalendar_current_calendar(0, 0)
}

/// `-[NSDateFormatter setCalendar:]` — no-op.
pub extern "C" fn shim_nsdateformatter_set_calendar(_obj: u64, _cmd: u64, _cal: u64) {
    // no-op
}

/// `-[NSDateFormatter timeZone]` — returns system time zone.
pub extern "C" fn shim_nsdateformatter_time_zone(_obj: u64, _cmd: u64) -> u64 {
    if _obj == 0 {
        return 0;
    }
    crate::nstimezone::shim_nstimezone_system_timezone(0, 0)
}

/// `-[NSDateFormatter setTimeZone:]` — no-op.
pub extern "C" fn shim_nsdateformatter_set_time_zone(_obj: u64, _cmd: u64, _tz: u64) {
    // no-op
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn make_formatter() -> u64 {
        alloc_obj_fake(32)
    }

    #[test]
    fn nil_date_format_returns_zero() {
        assert_eq!(shim_nsdateformatter_date_format(0, 0), 0);
    }

    #[test]
    fn nil_date_style_returns_zero() {
        assert_eq!(shim_nsdateformatter_date_style(0, 0), 0);
    }

    #[test]
    fn nil_time_style_returns_zero() {
        assert_eq!(shim_nsdateformatter_time_style(0, 0), 0);
    }

    #[test]
    fn init_returns_self() {
        let obj = make_formatter();
        assert_eq!(shim_nsdateformatter_init(obj, 0), obj);
        free_obj(obj);
    }

    #[test]
    fn date_format_roundtrip() {
        let obj = make_formatter();
        let fmt = make_string("yyyy-MM-dd");
        shim_nsdateformatter_set_date_format(obj, 0, fmt);
        assert_eq!(shim_nsdateformatter_date_format(obj, 0), fmt);
        free_obj(obj);
    }

    #[test]
    fn date_style_roundtrip() {
        let obj = make_formatter();
        shim_nsdateformatter_set_date_style(obj, 0, 2);
        assert_eq!(shim_nsdateformatter_date_style(obj, 0), 2);
        free_obj(obj);
    }

    #[test]
    fn time_style_roundtrip() {
        let obj = make_formatter();
        shim_nsdateformatter_set_time_style(obj, 0, 3);
        assert_eq!(shim_nsdateformatter_time_style(obj, 0), 3);
        free_obj(obj);
    }

    #[test]
    fn set_date_format_nil_obj_is_noop() {
        shim_nsdateformatter_set_date_format(0, 0, 0xABC);
    }

    #[test]
    fn locale_returns_non_zero() {
        let obj = make_formatter();
        let loc = shim_nsdateformatter_locale(obj, 0);
        assert_ne!(loc, 0);
        free_obj(obj);
    }

    #[test]
    fn locale_nil_returns_zero() {
        assert_eq!(shim_nsdateformatter_locale(0, 0), 0);
    }

    #[test]
    fn set_locale_is_noop() {
        let obj = make_formatter();
        shim_nsdateformatter_set_locale(obj, 0, 0xDEAD);
        // does not crash
        free_obj(obj);
    }

    #[test]
    fn calendar_returns_non_zero() {
        let obj = make_formatter();
        let cal = shim_nsdateformatter_calendar(obj, 0);
        assert_ne!(cal, 0);
        free_obj(obj);
    }

    #[test]
    fn calendar_nil_returns_zero() {
        assert_eq!(shim_nsdateformatter_calendar(0, 0), 0);
    }

    #[test]
    fn set_calendar_is_noop() {
        let obj = make_formatter();
        shim_nsdateformatter_set_calendar(obj, 0, 0xDEAD);
        free_obj(obj);
    }

    #[test]
    fn time_zone_nil_returns_zero() {
        assert_eq!(shim_nsdateformatter_time_zone(0, 0), 0);
    }

    // Note: shim_nsdateformatter_time_zone() with a non-nil obj calls
    // shim_nstimezone_system_timezone() which requires a platform provider.
    // Cannot be tested in unit tests without one configured.

    #[test]
    fn set_time_zone_is_noop() {
        let obj = make_formatter();
        shim_nsdateformatter_set_time_zone(obj, 0, 0xDEAD);
        free_obj(obj);
    }

    #[test]
    fn string_from_date_nil_returns_zero() {
        let obj = make_formatter();
        assert_eq!(shim_nsdateformatter_string_from_date(obj, 0, 0), 0);
        assert_eq!(shim_nsdateformatter_string_from_date(0, 0, 0xBEEF), 0);
        free_obj(obj);
    }
}
