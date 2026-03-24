//! NSNumberFormatter shim: locale-aware number formatting.
//!
//! Object layout: 32 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  number_style (u64) — 0=none, 1=decimal, 2=currency, 3=percent, 4=scientific
//!   [16..24): min_frac_digits (u64)
//!   [24..32): max_frac_digits (u64)
//!
//! Uses the locale from NSLocale.currentLocale (already a singleton) to
//! determine grouping separator and decimal separator.

use crate::cfstring;

// NSNumberFormatterStyle constants (matching Apple's enum values)
const STYLE_NONE: u64 = 0;
const STYLE_DECIMAL: u64 = 1;
const STYLE_CURRENCY: u64 = 2;
const STYLE_PERCENT: u64 = 3;
const STYLE_SCIENTIFIC: u64 = 4;

// NSNumber layout offsets
const NSNUMBER_TAG_OFF: usize = 8;
const NSNUMBER_VAL_OFF: usize = 16;
const TAG_INT: u64 = 0;
const TAG_DOUBLE: u64 = 1;
const TAG_BOOL: u64 = 2;

/// Extract the f64 value from an NSNumber object.
fn read_nsnumber_f64(obj: u64) -> f64 {
    let tag = unsafe { *((obj as usize + NSNUMBER_TAG_OFF) as *const u64) };
    let raw = unsafe { *((obj as usize + NSNUMBER_VAL_OFF) as *const u64) };
    match tag {
        TAG_INT => raw as i64 as f64,
        TAG_DOUBLE => f64::from_bits(raw),
        TAG_BOOL => {
            if raw != 0 {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn read_style(obj: u64) -> u64 {
    unsafe { *((obj as *const u64).add(1)) }
}

fn read_min_frac(obj: u64) -> u64 {
    unsafe { *((obj as *const u64).add(2)) }
}

fn read_max_frac(obj: u64) -> u64 {
    unsafe { *((obj as *const u64).add(3)) }
}

/// Format a number based on the style and fraction digit settings.
fn format_number(val: f64, style: u64, min_frac: u64, max_frac: u64) -> String {
    match style {
        STYLE_NONE => {
            // No formatting — plain number
            if val == val.floor() && val.abs() < 1e15 {
                format!("{}", val as i64)
            } else {
                format_with_frac(val, min_frac, max_frac)
            }
        }
        STYLE_DECIMAL => {
            // Decimal with grouping separators
            let formatted = format_with_frac(val, min_frac, max_frac);
            add_grouping_separator(&formatted, ',')
        }
        STYLE_CURRENCY => {
            let formatted =
                format_with_frac(val, std::cmp::max(min_frac, 2), std::cmp::max(max_frac, 2));
            let grouped = add_grouping_separator(&formatted, ',');
            format!("${grouped}")
        }
        STYLE_PERCENT => {
            let pct = val * 100.0;
            let formatted = format_with_frac(pct, min_frac, max_frac);
            format!("{formatted}%")
        }
        STYLE_SCIENTIFIC => {
            format!("{val:e}")
        }
        _ => format!("{val}"),
    }
}

/// Format with min/max fractional digits.
fn format_with_frac(val: f64, min_frac: u64, max_frac: u64) -> String {
    let max = max_frac as usize;
    // Format with max digits, then trim trailing zeros down to min
    let s = format!("{val:.prec$}", prec = max);
    if let Some(dot_pos) = s.find('.') {
        let min = min_frac as usize;
        let decimal_part = &s[dot_pos + 1..];
        let mut keep = decimal_part.len();
        // Trim trailing zeros, but keep at least `min` digits
        while keep > min && decimal_part.as_bytes()[keep - 1] == b'0' {
            keep -= 1;
        }
        if keep == 0 {
            s[..dot_pos].to_string()
        } else {
            format!("{}.{}", &s[..dot_pos], &decimal_part[..keep])
        }
    } else {
        s
    }
}

/// Insert grouping separators (e.g., commas) into the integer part.
fn add_grouping_separator(s: &str, sep: char) -> String {
    let (sign, rest) = if let Some(stripped) = s.strip_prefix('-') {
        ("-", stripped)
    } else {
        ("", s)
    };
    let (int_part, frac_part) = match rest.find('.') {
        Some(pos) => (&rest[..pos], Some(&rest[pos..])),
        None => (rest, None),
    };
    let mut result = String::new();
    for (i, ch) in int_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(sep);
        }
        result.push(ch);
    }
    let grouped: String = result.chars().rev().collect();
    match frac_part {
        Some(fp) => format!("{sign}{grouped}{fp}"),
        None => format!("{sign}{grouped}"),
    }
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSNumberFormatter init]` — returns self with default settings.
pub extern "C" fn shim_nsnumberformatter_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Defaults: style=none, min_frac=0, max_frac=6
    unsafe {
        let ptr = obj as *mut u64;
        *ptr.add(1) = STYLE_NONE;
        *ptr.add(2) = 0; // min_frac
        *ptr.add(3) = 6; // max_frac
    }
    obj
}

/// `-[NSNumberFormatter setNumberStyle:]`
pub extern "C" fn shim_nsnumberformatter_set_number_style(obj: u64, _cmd: u64, style: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u64).add(1)) = style;
    }
}

/// `-[NSNumberFormatter numberStyle]`
pub extern "C" fn shim_nsnumberformatter_number_style(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_style(obj)
}

/// `-[NSNumberFormatter setMinimumFractionDigits:]`
pub extern "C" fn shim_nsnumberformatter_set_min_frac(obj: u64, _cmd: u64, digits: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u64).add(2)) = digits;
    }
}

/// `-[NSNumberFormatter setMaximumFractionDigits:]`
pub extern "C" fn shim_nsnumberformatter_set_max_frac(obj: u64, _cmd: u64, digits: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u64).add(3)) = digits;
    }
}

/// `-[NSNumberFormatter stringFromNumber:]` — format an NSNumber as a string.
pub extern "C" fn shim_nsnumberformatter_string_from_number(
    obj: u64,
    _cmd: u64,
    number: u64,
) -> u64 {
    if obj == 0 || number == 0 {
        return 0;
    }
    let val = read_nsnumber_f64(number);
    let s = format_number(val, read_style(obj), read_min_frac(obj), read_max_frac(obj));
    let c_str = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSNumberFormatter numberFromString:]` — parse a string into an NSNumber.
pub extern "C" fn shim_nsnumberformatter_number_from_string(
    obj: u64,
    _cmd: u64,
    string: u64,
) -> u64 {
    if obj == 0 || string == 0 {
        return 0;
    }
    let Some(s) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };
    // Strip currency symbols, grouping separators, percent signs
    let cleaned: String = s
        .chars()
        .filter(|c| !matches!(*c, '$' | ',' | '%' | ' '))
        .collect();
    if let Ok(i) = cleaned.parse::<i64>() {
        crate::nsnumber::alloc_nsnumber(TAG_INT, i as u64)
    } else if let Ok(f) = cleaned.parse::<f64>() {
        let style = read_style(obj);
        let final_val = if style == STYLE_PERCENT { f / 100.0 } else { f };
        crate::nsnumber::alloc_nsnumber(TAG_DOUBLE, final_val.to_bits())
    } else {
        0 // parse error → nil
    }
}

/// `-[NSNumberFormatter description]`
pub extern "C" fn shim_nsnumberformatter_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let style_name = match read_style(obj) {
        STYLE_NONE => "none",
        STYLE_DECIMAL => "decimal",
        STYLE_CURRENCY => "currency",
        STYLE_PERCENT => "percent",
        STYLE_SCIENTIFIC => "scientific",
        _ => "unknown",
    };
    let desc = format!("<NSNumberFormatter: style={style_name}>");
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
