use crate::cfstring;
use crate::nsdictionary;
use crate::nsnumber;
use crate::nsobj;
use regex::Regex;

// ── KVC helper: extract keypath value from a dict ──────────────────

/// Given an object and a keypath string, extract the value using objectForKey:.
fn kvc_value(obj: u64, keypath: &str) -> u64 {
    if obj == 0 {
        return 0;
    }
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
    if obj == 0 {
        return 0;
    }
    if obj & (1u64 << 63) != 0 {
        // Tagged pointer NSNumber
        return crate::nsnumber::tagged_int_value(obj);
    }
    let (tag, raw) = nsnumber::read_number_pub(obj);
    match tag {
        0 | 2 => raw as i64,             // int or bool
        1 => f64::from_bits(raw) as i64, // double
        _ => raw as i64,
    }
}

/// Read a string value from an NSString.
fn str_value(obj: u64) -> Option<String> {
    if obj == 0 {
        return None;
    }
    cfstring::extract_cfstring_cstr(obj).map(|s| s.to_string())
}

// ── Format parser ──────────────────────────────────────────────────

pub(super) fn parse_format(fmt: &str, arg0: u64) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let trimmed = fmt.trim();

    if trimmed.eq_ignore_ascii_case("TRUEPREDICATE") {
        return Box::new(|_| true);
    }
    if trimmed.eq_ignore_ascii_case("FALSEPREDICATE") {
        return Box::new(|_| false);
    }

    // Compound: split on OR first (lower precedence), then AND (higher precedence)
    let upper = trimmed.to_uppercase();
    if upper.contains(" OR ") {
        let parts = split_on_keyword(trimmed, " OR ");
        if parts.len() >= 2 {
            let preds: Vec<Box<dyn Fn(u64) -> bool + Send + Sync>> = parts
                .into_iter()
                .map(|p| parse_format(p.trim(), arg0))
                .collect();
            return Box::new(move |obj| preds.iter().any(|p| p(obj)));
        }
    }

    if upper.contains(" AND ") {
        let parts = split_on_keyword(trimmed, " AND ");
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

/// Split a predicate string on a keyword (case-insensitive).
fn split_on_keyword<'a>(s: &'a str, keyword: &str) -> Vec<&'a str> {
    let upper = s.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let kw_len = kw_upper.len();
    let mut parts = Vec::new();
    let mut start = 0;
    while let Some(idx) = upper[start..].find(&kw_upper) {
        parts.push(&s[start..start + idx]);
        start = start + idx + kw_len;
    }
    parts.push(&s[start..]);
    parts
}

fn parse_single_predicate(expr: &str, arg0: u64) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let trimmed = expr.trim();
    let up = trimmed.to_uppercase();

    // NOT prefix: NOT expr or !(expr)
    if up.starts_with("NOT ") {
        let inner = &trimmed[4..];
        let inner_pred = parse_single_predicate(inner.trim(), arg0);
        return Box::new(move |obj| !inner_pred(obj));
    }
    if up.starts_with("!(") && trimmed.ends_with(')') {
        let inner = &trimmed[2..trimmed.len() - 1];
        let inner_pred = parse_single_predicate(inner.trim(), arg0);
        return Box::new(move |obj| !inner_pred(obj));
    }

    // SELF-based predicates
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
    for op in &[
        "BEGINSWITH",
        "CONTAINS[c]",
        "CONTAINS",
        "ENDSWITH",
        "IN",
        "BETWEEN",
        "LIKE[c]",
        "LIKE",
        "MATCHES",
    ] {
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
            let val_str = cfstring::extract_cfstring_cstr(arg0)
                .unwrap_or("")
                .to_string();
            return Box::new(move |obj| {
                cfstring::extract_cfstring_cstr(obj)
                    .map(|s| s == val_str)
                    .unwrap_or(false)
            });
        }
        let val = strip_quotes(rhs).to_string();
        return Box::new(move |obj| {
            cfstring::extract_cfstring_cstr(obj)
                .map(|s| s == val)
                .unwrap_or(false)
        });
    }

    // Check SELF string operators
    struct SelfOp {
        kw: &'static str,
    }
    let ops = [
        SelfOp { kw: "CONTAINS" },
        SelfOp { kw: "BEGINSWITH" },
        SelfOp { kw: "ENDSWITH" },
        SelfOp { kw: "LIKE[c]" },
        SelfOp { kw: "LIKE" },
        SelfOp { kw: "MATCHES" },
    ];
    for op in &ops {
        let pat = format!("SELF {}", op.kw);
        if up.contains(&pat) {
            let idx = up.find(&pat).unwrap();
            let rhs = expr[idx + pat.len()..].trim();
            let val = if rhs == "%@" {
                cfstring::extract_cfstring_cstr(arg0)
                    .unwrap_or("")
                    .to_string()
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
                    "LIKE[c]" => like_match(&s, &val, true),
                    "LIKE" => like_match(&s, &val, false),
                    "MATCHES" => Regex::new(&val).map(|r| r.is_match(&s)).unwrap_or(false),
                    _ => false,
                }
            });
        }
    }

    log::warn!("NSPredicate: unsupported SELF predicate: '{expr}'");
    Box::new(|_| false)
}

fn make_comparison_predicate(
    keypath: &str,
    op: &str,
    rhs: &str,
    arg0: u64,
) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let kp = keypath.to_string();

    if rhs.trim() == "%@" {
        // RHS is the variadic argument (an NSString or NSNumber)
        let rhs_str = cfstring::extract_cfstring_cstr(arg0).map(|s| s.to_string());
        let rhs_num = num_value(arg0);
        let op = op.to_string();
        return Box::new(move |obj| {
            let val = kvc_value(obj, &kp);
            if val == 0 {
                return false;
            }
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
        if val == 0 {
            return false;
        }
        let n = num_value(val);
        compare_nums(n, rhs_val, &op)
    })
}

fn make_string_predicate(
    keypath: &str,
    op: &str,
    rhs: &str,
    arg0: u64,
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
                    Some(s) if case_insensitive => {
                        s.to_lowercase().starts_with(&expected.to_lowercase())
                    }
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
                    Some(s) if case_insensitive => {
                        s.to_lowercase().contains(&expected.to_lowercase())
                    }
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
                    Some(s) if case_insensitive => {
                        s.to_lowercase().ends_with(&expected.to_lowercase())
                    }
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
                if val == 0 {
                    return false;
                }
                let n = num_value(val);
                n >= lo && n <= hi
            })
        }
        "LIKE" => {
            let pattern = resolve_string_arg(rhs_trimmed, arg0);
            Box::new(move |obj| {
                let val = kvc_value(obj, &kp);
                match str_value(val) {
                    Some(s) => like_match(&s, &pattern, case_insensitive),
                    None => false,
                }
            })
        }
        "MATCHES" => {
            let pattern_str = resolve_string_arg(rhs_trimmed, arg0);
            let re = Regex::new(&pattern_str).ok();
            Box::new(move |obj| {
                let val = kvc_value(obj, &kp);
                match (str_value(val), &re) {
                    (Some(s), Some(r)) => r.is_match(&s),
                    _ => false,
                }
            })
        }
        _ => {
            log::warn!("NSPredicate: unsupported string op '{op}'");
            Box::new(|_| false)
        }
    }
}

/// NSPredicate LIKE pattern matching: `*` = any sequence, `?` = any single char.
fn like_match(s: &str, pattern: &str, case_insensitive: bool) -> bool {
    let (s, pattern) = if case_insensitive {
        (s.to_lowercase(), pattern.to_lowercase())
    } else {
        (s.to_string(), pattern.to_string())
    };
    // Convert LIKE pattern to regex: escape regex chars, * → .*, ? → .
    let mut regex_pat = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex_pat.push_str(".*"),
            '?' => regex_pat.push('.'),
            c if ".+^${}[]|()\\".contains(c) => {
                regex_pat.push('\\');
                regex_pat.push(c);
            }
            c => regex_pat.push(c),
        }
    }
    regex_pat.push('$');
    Regex::new(&regex_pat)
        .map(|r| r.is_match(&s))
        .unwrap_or(false)
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
        cfstring::extract_cfstring_cstr(arg0)
            .unwrap_or("")
            .to_string()
    } else {
        strip_quotes(rhs).to_string()
    }
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Extract strings from an NSArray.
fn extract_string_array(arr: u64) -> Vec<String> {
    let mut result = Vec::new();
    if arr == 0 {
        return result;
    }
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
    if arr == 0 {
        return (0, 0);
    }
    let isa = unsafe { *(arr as *const u64) };
    if isa != nsobj::nsarray_class_addr() && isa != nsobj::nsmutablearray_class_addr() {
        return (0, 0);
    }
    let count = unsafe { *((arr + 8) as *const u64) } as usize;
    let elems = unsafe { *((arr + 16) as *const u64) };
    if count < 2 {
        return (0, 0);
    }
    let lo = unsafe { *((elems as *const u64).add(0)) };
    let hi = unsafe { *((elems as *const u64).add(1)) };
    (num_value(lo), num_value(hi))
}
