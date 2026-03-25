use crate::cfstring;

/// `-[NSDateFormatter dateFromString:]` — parse a string into an NSDate.
///
/// Supports ISO-8601-like patterns (yyyy-MM-dd HH:mm:ss) and basic
/// numeric format patterns.  Returns 0 (nil) if the string can't be parsed.
pub extern "C" fn shim_nsdateformatter_date_from_string(
    obj: u64,
    _cmd: u64,
    string: u64,
) -> u64 {
    if obj == 0 || string == 0 {
        return 0;
    }

    let Some(input) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };

    let fmt_ptr = super::read_format_ptr(obj);
    let format = if fmt_ptr != 0 {
        cfstring::extract_cfstring_cstr(fmt_ptr).unwrap_or("")
    } else {
        // Without an explicit format string, try ISO-8601 yyyy-MM-dd HH:mm:ss
        "yyyy-MM-dd HH:mm:ss"
    };

    // Parse using a simplified approach that matches the format string
    let parsed = parse_date_with_format(format, input);
    let Some((year, month, day, hour, min, sec)) = parsed else {
        log::debug!("dateFromString: failed to parse '{}' with format '{}'", input, format);
        return 0;
    };

    // Reconstruct CF absolute time
    let (days, _) = ymd_to_days(year, month, day);
    let unix_secs = days * 86400 + hour * 3600 + min * 60 + sec;
    let cf_ti = unix_secs as f64 - super::CF_EPOCH_OFFSET;

    crate::nsdate::alloc_nsdate(cf_ti)
}

/// Convert (year, month, day) to days since Unix epoch.
fn ymd_to_days(year: i64, month: i64, day: i64) -> (i64, i64) {
    let (y, m) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    (days, 0)
}

/// Parse a date string given a simplified Unicode TR35 format pattern.
///
/// Returns `Some((year, month, day, hour, minute, second))` on success.
fn parse_date_with_format(
    format: &str,
    input: &str,
) -> Option<(i64, i64, i64, i64, i64, i64)> {
    let mut year: i64 = 0;
    let mut month: i64 = 1;
    let mut day: i64 = 1;
    let mut hour: i64 = 0;
    let mut min: i64 = 0;
    let mut sec: i64 = 0;

    let fmt_chars: Vec<char> = format.chars().collect();
    let input_bytes = input.as_bytes();
    let mut fi = 0;
    let mut ii = 0;

    while fi < fmt_chars.len() && ii < input_bytes.len() {
        let ch = fmt_chars[fi];
        let mut count = 1;
        while fi + count < fmt_chars.len() && fmt_chars[fi + count] == ch {
            count += 1;
        }

        match ch {
            'y' | 'M' | 'd' | 'H' | 'h' | 'm' | 's' => {
                // Extract numeric digits from input
                let start = ii;
                while ii < input_bytes.len() && input_bytes[ii].is_ascii_digit() {
                    ii += 1;
                }
                if ii == start {
                    return None;
                }
                let num_str = &input[start..ii];
                let val: i64 = num_str.parse().ok()?;
                match ch {
                    'y' => year = val,
                    'M' => month = val,
                    'd' => day = val,
                    'H' | 'h' => hour = val,
                    'm' => min = val,
                    's' => sec = val,
                    _ => {}
                }
            }
            'a' => {
                // AM/PM
                if ii + 2 <= input_bytes.len() {
                    let ampm = &input[ii..ii + 2];
                    if ampm.eq_ignore_ascii_case("PM") && hour < 12 {
                        hour += 12;
                    } else if ampm.eq_ignore_ascii_case("AM") && hour == 12 {
                        hour = 0;
                    }
                    ii += 2;
                }
            }
            'E' => {
                // Day of week name — skip alpha chars
                while ii < input_bytes.len() && input_bytes[ii].is_ascii_alphabetic() {
                    ii += 1;
                }
            }
            '\'' => {
                // Literal text between quotes — skip matching chars
                fi += 1;
                while fi < fmt_chars.len() && fmt_chars[fi] != '\'' {
                    if ii < input_bytes.len() {
                        ii += 1;
                    }
                    fi += 1;
                }
                fi += 1; // skip closing quote
                continue;
            }
            _ => {
                // Literal char (space, colon, dash, slash, etc.) — skip in input
                if ii < input_bytes.len() {
                    ii += 1;
                }
            }
        }
        fi += count;
    }

    Some((year, month, day, hour, min, sec))
}
