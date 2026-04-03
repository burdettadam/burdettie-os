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


mod helpers;
use self::helpers::{
    value_for_key_direct, set_value_for_key_direct,
    value_for_undefined_key, set_value_for_undefined_key,
    handle_collection_operator,
};
pub use self::helpers::shim_dict_value_for_key_path;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_obj_value_for_key_returns_zero() {
        assert_eq!(shim_value_for_key(0, 0, 0), 0);
    }

    #[test]
    fn nil_key_value_for_key_returns_zero() {
        let obj = alloc_obj_fake(32);
        assert_eq!(shim_value_for_key(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn nil_obj_set_value_for_key_no_panic() {
        let key = make_string("x");
        shim_set_value_for_key(0, 0, 0xABC, key);
    }

    #[test]
    fn nil_obj_value_for_key_path_returns_zero() {
        assert_eq!(shim_value_for_key_path(0, 0, 0), 0);
    }

    #[test]
    fn nil_key_path_returns_zero() {
        let obj = alloc_obj_fake(32);
        assert_eq!(shim_value_for_key_path(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn unknown_key_falls_through_to_zero() {
        // No ObjC runtime active → try_ivar_read and try_getter_accessor both
        // return None → value_for_undefined_key logs and returns 0.
        let key = make_string("nosuchfield");
        let obj = alloc_obj_fake(32);
        assert_eq!(shim_value_for_key(obj, 0, key), 0);
        free_obj(obj);
    }

    #[test]
    fn undefined_key_method_returns_zero() {
        let key = make_string("missing");
        assert_eq!(shim_value_for_undefined_key_method(0xDEAD, 0, key), 0);
    }

    #[test]
    fn set_undefined_key_method_no_panic() {
        let key = make_string("missing");
        shim_set_value_for_undefined_key_method(0xDEAD, 0, 0xABC, key);
    }
}
