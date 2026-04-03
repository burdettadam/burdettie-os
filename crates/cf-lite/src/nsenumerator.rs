//! NSEnumerator — lightweight forward iterator for Foundation collections.
//!
//! Layout (24 bytes):
//!   offset  0: isa          (u64)  → NSEnumerator class addr
//!   offset  8: source       (u64)  → the collection being enumerated (NSArray or key-array)
//!   offset 16: index        (u64)  → current position

const NSENUM_SOURCE_OFF: usize = 8;
const NSENUM_INDEX_OFF: usize = 16;

/// `+[NSEnumerator alloc]` — internal; shouldn't be called directly,
/// but `objectEnumerator` / `keyEnumerator` create instances.
pub fn alloc_enumerator(source: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsenumerator_class_addr();
        *((obj as usize + NSENUM_SOURCE_OFF) as *mut u64) = source;
        // index starts at 0 (calloc zeroes it)
    }
    obj
}

/// `-[NSEnumerator nextObject]` — returns the next element or nil (0).
pub extern "C" fn shim_nsenumerator_next_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let source = unsafe { *((obj as usize + NSENUM_SOURCE_OFF) as *const u64) };
    let index = unsafe { *((obj as usize + NSENUM_INDEX_OFF) as *const u64) };
    if source == 0 {
        return 0;
    }

    // Read source count (offset 8 in NSArray layout)
    let count = unsafe { *((source as usize + 8) as *const u64) };
    if index >= count {
        return 0;
    }

    // Read element at index (NSArray: elements_ptr at offset 16)
    let elems = unsafe { *((source as usize + 16) as *const u64) };
    let element = unsafe { *((elems as usize + index as usize * 8) as *const u64) };

    // Advance index
    unsafe {
        *((obj as usize + NSENUM_INDEX_OFF) as *mut u64) = index + 1;
    }

    element
}

/// `-[NSEnumerator allObjects]` — returns an NSArray of remaining objects.
pub extern "C" fn shim_nsenumerator_all_objects(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let source = unsafe { *((obj as usize + NSENUM_SOURCE_OFF) as *const u64) };
    let index = unsafe { *((obj as usize + NSENUM_INDEX_OFF) as *const u64) };
    if source == 0 {
        // Return empty array
        return crate::nsarray::shim_nsarray_array_with_objects_count(
            crate::nsobj::nsarray_class_addr(),
            0,
            0,
            0,
        );
    }

    let count = unsafe { *((source as usize + 8) as *const u64) };
    if index >= count {
        return crate::nsarray::shim_nsarray_array_with_objects_count(
            crate::nsobj::nsarray_class_addr(),
            0,
            0,
            0,
        );
    }

    let elems = unsafe { *((source as usize + 16) as *const u64) };
    let remaining = (count - index) as usize;
    let mut ptrs: Vec<u64> = Vec::with_capacity(remaining);
    for i in 0..remaining {
        let elem = unsafe { *((elems as usize + (index as usize + i) * 8) as *const u64) };
        ptrs.push(elem);
    }

    // Advance index to end
    unsafe {
        *((obj as usize + NSENUM_INDEX_OFF) as *mut u64) = count;
    }

    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        ptrs.as_ptr() as u64,
        ptrs.len() as u64,
    )
}

// ── NSArray objectEnumerator / NSDictionary keyEnumerator ────────────

/// `-[NSArray objectEnumerator]` — returns an NSEnumerator over the array's elements.
pub extern "C" fn shim_nsarray_object_enumerator(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    alloc_enumerator(obj)
}

/// `-[NSDictionary keyEnumerator]` — returns an NSEnumerator over the dictionary's keys.
/// The NSDictionary layout stores keys at offset 16 as an internal array-like buffer.
/// We build a temporary NSArray from the keys and enumerate that.
pub extern "C" fn shim_nsdictionary_key_enumerator(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Get the allKeys NSArray and enumerate it
    let all_keys = crate::nsdictionary::shim_nsdictionary_all_keys(obj, 0);
    alloc_enumerator(all_keys)
}

/// `-[NSArray reverseObjectEnumerator]` — returns an NSEnumerator that iterates
/// elements in reverse order. We build a reversed-copy NSArray and enumerate it.
pub extern "C" fn shim_nsarray_reverse_object_enumerator(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + 8) as *const u64) } as usize;
    let elems = unsafe { *((obj as usize + 16) as *const u64) };
    if count == 0 || elems == 0 {
        // Return enumerator over empty array
        let empty = crate::nsarray::make_nsarray(&[]);
        return alloc_enumerator(empty);
    }
    let mut items: Vec<u64> = Vec::with_capacity(count);
    for i in (0..count).rev() {
        items.push(unsafe { *((elems as usize + i * 8) as *const u64) });
    }
    let reversed = crate::nsarray::make_nsarray(&items);
    alloc_enumerator(reversed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    /// Build a source NSArray with the given u64 elements, suitable for enumerator tests.
    fn make_source_array(items: &[u64]) -> u64 {
        crate::nsarray::make_nsarray(items)
    }

    // ── alloc_enumerator ─────────────────────────────────────────────

    #[test]
    fn alloc_enumerator_sets_fields() {
        let src = make_source_array(&[10, 20, 30]);
        let e = alloc_enumerator(src);
        assert_ne!(e, 0);
        assert_eq!(read_field(e, 0), crate::nsobj::nsenumerator_class_addr());
        assert_eq!(read_field(e, NSENUM_SOURCE_OFF), src);
        assert_eq!(read_field(e, NSENUM_INDEX_OFF), 0);
        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn alloc_enumerator_with_zero_source() {
        let e = alloc_enumerator(0);
        assert_ne!(e, 0);
        assert_eq!(read_field(e, NSENUM_SOURCE_OFF), 0);
        free_obj(e);
    }

    // ── shim_nsenumerator_next_object ────────────────────────────────

    #[test]
    fn next_object_iterates_all_elements() {
        let src = make_source_array(&[100, 200, 300]);
        let e = alloc_enumerator(src);

        assert_eq!(shim_nsenumerator_next_object(e, 0), 100);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 200);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 300);
        // Exhausted — returns 0
        assert_eq!(shim_nsenumerator_next_object(e, 0), 0);
        // Stays at 0 after exhaustion
        assert_eq!(shim_nsenumerator_next_object(e, 0), 0);

        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn next_object_empty_array() {
        let src = make_source_array(&[]);
        let e = alloc_enumerator(src);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 0);
        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn next_object_nil_obj() {
        assert_eq!(shim_nsenumerator_next_object(0, 0), 0);
    }

    #[test]
    fn next_object_nil_source() {
        let e = alloc_enumerator(0);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 0);
        free_obj(e);
    }

    #[test]
    fn next_object_advances_index() {
        let src = make_source_array(&[1, 2]);
        let e = alloc_enumerator(src);
        shim_nsenumerator_next_object(e, 0);
        assert_eq!(read_field(e, NSENUM_INDEX_OFF), 1);
        shim_nsenumerator_next_object(e, 0);
        assert_eq!(read_field(e, NSENUM_INDEX_OFF), 2);
        // After exhaustion, index stays at 2
        shim_nsenumerator_next_object(e, 0);
        assert_eq!(read_field(e, NSENUM_INDEX_OFF), 2);
        free_obj(e);
        free_obj(src);
    }

    // ── shim_nsenumerator_all_objects ────────────────────────────────

    #[test]
    fn all_objects_returns_remaining() {
        let src = make_source_array(&[10, 20, 30, 40]);
        let e = alloc_enumerator(src);

        // Advance past first two
        shim_nsenumerator_next_object(e, 0);
        shim_nsenumerator_next_object(e, 0);

        let rest = shim_nsenumerator_all_objects(e, 0);
        assert_ne!(rest, 0);
        // rest should be an NSArray of [30, 40]
        let count = read_field(rest, 8);
        assert_eq!(count, 2);
        let elems_ptr = read_field(rest, 16);
        let elem0 = unsafe { *((elems_ptr as usize) as *const u64) };
        let elem1 = unsafe { *((elems_ptr as usize + 8) as *const u64) };
        assert_eq!(elem0, 30);
        assert_eq!(elem1, 40);

        // Index should now be at end
        assert_eq!(read_field(e, NSENUM_INDEX_OFF), 4);

        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn all_objects_from_start() {
        let src = make_source_array(&[5, 6]);
        let e = alloc_enumerator(src);

        let all = shim_nsenumerator_all_objects(e, 0);
        let count = read_field(all, 8);
        assert_eq!(count, 2);
        let elems_ptr = read_field(all, 16);
        let elem0 = unsafe { *((elems_ptr as usize) as *const u64) };
        let elem1 = unsafe { *((elems_ptr as usize + 8) as *const u64) };
        assert_eq!(elem0, 5);
        assert_eq!(elem1, 6);

        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn all_objects_when_exhausted() {
        let src = make_source_array(&[1]);
        let e = alloc_enumerator(src);
        shim_nsenumerator_next_object(e, 0); // exhaust

        let rest = shim_nsenumerator_all_objects(e, 0);
        assert_ne!(rest, 0);
        assert_eq!(read_field(rest, 8), 0); // empty array

        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn all_objects_nil_obj() {
        assert_eq!(shim_nsenumerator_all_objects(0, 0), 0);
    }

    #[test]
    fn all_objects_nil_source() {
        let e = alloc_enumerator(0);
        let result = shim_nsenumerator_all_objects(e, 0);
        // Should return an empty array (not nil)
        assert_ne!(result, 0);
        assert_eq!(read_field(result, 8), 0);
        free_obj(e);
    }

    #[test]
    fn all_objects_empty_array() {
        let src = make_source_array(&[]);
        let e = alloc_enumerator(src);
        let result = shim_nsenumerator_all_objects(e, 0);
        assert_ne!(result, 0);
        assert_eq!(read_field(result, 8), 0);
        free_obj(e);
        free_obj(src);
    }

    // ── shim_nsarray_object_enumerator ───────────────────────────────

    #[test]
    fn object_enumerator_creates_enumerator() {
        let src = make_source_array(&[7, 8, 9]);
        let e = shim_nsarray_object_enumerator(src, 0);
        assert_ne!(e, 0);
        assert_eq!(read_field(e, NSENUM_SOURCE_OFF), src);
        assert_eq!(read_field(e, NSENUM_INDEX_OFF), 0);

        // Can iterate through it
        assert_eq!(shim_nsenumerator_next_object(e, 0), 7);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 8);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 9);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 0);

        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn object_enumerator_nil_obj() {
        assert_eq!(shim_nsarray_object_enumerator(0, 0), 0);
    }

    // ── shim_nsdictionary_key_enumerator ─────────────────────────────

    #[test]
    fn key_enumerator_nil_obj() {
        assert_eq!(shim_nsdictionary_key_enumerator(0, 0), 0);
    }

    // ── shim_nsarray_reverse_object_enumerator ───────────────────────

    #[test]
    fn reverse_object_enumerator_nil_obj() {
        assert_eq!(shim_nsarray_reverse_object_enumerator(0, 0), 0);
    }

    #[test]
    fn reverse_object_enumerator_iterates_backwards() {
        let src = make_source_array(&[10, 20, 30]);
        let e = shim_nsarray_reverse_object_enumerator(src, 0);
        assert_ne!(e, 0);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 30);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 20);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 10);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 0);
        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn reverse_object_enumerator_empty_array() {
        let src = make_source_array(&[]);
        let e = shim_nsarray_reverse_object_enumerator(src, 0);
        assert_ne!(e, 0);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 0);
        free_obj(e);
        free_obj(src);
    }

    #[test]
    fn reverse_object_enumerator_single_element() {
        let src = make_source_array(&[42]);
        let e = shim_nsarray_reverse_object_enumerator(src, 0);
        assert_ne!(e, 0);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 42);
        assert_eq!(shim_nsenumerator_next_object(e, 0), 0);
        free_obj(e);
        free_obj(src);
    }
}
