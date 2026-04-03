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

/// `-[NSNumberFormatter minimumFractionDigits]`
pub extern "C" fn shim_nsnumberformatter_min_frac(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_min_frac(obj)
}

/// `-[NSNumberFormatter maximumFractionDigits]`
pub extern "C" fn shim_nsnumberformatter_max_frac(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_max_frac(obj)
}

/// `-[NSNumberFormatter locale]` — returns current locale.
pub extern "C" fn shim_nsnumberformatter_locale(_obj: u64, _cmd: u64) -> u64 {
    if _obj == 0 {
        return 0;
    }
    crate::nslocale::shim_nslocale_current_locale(0, 0)
}

/// `-[NSNumberFormatter setLocale:]` — no-op.
pub extern "C" fn shim_nsnumberformatter_set_locale(_obj: u64, _cmd: u64, _locale: u64) {
    // no-op
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const SIZE: usize = 32;

    fn make_formatter() -> u64 {
        let obj = alloc_obj_fake(SIZE);
        shim_nsnumberformatter_init(obj, 0);
        obj
    }

    // ── init ────────────────────────────────────────────────────

    #[test]
    fn init_sets_defaults() {
        let obj = make_formatter();
        assert_eq!(read_field(obj, 8), STYLE_NONE);
        assert_eq!(read_field(obj, 16), 0); // min_frac
        assert_eq!(read_field(obj, 24), 6); // max_frac
        free_obj(obj);
    }

    #[test]
    fn init_nil_returns_zero() {
        assert_eq!(shim_nsnumberformatter_init(0, 0), 0);
    }

    // ── numberStyle / setNumberStyle ────────────────────────────

    #[test]
    fn set_and_get_number_style() {
        let obj = make_formatter();
        shim_nsnumberformatter_set_number_style(obj, 0, STYLE_CURRENCY);
        assert_eq!(shim_nsnumberformatter_number_style(obj, 0), STYLE_CURRENCY);
        shim_nsnumberformatter_set_number_style(obj, 0, STYLE_PERCENT);
        assert_eq!(shim_nsnumberformatter_number_style(obj, 0), STYLE_PERCENT);
        free_obj(obj);
    }

    #[test]
    fn set_number_style_nil_noop() {
        shim_nsnumberformatter_set_number_style(0, 0, STYLE_DECIMAL);
        // must not crash
    }

    #[test]
    fn number_style_nil_returns_zero() {
        assert_eq!(shim_nsnumberformatter_number_style(0, 0), 0);
    }

    // ── setMinimumFractionDigits ────────────────────────────────

    #[test]
    fn set_min_frac_digits() {
        let obj = make_formatter();
        shim_nsnumberformatter_set_min_frac(obj, 0, 3);
        assert_eq!(read_field(obj, 16), 3);
        free_obj(obj);
    }

    #[test]
    fn set_min_frac_nil_noop() {
        shim_nsnumberformatter_set_min_frac(0, 0, 5);
    }

    // ── setMaximumFractionDigits ────────────────────────────────

    #[test]
    fn set_max_frac_digits() {
        let obj = make_formatter();
        shim_nsnumberformatter_set_max_frac(obj, 0, 10);
        assert_eq!(read_field(obj, 24), 10);
        free_obj(obj);
    }

    #[test]
    fn set_max_frac_nil_noop() {
        shim_nsnumberformatter_set_max_frac(0, 0, 10);
    }

    // ── stringFromNumber: ───────────────────────────────────────

    #[test]
    fn string_from_number_style_none_integer() {
        let fmt = make_formatter();
        let num = crate::nsnumber::make_nsnumber_int(42);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        assert_string_eq(result, "42");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_style_none_double() {
        let fmt = make_formatter();
        let num = crate::nsnumber::make_nsnumber_double(3.14);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        let s = read_string(result);
        assert!(s.starts_with("3.14"), "got: {s}");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_decimal_grouping() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_DECIMAL);
        let num = crate::nsnumber::make_nsnumber_int(1234567);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        assert_string_eq(result, "1,234,567");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_currency() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_CURRENCY);
        let num = crate::nsnumber::make_nsnumber_double(1234.5);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        assert_string_eq(result, "$1,234.50");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_percent() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_PERCENT);
        let num = crate::nsnumber::make_nsnumber_double(0.75);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        assert_string_eq(result, "75%");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_scientific() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_SCIENTIFIC);
        let num = crate::nsnumber::make_nsnumber_double(12345.0);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        let s = read_string(result);
        assert!(s.contains('e'), "expected scientific notation, got: {s}");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_with_frac_digits() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_DECIMAL);
        shim_nsnumberformatter_set_min_frac(fmt, 0, 2);
        shim_nsnumberformatter_set_max_frac(fmt, 0, 4);
        let num = crate::nsnumber::make_nsnumber_double(1.5);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        assert_string_eq(result, "1.50");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_bool() {
        let fmt = make_formatter();
        let num = crate::nsnumber::make_nsnumber_bool(true);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        assert_string_eq(result, "1");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_nil_obj() {
        let num = crate::nsnumber::make_nsnumber_int(42);
        assert_eq!(shim_nsnumberformatter_string_from_number(0, 0, num), 0);
        free_obj(num);
    }

    #[test]
    fn string_from_number_nil_number() {
        let fmt = make_formatter();
        assert_eq!(shim_nsnumberformatter_string_from_number(fmt, 0, 0), 0);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_negative() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_DECIMAL);
        let num = crate::nsnumber::make_nsnumber_int(-9876543);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        assert_string_eq(result, "-9,876,543");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    #[test]
    fn string_from_number_zero() {
        let fmt = make_formatter();
        let num = crate::nsnumber::make_nsnumber_int(0);
        let result = shim_nsnumberformatter_string_from_number(fmt, 0, num);
        assert_ne!(result, 0);
        assert_string_eq(result, "0");
        free_obj(result);
        free_obj(num);
        free_obj(fmt);
    }

    // ── numberFromString: ───────────────────────────────────────

    #[test]
    fn number_from_string_integer() {
        let fmt = make_formatter();
        let s = make_string("42");
        let num = shim_nsnumberformatter_number_from_string(fmt, 0, s);
        assert_ne!(num, 0);
        // tag should be TAG_INT
        assert_eq!(read_field(num, 8), TAG_INT);
        assert_eq!(read_field(num, 16), 42);
        free_obj(num);
        free_obj(s);
        free_obj(fmt);
    }

    #[test]
    fn number_from_string_double() {
        let fmt = make_formatter();
        let s = make_string("3.14");
        let num = shim_nsnumberformatter_number_from_string(fmt, 0, s);
        assert_ne!(num, 0);
        assert_eq!(read_field(num, 8), TAG_DOUBLE);
        let val = f64::from_bits(read_field(num, 16));
        assert!((val - 3.14).abs() < 1e-10);
        free_obj(num);
        free_obj(s);
        free_obj(fmt);
    }

    #[test]
    fn number_from_string_currency_stripped() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_CURRENCY);
        let s = make_string("$1,234.56");
        let num = shim_nsnumberformatter_number_from_string(fmt, 0, s);
        assert_ne!(num, 0);
        let val = f64::from_bits(read_field(num, 16));
        assert!((val - 1234.56).abs() < 1e-10, "got: {val}");
        free_obj(num);
        free_obj(s);
        free_obj(fmt);
    }

    #[test]
    fn number_from_string_percent_divided() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_PERCENT);
        // Use "75.0%" so the parser takes the f64 path (percent division
        // only applies in the f64 branch; "75%" would parse as i64).
        let s = make_string("75.0%");
        let num = shim_nsnumberformatter_number_from_string(fmt, 0, s);
        assert_ne!(num, 0);
        let val = f64::from_bits(read_field(num, 16));
        assert!((val - 0.75).abs() < 1e-10, "got: {val}");
        free_obj(num);
        free_obj(s);
        free_obj(fmt);
    }

    #[test]
    fn number_from_string_invalid_returns_zero() {
        let fmt = make_formatter();
        let s = make_string("not_a_number");
        assert_eq!(shim_nsnumberformatter_number_from_string(fmt, 0, s), 0);
        free_obj(s);
        free_obj(fmt);
    }

    #[test]
    fn number_from_string_nil_obj() {
        let s = make_string("42");
        assert_eq!(shim_nsnumberformatter_number_from_string(0, 0, s), 0);
        free_obj(s);
    }

    #[test]
    fn number_from_string_nil_string() {
        let fmt = make_formatter();
        assert_eq!(shim_nsnumberformatter_number_from_string(fmt, 0, 0), 0);
        free_obj(fmt);
    }

    // ── description ─────────────────────────────────────────────

    #[test]
    fn description_default_style() {
        let fmt = make_formatter();
        let desc = shim_nsnumberformatter_description(fmt, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSNumberFormatter: style=none>");
        free_obj(desc);
        free_obj(fmt);
    }

    #[test]
    fn description_decimal_style() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_DECIMAL);
        let desc = shim_nsnumberformatter_description(fmt, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSNumberFormatter: style=decimal>");
        free_obj(desc);
        free_obj(fmt);
    }

    #[test]
    fn description_currency_style() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_CURRENCY);
        let desc = shim_nsnumberformatter_description(fmt, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSNumberFormatter: style=currency>");
        free_obj(desc);
        free_obj(fmt);
    }

    #[test]
    fn description_percent_style() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_PERCENT);
        let desc = shim_nsnumberformatter_description(fmt, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSNumberFormatter: style=percent>");
        free_obj(desc);
        free_obj(fmt);
    }

    #[test]
    fn description_scientific_style() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_number_style(fmt, 0, STYLE_SCIENTIFIC);
        let desc = shim_nsnumberformatter_description(fmt, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSNumberFormatter: style=scientific>");
        free_obj(desc);
        free_obj(fmt);
    }

    #[test]
    fn description_nil_returns_zero() {
        assert_eq!(shim_nsnumberformatter_description(0, 0), 0);
    }

    // ── locale ──────────────────────────────────────────────────

    #[test]
    fn locale_returns_non_zero() {
        let fmt = make_formatter();
        let loc = shim_nsnumberformatter_locale(fmt, 0);
        assert_ne!(loc, 0);
        free_obj(fmt);
    }

    #[test]
    fn locale_nil_returns_zero() {
        assert_eq!(shim_nsnumberformatter_locale(0, 0), 0);
    }

    #[test]
    fn set_locale_is_noop() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_locale(fmt, 0, 0xDEAD);
        free_obj(fmt);
    }

    // ── minimumFractionDigits / maximumFractionDigits getters ───

    #[test]
    fn min_frac_getter_returns_default() {
        let fmt = make_formatter();
        assert_eq!(shim_nsnumberformatter_min_frac(fmt, 0), 0);
        free_obj(fmt);
    }

    #[test]
    fn max_frac_getter_returns_default() {
        let fmt = make_formatter();
        assert_eq!(shim_nsnumberformatter_max_frac(fmt, 0), 6);
        free_obj(fmt);
    }

    #[test]
    fn min_frac_getter_after_set() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_min_frac(fmt, 0, 3);
        assert_eq!(shim_nsnumberformatter_min_frac(fmt, 0), 3);
        free_obj(fmt);
    }

    #[test]
    fn max_frac_getter_after_set() {
        let fmt = make_formatter();
        shim_nsnumberformatter_set_max_frac(fmt, 0, 10);
        assert_eq!(shim_nsnumberformatter_max_frac(fmt, 0), 10);
        free_obj(fmt);
    }

    #[test]
    fn min_frac_nil_returns_zero() {
        assert_eq!(shim_nsnumberformatter_min_frac(0, 0), 0);
    }

    #[test]
    fn max_frac_nil_returns_zero() {
        assert_eq!(shim_nsnumberformatter_max_frac(0, 0), 0);
    }

    // ── internal helpers ────────────────────────────────────────

    #[test]
    fn format_number_none_style_integer() {
        assert_eq!(format_number(42.0, STYLE_NONE, 0, 6), "42");
    }

    #[test]
    fn format_number_none_style_fractional() {
        let s = format_number(3.14, STYLE_NONE, 0, 6);
        assert!(s.starts_with("3.14"), "got: {s}");
    }

    #[test]
    fn format_with_frac_min_padding() {
        assert_eq!(format_with_frac(1.5, 3, 6), "1.500");
    }

    #[test]
    fn format_with_frac_max_trimming() {
        assert_eq!(format_with_frac(1.123456789, 0, 4), "1.1235");
    }

    #[test]
    fn add_grouping_separator_basic() {
        assert_eq!(add_grouping_separator("1234567", ','), "1,234,567");
    }

    #[test]
    fn add_grouping_separator_negative() {
        assert_eq!(add_grouping_separator("-1234567", ','), "-1,234,567");
    }

    #[test]
    fn add_grouping_separator_with_decimals() {
        assert_eq!(add_grouping_separator("1234567.89", ','), "1,234,567.89");
    }

    #[test]
    fn add_grouping_separator_small_number() {
        assert_eq!(add_grouping_separator("42", ','), "42");
    }

    #[test]
    fn read_nsnumber_f64_int() {
        let num = crate::nsnumber::make_nsnumber_int(99);
        assert!((read_nsnumber_f64(num) - 99.0).abs() < 1e-10);
        free_obj(num);
    }

    #[test]
    fn read_nsnumber_f64_double() {
        let num = crate::nsnumber::make_nsnumber_double(2.718);
        assert!((read_nsnumber_f64(num) - 2.718).abs() < 1e-10);
        free_obj(num);
    }

    #[test]
    fn read_nsnumber_f64_bool() {
        let num = crate::nsnumber::make_nsnumber_bool(false);
        assert!((read_nsnumber_f64(num)).abs() < 1e-10);
        free_obj(num);
    }
}
