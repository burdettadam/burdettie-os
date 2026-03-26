//! NSPredicate shim: predicate evaluation for filtered collections.
//!
//! Object layout (32 bytes):
//!   offset  0: isa           (u64)
//!   offset  8: format_ptr    (u64 → NSString)
//!   offset 16: predicate_fn  (u64 → leaked Box<dyn Fn(u64) -> bool>)
//!   offset 24: reserved      (u64)
//!
//! Supports keypath-based predicates evaluated against NSDictionary objects:
//!   - `keypath > N`, `keypath >= N`, `keypath < N`, `keypath <= N`
//!   - `keypath == %@`, `keypath == 'value'`
//!   - `keypath BEGINSWITH %@`, `keypath CONTAINS %@`, `keypath CONTAINS[c] %@`
//!   - `keypath IN %@`, `keypath BETWEEN %@`
//!   - Compound: `expr1 AND expr2`
//!   - `TRUEPREDICATE`, `FALSEPREDICATE`
//!   - `SELF == 'value'`, `SELF CONTAINS 'value'`

use crate::cfstring;
use crate::nsdictionary;
use crate::nsnumber;
use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 32;

fn make_predicate(format: u64, pred_fn: Box<dyn Fn(u64) -> bool + Send + Sync>) -> u64 {
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nspredicate_class_addr();
        *((obj + 8) as *mut u64) = format;
        *((obj + 16) as *mut u64) = Box::into_raw(Box::new(pred_fn)) as u64;
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────────

/// ARM64 Apple wrapper: variadic args live on the stack, not in registers.
/// This naked function loads the first variadic arg from [sp] into x3
/// before tail-calling the real implementation.
#[cfg(target_arch = "aarch64")]
#[unsafe(naked)]
pub unsafe extern "C" fn shim_predicate_with_format() {
    core::arch::naked_asm!(
        "ldr x3, [sp]",
        "b {inner}",
        inner = sym shim_predicate_with_format_inner,
    );
}

#[cfg(not(target_arch = "aarch64"))]
pub extern "C" fn shim_predicate_with_format(
    _cls: u64, _cmd: u64, format: u64, arg0: u64,
) -> u64 {
    shim_predicate_with_format_inner(_cls, _cmd, format, arg0)
}

/// `+[NSPredicate predicateWithFormat:]` — inner implementation.
pub extern "C" fn shim_predicate_with_format_inner(
    _cls: u64,
    _cmd: u64,
    format: u64,
    arg0: u64,
) -> u64 {
    let fmt_str = match cfstring::extract_cfstring_cstr(format) {
        Some(s) => s.to_string(),
        None => return 0,
    };

    let predicate_fn = parse_format(&fmt_str, arg0);
    make_predicate(format, predicate_fn)
}

/// `+[NSPredicate predicateWithValue:]` — always-true or always-false.
pub extern "C" fn shim_predicate_with_value(_cls: u64, _cmd: u64, value: u8) -> u64 {
    let val = value != 0;
    make_predicate(0, Box::new(move |_| val))
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSPredicate evaluateWithObject:]`
pub extern "C" fn shim_predicate_evaluate(obj: u64, _cmd: u64, target: u64) -> u8 {
    if obj == 0 { return 0; }
    let fn_ptr = unsafe { *((obj + 16) as *const u64) };
    if fn_ptr == 0 { return 0; }
    let predicate: &(dyn Fn(u64) -> bool + Send + Sync) =
        unsafe { &*(fn_ptr as *const Box<dyn Fn(u64) -> bool + Send + Sync>) };
    u8::from(predicate(target))
}

/// `-[NSPredicate predicateFormat]`
pub extern "C" fn shim_predicate_format(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 8) as *const u64) }
}

/// `-[NSPredicate description]`
pub extern "C" fn shim_predicate_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let fmt = unsafe { *((obj + 8) as *const u64) };
    if fmt != 0 { return fmt; }
    let c_str = c"<NSPredicate>";
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

// ── KVC helper: extract keypath value from a dict ──────────────────

/// Given an object and a keypath string, extract the value using objectForKey:.
fn kvc_value(obj: u64, keypath: &str) -> u64 {
    if obj == 0 { return 0; }
    let isa = unsafe { *(obj as *const u64) };
    if isa == nsobj::nsdictionary_class_addr()
        || isa == nsobj::nsmutabledictionary_class_addr()
        || isa == nsobj::nsconstantdictionary_class_addr()
    {
        // Create an NSString key and call objectForKey logic
        let count = nsdictionary::helpers_pub::dict_count_pub(obj) as usize;
        let keys_ptr = nsdictionary::helpers_pub::dict_keys_pub(obj);
        let vals_ptr = nsdictionary::helpers_pub::dict_vals_pub(obj);
        for i in 0..count {
            let k = unsafe { *((keys_ptr as *const u64).add(i)) };
            if let Some(key_str) = cfstring::extract_cfstring_cstr(k) {
                if key_str == keypath {
                    return unsafe { *((vals_ptr as *const u64).add(i)) };
                }
            }
        }
    }
    0
}

/// Read an integer from an NSNumber (handles tagged pointers and constant nums).
fn num_value(obj: u64) -> i64 {
    if obj == 0 { return 0; }
    if obj & (1u64 << 63) != 0 {
        // Tagged pointer NSNumber
        return crate::nsnumber::tagged_int_value(obj);
    }
    let (tag, raw) = nsnumber::read_number_pub(obj);
    match tag {
        0 | 2 => raw as i64,  // int or bool
        1 => f64::from_bits(raw) as i64,  // double
        _ => raw as i64,
    }
}

/// Read a string value from an NSString.
fn str_value(obj: u64) -> Option<String> {
    if obj == 0 { return None; }
    cfstring::extract_cfstring_cstr(obj).map(|s| s.to_string())
}

// ── Format parser ──────────────────────────────────────────────────

fn parse_format(fmt: &str, arg0: u64) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let trimmed = fmt.trim();

    if trimmed.eq_ignore_ascii_case("TRUEPREDICATE") {
        return Box::new(|_| true);
    }
    if trimmed.eq_ignore_ascii_case("FALSEPREDICATE") {
        return Box::new(|_| false);
    }

    // Compound: split on AND (case-insensitive)
    let upper = trimmed.to_uppercase();
    if upper.contains(" AND ") {
        let parts: Vec<&str> = split_on_and(trimmed);
        if parts.len() >= 2 {
            let preds: Vec<Box<dyn Fn(u64) -> bool + Send + Sync>> = parts
                .into_iter()
                .map(|p| parse_single_predicate(p.trim(), arg0))
                .collect();
            return Box::new(move |obj| preds.iter().all(|p| p(obj)));
        }
    }

    parse_single_predicate(trimmed, arg0)
}

/// Split a predicate string on ` AND ` (case-insensitive).
fn split_on_and(s: &str) -> Vec<&str> {
    let upper = s.to_uppercase();
    let mut parts = Vec::new();
    let mut start = 0;
    while let Some(idx) = upper[start..].find(" AND ") {
        parts.push(&s[start..start + idx]);
        start = start + idx + 5; // " AND ".len()
    }
    parts.push(&s[start..]);
    parts
}

fn parse_single_predicate(expr: &str, arg0: u64) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let trimmed = expr.trim();

    // SELF-based predicates
    let up = trimmed.to_uppercase();
    if up.starts_with("SELF ") {
        return parse_self_predicate(trimmed, arg0);
    }

    // Try keypath-based: `keypath OP value|%@`
    // Supported operators: >, >=, <, <=, ==, !=, BEGINSWITH, CONTAINS, CONTAINS[c], IN, BETWEEN
    for op in &[">=", "<=", "!=", ">", "<", "=="] {
        if let Some(idx) = trimmed.find(op) {
            let keypath = trimmed[..idx].trim().to_string();
            let rhs = trimmed[idx + op.len()..].trim();
            return make_comparison_predicate(&keypath, op, rhs, arg0);
        }
    }

    // String operators with keypath
    for op in &["BEGINSWITH", "CONTAINS[c]", "CONTAINS", "ENDSWITH", "IN", "BETWEEN"] {
        let up_expr = trimmed.to_uppercase();
        if let Some(idx) = up_expr.find(&format!(" {} ", op.to_uppercase())) {
            let keypath = trimmed[..idx].trim().to_string();
            let rhs = trimmed[idx + op.len() + 2..].trim(); // +2 for spaces
            return make_string_predicate(&keypath, op, rhs, arg0);
        }
    }

    log::warn!("NSPredicate: unsupported format '{trimmed}', returning FALSEPREDICATE");
    Box::new(|_| false)
}

fn parse_self_predicate(expr: &str, arg0: u64) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let up = expr.to_uppercase();

    // SELF == %@ or SELF == 'value'
    if let Some(idx) = up.find("SELF ==") {
        let rhs = expr[idx + 7..].trim();
        if rhs == "%@" {
            let val_str = cfstring::extract_cfstring_cstr(arg0).unwrap_or("").to_string();
            return Box::new(move |obj| {
                cfstring::extract_cfstring_cstr(obj).map(|s| s == val_str).unwrap_or(false)
            });
        }
        let val = strip_quotes(rhs).to_string();
        return Box::new(move |obj| {
            cfstring::extract_cfstring_cstr(obj).map(|s| s == val).unwrap_or(false)
        });
    }

    // Check SELF string operators
    struct SelfOp { kw: &'static str }
    let ops = [SelfOp { kw: "CONTAINS" }, SelfOp { kw: "BEGINSWITH" }, SelfOp { kw: "ENDSWITH" }];
    for op in &ops {
        let pat = format!("SELF {}", op.kw);
        if up.contains(&pat) {
            let idx = up.find(&pat).unwrap();
            let rhs = expr[idx + pat.len()..].trim();
            let val = if rhs == "%@" {
                cfstring::extract_cfstring_cstr(arg0).unwrap_or("").to_string()
            } else {
                strip_quotes(rhs).to_string()
            };
            let kw = op.kw.to_string();
            return Box::new(move |obj| {
                let s = match cfstring::extract_cfstring_cstr(obj) {
                    Some(s) => s.to_string(),
                    None => return false,
                };
                match kw.as_str() {
                    "CONTAINS" => s.contains(&*val),
                    "BEGINSWITH" => s.starts_with(&*val),
                    "ENDSWITH" => s.ends_with(&*val),
                    _ => false,
                }
            });
        }
    }

    log::warn!("NSPredicate: unsupported SELF predicate: '{expr}'");
    Box::new(|_| false)
}

fn make_comparison_predicate(
    keypath: &str, op: &str, rhs: &str, arg0: u64,
) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let kp = keypath.to_string();

    if rhs.trim() == "%@" {
        // RHS is the variadic argument (an NSString or NSNumber)
        let rhs_str = cfstring::extract_cfstring_cstr(arg0).map(|s| s.to_string());
        let rhs_num = num_value(arg0);
        let op = op.to_string();
        return Box::new(move |obj| {
            let val = kvc_value(obj, &kp);
            if val == 0 { return false; }
            // Try string comparison first
            if let Some(ref expected) = rhs_str {
                if let Some(actual) = str_value(val) {
                    return match op.as_str() {
                        "==" => actual == *expected,
                        "!=" => actual != *expected,
                        _ => false,
                    };
                }
            }
            // Numeric comparison
            let n = num_value(val);
            compare_nums(n, rhs_num, &op)
        });
    }

    // RHS is a literal number
    let rhs_val: i64 = rhs.trim().parse().unwrap_or(0);
    let op = op.to_string();
    Box::new(move |obj| {
        let val = kvc_value(obj, &kp);
        if val == 0 { return false; }
        let n = num_value(val);
        compare_nums(n, rhs_val, &op)
    })
}

fn make_string_predicate(
    keypath: &str, op: &str, rhs: &str, arg0: u64,
) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let kp = keypath.to_string();
    let op_up = op.to_uppercase();

    // Resolve rhs value
    let rhs_trimmed = rhs.trim();
    let case_insensitive = op_up.contains("[C]");

    match op_up.replace("[C]", "").trim() {
        "BEGINSWITH" => {
            let expected = resolve_string_arg(rhs_trimmed, arg0);
            Box::new(move |obj| {
                let val = kvc_value(obj, &kp);
                match str_value(val) {
                    Some(s) if case_insensitive =>
                        s.to_lowercase().starts_with(&expected.to_lowercase()),
                    Some(s) => s.starts_with(&*expected),
                    None => false,
                }
            })
        }
        "CONTAINS" => {
            let expected = resolve_string_arg(rhs_trimmed, arg0);
            Box::new(move |obj| {
                let val = kvc_value(obj, &kp);
                match str_value(val) {
                    Some(s) if case_insensitive =>
                        s.to_lowercase().contains(&expected.to_lowercase()),
                    Some(s) => s.contains(&*expected),
                    None => false,
                }
            })
        }
        "ENDSWITH" => {
            let expected = resolve_string_arg(rhs_trimmed, arg0);
            Box::new(move |obj| {
                let val = kvc_value(obj, &kp);
                match str_value(val) {
                    Some(s) if case_insensitive =>
                        s.to_lowercase().ends_with(&expected.to_lowercase()),
                    Some(s) => s.ends_with(&*expected),
                    None => false,
                }
            })
        }
        "IN" => {
            // arg0 is an NSArray of allowed values
            let allowed = extract_string_array(arg0);
            Box::new(move |obj| {
                let val = kvc_value(obj, &kp);
                if let Some(s) = str_value(val) {
                    allowed.contains(&s)
                } else {
                    false
                }
            })
        }
        "BETWEEN" => {
            // arg0 is an NSArray with [low, high]
            let (lo, hi) = extract_between_range(arg0);
            Box::new(move |obj| {
                let val = kvc_value(obj, &kp);
                if val == 0 { return false; }
                let n = num_value(val);
                n >= lo && n <= hi
            })
        }
        _ => {
            log::warn!("NSPredicate: unsupported string op '{op}'");
            Box::new(|_| false)
        }
    }
}

fn compare_nums(lhs: i64, rhs: i64, op: &str) -> bool {
    match op {
        ">" => lhs > rhs,
        ">=" => lhs >= rhs,
        "<" => lhs < rhs,
        "<=" => lhs <= rhs,
        "==" => lhs == rhs,
        "!=" => lhs != rhs,
        _ => false,
    }
}

fn resolve_string_arg(rhs: &str, arg0: u64) -> String {
    if rhs == "%@" {
        cfstring::extract_cfstring_cstr(arg0).unwrap_or("").to_string()
    } else {
        strip_quotes(rhs).to_string()
    }
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('\'') && s.ends_with('\''))
        || (s.starts_with('"') && s.ends_with('"'))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Extract strings from an NSArray.
fn extract_string_array(arr: u64) -> Vec<String> {
    let mut result = Vec::new();
    if arr == 0 { return result; }
    let isa = unsafe { *(arr as *const u64) };
    if isa != nsobj::nsarray_class_addr() && isa != nsobj::nsmutablearray_class_addr() {
        return result;
    }
    let count = unsafe { *((arr + 8) as *const u64) } as usize;
    let elems = unsafe { *((arr + 16) as *const u64) };
    for i in 0..count {
        let elem = unsafe { *((elems as *const u64).add(i)) };
        if let Some(s) = cfstring::extract_cfstring_cstr(elem) {
            result.push(s.to_string());
        }
    }
    result
}

/// Extract [low, high] from an NSArray of two NSNumbers.
fn extract_between_range(arr: u64) -> (i64, i64) {
    if arr == 0 { return (0, 0); }
    let isa = unsafe { *(arr as *const u64) };
    if isa != nsobj::nsarray_class_addr() && isa != nsobj::nsmutablearray_class_addr() {
        return (0, 0);
    }
    let count = unsafe { *((arr + 8) as *const u64) } as usize;
    let elems = unsafe { *((arr + 16) as *const u64) };
    if count < 2 { return (0, 0); }
    let lo = unsafe { *((elems as *const u64).add(0)) };
    let hi = unsafe { *((elems as *const u64).add(1)) };
    (num_value(lo), num_value(hi))
}
