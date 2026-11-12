//! NSOrderedSet / NSMutableOrderedSet shim: ordered unique collection.
//!
//! Object layout (32 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: count        (u64)
//!   offset 16: elems_ptr    (u64 → malloc'd u64[] array of unique elements in order)
//!   offset 24: capacity     (u64)

use crate::cfstring;
use crate::nsobj;

const COUNT_OFF: usize = 8;
const ELEMS_OFF: usize = 16;
const CAP_OFF: usize = 24;
const INITIAL_CAP: u64 = 4;

fn elements_equal(a: u64, b: u64) -> bool {
    if a == b {
        return true;
    }
    // String equality fallback
    if let (Some(sa), Some(sb)) = (
        cfstring::extract_cfstring_cstr(a),
        cfstring::extract_cfstring_cstr(b),
    ) {
        return sa == sb;
    }
    false
}

fn contains_element(elems: u64, count: u64, target: u64) -> bool {
    for i in 0..count as usize {
        let elem = unsafe { *((elems as usize + i * 8) as *const u64) };
        if elements_equal(elem, target) {
            return true;
        }
    }
    false
}

// ── NSOrderedSet ───────────────────────────────────────────────────

/// `+[NSOrderedSet orderedSetWithArray:]`
pub extern "C" fn shim_nsorderedset_ordered_set_with_array(
    _cls: u64,
    _cmd: u64,
    array: u64,
) -> u64 {
    let count = if array != 0 {
        crate::nsarray::shim_nsarray_count(array, 0)
    } else {
        0
    };
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsorderedset_class_addr();
    }

    let cap = if count == 0 { INITIAL_CAP } else { count };
    let buf = crate::pal_alloc().calloc(cap as usize, 8) as u64;
    unsafe {
        *((obj as usize + ELEMS_OFF) as *mut u64) = buf;
        *((obj as usize + CAP_OFF) as *mut u64) = cap;
    }

    // Add unique elements in order
    let mut n = 0u64;
    for i in 0..count {
        let elem = crate::nsarray::shim_nsarray_object_at_index(array, 0, i);
        if !contains_element(buf, n, elem) {
            unsafe {
                *((buf as usize + n as usize * 8) as *mut u64) = elem;
            }
            n += 1;
        }
    }
    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = n;
    }
    obj
}

/// `+[NSOrderedSet orderedSet]`
pub extern "C" fn shim_nsorderedset_ordered_set(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsorderedset_class_addr();
    }
    let buf = crate::pal_alloc().calloc(INITIAL_CAP as usize, 8) as u64;
    unsafe {
        *((obj as usize + ELEMS_OFF) as *mut u64) = buf;
        *((obj as usize + CAP_OFF) as *mut u64) = INITIAL_CAP;
    }
    obj
}

/// `-[NSOrderedSet count]`
pub extern "C" fn shim_nsorderedset_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

/// `-[NSOrderedSet objectAtIndex:]`
pub extern "C" fn shim_nsorderedset_object_at_index(obj: u64, _cmd: u64, idx: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    if idx >= count {
        return 0;
    }
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe { *((elems as usize + idx as usize * 8) as *const u64) }
}

/// `-[NSOrderedSet firstObject]`
pub extern "C" fn shim_nsorderedset_first_object(obj: u64, _cmd: u64) -> u64 {
    shim_nsorderedset_object_at_index(obj, _cmd, 0)
}

/// `-[NSOrderedSet lastObject]`
pub extern "C" fn shim_nsorderedset_last_object(obj: u64, _cmd: u64) -> u64 {
    let count = shim_nsorderedset_count(obj, 0);
    if count == 0 {
        return 0;
    }
    shim_nsorderedset_object_at_index(obj, 0, count - 1)
}

/// `-[NSOrderedSet containsObject:]`
pub extern "C" fn shim_nsorderedset_contains_object(obj: u64, _cmd: u64, target: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    u8::from(contains_element(elems, count, target))
}

/// `-[NSOrderedSet indexOfObject:]`
pub extern "C" fn shim_nsorderedset_index_of_object(obj: u64, _cmd: u64, target: u64) -> u64 {
    if obj == 0 {
        return u64::MAX;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    for i in 0..count as usize {
        let elem = unsafe { *((elems as usize + i * 8) as *const u64) };
        if elements_equal(elem, target) {
            return i as u64;
        }
    }
    u64::MAX // NSNotFound
}

/// `-[NSOrderedSet array]` — returns an NSArray of all elements in order.
pub extern "C" fn shim_nsorderedset_array(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if count == 0 || elems == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }
    let mut items = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        items.push(unsafe { *((elems as usize + i * 8) as *const u64) });
    }
    crate::nsarray::make_nsarray(&items)
}

/// `-[NSOrderedSet set]` — returns an NSSet containing all elements.
pub extern "C" fn shim_nsorderedset_set(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if count == 0 || elems == 0 {
        return crate::nsset::shim_nsset_set(0, 0);
    }
    crate::nsset::shim_nsset_with_objects_count(0, 0, elems, count)
}

/// `-[NSOrderedSet reverseObjectEnumerator]` — returns an enumerator that
/// iterates elements in reverse order.
pub extern "C" fn shim_nsorderedset_reverse_object_enumerator(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) } as usize;
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if count == 0 || elems == 0 {
        let empty = crate::nsarray::make_nsarray(&[]);
        return crate::nsenumerator::alloc_enumerator(empty);
    }
    let mut items: Vec<u64> = Vec::with_capacity(count);
    for i in (0..count).rev() {
        items.push(unsafe { *((elems as usize + i * 8) as *const u64) });
    }
    let reversed = crate::nsarray::make_nsarray(&items);
    crate::nsenumerator::alloc_enumerator(reversed)
}

/// `-[NSOrderedSet description]`
pub extern "C" fn shim_nsorderedset_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let desc = format!("<NSOrderedSet: {} objects>", count);
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

// ── NSMutableOrderedSet ────────────────────────────────────────────

unsafe fn grow_ordered_set(obj: u64) {
    let count = *((obj as usize + COUNT_OFF) as *const u64);
    let cap = *((obj as usize + CAP_OFF) as *const u64);
    if count < cap {
        return;
    }
    let new_cap = if cap == 0 { INITIAL_CAP } else { cap * 2 };
    let old_buf = *((obj as usize + ELEMS_OFF) as *const u64);
    let new_buf = crate::pal_alloc().realloc(old_buf as *mut u8, new_cap as usize * 8) as u64;
    if new_buf != 0 {
        let start = (new_buf as usize + count as usize * 8) as *mut u8;
        std::ptr::write_bytes(start, 0, (new_cap - count) as usize * 8);
        *((obj as usize + ELEMS_OFF) as *mut u64) = new_buf;
        *((obj as usize + CAP_OFF) as *mut u64) = new_cap;
    }
}

/// `+[NSMutableOrderedSet orderedSet]`
pub extern "C" fn shim_nsmutableorderedset_ordered_set(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsmutableorderedset_class_addr();
    }
    let buf = crate::pal_alloc().calloc(INITIAL_CAP as usize, 8) as u64;
    unsafe {
        *((obj as usize + ELEMS_OFF) as *mut u64) = buf;
        *((obj as usize + CAP_OFF) as *mut u64) = INITIAL_CAP;
    }
    obj
}

/// `+[NSMutableOrderedSet orderedSetWithCapacity:]` — ignores capacity, returns empty set.
pub extern "C" fn shim_nsmutableorderedset_ordered_set_with_capacity(
    cls: u64,
    cmd: u64,
    _capacity: u64,
) -> u64 {
    shim_nsmutableorderedset_ordered_set(cls, cmd)
}

/// `+[NSMutableOrderedSet orderedSetWithArray:]` — creates a mutable ordered set from array.
pub extern "C" fn shim_nsmutableorderedset_ordered_set_with_array(
    cls: u64,
    cmd: u64,
    array: u64,
) -> u64 {
    let obj = shim_nsmutableorderedset_ordered_set(cls, cmd);
    if obj == 0 || array == 0 {
        return obj;
    }
    let count = crate::nsarray::shim_nsarray_count(array, 0);
    for i in 0..count {
        let element = crate::nsarray::shim_nsarray_object_at_index(array, 0, i);
        shim_nsmutableorderedset_add_object(obj, 0, element);
    }
    obj
}

/// `-[NSMutableOrderedSet addObject:]`
pub extern "C" fn shim_nsmutableorderedset_add_object(obj: u64, _cmd: u64, element: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if contains_element(elems, count, element) {
        return 0;
    } // already present
    unsafe {
        grow_ordered_set(obj);
    }
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    unsafe {
        *((elems as usize + count as usize * 8) as *mut u64) = element;
        *((obj as usize + COUNT_OFF) as *mut u64) = count + 1;
    }
    0
}

/// `-[NSMutableOrderedSet removeObject:]`
pub extern "C" fn shim_nsmutableorderedset_remove_object(obj: u64, _cmd: u64, target: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) } as usize;
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if elems == 0 || count == 0 {
        return 0;
    }
    let mut write = 0usize;
    for read in 0..count {
        let elem = unsafe { *((elems as usize + read * 8) as *const u64) };
        if !elements_equal(elem, target) {
            if write != read {
                unsafe {
                    *((elems as usize + write * 8) as *mut u64) = elem;
                }
            }
            write += 1;
        }
    }
    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = write as u64;
    }
    0
}

/// `-[NSMutableOrderedSet removeAllObjects]`
pub extern "C" fn shim_nsmutableorderedset_remove_all(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = 0;
    }
    0
}

/// `-[NSMutableOrderedSet insertObject:atIndex:]`
pub extern "C" fn shim_nsmutableorderedset_insert_object(
    obj: u64,
    _cmd: u64,
    element: u64,
    idx: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if contains_element(elems, count, element) {
        return 0;
    }
    if idx > count {
        return 0;
    }
    unsafe {
        grow_ordered_set(obj);
    }
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    // Shift right
    if idx < count {
        unsafe {
            let src = (elems as usize + idx as usize * 8) as *const u8;
            let dst = (elems as usize + (idx as usize + 1) * 8) as *mut u8;
            std::ptr::copy(src, dst, (count - idx) as usize * 8);
        }
    }
    unsafe {
        *((elems as usize + idx as usize * 8) as *mut u64) = element;
        *((obj as usize + COUNT_OFF) as *mut u64) = count + 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── helpers ────────────────────────────────────────────────────

    fn make_ordered_set_from_strings(strs: &[&str]) -> (u64, Vec<u64>) {
        let strings: Vec<u64> = strs.iter().map(|s| make_string(s)).collect();
        let arr = crate::nsarray::make_nsarray(&strings);
        let oset = shim_nsorderedset_ordered_set_with_array(0, 0, arr);
        free_obj(arr);
        (oset, strings)
    }

    // ── orderedSetWithArray ────────────────────────────────────────

    #[test]
    fn ordered_set_with_array_basic() {
        let (oset, _strs) = make_ordered_set_from_strings(&["a", "b", "c"]);
        assert_eq!(shim_nsorderedset_count(oset, 0), 3);
        free_obj(oset);
    }

    #[test]
    fn ordered_set_with_array_deduplicates() {
        let (oset, _strs) = make_ordered_set_from_strings(&["x", "y", "x", "z", "y"]);
        assert_eq!(shim_nsorderedset_count(oset, 0), 3);
        free_obj(oset);
    }

    #[test]
    fn ordered_set_with_array_nil() {
        let oset = shim_nsorderedset_ordered_set_with_array(0, 0, 0);
        assert_eq!(shim_nsorderedset_count(oset, 0), 0);
        free_obj(oset);
    }

    #[test]
    fn ordered_set_with_array_empty() {
        let arr = crate::nsarray::make_nsarray(&[]);
        let oset = shim_nsorderedset_ordered_set_with_array(0, 0, arr);
        assert_eq!(shim_nsorderedset_count(oset, 0), 0);
        free_obj(arr);
        free_obj(oset);
    }

    // ── orderedSet (empty factory) ─────────────────────────────────

    #[test]
    fn ordered_set_empty_factory() {
        let oset = shim_nsorderedset_ordered_set(0, 0);
        assert_ne!(oset, 0);
        assert_eq!(shim_nsorderedset_count(oset, 0), 0);
        free_obj(oset);
    }

    // ── count ──────────────────────────────────────────────────────

    #[test]
    fn count_nil() {
        assert_eq!(shim_nsorderedset_count(0, 0), 0);
    }

    // ── objectAtIndex ──────────────────────────────────────────────

    #[test]
    fn object_at_index_basic() {
        let s_a = make_string("a");
        let s_b = make_string("b");
        let arr = crate::nsarray::make_nsarray(&[s_a, s_b]);
        let oset = shim_nsorderedset_ordered_set_with_array(0, 0, arr);

        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 0), "a");
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 1), "b");

        free_obj(arr);
        free_obj(oset);
    }

    #[test]
    fn object_at_index_out_of_bounds() {
        let (oset, _strs) = make_ordered_set_from_strings(&["x"]);
        assert_eq!(shim_nsorderedset_object_at_index(oset, 0, 1), 0);
        assert_eq!(shim_nsorderedset_object_at_index(oset, 0, 100), 0);
        free_obj(oset);
    }

    #[test]
    fn object_at_index_nil() {
        assert_eq!(shim_nsorderedset_object_at_index(0, 0, 0), 0);
    }

    // ── firstObject / lastObject ───────────────────────────────────

    #[test]
    fn first_last_basic() {
        let (oset, _strs) = make_ordered_set_from_strings(&["alpha", "beta", "gamma"]);
        assert_string_eq(shim_nsorderedset_first_object(oset, 0), "alpha");
        assert_string_eq(shim_nsorderedset_last_object(oset, 0), "gamma");
        free_obj(oset);
    }

    #[test]
    fn first_last_single_element() {
        let (oset, _strs) = make_ordered_set_from_strings(&["only"]);
        assert_string_eq(shim_nsorderedset_first_object(oset, 0), "only");
        assert_string_eq(shim_nsorderedset_last_object(oset, 0), "only");
        free_obj(oset);
    }

    #[test]
    fn first_last_empty() {
        let oset = shim_nsorderedset_ordered_set(0, 0);
        assert_eq!(shim_nsorderedset_first_object(oset, 0), 0);
        assert_eq!(shim_nsorderedset_last_object(oset, 0), 0);
        free_obj(oset);
    }

    #[test]
    fn first_last_nil() {
        assert_eq!(shim_nsorderedset_first_object(0, 0), 0);
        assert_eq!(shim_nsorderedset_last_object(0, 0), 0);
    }

    // ── containsObject ─────────────────────────────────────────────

    #[test]
    fn contains_object_present() {
        let s_hello = make_string("hello");
        let arr = crate::nsarray::make_nsarray(&[s_hello]);
        let oset = shim_nsorderedset_ordered_set_with_array(0, 0, arr);

        // Use a second string with the same content (different pointer)
        let s_hello2 = make_string("hello");
        assert_eq!(shim_nsorderedset_contains_object(oset, 0, s_hello2), 1);

        free_obj(arr);
        free_obj(oset);
    }

    #[test]
    fn contains_object_absent() {
        let (oset, _strs) = make_ordered_set_from_strings(&["a", "b"]);
        let s_c = make_string("c");
        assert_eq!(shim_nsorderedset_contains_object(oset, 0, s_c), 0);
        free_obj(oset);
    }

    #[test]
    fn contains_object_nil_receiver() {
        assert_eq!(shim_nsorderedset_contains_object(0, 0, 42), 0);
    }

    #[test]
    fn contains_object_empty_set() {
        let oset = shim_nsorderedset_ordered_set(0, 0);
        let s = make_string("x");
        assert_eq!(shim_nsorderedset_contains_object(oset, 0, s), 0);
        free_obj(oset);
    }

    // ── indexOfObject ──────────────────────────────────────────────

    #[test]
    fn index_of_object_found() {
        let (oset, _strs) = make_ordered_set_from_strings(&["x", "y", "z"]);
        let s_y = make_string("y");
        assert_eq!(shim_nsorderedset_index_of_object(oset, 0, s_y), 1);
        free_obj(oset);
    }

    #[test]
    fn index_of_object_not_found() {
        let (oset, _strs) = make_ordered_set_from_strings(&["a", "b"]);
        let s_c = make_string("c");
        assert_eq!(shim_nsorderedset_index_of_object(oset, 0, s_c), u64::MAX);
        free_obj(oset);
    }

    #[test]
    fn index_of_object_nil_receiver() {
        assert_eq!(shim_nsorderedset_index_of_object(0, 0, 1), u64::MAX);
    }

    // ── array (conversion) ─────────────────────────────────────────

    #[test]
    fn array_conversion_basic() {
        let (oset, _strs) = make_ordered_set_from_strings(&["a", "b", "c"]);
        let arr = shim_nsorderedset_array(oset, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 3);
        assert_string_eq(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0), "a");
        assert_string_eq(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 2), "c");
        free_obj(arr);
        free_obj(oset);
    }

    #[test]
    fn array_conversion_empty() {
        let oset = shim_nsorderedset_ordered_set(0, 0);
        let arr = shim_nsorderedset_array(oset, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
        free_obj(arr);
        free_obj(oset);
    }

    #[test]
    fn array_conversion_nil() {
        assert_eq!(shim_nsorderedset_array(0, 0), 0);
    }

    // ── set (conversion to NSSet) ──────────────────────────────────

    #[test]
    fn set_conversion_basic() {
        let (oset, _strs) = make_ordered_set_from_strings(&["a", "b", "c"]);
        let s = shim_nsorderedset_set(oset, 0);
        assert_ne!(s, 0);
        assert_eq!(crate::nsset::shim_nsset_count(s, 0), 3);
        free_obj(s);
        free_obj(oset);
    }

    #[test]
    fn set_conversion_empty() {
        let oset = shim_nsorderedset_ordered_set(0, 0);
        let s = shim_nsorderedset_set(oset, 0);
        assert_ne!(s, 0);
        assert_eq!(crate::nsset::shim_nsset_count(s, 0), 0);
        free_obj(s);
        free_obj(oset);
    }

    #[test]
    fn set_conversion_nil() {
        assert_eq!(shim_nsorderedset_set(0, 0), 0);
    }

    // ── reverseObjectEnumerator ────────────────────────────────────

    #[test]
    fn reverse_enumerator_basic() {
        let (oset, _strs) = make_ordered_set_from_strings(&["a", "b", "c"]);
        let e = shim_nsorderedset_reverse_object_enumerator(oset, 0);
        assert_ne!(e, 0);
        assert_string_eq(
            crate::nsenumerator::shim_nsenumerator_next_object(e, 0),
            "c",
        );
        assert_string_eq(
            crate::nsenumerator::shim_nsenumerator_next_object(e, 0),
            "b",
        );
        assert_string_eq(
            crate::nsenumerator::shim_nsenumerator_next_object(e, 0),
            "a",
        );
        assert_eq!(crate::nsenumerator::shim_nsenumerator_next_object(e, 0), 0);
        free_obj(e);
        free_obj(oset);
    }

    #[test]
    fn reverse_enumerator_empty() {
        let oset = shim_nsorderedset_ordered_set(0, 0);
        let e = shim_nsorderedset_reverse_object_enumerator(oset, 0);
        assert_ne!(e, 0);
        assert_eq!(crate::nsenumerator::shim_nsenumerator_next_object(e, 0), 0);
        free_obj(e);
        free_obj(oset);
    }

    #[test]
    fn reverse_enumerator_nil() {
        assert_eq!(shim_nsorderedset_reverse_object_enumerator(0, 0), 0);
    }

    // ── description ────────────────────────────────────────────────

    #[test]
    fn description_basic() {
        let (oset, _strs) = make_ordered_set_from_strings(&["a", "b"]);
        let desc = shim_nsorderedset_description(oset, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSOrderedSet: 2 objects>");
        free_obj(oset);
    }

    #[test]
    fn description_empty() {
        let oset = shim_nsorderedset_ordered_set(0, 0);
        let desc = shim_nsorderedset_description(oset, 0);
        assert_string_eq(desc, "<NSOrderedSet: 0 objects>");
        free_obj(oset);
    }

    #[test]
    fn description_nil() {
        assert_eq!(shim_nsorderedset_description(0, 0), 0);
    }

    // ── NSMutableOrderedSet: orderedSet ────────────────────────────

    #[test]
    fn mutable_ordered_set_factory() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        assert_ne!(oset, 0);
        assert_eq!(shim_nsorderedset_count(oset, 0), 0);
        free_obj(oset);
    }

    // ── NSMutableOrderedSet: addObject ─────────────────────────────

    #[test]
    fn mutable_add_object_basic() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s_a = make_string("a");
        let s_b = make_string("b");
        shim_nsmutableorderedset_add_object(oset, 0, s_a);
        shim_nsmutableorderedset_add_object(oset, 0, s_b);
        assert_eq!(shim_nsorderedset_count(oset, 0), 2);
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 0), "a");
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 1), "b");
        free_obj(oset);
    }

    #[test]
    fn mutable_add_object_dedup() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s1 = make_string("dup");
        let s2 = make_string("dup");
        shim_nsmutableorderedset_add_object(oset, 0, s1);
        shim_nsmutableorderedset_add_object(oset, 0, s2);
        assert_eq!(shim_nsorderedset_count(oset, 0), 1);
        free_obj(oset);
    }

    #[test]
    fn mutable_add_object_nil_receiver() {
        assert_eq!(shim_nsmutableorderedset_add_object(0, 0, 42), 0);
    }

    #[test]
    fn mutable_add_object_grows_beyond_initial_cap() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        // INITIAL_CAP is 4; add 6 unique elements to trigger growth
        let mut strings = Vec::new();
        for i in 0..6 {
            let s = make_string(&format!("item{}", i));
            strings.push(s);
            shim_nsmutableorderedset_add_object(oset, 0, s);
        }
        assert_eq!(shim_nsorderedset_count(oset, 0), 6);
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 5), "item5");
        free_obj(oset);
    }

    // ── NSMutableOrderedSet: removeObject ──────────────────────────

    #[test]
    fn mutable_remove_object_basic() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s_a = make_string("a");
        let s_b = make_string("b");
        let s_c = make_string("c");
        shim_nsmutableorderedset_add_object(oset, 0, s_a);
        shim_nsmutableorderedset_add_object(oset, 0, s_b);
        shim_nsmutableorderedset_add_object(oset, 0, s_c);

        let s_b2 = make_string("b");
        shim_nsmutableorderedset_remove_object(oset, 0, s_b2);
        assert_eq!(shim_nsorderedset_count(oset, 0), 2);
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 0), "a");
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 1), "c");
        free_obj(oset);
    }

    #[test]
    fn mutable_remove_object_not_present() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s = make_string("a");
        shim_nsmutableorderedset_add_object(oset, 0, s);

        let s_z = make_string("z");
        shim_nsmutableorderedset_remove_object(oset, 0, s_z);
        assert_eq!(shim_nsorderedset_count(oset, 0), 1);
        free_obj(oset);
    }

    #[test]
    fn mutable_remove_object_nil_receiver() {
        assert_eq!(shim_nsmutableorderedset_remove_object(0, 0, 1), 0);
    }

    // ── NSMutableOrderedSet: removeAllObjects ──────────────────────

    #[test]
    fn mutable_remove_all() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s_a = make_string("a");
        let s_b = make_string("b");
        shim_nsmutableorderedset_add_object(oset, 0, s_a);
        shim_nsmutableorderedset_add_object(oset, 0, s_b);
        assert_eq!(shim_nsorderedset_count(oset, 0), 2);

        shim_nsmutableorderedset_remove_all(oset, 0);
        assert_eq!(shim_nsorderedset_count(oset, 0), 0);
        free_obj(oset);
    }

    #[test]
    fn mutable_remove_all_nil() {
        assert_eq!(shim_nsmutableorderedset_remove_all(0, 0), 0);
    }

    // ── NSMutableOrderedSet: insertObject:atIndex: ─────────────────

    #[test]
    fn mutable_insert_at_beginning() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s_b = make_string("b");
        let s_a = make_string("a");
        shim_nsmutableorderedset_add_object(oset, 0, s_b);
        shim_nsmutableorderedset_insert_object(oset, 0, s_a, 0);

        assert_eq!(shim_nsorderedset_count(oset, 0), 2);
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 0), "a");
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 1), "b");
        free_obj(oset);
    }

    #[test]
    fn mutable_insert_at_end() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s_a = make_string("a");
        let s_b = make_string("b");
        shim_nsmutableorderedset_add_object(oset, 0, s_a);
        shim_nsmutableorderedset_insert_object(oset, 0, s_b, 1);

        assert_eq!(shim_nsorderedset_count(oset, 0), 2);
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 1), "b");
        free_obj(oset);
    }

    #[test]
    fn mutable_insert_in_middle() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s_a = make_string("a");
        let s_c = make_string("c");
        let s_b = make_string("b");
        shim_nsmutableorderedset_add_object(oset, 0, s_a);
        shim_nsmutableorderedset_add_object(oset, 0, s_c);
        shim_nsmutableorderedset_insert_object(oset, 0, s_b, 1);

        assert_eq!(shim_nsorderedset_count(oset, 0), 3);
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 0), "a");
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 1), "b");
        assert_string_eq(shim_nsorderedset_object_at_index(oset, 0, 2), "c");
        free_obj(oset);
    }

    #[test]
    fn mutable_insert_duplicate_is_noop() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s_a = make_string("a");
        shim_nsmutableorderedset_add_object(oset, 0, s_a);

        let s_a2 = make_string("a");
        shim_nsmutableorderedset_insert_object(oset, 0, s_a2, 0);
        assert_eq!(shim_nsorderedset_count(oset, 0), 1);
        free_obj(oset);
    }

    #[test]
    fn mutable_insert_out_of_bounds_is_noop() {
        let oset = shim_nsmutableorderedset_ordered_set(0, 0);
        let s = make_string("a");
        // count is 0, idx 5 > count → should be a no-op
        shim_nsmutableorderedset_insert_object(oset, 0, s, 5);
        assert_eq!(shim_nsorderedset_count(oset, 0), 0);
        free_obj(oset);
    }

    #[test]
    fn mutable_insert_nil_receiver() {
        assert_eq!(shim_nsmutableorderedset_insert_object(0, 0, 42, 0), 0);
    }
}
