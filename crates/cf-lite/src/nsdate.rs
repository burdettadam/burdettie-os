//! NSDate — basic date/time representation backed by `gettimeofday`.
//!
//! Layout (16 bytes):
//!   offset  0: isa              (u64)  → NSDate class addr
//!   offset  8: time_interval    (f64)  → seconds since 2001-01-01 00:00:00 UTC
//!                                        (Core Foundation absolute time / NSTimeInterval)
//!
//! Apple's reference date is 2001-01-01T00:00:00Z.
//! Unix epoch is 1970-01-01T00:00:00Z.
//! Offset: 978307200 seconds between them.

const NSDATE_TI_OFF: usize = 8;

/// Seconds between Unix epoch (1970) and Core Foundation reference date (2001).
const CF_EPOCH_OFFSET: f64 = 978_307_200.0;

fn now_cf_absolute_time() -> f64 {
    let (sec, usec) = crate::platform().clock().wall_time_usec();
    let unix_secs = sec as f64 + usec as f64 / 1_000_000.0;
    unix_secs - CF_EPOCH_OFFSET
}

pub fn alloc_nsdate(ti: f64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 16) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdate_class_addr();
        *((obj as usize + NSDATE_TI_OFF) as *mut f64) = ti;
    }
    obj
}

/// `+[NSDate date]` — current date/time.
pub extern "C" fn shim_nsdate_date(_cls: u64, _cmd: u64) -> u64 {
    alloc_nsdate(now_cf_absolute_time())
}

/// `+[NSDate dateWithTimeIntervalSince1970:]` — from Unix timestamp.
pub extern "C" fn shim_nsdate_date_with_ti_since_1970(_cls: u64, _cmd: u64, ti: f64) -> u64 {
    alloc_nsdate(ti - CF_EPOCH_OFFSET)
}

/// `+[NSDate dateWithTimeIntervalSinceReferenceDate:]` — from CF absolute time.
pub extern "C" fn shim_nsdate_date_with_ti_since_ref(_cls: u64, _cmd: u64, ti: f64) -> u64 {
    alloc_nsdate(ti)
}

/// `-[NSDate timeIntervalSince1970]` — Unix timestamp.
pub extern "C" fn shim_nsdate_ti_since_1970(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let ti = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    ti + CF_EPOCH_OFFSET
}

/// `-[NSDate timeIntervalSinceReferenceDate]` — CF absolute time.
pub extern "C" fn shim_nsdate_ti_since_ref(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) }
}

/// `-[NSDate timeIntervalSinceNow]`
pub extern "C" fn shim_nsdate_ti_since_now(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let ti = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    ti - now_cf_absolute_time()
}

/// `-[NSDate timeIntervalSinceDate:]`
pub extern "C" fn shim_nsdate_ti_since_date(obj: u64, _cmd: u64, other: u64) -> f64 {
    if obj == 0 || other == 0 {
        return 0.0;
    }
    let ti_self = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    let ti_other = unsafe { *((other as usize + NSDATE_TI_OFF) as *const f64) };
    ti_self - ti_other
}

/// `-[NSDate dateByAddingTimeInterval:]`
pub extern "C" fn shim_nsdate_date_by_adding(obj: u64, _cmd: u64, seconds: f64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ti = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    alloc_nsdate(ti + seconds)
}

/// `-[NSDate isEqualToDate:]`
pub extern "C" fn shim_nsdate_is_equal_to_date(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == 0 || other == 0 {
        return 0;
    }
    let ti_self = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    let ti_other = unsafe { *((other as usize + NSDATE_TI_OFF) as *const f64) };
    if (ti_self - ti_other).abs() < 1e-9 {
        1
    } else {
        0
    }
}

/// `-[NSDate compare:]` — returns NSComparisonResult (-1, 0, 1).
pub extern "C" fn shim_nsdate_compare(obj: u64, _cmd: u64, other: u64) -> i64 {
    if obj == 0 || other == 0 {
        return 0;
    }
    let ti_self = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    let ti_other = unsafe { *((other as usize + NSDATE_TI_OFF) as *const f64) };
    if (ti_self - ti_other).abs() < 1e-9 {
        0 // NSOrderedSame
    } else if ti_self < ti_other {
        -1 // NSOrderedAscending
    } else {
        1 // NSOrderedDescending
    }
}

/// `-[NSDate description]` — ISO 8601-ish string.
pub extern "C" fn shim_nsdate_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ti = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    let unix = ti + CF_EPOCH_OFFSET;
    let secs = unix as i64;

    // Simple UTC breakdown (no timezone conversion needed for description)
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since 1970-01-01 to y/m/d
    let (year, month, day) = days_to_ymd(days_since_epoch);

    let desc = format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02} +0000");
    let cstr = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSDate earlierDate:]` — return the earlier of self and other.
pub extern "C" fn shim_nsdate_earlier_date(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 {
        return other;
    }
    if other == 0 {
        return obj;
    }
    let ti_self = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    let ti_other = unsafe { *((other as usize + NSDATE_TI_OFF) as *const f64) };
    if ti_self <= ti_other {
        obj
    } else {
        other
    }
}

/// `-[NSDate laterDate:]` — return the later of self and other.
pub extern "C" fn shim_nsdate_later_date(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 {
        return other;
    }
    if other == 0 {
        return obj;
    }
    let ti_self = unsafe { *((obj as usize + NSDATE_TI_OFF) as *const f64) };
    let ti_other = unsafe { *((other as usize + NSDATE_TI_OFF) as *const f64) };
    if ti_self >= ti_other {
        obj
    } else {
        other
    }
}

/// Apple's distantFuture: ~4001-01-01 as CF absolute time.
const DISTANT_FUTURE_TI: f64 = 63_113_904_000.0;
/// Apple's distantPast: ~0001-01-01 as CF absolute time.
const DISTANT_PAST_TI: f64 = -63_114_076_800.0;

/// `+[NSDate distantFuture]` — a date far in the future.
pub extern "C" fn shim_nsdate_distant_future(_cls: u64, _cmd: u64) -> u64 {
    alloc_nsdate(DISTANT_FUTURE_TI)
}

/// `+[NSDate distantPast]` — a date far in the past.
pub extern "C" fn shim_nsdate_distant_past(_cls: u64, _cmd: u64) -> u64 {
    alloc_nsdate(DISTANT_PAST_TI)
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
    // Civil calendar algorithm
    days += 719_468; // shift to 0000-03-01
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn alloc_and_read_ti_since_ref() {
        let d = alloc_nsdate(0.0);
        assert_ne!(d, 0);
        let ti = shim_nsdate_ti_since_ref(d, 0);
        assert!((ti - 0.0).abs() < 1e-9);
        free_obj(d);
    }

    #[test]
    fn ti_since_1970_offset() {
        // CF reference = 2001-01-01 → Unix offset = 978307200
        let d = alloc_nsdate(0.0);
        let ti = shim_nsdate_ti_since_1970(d, 0);
        assert!((ti - CF_EPOCH_OFFSET).abs() < 1.0, "expected ~{CF_EPOCH_OFFSET} got {ti}");
        free_obj(d);
    }

    #[test]
    fn is_equal_to_date_same_value() {
        let a = alloc_nsdate(1234.5);
        let b = alloc_nsdate(1234.5);
        assert_eq!(shim_nsdate_is_equal_to_date(a, 0, b), 1);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_to_date_different_values() {
        let a = alloc_nsdate(1000.0);
        let b = alloc_nsdate(2000.0);
        assert_eq!(shim_nsdate_is_equal_to_date(a, 0, b), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn compare_ordering() {
        let a = alloc_nsdate(100.0);
        let b = alloc_nsdate(200.0);
        assert_eq!(shim_nsdate_compare(a, 0, b), -1);
        assert_eq!(shim_nsdate_compare(b, 0, a), 1);
        assert_eq!(shim_nsdate_compare(a, 0, a), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn date_by_adding_interval() {
        let d = alloc_nsdate(1000.0);
        let d2 = shim_nsdate_date_by_adding(d, 0, 60.0);
        assert_ne!(d2, 0);
        let ti = shim_nsdate_ti_since_ref(d2, 0);
        assert!((ti - 1060.0).abs() < 1e-9);
        free_obj(d);
        free_obj(d2);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsdate_ti_since_ref(0, 0), 0.0);
        assert_eq!(shim_nsdate_ti_since_1970(0, 0), 0.0);
        assert_eq!(shim_nsdate_ti_since_now(0, 0), 0.0);
        assert_eq!(shim_nsdate_is_equal_to_date(0, 0, 0), 0);
        assert_eq!(shim_nsdate_date_by_adding(0, 0, 1.0), 0);
    }

    #[test]
    fn description_non_zero() {
        let d = alloc_nsdate(0.0);
        let desc = shim_nsdate_description(d, 0);
        assert_ne!(desc, 0);
        // Should contain "2001" — CF reference date year
        let s = read_string(desc);
        assert!(s.contains("2001"), "expected year 2001 in '{s}'");
        free_obj(d);
        free_obj(desc);
    }

    #[test]
    fn earlier_date_returns_earlier() {
        let a = alloc_nsdate(100.0);
        let b = alloc_nsdate(200.0);
        assert_eq!(shim_nsdate_earlier_date(a, 0, b), a);
        assert_eq!(shim_nsdate_earlier_date(b, 0, a), a);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn later_date_returns_later() {
        let a = alloc_nsdate(100.0);
        let b = alloc_nsdate(200.0);
        assert_eq!(shim_nsdate_later_date(a, 0, b), b);
        assert_eq!(shim_nsdate_later_date(b, 0, a), b);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn earlier_later_nil_safety() {
        let d = alloc_nsdate(100.0);
        assert_eq!(shim_nsdate_earlier_date(0, 0, d), d);
        assert_eq!(shim_nsdate_earlier_date(d, 0, 0), d);
        assert_eq!(shim_nsdate_later_date(0, 0, d), d);
        assert_eq!(shim_nsdate_later_date(d, 0, 0), d);
        free_obj(d);
    }

    #[test]
    fn distant_future_is_positive() {
        let d = shim_nsdate_distant_future(0, 0);
        assert_ne!(d, 0);
        let ti = shim_nsdate_ti_since_ref(d, 0);
        assert!(ti > 1e9, "distantFuture should be far in the future, got {ti}");
        free_obj(d);
    }

    #[test]
    fn distant_past_is_negative() {
        let d = shim_nsdate_distant_past(0, 0);
        assert_ne!(d, 0);
        let ti = shim_nsdate_ti_since_ref(d, 0);
        assert!(ti < -1e9, "distantPast should be far in the past, got {ti}");
        free_obj(d);
    }

    #[test]
    fn distant_future_is_later_than_distant_past() {
        let future = shim_nsdate_distant_future(0, 0);
        let past = shim_nsdate_distant_past(0, 0);
        assert_eq!(shim_nsdate_compare(future, 0, past), 1);
        assert_eq!(shim_nsdate_earlier_date(future, 0, past), past);
        assert_eq!(shim_nsdate_later_date(future, 0, past), future);
        free_obj(future);
        free_obj(past);
    }
}
