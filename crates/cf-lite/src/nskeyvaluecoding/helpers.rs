use crate::cfstring;
use crate::nsnumber;

use super::accessor::{try_getter_accessor, try_ivar_read, try_setter_accessor, try_ivar_write};
use super::boxing::unbox_double;

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
pub(super) fn value_for_key_direct(obj: u64, key: &str) -> u64 {
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
pub(super) fn set_value_for_key_direct(obj: u64, key: &str, value: u64) {
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

pub(super) fn value_for_undefined_key(obj: u64, key: &str) -> u64 {
    log::warn!(
        "valueForUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
    0
}

pub(super) fn set_value_for_undefined_key(obj: u64, key: &str, _value: u64) {
    log::warn!(
        "setValue:forUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
}

pub(super) fn handle_collection_operator(collection: u64, operator: &str, full_path: &str) -> u64 {
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
pub(super) fn raw_count(collection: u64) -> usize {
    use objc_runtime::OBJC_RUNTIME;
    if collection == 0 {
        return 0;
    }
    let isa = unsafe { *(collection as *const u64) };
    let (imp, sel) = {
        let guard = OBJC_RUNTIME.read().unwrap_or_else(|e| e.into_inner());
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
pub(super) fn collect_items(collection: u64) -> Vec<u64> {
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
        let guard = OBJC_RUNTIME.read().unwrap_or_else(|e| e.into_inner());
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
pub(super) fn eval_keypath_str(obj: u64, keypath: &str) -> u64 {
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
pub(super) fn object_double_value(obj: u64) -> f64 {
    unbox_double(obj)
}

// ── Aggregate operators ───────────────────────────────────────────

pub(super) fn collection_count(collection: u64) -> u64 {
    let n = raw_count(collection);
    nsnumber::alloc_nsnumber(0, n as u64)
}

pub(super) fn collection_sum(collection: u64, sub_keypath: &str) -> u64 {
    let total: f64 = collect_items(collection)
        .into_iter()
        .map(|item| object_double_value(eval_keypath_str(item, sub_keypath)))
        .sum();
    nsnumber::make_nsnumber_double(total)
}

pub(super) fn collection_avg(collection: u64, sub_keypath: &str) -> u64 {
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

pub(super) fn collection_min(collection: u64, sub_keypath: &str) -> u64 {
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

pub(super) fn collection_max(collection: u64, sub_keypath: &str) -> u64 {
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

pub(super) fn collection_union_of_objects(collection: u64, sub_keypath: &str) -> u64 {
    let values: Vec<u64> = collect_items(collection)
        .into_iter()
        .map(|item| eval_keypath_str(item, sub_keypath))
        .collect();
    crate::nsarray::make_nsarray(&values)
}

/// Deduplicate by object identity (pointer equality).  Apple's implementation
/// uses `isEqual:`, but pointer-level deduplication is correct for the common
/// case where distinct objects are held in the array.
pub(super) fn collection_distinct_union_of_objects(collection: u64, sub_keypath: &str) -> u64 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn dict_value_for_key_path_nil_returns_zero() {
        assert_eq!(shim_dict_value_for_key_path(0, 0, 0), 0);
        let dict = crate::nsdictionary::make_nsdictionary(&[], &[]);
        assert_eq!(shim_dict_value_for_key_path(dict, 0, 0), 0);
        free_obj(dict);
    }

    #[test]
    fn dict_value_for_key_path_single_key() {
        let k = make_string("name");
        let v = make_string("Alice");
        let dict = crate::nsdictionary::make_nsdictionary(&[k], &[v]);
        let key_path = make_string("name");
        let result = shim_dict_value_for_key_path(dict, 0, key_path);
        assert_eq!(result, v);
        free_obj(dict);
        free_obj(key_path);
    }

    #[test]
    fn dict_value_for_key_path_missing_key_returns_zero() {
        let k = make_string("exists");
        let v = make_string("yes");
        let dict = crate::nsdictionary::make_nsdictionary(&[k], &[v]);
        let key_path = make_string("missing");
        let result = shim_dict_value_for_key_path(dict, 0, key_path);
        assert_eq!(result, 0);
        free_obj(dict);
        free_obj(key_path);
    }

    #[test]
    fn value_for_key_direct_nil_returns_zero() {
        assert_eq!(value_for_key_direct(0, "key"), 0);
        assert_eq!(value_for_key_direct(0, ""), 0);
    }
}
