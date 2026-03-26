//! Key-Value Coding (KVC) support for NSObject.
//!
//! Provides `valueForKey:`, `setValue:forKey:`, `valueForKeyPath:`,
//! `setValue:forKeyPath:`, and `valueForUndefinedKey:` / `setValue:forUndefinedKey:`.
//!
//! **Accessor search order** (Apple-compatible):
//!
//! `valueForKey:` getter search:
//!   1. `<key>`      — e.g. `name`
//!   2. `is<Key>`    — e.g. `isName`
//!   3. `_<key>`     — e.g. `_name`
//!   4. `_is<Key>`   — e.g. `_isName`
//!   5. ivar `_<key>` / `<key>` — direct ivar access with auto-boxing
//!
//! `setValue:forKey:` setter search:
//!   1. `set<Key>:`  — e.g. `setName:`
//!   2. `_set<Key>:` — e.g. `_setName:`
//!   3. ivar `_<key>` / `<key>` — direct ivar write with auto-unboxing

use crate::cfstring;
use crate::nsnumber;

mod accessor;
mod boxing;

use accessor::{try_getter_accessor, try_ivar_read, try_ivar_write, try_setter_accessor};

// ── Shim: valueForKey: ────────────────────────────────────────────

/// `- [NSObject valueForKey:]`
pub extern "C" fn shim_value_for_key(obj: u64, _sel: u64, key_nsstring: u64) -> u64 {
    if obj == 0 || key_nsstring == 0 {
        return 0;
    }

    let Some(key) = cfstring::extract_cfstring_cstr(key_nsstring) else {
        return 0;
    };
    if let Some(result) = try_ivar_read(obj, key) {
        return result;
    }
    if let Some(result) = try_getter_accessor(obj, key) {
        return result;
    }
    value_for_undefined_key(obj, key)
}

// ── Shim: setValue:forKey: ─────────────────────────────────────────

/// `- [NSObject setValue:forKey:]`
pub extern "C" fn shim_set_value_for_key(obj: u64, _sel: u64, value: u64, key_nsstring: u64) {
    if obj == 0 || key_nsstring == 0 {
        return;
    }

    let Some(key) = cfstring::extract_cfstring_cstr(key_nsstring) else {
        return;
    };
    if try_ivar_write(obj, key, value) {
        return;
    }
    if try_setter_accessor(obj, key, value) {
        return;
    }
    set_value_for_undefined_key(obj, key, value);
}

// ── Shim: valueForKeyPath: ────────────────────────────────────────

/// `- [NSObject valueForKeyPath:]`
pub extern "C" fn shim_value_for_key_path(obj: u64, _sel: u64, key_path_nsstring: u64) -> u64 {
    if obj == 0 || key_path_nsstring == 0 {
        return 0;
    }

    let Some(key_path) = cfstring::extract_cfstring_cstr(key_path_nsstring) else {
        return 0;
    };
    let mut current = obj;
    for component in key_path.split('.') {
        if current == 0 {
            return 0;
        }
        if component.starts_with('@') {
            return handle_collection_operator(current, component, key_path);
        }
        current = value_for_key_direct(current, component);
    }
    current
}

// ── Shim: setValue:forKeyPath: ─────────────────────────────────────

/// `- [NSObject setValue:forKeyPath:]`
pub extern "C" fn shim_set_value_for_key_path(
    obj: u64,
    _sel: u64,
    value: u64,
    key_path_nsstring: u64,
) {
    if obj == 0 || key_path_nsstring == 0 {
        return;
    }

    let Some(key_path) = cfstring::extract_cfstring_cstr(key_path_nsstring) else {
        return;
    };

    let components: Vec<&str> = key_path.split('.').collect();
    if components.is_empty() {
        return;
    }

    let mut current = obj;
    for component in &components[..components.len() - 1] {
        if current == 0 {
            return;
        }
        current = value_for_key_direct(current, component);
    }

    if current == 0 {
        return;
    }
    let final_key = components[components.len() - 1];
    set_value_for_key_direct(current, final_key, value);
}

// ── Shim: valueForUndefinedKey: ───────────────────────────────────

/// `- [NSObject valueForUndefinedKey:]`
pub extern "C" fn shim_value_for_undefined_key_method(
    obj: u64,
    _sel: u64,
    key_nsstring: u64,
) -> u64 {
    let key = cfstring::extract_cfstring_cstr(key_nsstring).unwrap_or("<unknown>");
    log::warn!(
        "valueForUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
    0
}

// ── Shim: setValue:forUndefinedKey: ────────────────────────────────

/// `- [NSObject setValue:forUndefinedKey:]`
pub extern "C" fn shim_set_value_for_undefined_key_method(
    obj: u64,
    _sel: u64,
    _value: u64,
    key_nsstring: u64,
) {
    let key = cfstring::extract_cfstring_cstr(key_nsstring).unwrap_or("<unknown>");
    log::warn!(
        "setValue:forUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
}

// ── Internal helpers ──────────────────────────────────────────────

/// `valueForKeyPath:` override for NSDictionary / NSConstantDictionary.
///
/// Splits the key-path on `.` and calls `objectForKey:` at each step,
/// creating an NSString key from each path component.
pub extern "C" fn shim_dict_value_for_key_path(
    obj: u64,
    _sel: u64,
    key_path_nsstring: u64,
) -> u64 {
    if obj == 0 || key_path_nsstring == 0 {
        return 0;
    }
    let Some(key_path) = cfstring::extract_cfstring_cstr(key_path_nsstring) else {
        return 0;
    };
    let mut current = obj;
    for component in key_path.split('.') {
        if current == 0 {
            return 0;
        }
        let c_key = std::ffi::CString::new(component).unwrap_or_default();
        let ns_key = cfstring::shim_cf_string_create_with_cstring(0, c_key.as_ptr(), 0);
        current = crate::nsdictionary::shim_nsdictionary_object_for_key(current, 0, ns_key);
    }
    current
}

/// Core `valueForKey:` logic without NSString extraction.
fn value_for_key_direct(obj: u64, key: &str) -> u64 {
    if obj == 0 || key.is_empty() {
        return 0;
    }
    if let Some(result) = try_getter_accessor(obj, key) {
        return result;
    }
    if let Some(result) = try_ivar_read(obj, key) {
        return result;
    }
    value_for_undefined_key(obj, key)
}

/// Core `setValue:forKey:` logic without NSString extraction.
fn set_value_for_key_direct(obj: u64, key: &str, value: u64) {
    if obj == 0 || key.is_empty() {
        return;
    }
    if try_setter_accessor(obj, key, value) {
        return;
    }
    if try_ivar_write(obj, key, value) {
        return;
    }
    set_value_for_undefined_key(obj, key, value);
}

fn value_for_undefined_key(obj: u64, key: &str) -> u64 {
    log::warn!(
        "valueForUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
    0
}

fn set_value_for_undefined_key(obj: u64, key: &str, _value: u64) {
    log::warn!(
        "setValue:forUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
}

fn handle_collection_operator(collection: u64, operator: &str, full_path: &str) -> u64 {
    let sub = collection_sub_keypath(operator, full_path);
    match operator {
        "@count" => collection_count(collection),
        "@sum" => collection_sum(collection, sub),
        "@avg" => collection_avg(collection, sub),
        "@min" => collection_min(collection, sub),
        "@max" => collection_max(collection, sub),
        "@unionOfObjects" => collection_union_of_objects(collection, sub),
        "@distinctUnionOfObjects" => collection_distinct_union_of_objects(collection, sub),
        _ => {
            log::warn!(
                "KVC collection operator '{}' not yet implemented",
                operator
            );
            0
        }
    }
}

// ── Collection operator helpers ───────────────────────────────────

/// Extract the right-hand sub-keypath after a collection operator.
///
/// `("@sum", "employees.@sum.salary")` → `"salary"`
/// `("@count", "items.@count")` → `""`
fn collection_sub_keypath<'a>(operator: &str, full_path: &'a str) -> &'a str {
    if let Some(pos) = full_path.find(operator) {
        let after = &full_path[pos + operator.len()..];
        after.strip_prefix('.').unwrap_or("")
    } else {
        ""
    }
}

/// Dispatch `count` on a collection object and return the raw integer count.
fn raw_count(collection: u64) -> usize {
    use objc_runtime::OBJC_RUNTIME;
    if collection == 0 {
        return 0;
    }
    let isa = unsafe { *(collection as *const u64) };
    let (imp, sel) = {
        let guard = OBJC_RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
        let Some(rt) = guard.as_ref() else { return 0 };
        let imp = rt.msg_lookup(isa, "count");
        let sel = rt.selector_ptr("count").unwrap_or(0) as u64;
        (imp, sel)
    };
    let Some(fn_ptr) = imp else { return 0 };
    let fn_ptr: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(fn_ptr) };
    fn_ptr(collection, sel) as usize
}

/// Collect all items from a collection by dispatching `objectAtIndex:`.
fn collect_items(collection: u64) -> Vec<u64> {
    use objc_runtime::OBJC_RUNTIME;
    if collection == 0 {
        return Vec::new();
    }
    let count = raw_count(collection);
    if count == 0 {
        return Vec::new();
    }
    let isa = unsafe { *(collection as *const u64) };
    let (at_imp, at_sel) = {
        let guard = OBJC_RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
        let Some(rt) = guard.as_ref() else { return Vec::new() };
        let at_imp = rt.msg_lookup(isa, "objectAtIndex:");
        let at_sel = rt.selector_ptr("objectAtIndex:").unwrap_or(0) as u64;
        (at_imp, at_sel)
    };
    let Some(at_fn) = at_imp else { return Vec::new() };
    let at_fn: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(at_fn) };
    (0..count as u64).map(|i| at_fn(collection, at_sel, i)).collect()
}

/// Evaluate a dot-separated key path on an object using ivar-first KVC lookup
/// (mirrors the behaviour of `valueForKey:` — ivar before accessor — so that
/// scalar ivars such as `double` are read directly without ABI-mismatch risk).
fn eval_keypath_str(obj: u64, keypath: &str) -> u64 {
    if keypath.is_empty() {
        return obj;
    }
    let mut current = obj;
    for component in keypath.split('.') {
        if current == 0 {
            return 0;
        }
        if let Some(v) = try_ivar_read(current, component) {
            current = v;
        } else if let Some(v) = try_getter_accessor(current, component) {
            current = v;
        } else {
            return 0;
        }
    }
    current
}

/// Extract a numeric double from an already-boxed NSNumber (or any object that
/// carries the double in the standard NSNumber layout produced by `try_ivar_read`).
fn object_double_value(obj: u64) -> f64 {
    boxing::unbox_double(obj)
}

// ── Aggregate operators ───────────────────────────────────────────

fn collection_count(collection: u64) -> u64 {
    let n = raw_count(collection);
    nsnumber::alloc_nsnumber(0, n as u64)
}

fn collection_sum(collection: u64, sub_keypath: &str) -> u64 {
    let total: f64 = collect_items(collection)
        .into_iter()
        .map(|item| object_double_value(eval_keypath_str(item, sub_keypath)))
        .sum();
    nsnumber::make_nsnumber_double(total)
}

fn collection_avg(collection: u64, sub_keypath: &str) -> u64 {
    let items = collect_items(collection);
    if items.is_empty() {
        return 0;
    }
    let total: f64 = items
        .iter()
        .map(|&item| object_double_value(eval_keypath_str(item, sub_keypath)))
        .sum();
    nsnumber::make_nsnumber_double(total / items.len() as f64)
}

fn collection_min(collection: u64, sub_keypath: &str) -> u64 {
    let items = collect_items(collection);
    let Some(min) = items
        .into_iter()
        .map(|item| object_double_value(eval_keypath_str(item, sub_keypath)))
        .reduce(f64::min)
    else {
        return 0;
    };
    nsnumber::make_nsnumber_double(min)
}

fn collection_max(collection: u64, sub_keypath: &str) -> u64 {
    let items = collect_items(collection);
    let Some(max) = items
        .into_iter()
        .map(|item| object_double_value(eval_keypath_str(item, sub_keypath)))
        .reduce(f64::max)
    else {
        return 0;
    };
    nsnumber::make_nsnumber_double(max)
}

// ── Collection operators ──────────────────────────────────────────

fn collection_union_of_objects(collection: u64, sub_keypath: &str) -> u64 {
    let values: Vec<u64> = collect_items(collection)
        .into_iter()
        .map(|item| eval_keypath_str(item, sub_keypath))
        .collect();
    crate::nsarray::make_nsarray(&values)
}

/// Deduplicate by object identity (pointer equality).  Apple's implementation
/// uses `isEqual:`, but pointer-level deduplication is correct for the common
/// case where distinct objects are held in the array.
fn collection_distinct_union_of_objects(collection: u64, sub_keypath: &str) -> u64 {
    let items = collect_items(collection);
    let mut seen = std::collections::HashSet::new();
    let mut values: Vec<u64> = Vec::new();
    for item in items {
        let val = eval_keypath_str(item, sub_keypath);
        if seen.insert(val) {
            values.push(val);
        }
    }
    crate::nsarray::make_nsarray(&values)
}
