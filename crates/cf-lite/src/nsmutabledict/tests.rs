use super::*;
use crate::nsdictionary;

fn make_str(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn make_empty(cap: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    unsafe {
        *(obj as *mut u64) = 0xDEAD; // fake ISA
        let key_buf = crate::pal_alloc().calloc(cap as usize, 8) as u64;
        let val_buf = crate::pal_alloc().calloc(cap as usize, 8) as u64;
        *((obj as usize + KEYS_OFF) as *mut u64) = key_buf;
        *((obj as usize + VALS_OFF) as *mut u64) = val_buf;
        *((obj as usize + CAP_OFF) as *mut u64) = cap;
    }
    obj
}

fn get_count(obj: u64) -> u64 {
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

fn free_dict(obj: u64) {
    let k = unsafe { *((obj as usize + KEYS_OFF) as *const u64) };
    let v = unsafe { *((obj as usize + VALS_OFF) as *const u64) };
    if k != 0 {
        crate::pal_alloc().free(k as *mut u8);
    }
    if v != 0 {
        crate::pal_alloc().free(v as *mut u8);
    }
    crate::pal_alloc().free(obj as *mut u8);
}

#[test]
fn set_and_get() {
    let dict = make_empty(4);
    let k1 = make_str("name");
    let v1 = make_str("Alice");
    shim_nsmutabledict_set_object_for_key(dict, 0, v1, k1);
    assert_eq!(get_count(dict), 1);

    let result = nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k1);
    let s = cfstring::extract_cfstring_cstr(result).unwrap();
    assert_eq!(s, "Alice");
    free_dict(dict);
}

#[test]
fn update_existing_key() {
    let dict = make_empty(4);
    let k = make_str("key");
    let v1 = make_str("old");
    let v2 = make_str("new");
    shim_nsmutabledict_set_object_for_key(dict, 0, v1, k);
    shim_nsmutabledict_set_object_for_key(dict, 0, v2, k);
    assert_eq!(get_count(dict), 1); // count stays 1

    let result = nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k);
    let s = cfstring::extract_cfstring_cstr(result).unwrap();
    assert_eq!(s, "new");
    free_dict(dict);
}

#[test]
fn remove_object_for_key() {
    let dict = make_empty(4);
    let k1 = make_str("a");
    let k2 = make_str("b");
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("1"), k1);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("2"), k2);
    assert_eq!(get_count(dict), 2);

    shim_nsmutabledict_remove_object_for_key(dict, 0, k1);
    assert_eq!(get_count(dict), 1);

    // "a" gone, "b" still present
    assert_eq!(
        nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k1),
        0
    );
    assert_ne!(
        nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k2),
        0
    );
    free_dict(dict);
}

#[test]
fn remove_all_objects() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v1"), make_str("k1"));
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v2"), make_str("k2"));
    assert_eq!(get_count(dict), 2);

    shim_nsmutabledict_remove_all_objects(dict, 0);
    assert_eq!(get_count(dict), 0);
    free_dict(dict);
}

#[test]
fn grow_beyond_capacity() {
    let dict = make_empty(2);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v1"), make_str("k1"));
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v2"), make_str("k2"));
    // This triggers grow (count == capacity)
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v3"), make_str("k3"));
    assert_eq!(get_count(dict), 3);
    let cap = unsafe { *((dict as usize + CAP_OFF) as *const u64) };
    assert!(cap >= 3, "capacity should have grown");
    free_dict(dict);
}

#[test]
fn subscript_setter() {
    let dict = make_empty(4);
    let k = make_str("x");
    let v = make_str("y");
    shim_nsmutabledict_set_object_for_keyed_subscript(dict, 0, v, k);
    assert_eq!(get_count(dict), 1);
    let result = nsdictionary::shim_nsdictionary_object_for_keyed_subscript(dict, 0, k);
    let s = cfstring::extract_cfstring_cstr(result).unwrap();
    assert_eq!(s, "y");
    free_dict(dict);
}

#[test]
fn nil_operations_safe() {
    assert_eq!(shim_nsmutabledict_set_object_for_key(0, 0, 0xA, 0xB), 0);
    assert_eq!(shim_nsmutabledict_remove_object_for_key(0, 0, 0xA), 0);
    assert_eq!(shim_nsmutabledict_remove_all_objects(0, 0), 0);
}

#[test]
fn string_content_key_lookup() {
    let dict = make_empty(4);
    let k1 = make_str("hello");
    let v1 = make_str("world");
    shim_nsmutabledict_set_object_for_key(dict, 0, v1, k1);

    // Look up with a different CFString that has the same content
    let k2 = make_str("hello");
    assert_ne!(k1, k2); // different pointers
    let result = nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k2);
    let s = cfstring::extract_cfstring_cstr(result).unwrap();
    assert_eq!(s, "world");
    free_dict(dict);
}
