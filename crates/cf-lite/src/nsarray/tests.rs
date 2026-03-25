use super::*;

#[test]
fn count_of_empty() {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    unsafe {
        *(obj as *mut u64) = 0xDEAD;
    } // fake ISA
    assert_eq!(shim_nsarray_count(obj, 0), 0);
    crate::pal_alloc().free(obj as *mut u8);
}

#[test]
fn count_nil_is_zero() {
    assert_eq!(shim_nsarray_count(0, 0), 0);
}

#[test]
fn object_at_index_out_of_bounds_returns_nil() {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    unsafe {
        *(obj as *mut u64) = 0xDEAD;
    }
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 0), 0);
    crate::pal_alloc().free(obj as *mut u8);
}

#[test]
fn first_last_on_empty_returns_nil() {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    unsafe {
        *(obj as *mut u64) = 0xDEAD;
    }
    assert_eq!(shim_nsarray_first_object(obj, 0), 0);
    assert_eq!(shim_nsarray_last_object(obj, 0), 0);
    crate::pal_alloc().free(obj as *mut u8);
}

#[test]
fn roundtrip_with_elements() {
    // Simulate 3 elements
    let elems: [u64; 3] = [0xAAA, 0xBBB, 0xCCC];
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    unsafe {
        *(obj as *mut u64) = 0xDEAD; // ISA
        *((obj as usize + 8) as *mut u64) = 3; // count
        let buf = crate::pal_alloc().malloc(24) as u64;
        std::ptr::copy_nonoverlapping(elems.as_ptr() as *const u8, buf as *mut u8, 24);
        *((obj as usize + 16) as *mut u64) = buf; // elements_ptr
    }

    assert_eq!(shim_nsarray_count(obj, 0), 3);
    assert_eq!(shim_nsarray_first_object(obj, 0), 0xAAA);
    assert_eq!(shim_nsarray_last_object(obj, 0), 0xCCC);
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 1), 0xBBB);
    assert_eq!(shim_nsarray_object_at_index(obj, 0, 3), 0); // OOB

    // Cleanup
    let buf = unsafe { *((obj as usize + 16) as *const u64) };
    crate::pal_alloc().free(buf as *mut u8);
    crate::pal_alloc().free(obj as *mut u8);
}
