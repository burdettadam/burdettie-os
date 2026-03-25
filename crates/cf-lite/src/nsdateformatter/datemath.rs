/// Convert days since Unix epoch to (year, month, day).
pub(super) fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
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
pub(super) fn day_of_week(year: i64, month: i64, day: i64) -> usize {
    // Tomohiko Sakamoto's algorithm
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let m = month as usize;
    ((y + y / 4 - y / 100 + y / 400 + t[m - 1] + day) % 7) as usize
}

/// Break a CF absolute time into date/time components (UTC).
pub(super) fn decompose(ti: f64) -> (i64, i64, i64, i64, i64, i64) {
    let unix = ti + super::CF_EPOCH_OFFSET;
    let secs = unix as i64;
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let (year, month, day) = days_to_ymd(days_since_epoch);
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    (year, month, day, hours, minutes, seconds)
}
