use super::datemath::day_of_week;

pub(super) const MONTH_NAMES: [&str; 12] = [
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
pub(super) const MONTH_ABBREVS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
pub(super) const DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Format a date using the dateFormat string (simplified subset of Unicode TR35).
pub(super) fn apply_format(
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
pub(super) fn format_with_styles(
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
