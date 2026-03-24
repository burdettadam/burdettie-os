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

fn alloc_nsdate(ti: f64) -> u64 {
    let obj = unsafe { libc::calloc(1, 16) } as u64;
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
