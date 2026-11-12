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

fn free_immutable_dict(obj: u64) {
    let k = crate::nsdictionary::helpers_pub::dict_keys_pub(obj);
    let v = crate::nsdictionary::helpers_pub::dict_vals_pub(obj);
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

// --- class method tests ---

#[test]
fn dictionary_creates_empty() {
    let cls = crate::nsobj::nsmutabledictionary_class_addr();
    let dict = shim_nsmutabledict_dictionary(cls, 0);
    assert_ne!(dict, 0);
    assert_eq!(get_count(dict), 0);
    let cap = unsafe { *((dict as usize + CAP_OFF) as *const u64) };
    assert_eq!(cap, INITIAL_CAPACITY);
    free_dict(dict);
}

#[test]
fn dictionary_nil_cls_returns_zero() {
    assert_eq!(shim_nsmutabledict_dictionary(0, 0), 0);
}

#[test]
fn dictionary_with_capacity_creates_empty() {
    let cls = crate::nsobj::nsmutabledictionary_class_addr();
    let dict = shim_nsmutabledict_dictionary_with_capacity(cls, 0, 8);
    assert_ne!(dict, 0);
    assert_eq!(get_count(dict), 0);
    let cap = unsafe { *((dict as usize + CAP_OFF) as *const u64) };
    assert_eq!(cap, 8);
    free_dict(dict);
}

#[test]
fn dictionary_with_capacity_nil_cls_returns_zero() {
    assert_eq!(shim_nsmutabledict_dictionary_with_capacity(0, 0, 8), 0);
}

#[test]
fn dictionary_with_capacity_zero_uses_default() {
    let cls = crate::nsobj::nsmutabledictionary_class_addr();
    let dict = shim_nsmutabledict_dictionary_with_capacity(cls, 0, 0);
    assert_ne!(dict, 0);
    let cap = unsafe { *((dict as usize + CAP_OFF) as *const u64) };
    assert_eq!(cap, INITIAL_CAPACITY);
    free_dict(dict);
}

#[test]
fn dictionary_sets_correct_isa() {
    let cls = crate::nsobj::nsmutabledictionary_class_addr();
    let dict = shim_nsmutabledict_dictionary(cls, 0);
    let isa = unsafe { *(dict as *const u64) };
    assert_eq!(isa, crate::nsobj::nsmutabledictionary_class_addr());
    free_dict(dict);
}

#[test]
fn init_with_capacity_initializes_allocated_dictionary() {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    let ret = shim_nsmutabledict_init_with_capacity(obj, 0, 7);
    assert_eq!(ret, obj);
    assert_eq!(get_count(obj), 0);
    assert_eq!(unsafe { *((obj as usize + CAP_OFF) as *const u64) }, 7);
    assert_ne!(unsafe { *((obj as usize + KEYS_OFF) as *const u64) }, 0);
    assert_ne!(unsafe { *((obj as usize + VALS_OFF) as *const u64) }, 0);
    assert_eq!(
        unsafe { *(obj as *const u64) },
        crate::nsobj::nsmutabledictionary_class_addr()
    );
    free_dict(obj);
}

#[test]
fn init_with_capacity_nil_is_safe() {
    assert_eq!(shim_nsmutabledict_init_with_capacity(0, 0, 3), 0);
}

#[test]
fn init_with_dictionary_copies_pairs() {
    let src = make_empty(4);
    let k1 = make_str("first");
    let v1 = make_str("one");
    let k2 = make_str("second");
    let v2 = make_str("two");
    shim_nsmutabledict_set_object_for_key(src, 0, v1, k1);
    shim_nsmutabledict_set_object_for_key(src, 0, v2, k2);

    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    let ret = shim_nsmutabledict_init_with_dictionary(obj, 0, src);
    assert_eq!(ret, obj);
    assert_eq!(get_count(obj), 2);

    let r1 = nsdictionary::shim_nsdictionary_object_for_key(obj, 0, make_str("first"));
    let r2 = nsdictionary::shim_nsdictionary_object_for_key(obj, 0, make_str("second"));
    assert_eq!(cfstring::extract_cfstring_cstr(r1), Some("one"));
    assert_eq!(cfstring::extract_cfstring_cstr(r2), Some("two"));

    free_dict(src);
    free_dict(obj);
}

#[test]
fn init_with_dictionary_nil_source_returns_empty_dictionary() {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    shim_nsmutabledict_init_with_dictionary(obj, 0, 0);
    assert_eq!(get_count(obj), 0);
    assert_eq!(
        unsafe { *((obj as usize + CAP_OFF) as *const u64) },
        INITIAL_CAPACITY
    );
    free_dict(obj);
}

#[test]
fn init_with_dictionary_nil_is_safe() {
    assert_eq!(shim_nsmutabledict_init_with_dictionary(0, 0, 0xAA), 0);
}

#[test]
fn copy_returns_immutable_dictionary_clone() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("one"), make_str("first"));
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("two"), make_str("second"));

    let copy = nsdictionary::shim_nsmutabledict_copy(dict, 0);
    assert_ne!(copy, 0);
    assert_eq!(
        unsafe { *(copy as *const u64) },
        crate::nsobj::nsdictionary_class_addr()
    );
    assert_eq!(nsdictionary::shim_nsdictionary_count(copy, 0), 2);
    assert_eq!(
        cfstring::extract_cfstring_cstr(nsdictionary::shim_nsdictionary_object_for_key(
            copy,
            0,
            make_str("first"),
        )),
        Some("one")
    );

    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("updated"), make_str("first"));
    assert_eq!(
        cfstring::extract_cfstring_cstr(nsdictionary::shim_nsdictionary_object_for_key(
            copy,
            0,
            make_str("first"),
        )),
        Some("one")
    );

    free_dict(dict);
    free_immutable_dict(copy);
}

#[test]
fn mutable_copy_returns_distinct_mutable_dictionary() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("value"), make_str("key"));

    let copy = nsdictionary::shim_nsmutabledict_mutable_copy(dict, 0);
    assert_ne!(copy, 0);
    assert_ne!(copy, dict);
    assert_eq!(
        unsafe { *(copy as *const u64) },
        crate::nsobj::nsmutabledictionary_class_addr()
    );
    assert_eq!(get_count(copy), 1);

    shim_nsmutabledict_set_object_for_key(copy, 0, make_str("updated"), make_str("key"));
    assert_eq!(
        cfstring::extract_cfstring_cstr(nsdictionary::shim_nsdictionary_object_for_key(
            copy,
            0,
            make_str("key"),
        )),
        Some("updated")
    );
    assert_eq!(
        cfstring::extract_cfstring_cstr(nsdictionary::shim_nsdictionary_object_for_key(
            dict,
            0,
            make_str("key"),
        )),
        Some("value")
    );

    free_dict(dict);
    free_dict(copy);
}

// --- add entries from dictionary ---

#[test]
fn add_entries_from_dictionary() {
    let src = make_empty(4);
    shim_nsmutabledict_set_object_for_key(src, 0, make_str("v1"), make_str("k1"));
    shim_nsmutabledict_set_object_for_key(src, 0, make_str("v2"), make_str("k2"));

    let dst = make_empty(4);
    shim_nsmutabledict_add_entries_from_dictionary(dst, 0, src);
    assert_eq!(get_count(dst), 2);

    let k1 = make_str("k1");
    let r = nsdictionary::shim_nsdictionary_object_for_key(dst, 0, k1);
    let s = cfstring::extract_cfstring_cstr(r).unwrap();
    assert_eq!(s, "v1");

    free_dict(src);
    free_dict(dst);
}

#[test]
fn add_entries_merges_and_overwrites() {
    let src = make_empty(4);
    shim_nsmutabledict_set_object_for_key(src, 0, make_str("new_val"), make_str("shared"));

    let dst = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dst, 0, make_str("old_val"), make_str("shared"));
    shim_nsmutabledict_set_object_for_key(dst, 0, make_str("other"), make_str("unique"));
    assert_eq!(get_count(dst), 2);

    shim_nsmutabledict_add_entries_from_dictionary(dst, 0, src);
    // "shared" was overwritten, "unique" preserved, count stays 2
    assert_eq!(get_count(dst), 2);

    let k = make_str("shared");
    let r = nsdictionary::shim_nsdictionary_object_for_key(dst, 0, k);
    let s = cfstring::extract_cfstring_cstr(r).unwrap();
    assert_eq!(s, "new_val");

    free_dict(src);
    free_dict(dst);
}

#[test]
fn add_entries_nil_obj_safe() {
    let src = make_empty(4);
    assert_eq!(shim_nsmutabledict_add_entries_from_dictionary(0, 0, src), 0);
    free_dict(src);
}

#[test]
fn add_entries_nil_other_safe() {
    let dst = make_empty(4);
    assert_eq!(shim_nsmutabledict_add_entries_from_dictionary(dst, 0, 0), 0);
    free_dict(dst);
}

#[test]
fn add_entries_from_empty_dictionary() {
    let src = make_empty(4);
    let dst = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dst, 0, make_str("v"), make_str("k"));
    shim_nsmutabledict_add_entries_from_dictionary(dst, 0, src);
    assert_eq!(get_count(dst), 1); // unchanged
    free_dict(src);
    free_dict(dst);
}

// --- additional nil / edge case tests ---

#[test]
fn subscript_setter_nil_safe() {
    assert_eq!(
        shim_nsmutabledict_set_object_for_keyed_subscript(0, 0, 0xA, 0xB),
        0
    );
}

#[test]
fn remove_nonexistent_key() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v"), make_str("a"));
    let missing = make_str("zzz");
    shim_nsmutabledict_remove_object_for_key(dict, 0, missing);
    assert_eq!(get_count(dict), 1); // unchanged
    free_dict(dict);
}

#[test]
fn remove_from_empty_dict() {
    let dict = make_empty(4);
    let k = make_str("any");
    shim_nsmutabledict_remove_object_for_key(dict, 0, k);
    assert_eq!(get_count(dict), 0);
    free_dict(dict);
}

#[test]
fn remove_last_element() {
    let dict = make_empty(4);
    let k = make_str("only");
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("val"), k);
    assert_eq!(get_count(dict), 1);
    shim_nsmutabledict_remove_object_for_key(dict, 0, k);
    assert_eq!(get_count(dict), 0);
    free_dict(dict);
}

#[test]
fn remove_all_then_reuse() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v1"), make_str("k1"));
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v2"), make_str("k2"));
    shim_nsmutabledict_remove_all_objects(dict, 0);
    assert_eq!(get_count(dict), 0);

    // Re-add after removeAll
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v3"), make_str("k3"));
    assert_eq!(get_count(dict), 1);

    let k = make_str("k3");
    let r = nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k);
    let s = cfstring::extract_cfstring_cstr(r).unwrap();
    assert_eq!(s, "v3");
    free_dict(dict);
}

#[test]
fn multiple_grows() {
    let dict = make_empty(1);
    for i in 0..10 {
        let k = make_str(&format!("key{i}"));
        let v = make_str(&format!("val{i}"));
        shim_nsmutabledict_set_object_for_key(dict, 0, v, k);
    }
    assert_eq!(get_count(dict), 10);
    let cap = unsafe { *((dict as usize + CAP_OFF) as *const u64) };
    assert!(cap >= 10);
    free_dict(dict);
}

// --- removeObjectsForKeys: ---

#[test]
fn remove_objects_for_keys_removes_matching() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v1"), make_str("a"));
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v2"), make_str("b"));
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v3"), make_str("c"));
    assert_eq!(get_count(dict), 3);

    // Build an NSArray with keys "a" and "c"
    let items = [make_str("a"), make_str("c")];
    let arr = crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        items.as_ptr() as u64,
        2,
    );
    shim_nsmutabledict_remove_objects_for_keys(dict, 0, arr);
    assert_eq!(get_count(dict), 1);
    // "b" remains
    assert_ne!(
        nsdictionary::shim_nsdictionary_object_for_key(dict, 0, make_str("b")),
        0
    );
    assert_eq!(
        nsdictionary::shim_nsdictionary_object_for_key(dict, 0, make_str("a")),
        0
    );
    free_dict(dict);
}

#[test]
fn remove_objects_for_keys_nil_obj_safe() {
    assert_eq!(shim_nsmutabledict_remove_objects_for_keys(0, 0, 0xAA), 0);
}

#[test]
fn remove_objects_for_keys_nil_keys_safe() {
    let dict = make_empty(4);
    assert_eq!(shim_nsmutabledict_remove_objects_for_keys(dict, 0, 0), 0);
    free_dict(dict);
}

#[test]
fn remove_objects_for_keys_nonexistent_no_crash() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v"), make_str("k"));
    let items = [make_str("missing")];
    let arr = crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        items.as_ptr() as u64,
        1,
    );
    shim_nsmutabledict_remove_objects_for_keys(dict, 0, arr);
    assert_eq!(get_count(dict), 1);
    free_dict(dict);
}

// --- setDictionary: ---

#[test]
fn set_dictionary_replaces_contents() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("old"), make_str("existing"));
    assert_eq!(get_count(dict), 1);

    let other = make_empty(4);
    shim_nsmutabledict_set_object_for_key(other, 0, make_str("v1"), make_str("k1"));
    shim_nsmutabledict_set_object_for_key(other, 0, make_str("v2"), make_str("k2"));

    shim_nsmutabledict_set_dictionary(dict, 0, other);
    assert_eq!(get_count(dict), 2);
    assert_eq!(
        nsdictionary::shim_nsdictionary_object_for_key(dict, 0, make_str("existing")),
        0
    );
    assert_ne!(
        nsdictionary::shim_nsdictionary_object_for_key(dict, 0, make_str("k1")),
        0
    );
    assert_ne!(
        nsdictionary::shim_nsdictionary_object_for_key(dict, 0, make_str("k2")),
        0
    );
    free_dict(dict);
    free_dict(other);
}

#[test]
fn set_dictionary_nil_other_clears() {
    let dict = make_empty(4);
    shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v"), make_str("k"));
    assert_eq!(get_count(dict), 1);
    shim_nsmutabledict_set_dictionary(dict, 0, 0);
    assert_eq!(get_count(dict), 0);
    free_dict(dict);
}

#[test]
fn set_dictionary_nil_obj_safe() {
    assert_eq!(shim_nsmutabledict_set_dictionary(0, 0, 0xAA), 0);
}
