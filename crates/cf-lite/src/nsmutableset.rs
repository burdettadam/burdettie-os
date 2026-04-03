//! NSMutableSet shim: mutable unordered collection of unique objects.
//!
//! Object layout — same as NSSet (24 bytes):
//!
//!   offset  0: isa          (u64 — pointer to NSMutableSet class)
//!   offset  8: count        (u64 — number of elements)
//!   offset 16: elements_ptr (u64 — pointer to malloc'd id[] array)
//!
//! Mutation methods reallocate the backing array as needed.

use crate::cfstring;
use crate::nsobj;

const COUNT_OFF: usize = 8;
const ELEMS_OFF: usize = 16;

fn set_count(obj: u64) -> u64 {
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

fn set_elems(obj: u64) -> u64 {
    unsafe { *((obj as usize + ELEMS_OFF) as *const u64) }
}

/// Check if needle exists in the set.
fn contains(obj: u64, needle: u64) -> bool {
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return false;
    }
    let needle_cstr = cfstring::extract_cfstring_cstr(needle);
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if e == needle {
            return true;
        }
        if let Some(ns) = &needle_cstr {
            if let Some(es) = cfstring::extract_cfstring_cstr(e) {
                if es == *ns {
                    return true;
                }
            }
        }
    }
    false
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSMutableSet set]` — empty mutable set.
pub extern "C" fn shim_nsmutableset_set(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsmutableset_class_addr();
    }
    obj
}

/// `+[NSMutableSet setWithCapacity:]` — same as `set` (we don't pre-allocate).
pub extern "C" fn shim_nsmutableset_set_with_capacity(_cls: u64, _cmd: u64, _cap: u64) -> u64 {
    shim_nsmutableset_set(0, 0)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSMutableSet addObject:]` — adds the object if not already present.
pub extern "C" fn shim_nsmutableset_add_object(obj: u64, _cmd: u64, element: u64) {
    if obj == 0 || element == 0 {
        return;
    }
    if contains(obj, element) {
        return;
    }

    let count = set_count(obj) as usize;
    let old_elems = set_elems(obj);
    let new_count = count + 1;
    let new_buf = crate::pal_alloc().malloc(new_count * 8) as u64;
    if new_buf == 0 {
        return;
    }

    // Copy existing elements
    if count > 0 && old_elems != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(old_elems as *const u8, new_buf as *mut u8, count * 8);
        }
    }
    // Append new element
    unsafe {
        *((new_buf as usize + count * 8) as *mut u64) = element;
    }

    // Free old buffer
    if old_elems != 0 {
        crate::pal_alloc().free(old_elems as *mut u8);
    }

    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = new_count as u64;
        *((obj as usize + ELEMS_OFF) as *mut u64) = new_buf;
    }
}

/// `-[NSMutableSet addObjectsFromArray:]` — adds unique objects from an array.
pub extern "C" fn shim_nsmutableset_add_objects_from_array(obj: u64, _cmd: u64, array: u64) {
    if obj == 0 || array == 0 {
        return;
    }

    let count = crate::nsarray::shim_nsarray_count(array, 0);
    for i in 0..count {
        let element = crate::nsarray::shim_nsarray_object_at_index(array, 0, i);
        shim_nsmutableset_add_object(obj, 0, element);
    }
}

/// `-[NSMutableSet removeObject:]` — removes the object if present.
pub extern "C" fn shim_nsmutableset_remove_object(obj: u64, _cmd: u64, element: u64) {
    if obj == 0 || element == 0 {
        return;
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return;
    }

    let element_cstr = cfstring::extract_cfstring_cstr(element);

    // Find the index to remove
    let mut found_idx = None;
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if e == element {
            found_idx = Some(i);
            break;
        }
        if let Some(ns) = &element_cstr {
            if let Some(es) = cfstring::extract_cfstring_cstr(e) {
                if es == *ns {
                    found_idx = Some(i);
                    break;
                }
            }
        }
    }

    if let Some(idx) = found_idx {
        // Shift remaining elements down
        for i in idx..count - 1 {
            let next = unsafe { *((elems as usize + (i + 1) * 8) as *const u64) };
            unsafe {
                *((elems as usize + i * 8) as *mut u64) = next;
            }
        }
        unsafe {
            *((obj as usize + COUNT_OFF) as *mut u64) = (count - 1) as u64;
        }
    }
}

/// `-[NSMutableSet removeAllObjects]`
pub extern "C" fn shim_nsmutableset_remove_all_objects(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let elems = set_elems(obj);
    if elems != 0 {
        crate::pal_alloc().free(elems as *mut u8);
    }
    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = 0;
        *((obj as usize + ELEMS_OFF) as *mut u64) = 0;
    }
}

/// `-[NSMutableSet unionSet:]` — adds all objects from another set.
pub extern "C" fn shim_nsmutableset_union_set(obj: u64, _cmd: u64, other: u64) {
    if obj == 0 || other == 0 {
        return;
    }
    let other_count = set_count(other) as usize;
    let other_elems = set_elems(other);
    if other_count == 0 || other_elems == 0 {
        return;
    }
    for i in 0..other_count {
        let e = unsafe { *((other_elems as usize + i * 8) as *const u64) };
        shim_nsmutableset_add_object(obj, 0, e);
    }
}

/// `-[NSMutableSet intersectSet:]` — removes objects not in `other`.
pub extern "C" fn shim_nsmutableset_intersect_set(obj: u64, _cmd: u64, other: u64) {
    if obj == 0 {
        return;
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return;
    }

    // Collect elements to keep
    let mut keep: Vec<u64> = Vec::new();
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if other != 0 && crate::nsset::contains(other, e) {
            keep.push(e);
        }
    }

    // Rebuild the elements buffer
    if elems != 0 {
        crate::pal_alloc().free(elems as *mut u8);
    }
    if keep.is_empty() {
        unsafe {
            *((obj as usize + COUNT_OFF) as *mut u64) = 0;
            *((obj as usize + ELEMS_OFF) as *mut u64) = 0;
        }
    } else {
        let buf = crate::pal_alloc().malloc(keep.len() * 8) as u64;
        if buf != 0 {
            for (i, &e) in keep.iter().enumerate() {
                unsafe {
                    *((buf as usize + i * 8) as *mut u64) = e;
                }
            }
        }
        unsafe {
            *((obj as usize + COUNT_OFF) as *mut u64) = keep.len() as u64;
            *((obj as usize + ELEMS_OFF) as *mut u64) = buf;
        }
    }
}

/// `-[NSMutableSet minusSet:]` — removes objects present in `other`.
pub extern "C" fn shim_nsmutableset_minus_set(obj: u64, _cmd: u64, other: u64) {
    if obj == 0 || other == 0 {
        return;
    }
    let other_count = set_count(other) as usize;
    let other_elems = set_elems(other);
    if other_count == 0 || other_elems == 0 {
        return;
    }
    for i in 0..other_count {
        let e = unsafe { *((other_elems as usize + i * 8) as *const u64) };
        shim_nsmutableset_remove_object(obj, 0, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // -- Construction -------------------------------------------------

    #[test]
    fn set_returns_empty_set() {
        let s = shim_nsmutableset_set(0, 0);
        assert_ne!(s, 0);
        assert_eq!(set_count(s), 0);
        assert_eq!(set_elems(s), 0);
        free_obj(s);
    }

    #[test]
    fn set_with_capacity_returns_empty_set() {
        let s = shim_nsmutableset_set_with_capacity(0, 0, 16);
        assert_ne!(s, 0);
        assert_eq!(set_count(s), 0);
        free_obj(s);
    }

    // -- addObject / count tracking -----------------------------------

    #[test]
    fn add_object_increments_count() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("alpha");
        let b = make_string("beta");

        shim_nsmutableset_add_object(s, 0, a);
        assert_eq!(set_count(s), 1);

        shim_nsmutableset_add_object(s, 0, b);
        assert_eq!(set_count(s), 2);

        // Cross-check via nsset read helpers (same layout)
        assert_eq!(crate::nsset::shim_nsset_count(s, 0), 2);
        assert_eq!(crate::nsset::shim_nsset_contains_object(s, 0, a), 1);
        assert_eq!(crate::nsset::shim_nsset_contains_object(s, 0, b), 1);

        free_obj(a);
        free_obj(b);
        shim_nsmutableset_remove_all_objects(s, 0);
        free_obj(s);
    }

    // -- Deduplication ------------------------------------------------

    #[test]
    fn add_duplicate_pointer_is_noop() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("dup");

        shim_nsmutableset_add_object(s, 0, a);
        shim_nsmutableset_add_object(s, 0, a); // same pointer
        assert_eq!(set_count(s), 1);

        free_obj(a);
        shim_nsmutableset_remove_all_objects(s, 0);
        free_obj(s);
    }

    #[test]
    fn add_duplicate_string_content_is_noop() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("same");
        let b = make_string("same"); // different pointer, same content

        shim_nsmutableset_add_object(s, 0, a);
        shim_nsmutableset_add_object(s, 0, b);
        assert_eq!(set_count(s), 1);

        free_obj(a);
        free_obj(b);
        shim_nsmutableset_remove_all_objects(s, 0);
        free_obj(s);
    }

    #[test]
    fn add_objects_from_array_merges_unique_elements() {
        let s = shim_nsmutableset_set(0, 0);
        let existing = make_string("existing");
        let duplicate = make_string("existing");
        let fresh = make_string("fresh");
        let array = crate::nsarray::make_nsarray(&[duplicate, fresh]);

        shim_nsmutableset_add_object(s, 0, existing);
        shim_nsmutableset_add_objects_from_array(s, 0, array);

        assert_eq!(set_count(s), 2);
        assert!(contains(s, existing));
        assert!(contains(s, fresh));

        free_obj(existing);
        free_obj(duplicate);
        free_obj(fresh);
        free_buf(read_field(array, crate::nsarray::NSARRAY_ELEMS_OFF));
        free_obj(array);
        shim_nsmutableset_remove_all_objects(s, 0);
        free_obj(s);
    }

    #[test]
    fn add_objects_from_array_nil_safety() {
        shim_nsmutableset_add_objects_from_array(0, 0, 0x1234);

        let s = shim_nsmutableset_set(0, 0);
        shim_nsmutableset_add_objects_from_array(s, 0, 0);
        assert_eq!(set_count(s), 0);
        free_obj(s);
    }

    // -- removeObject -------------------------------------------------

    #[test]
    fn remove_object_decrements_count() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("alpha");
        let b = make_string("beta");

        shim_nsmutableset_add_object(s, 0, a);
        shim_nsmutableset_add_object(s, 0, b);
        assert_eq!(set_count(s), 2);

        shim_nsmutableset_remove_object(s, 0, a);
        assert_eq!(set_count(s), 1);
        assert!(!contains(s, a));
        assert!(contains(s, b));

        free_obj(a);
        free_obj(b);
        shim_nsmutableset_remove_all_objects(s, 0);
        free_obj(s);
    }

    #[test]
    fn remove_by_string_content() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("target");
        shim_nsmutableset_add_object(s, 0, a);

        // Remove using a different pointer with the same content
        let a2 = make_string("target");
        shim_nsmutableset_remove_object(s, 0, a2);
        assert_eq!(set_count(s), 0);

        free_obj(a);
        free_obj(a2);
        free_obj(s);
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("present");
        let b = make_string("absent");

        shim_nsmutableset_add_object(s, 0, a);
        shim_nsmutableset_remove_object(s, 0, b);
        assert_eq!(set_count(s), 1);

        free_obj(a);
        free_obj(b);
        shim_nsmutableset_remove_all_objects(s, 0);
        free_obj(s);
    }

    // -- removeAllObjects ---------------------------------------------

    #[test]
    fn remove_all_objects_clears_set() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("x");
        let b = make_string("y");

        shim_nsmutableset_add_object(s, 0, a);
        shim_nsmutableset_add_object(s, 0, b);
        shim_nsmutableset_remove_all_objects(s, 0);

        assert_eq!(set_count(s), 0);
        assert_eq!(set_elems(s), 0);

        free_obj(a);
        free_obj(b);
        free_obj(s);
    }

    #[test]
    fn remove_all_on_empty_set_is_safe() {
        let s = shim_nsmutableset_set(0, 0);
        shim_nsmutableset_remove_all_objects(s, 0); // must not crash
        assert_eq!(set_count(s), 0);
        free_obj(s);
    }

    // -- unionSet -----------------------------------------------------

    #[test]
    fn union_set_merges_elements() {
        let s1 = shim_nsmutableset_set(0, 0);
        let s2 = shim_nsmutableset_set(0, 0);

        let a = make_string("a");
        let b = make_string("b");
        let c = make_string("c");

        shim_nsmutableset_add_object(s1, 0, a);
        shim_nsmutableset_add_object(s2, 0, b);
        shim_nsmutableset_add_object(s2, 0, c);

        shim_nsmutableset_union_set(s1, 0, s2);
        assert_eq!(set_count(s1), 3);
        assert!(contains(s1, a));
        assert!(contains(s1, b));
        assert!(contains(s1, c));

        free_obj(a);
        free_obj(b);
        free_obj(c);
        shim_nsmutableset_remove_all_objects(s1, 0);
        shim_nsmutableset_remove_all_objects(s2, 0);
        free_obj(s1);
        free_obj(s2);
    }

    #[test]
    fn union_set_deduplicates() {
        let s1 = shim_nsmutableset_set(0, 0);
        let s2 = shim_nsmutableset_set(0, 0);

        let a1 = make_string("shared");
        let a2 = make_string("shared");

        shim_nsmutableset_add_object(s1, 0, a1);
        shim_nsmutableset_add_object(s2, 0, a2);

        shim_nsmutableset_union_set(s1, 0, s2);
        assert_eq!(set_count(s1), 1);

        free_obj(a1);
        free_obj(a2);
        shim_nsmutableset_remove_all_objects(s1, 0);
        shim_nsmutableset_remove_all_objects(s2, 0);
        free_obj(s1);
        free_obj(s2);
    }

    // -- intersectSet -------------------------------------------------

    #[test]
    fn intersect_set_keeps_common_elements() {
        let s1 = shim_nsmutableset_set(0, 0);
        let s2 = shim_nsmutableset_set(0, 0);

        let a = make_string("common");
        let b = make_string("only_s1");
        let c = make_string("common"); // same content as a

        shim_nsmutableset_add_object(s1, 0, a);
        shim_nsmutableset_add_object(s1, 0, b);
        shim_nsmutableset_add_object(s2, 0, c);

        shim_nsmutableset_intersect_set(s1, 0, s2);
        assert_eq!(set_count(s1), 1);

        free_obj(a);
        free_obj(b);
        free_obj(c);
        shim_nsmutableset_remove_all_objects(s1, 0);
        shim_nsmutableset_remove_all_objects(s2, 0);
        free_obj(s1);
        free_obj(s2);
    }

    #[test]
    fn intersect_with_empty_clears_set() {
        let s1 = shim_nsmutableset_set(0, 0);
        let s2 = shim_nsmutableset_set(0, 0);

        let a = make_string("x");
        shim_nsmutableset_add_object(s1, 0, a);

        shim_nsmutableset_intersect_set(s1, 0, s2);
        assert_eq!(set_count(s1), 0);

        free_obj(a);
        free_obj(s1);
        free_obj(s2);
    }

    // -- minusSet -----------------------------------------------------

    #[test]
    fn minus_set_removes_common_elements() {
        let s1 = shim_nsmutableset_set(0, 0);
        let s2 = shim_nsmutableset_set(0, 0);

        let a = make_string("keep");
        let b = make_string("remove_me");
        let c = make_string("remove_me");

        shim_nsmutableset_add_object(s1, 0, a);
        shim_nsmutableset_add_object(s1, 0, b);
        shim_nsmutableset_add_object(s2, 0, c);

        shim_nsmutableset_minus_set(s1, 0, s2);
        assert_eq!(set_count(s1), 1);
        assert!(contains(s1, a));

        free_obj(a);
        free_obj(b);
        free_obj(c);
        shim_nsmutableset_remove_all_objects(s1, 0);
        shim_nsmutableset_remove_all_objects(s2, 0);
        free_obj(s1);
        free_obj(s2);
    }

    #[test]
    fn minus_empty_set_is_noop() {
        let s1 = shim_nsmutableset_set(0, 0);
        let s2 = shim_nsmutableset_set(0, 0);

        let a = make_string("stays");
        shim_nsmutableset_add_object(s1, 0, a);

        shim_nsmutableset_minus_set(s1, 0, s2);
        assert_eq!(set_count(s1), 1);

        free_obj(a);
        shim_nsmutableset_remove_all_objects(s1, 0);
        free_obj(s1);
        free_obj(s2);
    }

    // -- Nil/zero safety ----------------------------------------------

    #[test]
    fn set_nil_safety() {
        // Class methods with cls=0 still work (they ignore cls)
        let s = shim_nsmutableset_set(0, 0);
        assert_ne!(s, 0);
        free_obj(s);
    }

    #[test]
    fn add_object_nil_obj() {
        shim_nsmutableset_add_object(0, 0, 42); // must not crash
    }

    #[test]
    fn add_object_nil_element() {
        let s = shim_nsmutableset_set(0, 0);
        shim_nsmutableset_add_object(s, 0, 0); // must not crash
        assert_eq!(set_count(s), 0);
        free_obj(s);
    }

    #[test]
    fn remove_object_nil_obj() {
        shim_nsmutableset_remove_object(0, 0, 42); // must not crash
    }

    #[test]
    fn remove_object_nil_element() {
        let s = shim_nsmutableset_set(0, 0);
        shim_nsmutableset_remove_object(s, 0, 0); // must not crash
        free_obj(s);
    }

    #[test]
    fn remove_all_nil_obj() {
        shim_nsmutableset_remove_all_objects(0, 0); // must not crash
    }

    #[test]
    fn union_set_nil_obj() {
        let s2 = shim_nsmutableset_set(0, 0);
        shim_nsmutableset_union_set(0, 0, s2); // must not crash
        free_obj(s2);
    }

    #[test]
    fn union_set_nil_other() {
        let s1 = shim_nsmutableset_set(0, 0);
        shim_nsmutableset_union_set(s1, 0, 0); // must not crash
        assert_eq!(set_count(s1), 0);
        free_obj(s1);
    }

    #[test]
    fn intersect_set_nil_obj() {
        shim_nsmutableset_intersect_set(0, 0, 42); // must not crash
    }

    #[test]
    fn intersect_set_nil_other_clears() {
        let s1 = shim_nsmutableset_set(0, 0);
        let a = make_string("x");
        shim_nsmutableset_add_object(s1, 0, a);

        // intersect with nil "other" — nothing is in nil, so set should empty
        shim_nsmutableset_intersect_set(s1, 0, 0);
        assert_eq!(set_count(s1), 0);

        free_obj(a);
        free_obj(s1);
    }

    #[test]
    fn minus_set_nil_obj() {
        let s2 = shim_nsmutableset_set(0, 0);
        shim_nsmutableset_minus_set(0, 0, s2); // must not crash
        free_obj(s2);
    }

    #[test]
    fn minus_set_nil_other() {
        let s1 = shim_nsmutableset_set(0, 0);
        let a = make_string("x");
        shim_nsmutableset_add_object(s1, 0, a);

        shim_nsmutableset_minus_set(s1, 0, 0); // must not crash
        assert_eq!(set_count(s1), 1);

        free_obj(a);
        shim_nsmutableset_remove_all_objects(s1, 0);
        free_obj(s1);
    }

    // -- Edge: operations on empty set --------------------------------

    #[test]
    fn remove_from_empty_set() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("ghost");
        shim_nsmutableset_remove_object(s, 0, a);
        assert_eq!(set_count(s), 0);
        free_obj(a);
        free_obj(s);
    }

    #[test]
    fn cross_check_nsset_count_on_mutable() {
        let s = shim_nsmutableset_set(0, 0);
        let a = make_string("one");
        let b = make_string("two");
        let c = make_string("three");

        shim_nsmutableset_add_object(s, 0, a);
        shim_nsmutableset_add_object(s, 0, b);
        shim_nsmutableset_add_object(s, 0, c);

        // NSSet's count/contains shims work because layout is identical
        assert_eq!(crate::nsset::shim_nsset_count(s, 0), 3);
        assert_eq!(crate::nsset::shim_nsset_contains_object(s, 0, a), 1);
        assert_eq!(crate::nsset::shim_nsset_contains_object(s, 0, b), 1);
        assert_eq!(crate::nsset::shim_nsset_contains_object(s, 0, c), 1);

        let absent = make_string("nope");
        assert_eq!(crate::nsset::shim_nsset_contains_object(s, 0, absent), 0);

        free_obj(a);
        free_obj(b);
        free_obj(c);
        free_obj(absent);
        shim_nsmutableset_remove_all_objects(s, 0);
        free_obj(s);
    }
}
