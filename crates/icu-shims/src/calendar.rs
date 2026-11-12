//! ICU Calendar (ucal_*) shims backed by `icu_calendar`.
//!
//! Provides time-zone–aware date arithmetic used by NSCalendar and
//! NSDateFormatter for component extraction and field manipulation.
//!
//! UCalendar handles map to a stored calendar state (epoch millis +
//! gregorian decomposition).  Operations that modify fields recompose
//! back to epoch millis.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

/// U_ZERO_ERROR
const U_ZERO_ERROR: i32 = 0;

// ── ICU calendar field constants ───────────────────────────────
// These match ICU4C UCalendarDateFields from ucal.h.

const UCAL_ERA: i32 = 0;
const UCAL_YEAR: i32 = 1;
const UCAL_MONTH: i32 = 2; // 0-based
const UCAL_WEEK_OF_YEAR: i32 = 3;
const UCAL_WEEK_OF_MONTH: i32 = 4;
const UCAL_DATE: i32 = 5; // day of month
const UCAL_DAY_OF_YEAR: i32 = 6;
const UCAL_DAY_OF_WEEK: i32 = 7; // 1=Sunday
const UCAL_HOUR_OF_DAY: i32 = 11;
const UCAL_MINUTE: i32 = 12;
const UCAL_SECOND: i32 = 13;
const UCAL_MILLISECOND: i32 = 14;

// ── Handle table ───────────────────────────────────────────────

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Stored state for each UCalendar handle.
struct CalState {
    /// Epoch milliseconds (UDate).
    millis: f64,
}

static CALENDARS: Mutex<Option<HashMap<u64, CalState>>> = Mutex::new(None);

fn with_table<R>(f: impl FnOnce(&mut HashMap<u64, CalState>) -> R) -> R {
    let mut guard = CALENDARS.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    f(table)
}

// ── Gregorian decomposition helpers ────────────────────────────
//
// We do the decomposition manually (no ICU4X calendar needed for
// the Gregorian case which covers >99% of real usage).

/// Days from epoch (1970-01-01) to the given date.
fn days_from_epoch(y: i64, m: i32, d: i32) -> i64 {
    // Algorithm from Howard Hinnant's date library.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = m as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as u32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146097 + doe as i64 - 719468
}

/// Decompose epoch millis into (year, month 0-based, day 1-based, hour, min, sec, ms).
fn decompose(millis: f64) -> (i32, i32, i32, i32, i32, i32, i32) {
    let total_ms = millis as i64;
    let mut day_ms = total_ms.rem_euclid(86_400_000);
    let mut z = (total_ms - day_ms) / 86_400_000;
    // z is days since epoch
    z += 719468; // shift to March 1, year 0
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y } as i32;

    let hour = (day_ms / 3_600_000) as i32;
    day_ms %= 3_600_000;
    let min = (day_ms / 60_000) as i32;
    day_ms %= 60_000;
    let sec = (day_ms / 1000) as i32;
    let ms = (day_ms % 1000) as i32;

    (year, m as i32 - 1, d as i32, hour, min, sec, ms) // month 0-based
}

/// Recompose (year, month 0-based, day, hour, min, sec, ms) → epoch millis.
fn recompose(year: i32, month0: i32, day: i32, hour: i32, min: i32, sec: i32, ms: i32) -> f64 {
    let days = days_from_epoch(year as i64, month0 + 1, day);
    let day_ms = hour as i64 * 3_600_000 + min as i64 * 60_000 + sec as i64 * 1000 + ms as i64;
    (days * 86_400_000 + day_ms) as f64
}

/// Day of week (1=Sunday, 2=Monday, …, 7=Saturday) for epoch millis.
fn day_of_week(millis: f64) -> i32 {
    let days = (millis / 86_400_000.0).floor() as i64;
    // Jan 1, 1970 was Thursday (5 in ICU 1-based Sunday start).
    let dow = ((days + 4) % 7) as i32; // 0=Sunday
    if dow < 0 {
        dow + 7 + 1
    } else {
        dow + 1
    }
}

/// Day of year (1-based).
fn day_of_year(year: i32, month0: i32, day: i32) -> i32 {
    let jan1 = days_from_epoch(year as i64, 1, 1);
    let cur = days_from_epoch(year as i64, month0 + 1, day);
    (cur - jan1 + 1) as i32
}

/// ISO week of year (simplified).
fn week_of_year(year: i32, month0: i32, day: i32) -> i32 {
    (day_of_year(year, month0, day) + 6) / 7
}

// ── Registration ───────────────────────────────────────────────

pub fn install(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_ucal_open",
            shim_ucal_open as *const () as u64,
            "ucal_open (Gregorian calendar)"
        ),
        (
            "_ucal_close",
            shim_ucal_close as *const () as u64,
            "ucal_close"
        ),
        (
            "_ucal_get",
            shim_ucal_get as *const () as u64,
            "ucal_get (field extraction)"
        ),
        (
            "_ucal_set",
            shim_ucal_set as *const () as u64,
            "ucal_set (field modification)"
        ),
        (
            "_ucal_getMillis",
            shim_ucal_get_millis as *const () as u64,
            "ucal_getMillis"
        ),
        (
            "_ucal_setMillis",
            shim_ucal_set_millis as *const () as u64,
            "ucal_setMillis"
        ),
        (
            "_ucal_add",
            shim_ucal_add as *const () as u64,
            "ucal_add (date arithmetic)"
        ),
        (
            "_ucal_getTimeZoneDisplayName",
            shim_ucal_get_tz_display_name as *const () as u64,
            "ucal_getTimeZoneDisplayName (stub → UTC)"
        )
    );
}

// ── Shim implementations ──────────────────────────────────────

/// `ucal_open(zoneID, len, locale, caltype, status) → UCalendar*`
pub extern "C" fn shim_ucal_open(
    _zone_id: u64,
    _zone_len: i32,
    _locale: u64,
    _cal_type: i32,
    status: *mut i32,
) -> u64 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    with_table(|t| {
        t.insert(handle, CalState { millis: 0.0 });
    });
    handle
}

/// `ucal_close(cal)`
pub extern "C" fn shim_ucal_close(cal: u64) {
    with_table(|t| {
        t.remove(&cal);
    });
}

/// `ucal_get(cal, field, status) → int32_t`
pub extern "C" fn shim_ucal_get(cal: u64, field: i32, status: *mut i32) -> i32 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    let millis = with_table(|t| t.get(&cal).map(|c| c.millis).unwrap_or(0.0));
    let (year, month0, day, hour, min, sec, ms) = decompose(millis);
    match field {
        UCAL_ERA => i32::from(year >= 1), // 1=AD, 0=BC
        UCAL_YEAR => year.unsigned_abs() as i32,
        UCAL_MONTH => month0,
        UCAL_DATE => day,
        UCAL_DAY_OF_YEAR => day_of_year(year, month0, day),
        UCAL_DAY_OF_WEEK => day_of_week(millis),
        UCAL_WEEK_OF_YEAR => week_of_year(year, month0, day),
        UCAL_WEEK_OF_MONTH => (day + 6) / 7,
        UCAL_HOUR_OF_DAY => hour,
        UCAL_MINUTE => min,
        UCAL_SECOND => sec,
        UCAL_MILLISECOND => ms,
        _ => 0,
    }
}

/// `ucal_set(cal, field, value)`
pub extern "C" fn shim_ucal_set(cal: u64, field: i32, value: i32) {
    with_table(|t| {
        let Some(state) = t.get_mut(&cal) else {
            return;
        };
        let (mut y, mut m, mut d, mut h, mut mi, mut s, mut ms) = decompose(state.millis);
        match field {
            UCAL_YEAR => y = value,
            UCAL_MONTH => m = value,
            UCAL_DATE => d = value,
            UCAL_HOUR_OF_DAY => h = value,
            UCAL_MINUTE => mi = value,
            UCAL_SECOND => s = value,
            UCAL_MILLISECOND => ms = value,
            _ => {}
        }
        state.millis = recompose(y, m, d, h, mi, s, ms);
    });
}

/// `ucal_getMillis(cal, status) → UDate (double)`
pub extern "C" fn shim_ucal_get_millis(cal: u64, status: *mut i32) -> f64 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    with_table(|t| t.get(&cal).map(|c| c.millis).unwrap_or(0.0))
}

/// `ucal_setMillis(cal, dateTime, status)`
pub extern "C" fn shim_ucal_set_millis(cal: u64, date_time: f64, status: *mut i32) {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    with_table(|t| {
        if let Some(state) = t.get_mut(&cal) {
            state.millis = date_time;
        }
    });
}

/// `ucal_add(cal, field, amount, status)`
pub extern "C" fn shim_ucal_add(cal: u64, field: i32, amount: i32, status: *mut i32) {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    with_table(|t| {
        let Some(state) = t.get_mut(&cal) else {
            return;
        };
        let (mut y, mut m, d, h, mi, s, ms) = decompose(state.millis);
        match field {
            UCAL_YEAR => y += amount,
            UCAL_MONTH => {
                let total = m + amount;
                y += total.div_euclid(12);
                m = total.rem_euclid(12);
            }
            UCAL_DATE => {
                // Simple: add days via millis
                state.millis += amount as f64 * 86_400_000.0;
                return;
            }
            UCAL_HOUR_OF_DAY => {
                state.millis += amount as f64 * 3_600_000.0;
                return;
            }
            UCAL_MINUTE => {
                state.millis += amount as f64 * 60_000.0;
                return;
            }
            UCAL_SECOND => {
                state.millis += amount as f64 * 1000.0;
                return;
            }
            UCAL_MILLISECOND => {
                state.millis += amount as f64;
                return;
            }
            _ => return,
        }
        state.millis = recompose(y, m, d, h, mi, s, ms);
    });
}

/// `ucal_getTimeZoneDisplayName(cal, type, locale, result, resultLen, status) → int32_t`
///
/// Returns "UTC" for all requests (timezone support is UTC-only).
pub extern "C" fn shim_ucal_get_tz_display_name(
    _cal: u64,
    _name_type: i32,
    _locale: u64,
    result: *mut u16,
    result_capacity: i32,
    status: *mut i32,
) -> i32 {
    if !status.is_null() {
        unsafe {
            *status = U_ZERO_ERROR;
        }
    }
    let name: &[u16] = &[b'U' as u16, b'T' as u16, b'C' as u16];
    let copy_len = std::cmp::min(name.len(), result_capacity.max(0) as usize);
    if !result.is_null() && copy_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(name.as_ptr(), result, copy_len);
        }
    }
    name.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_open_close() {
        let mut status = -1i32;
        let cal = shim_ucal_open(0, 0, 0, 0, &mut status);
        assert_eq!(status, U_ZERO_ERROR);
        assert_ne!(cal, 0);
        shim_ucal_close(cal);
    }

    #[test]
    fn set_and_get_millis() {
        let mut status = 0i32;
        let cal = shim_ucal_open(0, 0, 0, 0, &mut status);
        // Jan 15, 2024 00:00:00 UTC in millis
        let millis = 1705276800000.0;
        shim_ucal_set_millis(cal, millis, &mut status);
        let got = shim_ucal_get_millis(cal, &mut status);
        assert_eq!(got, millis);
        shim_ucal_close(cal);
    }

    #[test]
    fn get_date_fields() {
        let mut status = 0i32;
        let cal = shim_ucal_open(0, 0, 0, 0, &mut status);
        // 2024-01-15 14:30:45.123 UTC
        // = 1705276800000 + 14*3600000 + 30*60000 + 45*1000 + 123
        let millis = 1705276800000.0 + 14.0 * 3_600_000.0 + 30.0 * 60_000.0 + 45.0 * 1000.0 + 123.0;
        shim_ucal_set_millis(cal, millis, &mut status);

        assert_eq!(shim_ucal_get(cal, UCAL_YEAR, &mut status), 2024);
        assert_eq!(shim_ucal_get(cal, UCAL_MONTH, &mut status), 0); // January = 0
        assert_eq!(shim_ucal_get(cal, UCAL_DATE, &mut status), 15);
        assert_eq!(shim_ucal_get(cal, UCAL_HOUR_OF_DAY, &mut status), 14);
        assert_eq!(shim_ucal_get(cal, UCAL_MINUTE, &mut status), 30);
        assert_eq!(shim_ucal_get(cal, UCAL_SECOND, &mut status), 45);
        assert_eq!(shim_ucal_get(cal, UCAL_MILLISECOND, &mut status), 123);
        shim_ucal_close(cal);
    }

    #[test]
    fn set_field_and_recompose() {
        let mut status = 0i32;
        let cal = shim_ucal_open(0, 0, 0, 0, &mut status);
        // Start at 2024-01-15 00:00:00.000 UTC
        shim_ucal_set_millis(cal, 1705276800000.0, &mut status);
        // Set month to July (6)
        shim_ucal_set(cal, UCAL_MONTH, 6);
        assert_eq!(shim_ucal_get(cal, UCAL_MONTH, &mut status), 6);
        assert_eq!(shim_ucal_get(cal, UCAL_YEAR, &mut status), 2024);
        assert_eq!(shim_ucal_get(cal, UCAL_DATE, &mut status), 15);
        shim_ucal_close(cal);
    }

    #[test]
    fn add_months_wraps_year() {
        let mut status = 0i32;
        let cal = shim_ucal_open(0, 0, 0, 0, &mut status);
        // Start at 2024-11-15 (month=10 in 0-based)
        let nov_15 = 1705276800000.0 + 304.0 * 86_400_000.0; // approx
        shim_ucal_set_millis(cal, nov_15, &mut status);
        let start_year = shim_ucal_get(cal, UCAL_YEAR, &mut status);
        let start_month = shim_ucal_get(cal, UCAL_MONTH, &mut status);
        // Add 3 months
        shim_ucal_add(cal, UCAL_MONTH, 3, &mut status);
        let end_month = shim_ucal_get(cal, UCAL_MONTH, &mut status);
        let end_year = shim_ucal_get(cal, UCAL_YEAR, &mut status);
        // Should have wrapped to next year
        assert_eq!(end_month, (start_month + 3) % 12);
        assert_eq!(end_year, start_year + 1);
        shim_ucal_close(cal);
    }

    #[test]
    fn add_days() {
        let mut status = 0i32;
        let cal = shim_ucal_open(0, 0, 0, 0, &mut status);
        shim_ucal_set_millis(cal, 1705276800000.0, &mut status); // Jan 15
        shim_ucal_add(cal, UCAL_DATE, 20, &mut status);
        assert_eq!(shim_ucal_get(cal, UCAL_DATE, &mut status), 4); // Feb 4
        assert_eq!(shim_ucal_get(cal, UCAL_MONTH, &mut status), 1); // February
        shim_ucal_close(cal);
    }

    #[test]
    fn timezone_display_name() {
        let mut buf = [0u16; 10];
        let mut status = 0i32;
        let len = shim_ucal_get_tz_display_name(0, 0, 0, buf.as_mut_ptr(), 10, &mut status);
        assert_eq!(len, 3);
        assert_eq!(&buf[..3], &[b'U' as u16, b'T' as u16, b'C' as u16]);
    }
}
