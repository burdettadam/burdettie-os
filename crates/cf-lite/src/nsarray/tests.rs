use super::*;
use crate::shim_test_helpers::*;

// ---------------------------------------------------------------------------
// make_nsarray helper
// ---------------------------------------------------------------------------

#[test]
fn make_nsarray_empty() {
    let arr = make_nsarray(&[]);
    assert_ne!(arr, 0);
    assert_eq!(shim_nsarray_count(arr, 0), 0);
    assert_eq!(shim_nsarray_first_object(arr, 0), 0);
    assert_eq!(shim_nsarray_last_object(arr, 0), 0);
    // No elements buffer to free for empty array
    free_obj(arr);
}

#[test]
fn make_nsarray_single_element() {
    let arr = make_nsarray(&[0xAAA]);
    assert_ne!(arr, 0);
    assert_eq!(shim_nsarray_count(arr, 0), 1);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(shim_nsarray_first_object(arr, 0), 0xAAA);
    assert_eq!(shim_nsarray_last_object(arr, 0), 0xAAA);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn make_nsarray_multiple_elements() {
    let arr = make_nsarray(&[0xAAA, 0xBBB, 0xCCC]);
    assert_ne!(arr, 0);
    assert_eq!(shim_nsarray_count(arr, 0), 3);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 2), 0xCCC);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn make_nsarray_sets_correct_isa() {
    let arr = make_nsarray(&[0x1]);
    let isa = read_field(arr, 0);
    assert_eq!(isa, crate::nsobj::nsarray_class_addr());
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// shim_nsarray_array_with_objects_count (class factory)
// ---------------------------------------------------------------------------

#[test]
fn array_with_objects_count_cls_zero_returns_zero() {
    let buf = make_u64_array(&[0xAAA, 0xBBB]);
    let result = shim_nsarray_array_with_objects_count(0, 0, buf, 2);
    assert_eq!(result, 0);
    free_buf(buf);
}

#[test]
fn array_with_objects_count_empty() {
    let cls = crate::nsobj::nsarray_class_addr();
    let arr = shim_nsarray_array_with_objects_count(cls, 0, 0, 0);
    assert_ne!(arr, 0);
    assert_eq!(shim_nsarray_count(arr, 0), 0);
    free_obj(arr);
}

#[test]
fn array_with_objects_count_creates_array() {
    let cls = crate::nsobj::nsarray_class_addr();
    let buf = make_u64_array(&[0x111, 0x222, 0x333]);
    let arr = shim_nsarray_array_with_objects_count(cls, 0, buf, 3);
    assert_ne!(arr, 0);
    assert_eq!(shim_nsarray_count(arr, 0), 3);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 0), 0x111);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 1), 0x222);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 2), 0x333);
    free_buf(buf);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn array_with_objects_count_sets_isa() {
    let cls = crate::nsobj::nsarray_class_addr();
    let buf = make_u64_array(&[0xAA]);
    let arr = shim_nsarray_array_with_objects_count(cls, 0, buf, 1);
    assert_eq!(read_field(arr, 0), crate::nsobj::nsarray_class_addr());
    free_buf(buf);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn array_with_objects_count_copies_elements() {
    // Verify the factory copies elements (modify source, array unchanged)
    let cls = crate::nsobj::nsarray_class_addr();
    let buf = make_u64_array(&[0x100, 0x200]);
    let arr = shim_nsarray_array_with_objects_count(cls, 0, buf, 2);
    // Mutate source buffer
    write_field(buf, 0, 0xDEAD);
    // Array should still have original value
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 0), 0x100);
    free_buf(buf);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn array_with_object_cls_zero_returns_zero() {
    assert_eq!(shim_nsarray_array_with_object(0, 0, 0xAAA), 0);
}

#[test]
fn array_with_object_nil_object_returns_zero() {
    let cls = crate::nsobj::nsarray_class_addr();
    assert_eq!(shim_nsarray_array_with_object(cls, 0, 0), 0);
}

#[test]
fn array_with_object_creates_single_element_array() {
    let cls = crate::nsobj::nsarray_class_addr();
    let arr = shim_nsarray_array_with_object(cls, 0, 0xABC);
    assert_ne!(arr, 0);
    assert_eq!(shim_nsarray_count(arr, 0), 1);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 0), 0xABC);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn array_with_objects_variadic_stops_at_nil() {
    let cls = crate::nsobj::nsarray_class_addr();
    let arr = shim_nsarray_array_with_objects(cls, 0, 0x111, 0x222, 0, 0x333, 0, 0, 0, 0);
    assert_ne!(arr, 0);
    assert_eq!(shim_nsarray_count(arr, 0), 2);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 0), 0x111);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 1), 0x222);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn init_with_objects_variadic_initializes_allocated_array() {
    let obj = crate::pal_alloc().calloc(1, NSARRAY_SIZE) as u64;
    let ret = shim_nsarray_init_with_objects(obj, 0, 0xAAA, 0xBBB, 0xCCC, 0, 0, 0, 0, 0);
    assert_eq!(ret, obj);
    assert_eq!(read_field(obj, 0), crate::nsobj::nsarray_class_addr());
    assert_eq!(shim_nsarray_count(obj, 0), 3);
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 0), 0xAAA);
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 1), 0xBBB);
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 2), 0xCCC);
    free_buf(read_field(obj, NSARRAY_ELEMS_OFF));
    free_obj(obj);
}

// ---------------------------------------------------------------------------
// shim_nsarray_count
// ---------------------------------------------------------------------------

#[test]
fn count_nil_is_zero() {
    assert_eq!(shim_nsarray_count(0, 0), 0);
}

#[test]
fn count_of_empty() {
    let arr = make_nsarray(&[]);
    assert_eq!(shim_nsarray_count(arr, 0), 0);
    free_obj(arr);
}

#[test]
fn count_of_three() {
    let arr = make_nsarray(&[0xA, 0xB, 0xC]);
    assert_eq!(shim_nsarray_count(arr, 0), 3);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// shim_nsarray_object_at_index
// ---------------------------------------------------------------------------

#[test]
fn object_at_index_nil_obj_returns_zero() {
    assert_eq!(shim_nsarray_object_at_index(0, 0, 0), 0);
}

#[test]
fn object_at_index_valid() {
    let arr = make_nsarray(&[0xAAA, 0xBBB, 0xCCC]);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 2), 0xCCC);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn object_at_index_out_of_bounds_returns_zero() {
    let arr = make_nsarray(&[0xAAA]);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 1), 0);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 99), 0);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn object_at_index_empty_array_returns_zero() {
    let arr = make_nsarray(&[]);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 0), 0);
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// shim_nsarray_first_object
// ---------------------------------------------------------------------------

#[test]
fn first_object_nil_returns_zero() {
    assert_eq!(shim_nsarray_first_object(0, 0), 0);
}

#[test]
fn first_object_empty_returns_zero() {
    let arr = make_nsarray(&[]);
    assert_eq!(shim_nsarray_first_object(arr, 0), 0);
    free_obj(arr);
}

#[test]
fn first_object_returns_first() {
    let arr = make_nsarray(&[0x111, 0x222, 0x333]);
    assert_eq!(shim_nsarray_first_object(arr, 0), 0x111);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// shim_nsarray_last_object
// ---------------------------------------------------------------------------

#[test]
fn last_object_nil_returns_zero() {
    assert_eq!(shim_nsarray_last_object(0, 0), 0);
}

#[test]
fn last_object_empty_returns_zero() {
    let arr = make_nsarray(&[]);
    assert_eq!(shim_nsarray_last_object(arr, 0), 0);
    free_obj(arr);
}

#[test]
fn last_object_returns_last() {
    let arr = make_nsarray(&[0x111, 0x222, 0x333]);
    assert_eq!(shim_nsarray_last_object(arr, 0), 0x333);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn first_last_single_element_same() {
    let arr = make_nsarray(&[0xABC]);
    assert_eq!(shim_nsarray_first_object(arr, 0), 0xABC);
    assert_eq!(shim_nsarray_last_object(arr, 0), 0xABC);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// shim_nsarray_contains_object
// ---------------------------------------------------------------------------

#[test]
fn contains_object_nil_obj_returns_zero() {
    assert_eq!(shim_nsarray_contains_object(0, 0, 0xAAA), 0);
}

#[test]
fn contains_object_nil_target_returns_zero() {
    let arr = make_nsarray(&[0xAAA]);
    assert_eq!(shim_nsarray_contains_object(arr, 0, 0), 0);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn contains_object_found_by_pointer() {
    let a = make_string("aaa");
    let b = make_string("bbb");
    let c = make_string("ccc");
    let arr = make_nsarray(&[a, b, c]);
    // Pointer-equal match on b
    assert_eq!(shim_nsarray_contains_object(arr, 0, b), 1);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn contains_object_not_found() {
    let a = make_string("aaa");
    let b = make_string("bbb");
    let needle = make_string("zzz");
    let arr = make_nsarray(&[a, b]);
    assert_eq!(shim_nsarray_contains_object(arr, 0, needle), 0);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn contains_object_found_by_string_equality() {
    let s1 = make_string("hello");
    let s2 = make_string("hello"); // different pointer, same content
    assert_ne!(s1, s2);
    let arr = make_nsarray(&[s1]);
    assert_eq!(shim_nsarray_contains_object(arr, 0, s2), 1);
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn contains_object_empty_array() {
    let arr = make_nsarray(&[]);
    assert_eq!(shim_nsarray_contains_object(arr, 0, 0xAAA), 0);
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// shim_nsarray_description
// ---------------------------------------------------------------------------

#[test]
fn description_nil_returns_zero() {
    assert_eq!(shim_nsarray_description(0, 0), 0);
}

#[test]
fn description_empty_array() {
    let arr = make_nsarray(&[]);
    let desc = shim_nsarray_description(arr, 0);
    assert_ne!(desc, 0);
    assert_string_eq(desc, "()");
    free_obj(arr);
}

#[test]
fn description_with_strings() {
    let s1 = make_string("alpha");
    let s2 = make_string("beta");
    let arr = make_nsarray(&[s1, s2]);
    let desc = shim_nsarray_description(arr, 0);
    assert_ne!(desc, 0);
    assert_string_eq(desc, "(\"alpha\", \"beta\")");
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn description_with_mixed_elements() {
    // Use valid string objects — description includes quoted content
    let s = make_string("test");
    let arr = make_nsarray(&[s]);
    let desc = shim_nsarray_description(arr, 0);
    assert_ne!(desc, 0);
    let desc_text = read_string(desc);
    assert!(
        desc_text.starts_with('('),
        "desc should start with '(': {}",
        desc_text
    );
    assert!(
        desc_text.contains("test"),
        "desc should contain element: {}",
        desc_text
    );
    assert!(
        desc_text.ends_with(')'),
        "desc should end with ')': {}",
        desc_text
    );
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// roundtrip: full create → query → cleanup
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_with_elements() {
    let arr = make_nsarray(&[0xAAA, 0xBBB, 0xCCC]);
    assert_eq!(shim_nsarray_count(arr, 0), 3);
    assert_eq!(shim_nsarray_first_object(arr, 0), 0xAAA);
    assert_eq!(shim_nsarray_last_object(arr, 0), 0xCCC);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
    assert_eq!(shim_nsarray_object_at_index(arr, 0, 3), 0); // OOB
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// shim_nsarray_count_by_enumerating_with_state
// ---------------------------------------------------------------------------

#[test]
fn fast_enumeration_nil_inputs_return_zero() {
    assert_eq!(
        shim_nsarray_count_by_enumerating_with_state(0, 0, 1, 1, 1),
        0
    );
    let arr = make_nsarray(&[0xAAA]);
    assert_eq!(
        shim_nsarray_count_by_enumerating_with_state(arr, 0, 0, 1, 1),
        0
    );
    assert_eq!(
        shim_nsarray_count_by_enumerating_with_state(arr, 0, 1, 0, 1),
        0
    );
    assert_eq!(
        shim_nsarray_count_by_enumerating_with_state(arr, 0, 1, 1, 0),
        0
    );
    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

#[test]
fn fast_enumeration_batches_results() {
    let arr = make_nsarray(&[0xAAA, 0xBBB, 0xCCC]);
    let mut state = [0u64; 8];
    let mut stackbuf = [0u64; 2];

    let first = shim_nsarray_count_by_enumerating_with_state(
        arr,
        0,
        state.as_mut_ptr() as u64,
        stackbuf.as_mut_ptr() as u64,
        stackbuf.len() as u64,
    );
    assert_eq!(first, 2);
    assert_eq!(stackbuf[0], 0xAAA);
    assert_eq!(stackbuf[1], 0xBBB);
    assert_eq!(state[0], 2);
    assert_eq!(state[1], stackbuf.as_ptr() as u64);
    assert_ne!(state[2], 0);

    stackbuf.fill(0);
    let second = shim_nsarray_count_by_enumerating_with_state(
        arr,
        0,
        state.as_mut_ptr() as u64,
        stackbuf.as_mut_ptr() as u64,
        stackbuf.len() as u64,
    );
    assert_eq!(second, 1);
    assert_eq!(stackbuf[0], 0xCCC);

    let third = shim_nsarray_count_by_enumerating_with_state(
        arr,
        0,
        state.as_mut_ptr() as u64,
        stackbuf.as_mut_ptr() as u64,
        stackbuf.len() as u64,
    );
    assert_eq!(third, 0);

    free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
    free_obj(arr);
}

// ---------------------------------------------------------------------------
// shim_nsarray_init_with_array
// ---------------------------------------------------------------------------

#[test]
fn init_with_array_nil_obj_returns_zero() {
    assert_eq!(shim_nsarray_init_with_array(0, 0, 0), 0);
}

#[test]
fn init_with_array_nil_source_returns_empty() {
    let obj = crate::pal_alloc().calloc(1, 24) as u64; // NSARRAY_SIZE
    let result = shim_nsarray_init_with_array(obj, 0, 0);
    assert_eq!(result, obj);
    assert_eq!(shim_nsarray_count(obj, 0), 0);
    free_obj(obj);
}

#[test]
fn init_with_array_copies_elements() {
    let source = make_nsarray(&[0xAAA, 0xBBB, 0xCCC]);
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    let result = shim_nsarray_init_with_array(obj, 0, source);
    assert_eq!(result, obj);
    assert_eq!(shim_nsarray_count(obj, 0), 3);
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 0), 0xAAA);
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 1), 0xBBB);
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 2), 0xCCC);
    free_buf(read_field(source, NSARRAY_ELEMS_OFF));
    free_obj(source);
    free_buf(read_field(obj, NSARRAY_ELEMS_OFF));
    free_obj(obj);
}

#[test]
fn init_with_array_empty_source() {
    let source = make_nsarray(&[]);
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    let result = shim_nsarray_init_with_array(obj, 0, source);
    assert_eq!(result, obj);
    assert_eq!(shim_nsarray_count(obj, 0), 0);
    free_obj(source);
    free_obj(obj);
}
