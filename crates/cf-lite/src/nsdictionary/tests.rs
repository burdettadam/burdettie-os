use super::helpers::{dict_keys, dict_vals, make_nsdictionary};
use super::*;
use crate::cfstring;
use crate::nsarray;
use crate::nsobj;
use crate::shim_test_helpers::*;

fn make_cfstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn free_dictionary(obj: u64) {
    let keys = dict_keys(obj);
    let vals = dict_vals(obj);
    if keys != 0 {
        crate::pal_alloc().free(keys as *mut u8);
    }
    if vals != 0 {
        crate::pal_alloc().free(vals as *mut u8);
    }
    crate::pal_alloc().free(obj as *mut u8);
}

fn make_nsconstantdictionary(keys: &[u64], vals: &[u64]) -> u64 {
    let count = keys.len().min(vals.len());
    let obj = alloc_obj(40, nsobj::nsconstantdictionary_class_addr());
    write_field(obj, 16, count as u64);
    if count > 0 {
        write_field(obj, 24, make_u64_array(&keys[..count]));
        write_field(obj, 32, make_u64_array(&vals[..count]));
    }
    obj
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
    let dict =
        shim_nsdictionary_with_objects_for_keys_count(nsobj::nsdictionary_class_addr(), 0, 0, 0, 0);
    assert_ne!(dict, 0);
    assert_eq!(shim_nsdictionary_count(dict, 0), 0);
    assert_eq!(shim_nsdictionary_object_for_key(dict, 0, 42), 0);
    assert!(shim_nsdictionary_all_keys(dict, 0) != 0);

    crate::pal_alloc().free(dict as *mut u8);
}

// ── Nil safety ─────────────────────────────────────────────────

#[test]
fn count_nil() {
    assert_eq!(shim_nsdictionary_count(0, 0), 0);
}

#[test]
fn object_for_key_nil_obj() {
    assert_eq!(shim_nsdictionary_object_for_key(0, 0, 42), 0);
}

#[test]
fn object_for_key_nil_key() {
    let k = make_string("k");
    let v = make_string("v");
    let dict = make_nsdictionary(&[k], &[v]);
    assert_eq!(shim_nsdictionary_object_for_key(dict, 0, 0), 0);
    free_obj(dict);
}

#[test]
fn object_for_keyed_subscript_nil_obj() {
    assert_eq!(shim_nsdictionary_object_for_keyed_subscript(0, 0, 42), 0);
}

#[test]
fn all_keys_nil() {
    assert_eq!(shim_nsdictionary_all_keys(0, 0), 0);
}

#[test]
fn all_values_nil() {
    assert_eq!(shim_nsdictionary_all_values(0, 0), 0);
}

#[test]
fn description_nil() {
    assert_eq!(shim_nsdictionary_description(0, 0), 0);
}

#[test]
fn enumerate_nil_obj() {
    // Must not crash
    shim_nsdictionary_enumerate_keys_and_objects_using_block(0, 0, 1);
}

#[test]
fn enumerate_nil_block() {
    let dict = make_nsdictionary(&[], &[]);
    // Must not crash
    shim_nsdictionary_enumerate_keys_and_objects_using_block(dict, 0, 0);
    free_obj(dict);
}

// ── objectForKeyedSubscript ────────────────────────────────────

#[test]
fn keyed_subscript_hit() {
    let k = make_string("color");
    let v = make_string("red");
    let dict = make_nsdictionary(&[k], &[v]);

    let lookup = make_string("color");
    let result = shim_nsdictionary_object_for_keyed_subscript(dict, 0, lookup);
    assert_string_eq(result, "red");

    free_obj(dict);
}

#[test]
fn keyed_subscript_miss() {
    let k = make_string("a");
    let v = make_string("b");
    let dict = make_nsdictionary(&[k], &[v]);

    let lookup = make_string("zzz");
    assert_eq!(
        shim_nsdictionary_object_for_keyed_subscript(dict, 0, lookup),
        0
    );

    free_obj(dict);
}

// ── allValues ──────────────────────────────────────────────────

#[test]
fn all_values_returns_nsarray() {
    let k1 = make_string("a");
    let k2 = make_string("b");
    let v1 = make_string("1");
    let v2 = make_string("2");
    let dict = make_nsdictionary(&[k1, k2], &[v1, v2]);

    let arr = shim_nsdictionary_all_values(dict, 0);
    assert_ne!(arr, 0);
    assert_eq!(nsarray::shim_nsarray_count(arr, 0), 2);

    free_obj(dict);
}

#[test]
fn all_values_empty() {
    let dict = make_nsdictionary(&[], &[]);
    let arr = shim_nsdictionary_all_values(dict, 0);
    assert_ne!(arr, 0);
    assert_eq!(nsarray::shim_nsarray_count(arr, 0), 0);

    free_obj(dict);
}

// ── copy / mutableCopy ─────────────────────────────────────────

#[test]
fn copy_returns_self_for_immutable_dictionary() {
    let dict = make_nsdictionary(&[make_string("name")], &[make_string("Alice")]);
    let before = objc_runtime::shim_nsobject_retain_count(dict, 0);

    let copy = shim_nsdictionary_copy(dict, 0);
    assert_eq!(copy, dict);
    assert_eq!(
        objc_runtime::shim_nsobject_retain_count(dict, 0),
        before + 1
    );

    let keys = dict_keys(dict);
    let vals = dict_vals(dict);
    objc_runtime::shim_nsobject_release(copy, 0);
    if keys != 0 {
        crate::pal_alloc().free(keys as *mut u8);
    }
    if vals != 0 {
        crate::pal_alloc().free(vals as *mut u8);
    }
    objc_runtime::shim_nsobject_release(dict, 0);
}

#[test]
fn mutable_copy_returns_mutable_dictionary_clone() {
    let dict = make_nsdictionary(
        &[make_string("name"), make_string("age")],
        &[make_string("Alice"), make_string("30")],
    );

    let copy = shim_nsdictionary_mutable_copy(dict, 0);
    assert_ne!(copy, 0);
    assert_ne!(copy, dict);
    assert_eq!(read_field(copy, 0), nsobj::nsmutabledictionary_class_addr());
    assert_eq!(shim_nsdictionary_count(copy, 0), 2);
    assert_string_eq(
        shim_nsdictionary_object_for_key(copy, 0, make_string("name")),
        "Alice",
    );

    crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(
        copy,
        0,
        make_string("Bob"),
        make_string("name"),
    );

    assert_string_eq(
        shim_nsdictionary_object_for_key(copy, 0, make_string("name")),
        "Bob",
    );
    assert_string_eq(
        shim_nsdictionary_object_for_key(dict, 0, make_string("name")),
        "Alice",
    );

    free_dictionary(dict);
    free_dictionary(copy);
}

#[test]
fn constant_dictionary_mutable_copy_clones_entries() {
    let dict = make_nsconstantdictionary(&[make_string("k")], &[make_string("v")]);

    let copy = shim_nsdictionary_mutable_copy(dict, 0);
    assert_ne!(copy, 0);
    assert_eq!(read_field(copy, 0), nsobj::nsmutabledictionary_class_addr());
    assert_string_eq(
        shim_nsdictionary_object_for_key(copy, 0, make_string("k")),
        "v",
    );

    free_dictionary(dict);
    free_dictionary(copy);
}

// ── description ────────────────────────────────────────────────

#[test]
fn description_single_pair() {
    let k = make_string("key");
    let v = make_string("val");
    let dict = make_nsdictionary(&[k], &[v]);

    let desc = shim_nsdictionary_description(dict, 0);
    assert_ne!(desc, 0);
    let s = read_string(desc);
    assert!(s.contains("key"), "description should contain key: {}", s);
    assert!(s.contains("val"), "description should contain val: {}", s);

    free_obj(dict);
}

#[test]
fn description_empty() {
    let dict = make_nsdictionary(&[], &[]);
    let desc = shim_nsdictionary_description(dict, 0);
    assert_ne!(desc, 0);
    let s = read_string(desc);
    assert_eq!(s, "{}");

    free_obj(dict);
}

#[test]
fn description_multiple_pairs() {
    let k1 = make_string("x");
    let k2 = make_string("y");
    let v1 = make_string("10");
    let v2 = make_string("20");
    let dict = make_nsdictionary(&[k1, k2], &[v1, v2]);

    let desc = shim_nsdictionary_description(dict, 0);
    assert_ne!(desc, 0);
    let s = read_string(desc);
    assert!(s.starts_with('{') && s.ends_with('}'), "bad format: {}", s);
    assert!(s.contains("x"), "missing key x: {}", s);
    assert!(s.contains("y"), "missing key y: {}", s);

    free_obj(dict);
}

// ── make_nsdictionary helper ───────────────────────────────────

#[test]
fn make_nsdictionary_basic() {
    let k = make_string("hello");
    let v = make_string("world");
    let dict = make_nsdictionary(&[k], &[v]);
    assert_ne!(dict, 0);
    assert_eq!(shim_nsdictionary_count(dict, 0), 1);

    let lookup = make_string("hello");
    let result = shim_nsdictionary_object_for_key(dict, 0, lookup);
    assert_string_eq(result, "world");

    free_obj(dict);
}

#[test]
fn make_nsdictionary_empty() {
    let dict = make_nsdictionary(&[], &[]);
    assert_ne!(dict, 0);
    assert_eq!(shim_nsdictionary_count(dict, 0), 0);
    free_obj(dict);
}

// ── Pointer-equality key lookup ────────────────────────────────

#[test]
fn object_for_key_pointer_equality() {
    let k = make_string("same");
    let v = make_string("found");
    let dict = make_nsdictionary(&[k], &[v]);

    // Use the exact same key pointer — should match by pointer equality
    let result = shim_nsdictionary_object_for_key(dict, 0, k);
    assert_string_eq(result, "found");

    free_obj(dict);
}

// ── Class factory with make_u64_array ──────────────────────────

#[test]
fn factory_with_u64_buf() {
    let k1 = make_string("p");
    let k2 = make_string("q");
    let v1 = make_string("10");
    let v2 = make_string("20");

    let keys_buf = make_u64_array(&[k1, k2]);
    let vals_buf = make_u64_array(&[v1, v2]);

    let dict = shim_nsdictionary_with_objects_for_keys_count(
        nsobj::nsdictionary_class_addr(),
        0,
        vals_buf,
        keys_buf,
        2,
    );
    assert_ne!(dict, 0);
    assert_eq!(shim_nsdictionary_count(dict, 0), 2);

    let lookup = make_string("q");
    let result = shim_nsdictionary_object_for_key(dict, 0, lookup);
    assert_string_eq(result, "20");

    free_buf(keys_buf);
    free_buf(vals_buf);
    crate::pal_alloc().free(dict_keys(dict) as *mut u8);
    crate::pal_alloc().free(dict_vals(dict) as *mut u8);
    crate::pal_alloc().free(dict as *mut u8);
}

// ── all_keys for empty dict ────────────────────────────────────

#[test]
fn all_keys_empty() {
    let dict = make_nsdictionary(&[], &[]);
    let arr = shim_nsdictionary_all_keys(dict, 0);
    assert_ne!(arr, 0);
    assert_eq!(nsarray::shim_nsarray_count(arr, 0), 0);
    free_obj(dict);
}

#[test]
fn dictionary_with_contents_of_file_reads_plist() {
    let root = std::env::temp_dir().join(format!("nsdictionary_plist_{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let plist_path = root.join("Info.plist");
    std::fs::write(
        &plist_path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.example.dictionary</string>
</dict>
</plist>
"#,
    )
    .unwrap();

    let path = make_string(plist_path.to_string_lossy().as_ref());
    let dict = shim_nsdictionary_with_contents_of_file(nsobj::nsdictionary_class_addr(), 0, path);
    assert_ne!(dict, 0);

    let key = make_string("CFBundleIdentifier");
    let value = shim_nsdictionary_object_for_key(dict, 0, key);
    assert_string_eq(value, "com.example.dictionary");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn dictionary_with_contents_of_url_error_reads_plist() {
    let root = std::env::temp_dir().join(format!("nsdictionary_url_plist_{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let plist_path = root.join("Info.plist");
    std::fs::write(
        &plist_path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleVersion</key>
  <string>42</string>
</dict>
</plist>
"#,
    )
    .unwrap();

    let path = make_string(plist_path.to_string_lossy().as_ref());
    let url =
        crate::nsurl::shim_nsurl_file_url_with_path(crate::nsobj::nsurl_class_addr(), 0, path);
    let mut error = 99u64;
    let dict = shim_nsdictionary_with_contents_of_url_error(
        nsobj::nsdictionary_class_addr(),
        0,
        url,
        &mut error,
    );
    assert_ne!(dict, 0);
    assert_eq!(error, 0);

    let key = make_string("CFBundleVersion");
    let value = shim_nsdictionary_object_for_key(dict, 0, key);
    assert_string_eq(value, "42");

    let _ = std::fs::remove_dir_all(root);
}

// ── dictionaryWithObject:forKey: ──────────────────────────────

#[test]
fn dict_with_object_for_key_creates_single_entry() {
    let k = make_cfstring("name");
    let v = make_cfstring("Alice");
    let dict = shim_nsdictionary_with_object_for_key(nsobj::nsdictionary_class_addr(), 0, v, k);
    assert_ne!(dict, 0);
    assert_eq!(shim_nsdictionary_count(dict, 0), 1);
    let lookup = make_cfstring("name");
    let result = shim_nsdictionary_object_for_key(dict, 0, lookup);
    assert_ne!(result, 0);
    assert_string_eq(result, "Alice");
    free_dictionary(dict);
}

// ── dictionaryWithDictionary: ─────────────────────────────────

#[test]
fn dict_with_dictionary_copies_source() {
    let k1 = make_cfstring("a");
    let k2 = make_cfstring("b");
    let v1 = make_cfstring("1");
    let v2 = make_cfstring("2");
    let source = make_nsdictionary(&[k1, k2], &[v1, v2]);

    let copy = shim_nsdictionary_with_dictionary(nsobj::nsdictionary_class_addr(), 0, source);
    assert_ne!(copy, 0);
    assert_ne!(copy, source);
    assert_eq!(shim_nsdictionary_count(copy, 0), 2);

    let lookup = make_cfstring("a");
    assert_string_eq(shim_nsdictionary_object_for_key(copy, 0, lookup), "1");

    free_dictionary(source);
    free_dictionary(copy);
}

#[test]
fn dict_with_dictionary_nil_source_returns_empty() {
    let dict = shim_nsdictionary_with_dictionary(nsobj::nsdictionary_class_addr(), 0, 0);
    assert_ne!(dict, 0);
    assert_eq!(shim_nsdictionary_count(dict, 0), 0);
    free_obj(dict);
}

// ── isEqualToDictionary: ──────────────────────────────────────

#[test]
fn is_equal_to_dictionary_same_pointer() {
    let dict = make_nsdictionary(&[make_cfstring("k")], &[make_cfstring("v")]);
    assert_eq!(shim_nsdictionary_is_equal_to_dictionary(dict, 0, dict), 1);
    free_dictionary(dict);
}

#[test]
fn is_equal_to_dictionary_nil_obj() {
    let other = make_nsdictionary(&[], &[]);
    assert_eq!(shim_nsdictionary_is_equal_to_dictionary(0, 0, other), 0);
    free_obj(other);
}

#[test]
fn is_equal_to_dictionary_nil_other() {
    let dict = make_nsdictionary(&[], &[]);
    assert_eq!(shim_nsdictionary_is_equal_to_dictionary(dict, 0, 0), 0);
    free_obj(dict);
}

#[test]
fn is_equal_to_dictionary_both_empty() {
    let a = make_nsdictionary(&[], &[]);
    let b = make_nsdictionary(&[], &[]);
    assert_eq!(shim_nsdictionary_is_equal_to_dictionary(a, 0, b), 1);
    free_obj(a);
    free_obj(b);
}

#[test]
fn is_equal_to_dictionary_different_counts() {
    let a = make_nsdictionary(&[make_cfstring("k")], &[make_cfstring("v")]);
    let b = make_nsdictionary(&[], &[]);
    assert_eq!(shim_nsdictionary_is_equal_to_dictionary(a, 0, b), 0);
    free_dictionary(a);
    free_obj(b);
}

#[test]
fn is_equal_to_dictionary_same_entries() {
    let a = make_nsdictionary(
        &[make_cfstring("name"), make_cfstring("age")],
        &[make_cfstring("Alice"), make_cfstring("30")],
    );
    let b = make_nsdictionary(
        &[make_cfstring("name"), make_cfstring("age")],
        &[make_cfstring("Alice"), make_cfstring("30")],
    );
    assert_eq!(shim_nsdictionary_is_equal_to_dictionary(a, 0, b), 1);
    free_dictionary(a);
    free_dictionary(b);
}

#[test]
fn is_equal_to_dictionary_different_values() {
    let a = make_nsdictionary(&[make_cfstring("k")], &[make_cfstring("v1")]);
    let b = make_nsdictionary(&[make_cfstring("k")], &[make_cfstring("v2")]);
    assert_eq!(shim_nsdictionary_is_equal_to_dictionary(a, 0, b), 0);
    free_dictionary(a);
    free_dictionary(b);
}

// ── writeToFile:atomically: ───────────────────────────────────

#[test]
fn write_to_file_nil_safe() {
    assert_eq!(shim_nsdictionary_write_to_file(0, 0, 0, 0), 0);
}

#[test]
fn write_to_file_returns_zero_stub() {
    let dict = make_nsdictionary(&[make_cfstring("k")], &[make_cfstring("v")]);
    let path = make_cfstring("/tmp/test.plist");
    assert_eq!(shim_nsdictionary_write_to_file(dict, 0, path, 1), 0);
    free_dictionary(dict);
}

// ── Fast enumeration tests ─────────────────────────

#[test]
fn fast_enum_nil_returns_zero() {
    assert_eq!(
        shim_nsdictionary_count_by_enumerating_with_state(0, 0, 1, 1, 16),
        0
    );
}

#[test]
fn fast_enum_empty_dict() {
    let dict = make_nsdictionary(&[], &[]);
    let mut state = [0u64; 4];
    let mut buf = [0u64; 16];
    let n = shim_nsdictionary_count_by_enumerating_with_state(
        dict,
        0,
        state.as_mut_ptr() as u64,
        buf.as_mut_ptr() as u64,
        16,
    );
    assert_eq!(n, 0);
    free_dictionary(dict);
}

#[test]
fn fast_enum_returns_all_keys() {
    let k1 = make_cfstring("alpha");
    let k2 = make_cfstring("beta");
    let v1 = make_cfstring("1");
    let v2 = make_cfstring("2");
    let dict = make_nsdictionary(&[k1, k2], &[v1, v2]);

    let mut state = [0u64; 4];
    let mut buf = [0u64; 16];
    let n = shim_nsdictionary_count_by_enumerating_with_state(
        dict,
        0,
        state.as_mut_ptr() as u64,
        buf.as_mut_ptr() as u64,
        16,
    );
    assert_eq!(n, 2);
    // Keys should be k1 and k2
    assert_eq!(buf[0], k1);
    assert_eq!(buf[1], k2);
    // state->state should advance past all items
    assert_eq!(state[0], 2);
    // state->itemsPtr should point to buf
    assert_eq!(state[1], buf.as_ptr() as u64);

    // Second call should return 0 (exhausted)
    let n2 = shim_nsdictionary_count_by_enumerating_with_state(
        dict,
        0,
        state.as_mut_ptr() as u64,
        buf.as_mut_ptr() as u64,
        16,
    );
    assert_eq!(n2, 0);
    free_dictionary(dict);
}

#[test]
fn fast_enum_batched() {
    let keys: Vec<u64> = (0..5).map(|i| make_cfstring(&format!("k{i}"))).collect();
    let vals: Vec<u64> = (0..5).map(|i| make_cfstring(&format!("v{i}"))).collect();
    let dict = make_nsdictionary(&keys, &vals);

    let mut state = [0u64; 4];
    let mut buf = [0u64; 2]; // buffer smaller than count → forces batching

    let n1 = shim_nsdictionary_count_by_enumerating_with_state(
        dict,
        0,
        state.as_mut_ptr() as u64,
        buf.as_mut_ptr() as u64,
        2,
    );
    assert_eq!(n1, 2);
    assert_eq!(state[0], 2);

    let n2 = shim_nsdictionary_count_by_enumerating_with_state(
        dict,
        0,
        state.as_mut_ptr() as u64,
        buf.as_mut_ptr() as u64,
        2,
    );
    assert_eq!(n2, 2);
    assert_eq!(state[0], 4);

    let n3 = shim_nsdictionary_count_by_enumerating_with_state(
        dict,
        0,
        state.as_mut_ptr() as u64,
        buf.as_mut_ptr() as u64,
        2,
    );
    assert_eq!(n3, 1);
    assert_eq!(state[0], 5);

    let n4 = shim_nsdictionary_count_by_enumerating_with_state(
        dict,
        0,
        state.as_mut_ptr() as u64,
        buf.as_mut_ptr() as u64,
        2,
    );
    assert_eq!(n4, 0);
    free_dictionary(dict);
}

#[test]
fn fast_enum_constant_dict() {
    let k = make_cfstring("key");
    let v = make_cfstring("val");
    let dict = make_nsconstantdictionary(&[k], &[v]);

    let mut state = [0u64; 4];
    let mut buf = [0u64; 16];
    let n = shim_nsdictionary_count_by_enumerating_with_state(
        dict,
        0,
        state.as_mut_ptr() as u64,
        buf.as_mut_ptr() as u64,
        16,
    );
    assert_eq!(n, 1);
    assert_eq!(buf[0], k);
}
