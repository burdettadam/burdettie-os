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

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
    days += 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Day of week: 0=Sunday, 1=Monday, ..., 6=Saturday.
fn day_of_week(year: i64, month: i64, day: i64) -> usize {
    // Tomohiko Sakamoto's algorithm
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let m = month as usize;
    ((y + y / 4 - y / 100 + y / 400 + t[m - 1] + day) % 7) as usize
}

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTH_ABBREVS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Format a date using the dateFormat string (simplified subset of Unicode TR35).
fn apply_format(
    format_str: &str,
    year: i64,
    month: i64,
    day: i64,
    h: i64,
    m: i64,
    s: i64,
) -> String {
    let mut result = String::new();
    let chars: Vec<char> = format_str.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        // Count consecutive identical pattern chars
        let mut count = 1;
        while i + count < len && chars[i + count] == ch {
            count += 1;
        }

        match ch {
            'y' => {
                if count <= 2 {
                    result.push_str(&format!("{:02}", year % 100));
                } else {
                    result.push_str(&format!("{:04}", year));
                }
            }
            'M' => match count {
                1 => result.push_str(&format!("{}", month)),
                2 => result.push_str(&format!("{:02}", month)),
                3 => result.push_str(MONTH_ABBREVS[(month - 1).clamp(0, 11) as usize]),
                _ => result.push_str(MONTH_NAMES[(month - 1).clamp(0, 11) as usize]),
            },
            'd' => {
                if count <= 1 {
                    result.push_str(&format!("{}", day));
                } else {
                    result.push_str(&format!("{:02}", day));
                }
            }
            'H' => {
                if count <= 1 {
                    result.push_str(&format!("{}", h));
                } else {
                    result.push_str(&format!("{:02}", h));
                }
            }
            'h' => {
                let h12 = if h == 0 {
                    12
                } else if h > 12 {
                    h - 12
                } else {
                    h
                };
                if count <= 1 {
                    result.push_str(&format!("{}", h12));
                } else {
                    result.push_str(&format!("{:02}", h12));
                }
            }
            'm' if ch == 'm' => {
                if count <= 1 {
                    result.push_str(&format!("{}", m));
                } else {
                    result.push_str(&format!("{:02}", m));
                }
            }
            's' => {
                if count <= 1 {
                    result.push_str(&format!("{}", s));
                } else {
                    result.push_str(&format!("{:02}", s));
                }
            }
            'a' => {
                result.push_str(if h < 12 { "AM" } else { "PM" });
            }
            'E' => {
                let dow = day_of_week(year, month, day);
                if count <= 3 {
                    result.push_str(&DAY_NAMES[dow][..3]);
                } else {
                    result.push_str(DAY_NAMES[dow]);
                }
            }
            '\'' => {
                // Literal text between single quotes
                i += 1; // skip opening quote
                while i < len && chars[i] != '\'' {
                    result.push(chars[i]);
                    i += 1;
                }
                // skip closing quote (handled by i += count below, but count=1 here)
                i += 1;
                continue;
            }
            _ => {
                // Pass through literal characters (spaces, colons, slashes, etc.)
                for _ in 0..count {
                    result.push(ch);
                }
            }
        }
        i += count;
    }
    result
}

/// Format a date using style-based formatting (when no explicit dateFormat is set).
#[allow(clippy::too_many_arguments)]
fn format_with_styles(
    date_style: u64,
    time_style: u64,
    year: i64,
    month: i64,
    day: i64,
    h: i64,
    m: i64,
    s: i64,
) -> String {
    let date_part = match date_style {
        1 => format!("{}/{}/{:02}", month, day, year % 100), // short
        2 => format!(
            "{} {}, {}",
            MONTH_ABBREVS[(month - 1).clamp(0, 11) as usize],
            day,
            year
        ), // medium
        3 => format!(
            "{} {}, {}",
            MONTH_NAMES[(month - 1).clamp(0, 11) as usize],
            day,
            year
        ), // long
        4 => {
            // full
            let dow = day_of_week(year, month, day);
            format!(
                "{}, {} {}, {}",
                DAY_NAMES[dow],
                MONTH_NAMES[(month - 1).clamp(0, 11) as usize],
                day,
                year
            )
        }
        _ => String::new(), // no style
    };

    let time_part = match time_style {
        1 => {
            // short: "1:00 AM"
            let h12 = if h == 0 {
                12
            } else if h > 12 {
                h - 12
            } else {
                h
            };
            let ampm = if h < 12 { "AM" } else { "PM" };
            format!("{}:{:02} {}", h12, m, ampm)
        }
        2 => {
            // medium: "1:00:00 AM"
            let h12 = if h == 0 {
                12
            } else if h > 12 {
                h - 12
            } else {
                h
            };
            let ampm = if h < 12 { "AM" } else { "PM" };
            format!("{}:{:02}:{:02} {}", h12, m, s, ampm)
        }
        3 => {
            // long: "1:00:00 AM UTC"
            let h12 = if h == 0 {
                12
            } else if h > 12 {
                h - 12
            } else {
                h
            };
            let ampm = if h < 12 { "AM" } else { "PM" };
            format!("{}:{:02}:{:02} {} UTC", h12, m, s, ampm)
        }
        4 => {
            // full: same as long for our purposes
            let h12 = if h == 0 {
                12
            } else if h > 12 {
                h - 12
            } else {
                h
            };
            let ampm = if h < 12 { "AM" } else { "PM" };
            format!("{}:{:02}:{:02} {} UTC", h12, m, s, ampm)
        }
        _ => String::new(), // no style
    };

    match (date_part.is_empty(), time_part.is_empty()) {
        (true, true) => String::new(),
        (false, true) => date_part,
        (true, false) => time_part,
        (false, false) => format!("{}, {}", date_part, time_part),
    }
}

/// Break a CF absolute time into date/time components (UTC).
fn decompose(ti: f64) -> (i64, i64, i64, i64, i64, i64) {
    let unix = ti + CF_EPOCH_OFFSET;
    let secs = unix as i64;
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let (year, month, day) = days_to_ymd(days_since_epoch);
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    (year, month, day, hours, minutes, seconds)
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
    let (year, month, day, h, m, s) = decompose(ti);

    let fmt_ptr = read_format_ptr(obj);
    let formatted = if fmt_ptr != 0 {
        if let Some(fmt_str) = cfstring::extract_cfstring_cstr(fmt_ptr) {
            apply_format(fmt_str, year, month, day, h, m, s)
        } else {
            String::new()
        }
    } else {
        format_with_styles(
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
