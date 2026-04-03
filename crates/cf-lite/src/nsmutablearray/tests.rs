use super::*;
use crate::nsarray;
use crate::shim_test_helpers::*;

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

fn cls() -> u64 {
    crate::nsobj::nsmutablearray_class_addr()
}

// ── Class constructors ─────────────────────────────────────────────

#[test]
fn array_creates_empty() {
    let arr = shim_nsmutablearray_array(cls(), 0);
    assert_ne!(arr, 0);
    assert_eq!(get_count(arr), 0);
    free_array(arr);
}

#[test]
fn array_nil_cls_returns_zero() {
    assert_eq!(shim_nsmutablearray_array(0, 0), 0);
}

#[test]
fn array_with_capacity_creates_empty() {
    let arr = shim_nsmutablearray_array_with_capacity(cls(), 0, 16);
    assert_ne!(arr, 0);
    assert_eq!(get_count(arr), 0);
    let cap = read_field(arr, NSMUTABLEARRAY_CAP_OFF);
    assert_eq!(cap, 16);
    free_array(arr);
}

#[test]
fn array_with_capacity_zero_uses_default() {
    let arr = shim_nsmutablearray_array_with_capacity(cls(), 0, 0);
    assert_ne!(arr, 0);
    let cap = read_field(arr, NSMUTABLEARRAY_CAP_OFF);
    assert_eq!(cap, INITIAL_CAPACITY);
    free_array(arr);
}

#[test]
fn array_with_capacity_nil_cls_returns_zero() {
    assert_eq!(shim_nsmutablearray_array_with_capacity(0, 0, 16), 0);
}

#[test]
fn array_with_array_copies_elements() {
    let src = make_empty(4);
    shim_nsmutablearray_add_object(src, 0, 0xAA);
    shim_nsmutablearray_add_object(src, 0, 0xBB);

    let copy = shim_nsmutablearray_array_with_array(cls(), 0, src);
    assert_ne!(copy, 0);
    assert_eq!(get_count(copy), 2);
    assert_eq!(nsarray::shim_nsarray_object_at_index(copy, 0, 0), 0xAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(copy, 0, 1), 0xBB);

    free_array(src);
    free_array(copy);
}

#[test]
fn array_with_array_nil_source_returns_empty() {
    let arr = shim_nsmutablearray_array_with_array(cls(), 0, 0);
    assert_ne!(arr, 0);
    assert_eq!(get_count(arr), 0);
    free_array(arr);
}

#[test]
fn array_with_array_nil_cls_returns_zero() {
    let src = make_empty(4);
    shim_nsmutablearray_add_object(src, 0, 0xAA);
    assert_eq!(shim_nsmutablearray_array_with_array(0, 0, src), 0);
    free_array(src);
}

#[test]
fn array_with_objects_collects_variadic_items_until_nil() {
    let arr = shim_nsmutablearray_array_with_objects(cls(), 0, 0xAA, 0xBB, 0, 0xCC, 0, 0, 0, 0);
    assert_ne!(arr, 0);
    assert_eq!(get_count(arr), 2);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBB);
    free_array(arr);
}

// ── Instance initializers ────────────────────────────────────────

#[test]
fn init_with_capacity_initializes_allocated_object() {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    let ret = shim_nsmutablearray_init_with_capacity(obj, 0, 6);
    assert_eq!(ret, obj);
    assert_eq!(get_count(obj), 0);
    assert_eq!(read_field(obj, NSMUTABLEARRAY_CAP_OFF), 6);
    assert_ne!(read_field(obj, NSMUTABLEARRAY_ELEMS_OFF), 0);
    assert_eq!(read_field(obj, 0), crate::nsobj::nsmutablearray_class_addr());
    free_array(obj);
}

#[test]
fn init_with_capacity_zero_uses_default_capacity() {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    shim_nsmutablearray_init_with_capacity(obj, 0, 0);
    assert_eq!(read_field(obj, NSMUTABLEARRAY_CAP_OFF), INITIAL_CAPACITY);
    free_array(obj);
}

#[test]
fn init_with_capacity_nil_is_safe() {
    assert_eq!(shim_nsmutablearray_init_with_capacity(0, 0, 4), 0);
}

#[test]
fn init_with_array_copies_source_elements() {
    let src = make_empty(4);
    shim_nsmutablearray_add_object(src, 0, 0xAA);
    shim_nsmutablearray_add_object(src, 0, 0xBB);

    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    let ret = shim_nsmutablearray_init_with_array(obj, 0, src);
    assert_eq!(ret, obj);
    assert_eq!(get_count(obj), 2);
    assert_eq!(nsarray::shim_nsarray_object_at_index(obj, 0, 0), 0xAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(obj, 0, 1), 0xBB);

    free_array(src);
    free_array(obj);
}

#[test]
fn init_with_array_nil_source_returns_empty_array() {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    shim_nsmutablearray_init_with_array(obj, 0, 0);
    assert_eq!(get_count(obj), 0);
    assert_eq!(read_field(obj, NSMUTABLEARRAY_CAP_OFF), INITIAL_CAPACITY);
    free_array(obj);
}

#[test]
fn init_with_array_nil_object_is_safe() {
    assert_eq!(shim_nsmutablearray_init_with_array(0, 0, 0xAA), 0);
}

#[test]
fn init_with_objects_collects_variadic_items_until_nil() {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    let ret = shim_nsmutablearray_init_with_objects(obj, 0, 0xAA, 0xBB, 0xCC, 0, 0, 0, 0, 0);
    assert_eq!(ret, obj);
    assert_eq!(get_count(obj), 3);
    assert_eq!(nsarray::shim_nsarray_object_at_index(obj, 0, 0), 0xAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(obj, 0, 1), 0xBB);
    assert_eq!(nsarray::shim_nsarray_object_at_index(obj, 0, 2), 0xCC);
    assert_eq!(read_field(obj, 0), crate::nsobj::nsmutablearray_class_addr());
    free_array(obj);
}

// ── addObject: ─────────────────────────────────────────────────────

#[test]
fn add_and_count() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBBB);
    shim_nsmutablearray_add_object(arr, 0, 0xCCC);
    assert_eq!(get_count(arr), 3);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCCC);
    free_array(arr);
}

#[test]
fn add_object_nil_is_noop() {
    assert_eq!(shim_nsmutablearray_add_object(0, 0, 0xABC), 0);
}

#[test]
fn add_object_grows_beyond_capacity() {
    let arr = make_empty(2);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBB);
    // This should trigger grow
    shim_nsmutablearray_add_object(arr, 0, 0xCC);
    assert_eq!(get_count(arr), 3);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCC);
    let cap = read_field(arr, NSMUTABLEARRAY_CAP_OFF);
    assert!(cap >= 3, "capacity should have grown: {}", cap);
    free_array(arr);
}

#[test]
fn add_object_retains_and_remove_releases_heap_elements() {
    let arr = shim_nsmutablearray_array(cls(), 0);
    let element = make_string("owned");

    assert_eq!(objc_runtime::shim_nsobject_retain_count(element, 0), 1);
    shim_nsmutablearray_add_object(arr, 0, element);
    assert_eq!(objc_runtime::shim_nsobject_retain_count(element, 0), 2);

    shim_nsmutablearray_remove_object_at_index(arr, 0, 0);
    assert_eq!(objc_runtime::shim_nsobject_retain_count(element, 0), 1);

    objc_runtime::shim_nsobject_release(element, 0);
    free_array(arr);
}

// ── insertObject:atIndex: ──────────────────────────────────────────

#[test]
fn insert_at_index() {
    let arr = make_empty(8);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xCCC);
    shim_nsmutablearray_insert_object_at_index(arr, 0, 0xBBB, 1);
    assert_eq!(get_count(arr), 3);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCCC);
    free_array(arr);
}

#[test]
fn insert_at_beginning() {
    let arr = make_empty(8);
    shim_nsmutablearray_add_object(arr, 0, 0xBB);
    shim_nsmutablearray_add_object(arr, 0, 0xCC);
    shim_nsmutablearray_insert_object_at_index(arr, 0, 0xAA, 0);
    assert_eq!(get_count(arr), 3);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBB);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCC);
    free_array(arr);
}

#[test]
fn insert_at_end() {
    let arr = make_empty(8);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBB);
    // index == count means append
    shim_nsmutablearray_insert_object_at_index(arr, 0, 0xCC, 2);
    assert_eq!(get_count(arr), 3);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCC);
    free_array(arr);
}

#[test]
fn insert_beyond_count_is_noop() {
    let arr = make_empty(8);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    // index 5 > count 1
    shim_nsmutablearray_insert_object_at_index(arr, 0, 0xBB, 5);
    assert_eq!(get_count(arr), 1);
    free_array(arr);
}

#[test]
fn insert_nil_obj_is_noop() {
    assert_eq!(shim_nsmutablearray_insert_object_at_index(0, 0, 0xABC, 0), 0);
}

// ── removeObjectAtIndex: ───────────────────────────────────────────

#[test]
fn remove_at_index() {
    let arr = make_empty(8);
    shim_nsmutablearray_add_object(arr, 0, 0xAAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBBB);
    shim_nsmutablearray_add_object(arr, 0, 0xCCC);
    shim_nsmutablearray_remove_object_at_index(arr, 0, 1);
    assert_eq!(get_count(arr), 2);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xCCC);
    free_array(arr);
}

#[test]
fn remove_first_element() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBB);
    shim_nsmutablearray_remove_object_at_index(arr, 0, 0);
    assert_eq!(get_count(arr), 1);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xBB);
    free_array(arr);
}

#[test]
fn remove_last_element_via_index() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBB);
    shim_nsmutablearray_remove_object_at_index(arr, 0, 1);
    assert_eq!(get_count(arr), 1);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAA);
    free_array(arr);
}

#[test]
fn remove_at_index_out_of_bounds_is_noop() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    shim_nsmutablearray_remove_object_at_index(arr, 0, 5);
    assert_eq!(get_count(arr), 1);
    free_array(arr);
}

#[test]
fn remove_at_index_nil_is_noop() {
    assert_eq!(shim_nsmutablearray_remove_object_at_index(0, 0, 0), 0);
}

// ── removeLastObject ───────────────────────────────────────────────

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
fn remove_last_on_empty_is_noop() {
    let arr = make_empty(4);
    assert_eq!(shim_nsmutablearray_remove_last_object(arr, 0), 0);
    assert_eq!(get_count(arr), 0);
    free_array(arr);
}

#[test]
fn remove_last_nil_is_noop() {
    assert_eq!(shim_nsmutablearray_remove_last_object(0, 0), 0);
}

// ── replaceObjectAtIndex:withObject: ───────────────────────────────

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
fn replace_out_of_bounds_is_noop() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    shim_nsmutablearray_replace_object_at_index(arr, 0, 5, 0xFF);
    // Original element unchanged
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAA);
    assert_eq!(get_count(arr), 1);
    free_array(arr);
}

#[test]
fn replace_nil_is_noop() {
    assert_eq!(shim_nsmutablearray_replace_object_at_index(0, 0, 0, 0xABC), 0);
}

#[test]
fn replace_zero_index_on_empty_array_appends_for_wrapper_compat() {
    let arr = make_empty(4);
    shim_nsmutablearray_replace_object_at_index(arr, 0, 0, 0xABC);
    assert_eq!(get_count(arr), 1);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xABC);
    free_array(arr);
}

// ── removeAllObjects ───────────────────────────────────────────────

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
fn remove_all_on_empty_is_noop() {
    let arr = make_empty(4);
    shim_nsmutablearray_remove_all_objects(arr, 0);
    assert_eq!(get_count(arr), 0);
    free_array(arr);
}

#[test]
fn remove_all_nil_is_noop() {
    assert_eq!(shim_nsmutablearray_remove_all_objects(0, 0), 0);
}

// ── description ────────────────────────────────────────────────────

#[test]
fn description_empty_array() {
    let arr = make_empty(4);
    let desc = shim_nsmutablearray_description(arr, 0);
    assert_ne!(desc, 0);
    assert_string_eq(desc, "()");
    free_array(arr);
}

#[test]
fn description_with_elements() {
    let arr = make_empty(4);
    let s1 = make_string("hello");
    let s2 = make_string("world");
    shim_nsmutablearray_add_object(arr, 0, s1);
    shim_nsmutablearray_add_object(arr, 0, s2);
    let desc = shim_nsmutablearray_description(arr, 0);
    assert_ne!(desc, 0);
    assert_string_eq(desc, "(\"hello\", \"world\")");
    free_array(arr);
}

#[test]
fn description_nil_is_noop() {
    assert_eq!(shim_nsmutablearray_description(0, 0), 0);
}

// ── combined operations ────────────────────────────────────────────

#[test]
fn add_remove_add_sequence() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBB);
    shim_nsmutablearray_remove_object_at_index(arr, 0, 0);
    assert_eq!(get_count(arr), 1);
    shim_nsmutablearray_add_object(arr, 0, 0xCC);
    assert_eq!(get_count(arr), 2);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xBB);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xCC);
    free_array(arr);
}

#[test]
fn remove_all_then_add() {
    let arr = make_empty(4);
    shim_nsmutablearray_add_object(arr, 0, 0xAA);
    shim_nsmutablearray_add_object(arr, 0, 0xBB);
    shim_nsmutablearray_remove_all_objects(arr, 0);
    shim_nsmutablearray_add_object(arr, 0, 0xCC);
    assert_eq!(get_count(arr), 1);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xCC);
    free_array(arr);
}

#[test]
fn insert_into_empty_array() {
    let arr = make_empty(4);
    shim_nsmutablearray_insert_object_at_index(arr, 0, 0xAA, 0);
    assert_eq!(get_count(arr), 1);
    assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAA);
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
