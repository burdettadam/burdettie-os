use super::*;
use super::helpers::{dict_keys, dict_vals};
use crate::cfstring;
use crate::nsarray;
use crate::nsobj;

fn make_cfstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

#[test]
fn create_and_count() {
    let k1 = make_cfstring("name");
    let k2 = make_cfstring("age");
    let v1 = make_cfstring("Alice");
    let v2 = make_cfstring("30");

    let keys = [k1, k2];
    let vals = [v1, v2];

    let dict = shim_nsdictionary_with_objects_for_keys_count(
        nsobj::nsdictionary_class_addr(),
        0,
        vals.as_ptr() as u64,
        keys.as_ptr() as u64,
        2,
    );
    assert_ne!(dict, 0);
    assert_eq!(shim_nsdictionary_count(dict, 0), 2);

    crate::pal_alloc().free(dict_keys(dict) as *mut u8);
    crate::pal_alloc().free(dict_vals(dict) as *mut u8);
    crate::pal_alloc().free(dict as *mut u8);
}

#[test]
fn object_for_key_hit() {
    let k1 = make_cfstring("x");
    let v1 = make_cfstring("hello");

    let keys = [k1];
    let vals = [v1];

    let dict = shim_nsdictionary_with_objects_for_keys_count(
        nsobj::nsdictionary_class_addr(),
        0,
        vals.as_ptr() as u64,
        keys.as_ptr() as u64,
        1,
    );

    // Look up by a *different* CFString with the same content
    let k_lookup = make_cfstring("x");
    let result = shim_nsdictionary_object_for_key(dict, 0, k_lookup);
    assert_ne!(result, 0);
    let s = cfstring::extract_cfstring_cstr(result).unwrap();
    assert_eq!(s, "hello");

    crate::pal_alloc().free(dict_keys(dict) as *mut u8);
    crate::pal_alloc().free(dict_vals(dict) as *mut u8);
    crate::pal_alloc().free(dict as *mut u8);
}

#[test]
fn object_for_key_miss() {
    let k1 = make_cfstring("x");
    let v1 = make_cfstring("y");
    let keys = [k1];
    let vals = [v1];

    let dict = shim_nsdictionary_with_objects_for_keys_count(
        nsobj::nsdictionary_class_addr(),
        0,
        vals.as_ptr() as u64,
        keys.as_ptr() as u64,
        1,
    );

    let k_miss = make_cfstring("z");
    assert_eq!(shim_nsdictionary_object_for_key(dict, 0, k_miss), 0);

    crate::pal_alloc().free(dict_keys(dict) as *mut u8);
    crate::pal_alloc().free(dict_vals(dict) as *mut u8);
    crate::pal_alloc().free(dict as *mut u8);
}

#[test]
fn all_keys_returns_nsarray() {
    let k1 = make_cfstring("a");
    let k2 = make_cfstring("b");
    let v1 = make_cfstring("1");
    let v2 = make_cfstring("2");
    let keys = [k1, k2];
    let vals = [v1, v2];

    let dict = shim_nsdictionary_with_objects_for_keys_count(
        nsobj::nsdictionary_class_addr(),
        0,
        vals.as_ptr() as u64,
        keys.as_ptr() as u64,
        2,
    );

    let arr = shim_nsdictionary_all_keys(dict, 0);
    assert_ne!(arr, 0);
    assert_eq!(nsarray::shim_nsarray_count(arr, 0), 2);

    crate::pal_alloc().free(dict_keys(dict) as *mut u8);
    crate::pal_alloc().free(dict_vals(dict) as *mut u8);
    crate::pal_alloc().free(dict as *mut u8);
}

#[test]
fn empty_dictionary() {
    let dict = shim_nsdictionary_with_objects_for_keys_count(
        nsobj::nsdictionary_class_addr(),
        0,
        0,
        0,
        0,
    );
    assert_ne!(dict, 0);
    assert_eq!(shim_nsdictionary_count(dict, 0), 0);
    assert_eq!(shim_nsdictionary_object_for_key(dict, 0, 42), 0);
    assert!(shim_nsdictionary_all_keys(dict, 0) != 0);

    crate::pal_alloc().free(dict as *mut u8);
}
