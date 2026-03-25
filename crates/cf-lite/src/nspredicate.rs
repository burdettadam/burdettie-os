//! NSPredicate shim: predicate evaluation for filtered collections.
//!
//! Object layout (32 bytes):
//!   offset  0: isa           (u64)
//!   offset  8: format_ptr    (u64 → NSString)
//!   offset 16: predicate_fn  (u64 → leaked Box<dyn Fn(u64) -> bool>)
//!   offset 24: reserved      (u64)
//!
//! Supports:
//!   - `predicateWithFormat:` — parses `TRUEPREDICATE`, `FALSEPREDICATE`,
//!     `SELF == <value>`, `SELF CONTAINS <value>`, `SELF BEGINSWITH <value>`
//!   - `predicateWithBlock:` — captures a block for evaluation
//!   - `evaluateWithObject:` — tests an object
//!
//! This is a simplified subset; real NSPredicate is far more complex.

use crate::cfstring;
use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 32;

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSPredicate predicateWithFormat:]`
///
/// Parses a simple format string. Supported forms:
///   - `TRUEPREDICATE` / `FALSEPREDICATE`
///   - `SELF == 'value'` / `SELF == "value"`
///   - `SELF CONTAINS 'value'`
///   - `SELF BEGINSWITH 'value'`
///   - `SELF ENDSWITH 'value'`
pub extern "C" fn shim_predicate_with_format(
    _cls: u64,
    _cmd: u64,
    format: u64,
) -> u64 {
    let fmt_str = match cfstring::extract_cfstring_cstr(format) {
        Some(s) => s.to_string(),
        None => return 0,
    };

    let predicate_fn: Box<dyn Fn(u64) -> bool + Send + Sync> = parse_format(&fmt_str);

    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    unsafe {
        *(obj as *mut u64) = nsobj::nspredicate_class_addr();
        *((obj + 8) as *mut u64) = format;
        *((obj + 16) as *mut u64) = Box::into_raw(Box::new(predicate_fn)) as u64;
    }
    obj
}

/// `+[NSPredicate predicateWithValue:]` — always-true or always-false.
pub extern "C" fn shim_predicate_with_value(_cls: u64, _cmd: u64, value: u8) -> u64 {
    let val = value != 0;
    let predicate_fn: Box<dyn Fn(u64) -> bool + Send + Sync> = Box::new(move |_| val);

    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    unsafe {
        *(obj as *mut u64) = nsobj::nspredicate_class_addr();
        *((obj + 16) as *mut u64) = Box::into_raw(Box::new(predicate_fn)) as u64;
    }
    obj
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

// ── Format parser ──────────────────────────────────────────────────

fn parse_format(fmt: &str) -> Box<dyn Fn(u64) -> bool + Send + Sync> {
    let trimmed = fmt.trim();

    if trimmed.eq_ignore_ascii_case("TRUEPREDICATE") {
        return Box::new(|_| true);
    }
    if trimmed.eq_ignore_ascii_case("FALSEPREDICATE") {
        return Box::new(|_| false);
    }

    // SELF == 'value' or SELF == "value"
    if let Some(val) = extract_self_op(trimmed, "==") {
        let val = val.to_string();
        return Box::new(move |obj| {
            cfstring::extract_cfstring_cstr(obj)
                .map(|s| s == val)
                .unwrap_or(false)
        });
    }

    // SELF CONTAINS 'value'
    if let Some(val) = extract_self_op(trimmed, "CONTAINS") {
        let val = val.to_string();
        return Box::new(move |obj| {
            cfstring::extract_cfstring_cstr(obj)
                .map(|s| s.contains(&val))
                .unwrap_or(false)
        });
    }

    // SELF BEGINSWITH 'value'
    if let Some(val) = extract_self_op(trimmed, "BEGINSWITH") {
        let val = val.to_string();
        return Box::new(move |obj| {
            cfstring::extract_cfstring_cstr(obj)
                .map(|s| s.starts_with(&val))
                .unwrap_or(false)
        });
    }

    // SELF ENDSWITH 'value'
    if let Some(val) = extract_self_op(trimmed, "ENDSWITH") {
        let val = val.to_string();
        return Box::new(move |obj| {
            cfstring::extract_cfstring_cstr(obj)
                .map(|s| s.ends_with(&val))
                .unwrap_or(false)
        });
    }

    // Fallback: always false
    log::warn!("NSPredicate: unsupported format '{trimmed}', returning FALSEPREDICATE");
    Box::new(|_| false)
}

/// Extract the value from `SELF <OP> 'value'` or `SELF <OP> "value"`.
fn extract_self_op<'a>(fmt: &'a str, op: &str) -> Option<&'a str> {
    let upper = fmt.to_uppercase();
    let pattern = format!("SELF {}", op.to_uppercase());
    let idx = upper.find(&pattern)?;
    let rest = &fmt[idx + pattern.len()..].trim_start();
    // Strip quotes
    let rest = rest.trim();
    if (rest.starts_with('\'') && rest.ends_with('\''))
        || (rest.starts_with('"') && rest.ends_with('"'))
    {
        Some(&rest[1..rest.len() - 1])
    } else {
        Some(rest)
    }
}
