use super::*;
use crate::nsarray;

/// Helper: create a bare mutable array object with the given capacity.
fn make_empty(cap: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    unsafe {
        *(obj as *mut u64) = 0xDEAD; // fake ISA
        let buf = crate::pal_alloc().calloc(cap as usize, 8) as u64;
        *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *mut u64) = buf;
        *((obj as usize + NSMUTABLEARRAY_CAP_OFF) as *mut u64) = cap;
    }
    obj
}

fn get_count(obj: u64) -> u64 {
    unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) }
}

fn free_array(obj: u64) {
    let buf = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if buf != 0 {
        crate::pal_alloc().free(buf as *mut u8);
    }
    crate::pal_alloc().free(obj as *mut u8);
}

#[test]
fn add_and_count() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBBB);
    shim_nsmutablearray_add_object(arr, 0, 0xCCC);
    assert_eq!(get_count(arr), 3);
    // Use NSArray read functions (same layout)
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCCC);
    free_array(arr);
}

#[test]
fn remove_last() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBBB);
    shim_nsmutablearray_remove_last_object(arr, 0);
    assert_eq!(get_count(arr), 1);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    free_array(arr);
}

#[test]
fn remove_at_index() {
    let arr = make_empty(8);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBBB);
    shim_nsmutablearray_add_object(arr, 0, 0xCCC);
    // Remove middle element (index 1)
    shim_nsmutablearray_remove_object_at_index(arr, 0, 1);
    assert_eq!(get_count(arr), 2);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xCCC);
    free_array(arr);
}

#[test]
fn insert_at_index() {
    let arr = make_empty(8);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xCCC);
    // Insert at index 1 (between AAA and CCC)
    shim_nsmutablearray_insert_object_at_index(arr, 0, 0xBBB, 1);
    assert_eq!(get_count(arr), 3);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCCC);
    free_array(arr);
}

#[test]
fn replace_at_index() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBBB);
    shim_nsmutablearray_replace_object_at_index(arr, 0, 0, 0xDDD);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xDDD);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
    free_array(arr);
}

#[test]
fn remove_all() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBBB);
    shim_nsmutablearray_remove_all_objects(arr, 0);
    assert_eq!(get_count(arr), 0);
    free_array(arr);
}

#[test]
fn grow_beyond_capacity() {
    let arr = make_empty(2);
    shim_nsmutablearray_add_object(arr, 0, 0x111);
    shim_nsmutablearray_add_object(arr, 0, 0x222);
    // This should trigger a grow (count == capacity)
    shim_nsmutablearray_add_object(arr, 0, 0x333);
    assert_eq!(get_count(arr), 3);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0x333);
    let cap = unsafe { *((arr as usize + NSMUTABLEARRAY_CAP_OFF) as *const u64) };
    assert!(cap >= 3, "capacity should have grown");
    free_array(arr);
}

#[test]
fn nil_operations_safe() {
    assert_eq!(shim_nsmutablearray_add_object(0, 0, 0xAAA), 0);
    assert_eq!(shim_nsmutablearray_remove_last_object(0, 0), 0);
    assert_eq!(shim_nsmutablearray_remove_object_at_index(0, 0, 0), 0);
    assert_eq!(shim_nsmutablearray_remove_all_objects(0, 0), 0);
    assert_eq!(
        shim_nsmutablearray_insert_object_at_index(0, 0, 0xAAA, 0),
        0
    );
    assert_eq!(
        shim_nsmutablearray_replace_object_at_index(0, 0, 0, 0xAAA),
        0
    );
}
