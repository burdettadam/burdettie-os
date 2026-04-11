//! NSMutableArray extended operations: sort, remove by value, bulk ops.

use super::{NSMUTABLEARRAY_COUNT_OFF, NSMUTABLEARRAY_ELEMS_OFF};

/// `-[NSMutableArray sortUsingComparator:]`
///
/// Sorts the array in place using a comparator block.
/// Block signature: `NSComparisonResult (^)(id obj1, id obj2)`
pub extern "C" fn shim_nsmutablearray_sort_using_comparator(obj: u64, _cmd: u64, comparator: u64) {
    if obj == 0 || comparator == 0 {
        return;
    }
    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) } as usize;
    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if count <= 1 || elems == 0 {
        return;
    }

    // Read elements into a Vec
    let mut items: Vec<u64> = Vec::with_capacity(count);
    for i in 0..count {
        let elem = unsafe { *((elems as usize + i * 8) as *const u64) };
        items.push(elem);
    }

    // Block invoke: NSComparisonResult (^)(void *block, id obj1, id obj2)
    let invoke: extern "C" fn(u64, u64, u64) -> i64 =
        unsafe { std::mem::transmute(*((comparator as usize + 16) as *const u64)) };

    items.sort_by(|a, b| {
        let result = invoke(comparator, *a, *b);
        match result {
            r if r < 0 => std::cmp::Ordering::Less,
            0 => std::cmp::Ordering::Equal,
            _ => std::cmp::Ordering::Greater,
        }
    });

    // Write sorted elements back
    for (i, item) in items.iter().enumerate() {
        unsafe {
            *((elems as usize + i * 8) as *mut u64) = *item;
        }
    }
}

/// `-[NSMutableArray removeObject:]` — removes all occurrences by value.
pub extern "C" fn shim_nsmutablearray_remove_object(obj: u64, _cmd: u64, target: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) } as usize;
    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 || count == 0 {
        return 0;
    }

    let target_cstr = crate::cfstring::extract_cfstring_cstr(target);
    let mut write_idx = 0usize;
    for read_idx in 0..count {
        let elem = unsafe { *((elems as usize + read_idx * 8) as *const u64) };
        let mut is_match = elem == target;
        if !is_match {
            if let Some(t) = &target_cstr {
                if let Some(e) = crate::cfstring::extract_cfstring_cstr(elem) {
                    is_match = e == *t;
                }
            }
        }
        if !is_match {
            if write_idx != read_idx {
                unsafe {
                    *((elems as usize + write_idx * 8) as *mut u64) = elem;
                }
            }
            write_idx += 1;
        } else {
            crate::nsarray::release_collection_object(elem);
        }
    }
    for idx in write_idx..count {
        unsafe {
            *((elems as usize + idx * 8) as *mut u64) = 0;
        }
    }
    unsafe {
        *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *mut u64) = write_idx as u64;
    }
    0
}

/// `-[NSMutableArray addObjectsFromArray:]` — appends all elements from another array.
pub extern "C" fn shim_nsmutablearray_add_objects_from_array(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 || other == 0 {
        return 0;
    }
    let other_count = crate::nsarray::shim_nsarray_count(other, 0);
    for i in 0..other_count {
        let elem = crate::nsarray::shim_nsarray_object_at_index(other, 0, i);
        super::shim_nsmutablearray_add_object(obj, 0, elem);
    }
    0
}

/// `-[NSMutableArray exchangeObjectAtIndex:withObjectAtIndex:]`
pub extern "C" fn shim_nsmutablearray_exchange_object_at_index(
    obj: u64,
    _cmd: u64,
    idx1: u64,
    idx2: u64,
) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if idx1 >= count || idx2 >= count || elems == 0 { return 0; }
    unsafe {
        let p1 = (elems as usize + idx1 as usize * 8) as *mut u64;
        let p2 = (elems as usize + idx2 as usize * 8) as *mut u64;
        std::ptr::swap(p1, p2);
    }
    0
}

/// `-[NSMutableArray removeObjectsAtIndexes:]`
pub extern "C" fn shim_nsmutablearray_remove_objects_at_indexes(
    obj: u64,
    _cmd: u64,
    indexes: u64,
) -> u64 {
    if obj == 0 || indexes == 0 {
        return 0;
    }

    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    for idx in (0..count).rev() {
        if crate::nsindexset::shim_nsindexset_contains_index(indexes, 0, idx) != 0 {
            super::shim_nsmutablearray_remove_object_at_index(obj, 0, idx);
        }
    }
    0
}

/// `-[NSMutableArray sortUsingDescriptors:]` — sort using NSSortDescriptor array.
/// No-op stub: descriptor-based sorting requires runtime introspection of key paths
/// and comparator selectors that the shim environment doesn't fully support yet.
/// `-[NSMutableArray sortUsingDescriptors:]` — sort using NSSortDescriptor array.
/// Implements real sorting using the provided descriptors.
pub extern "C" fn shim_nsmutablearray_sort_using_descriptors(obj: u64, _cmd: u64, descriptors: u64) {
    if obj == 0 || descriptors == 0 {
        return;
    }
    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) } as usize;
    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if count <= 1 || elems == 0 {
        return;
    }

    // Read elements into a Vec
    let mut items: Vec<u64> = Vec::with_capacity(count);
    for i in 0..count {
        let elem = unsafe { *((elems as usize + i * 8) as *const u64) };
        items.push(elem);
    }

    // Read descriptors into a Vec
    let desc_count = crate::nsarray::shim_nsarray_count(descriptors, 0);
    let mut descs = Vec::with_capacity(desc_count as usize);
    for i in 0..desc_count {
        let desc = crate::nsarray::shim_nsarray_object_at_index(descriptors, 0, i);
        if desc != 0 {
            descs.push(desc);
        }
    }

    // Sort using the descriptors
    items.sort_by(|&a, &b| {
        for &desc in &descs {
            // NSSortDescriptor: get key and ascending
            let key_ptr = crate::nssortdescriptor::shim_nssortdescriptor_key(desc, 0);
            let ascending = crate::nssortdescriptor::shim_nssortdescriptor_ascending(desc, 0) != 0;
            let key = if key_ptr == 0 {
                "self"
            } else {
                crate::cfstring::extract_cfstring_cstr(key_ptr).unwrap_or("self")
            };

            // Get values for key
            let va = if key == "self" {
                a
            } else {
                crate::nskeyvaluecoding::shim_value_for_key(a, 0, key_ptr)
            };
            let vb = if key == "self" {
                b
            } else {
                crate::nskeyvaluecoding::shim_value_for_key(b, 0, key_ptr)
            };

            // Compare values (NSString only for now)
            let cmp = crate::cfstring::shim_nsstring_compare(va, 0, vb);
            let ord = match cmp {
                x if x < 0 => std::cmp::Ordering::Less,
                0 => std::cmp::Ordering::Equal,
                _ => std::cmp::Ordering::Greater,
            };
            let ord = if ascending { ord } else { ord.reverse() };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });

    // Write sorted elements back
    for (i, item) in items.iter().enumerate() {
        unsafe {
            *((elems as usize + i * 8) as *mut u64) = *item;
        }
    }
}

/// `-[NSMutableArray removeObjectsInRange:]` — removes elements in the given range
/// by shifting remaining elements down. Takes location (u64) and length (u64).
pub extern "C" fn shim_nsmutablearray_remove_objects_in_range(
    obj: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    let loc = location as usize;
    let len = length as usize;
    if len == 0 {
        return 0;
    }
    if loc + len > count as usize {
        eprintln!(
            "NSMutableArray removeObjectsInRange: range {},{} out of bounds for count={}",
            loc, len, count
        );
        return 0;
    }
    // Remove from back to front so indices remain valid
    for i in (loc..loc + len).rev() {
        super::shim_nsmutablearray_remove_object_at_index(obj, 0, i as u64);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsindexset;
    use crate::shim_test_helpers::*;
    use crate::nsmutablearray::{NSMUTABLEARRAY_COUNT_OFF, NSMUTABLEARRAY_ELEMS_OFF};

    const CAP_OFF: usize = 24;

    fn make_array_with(items: &[u64]) -> u64 {
        let cap = if items.is_empty() { 4 } else { items.len() * 2 };
        let obj = crate::pal_alloc().calloc(1, 32) as u64;
        let buf = crate::pal_alloc().calloc(cap, 8) as u64;
        write_field(obj, 0, 0xDEAD);
        write_field(obj, NSMUTABLEARRAY_ELEMS_OFF, buf);
        write_field(obj, CAP_OFF, cap as u64);
        for (i, &item) in items.iter().enumerate() {
            write_field(buf, i * 8, item);
        }
        write_field(obj, NSMUTABLEARRAY_COUNT_OFF, items.len() as u64);
        obj
    }

    fn free_array(obj: u64) {
        free_buf(read_field(obj, NSMUTABLEARRAY_ELEMS_OFF));
        free_obj(obj);
    }

    #[test]
    fn nil_sort_is_noop() {
        shim_nsmutablearray_sort_using_comparator(0, 0, 0);
    }

    #[test]
    fn nil_remove_object_is_noop() {
        assert_eq!(shim_nsmutablearray_remove_object(0, 0, 0xABC), 0);
    }

    #[test]
    fn remove_object_decreases_count() {
        let s1 = make_string("aa");
        let s2 = make_string("bb");
        let s3 = make_string("cc");
        let arr = make_array_with(&[s1, s2, s3]);
        shim_nsmutablearray_remove_object(arr, 0, s2);
        assert_eq!(read_field(arr, NSMUTABLEARRAY_COUNT_OFF), 2);
        let buf = read_field(arr, NSMUTABLEARRAY_ELEMS_OFF);
        assert_eq!(read_field(buf, 0), s1);
        assert_eq!(read_field(buf, 8), s3);
        free_array(arr);
    }

    #[test]
    fn nil_add_objects_from_array_is_noop() {
        assert_eq!(shim_nsmutablearray_add_objects_from_array(0, 0, 0xABC), 0);
        assert_eq!(shim_nsmutablearray_add_objects_from_array(0xABC, 0, 0), 0);
    }

    #[test]
    fn exchange_swaps_elements() {
        let arr = make_array_with(&[0xAA, 0xBB, 0xCC]);
        shim_nsmutablearray_exchange_object_at_index(arr, 0, 0, 2);
        let buf = read_field(arr, NSMUTABLEARRAY_ELEMS_OFF);
        assert_eq!(read_field(buf, 0), 0xCC);
        assert_eq!(read_field(buf, 8), 0xBB);
        assert_eq!(read_field(buf, 16), 0xAA);
        free_array(arr);
    }

    #[test]
    fn exchange_out_of_bounds_is_noop() {
        let arr = make_array_with(&[0xAA, 0xBB]);
        shim_nsmutablearray_exchange_object_at_index(arr, 0, 0, 5);
        let buf = read_field(arr, NSMUTABLEARRAY_ELEMS_OFF);
        assert_eq!(read_field(buf, 0), 0xAA);
        free_array(arr);
    }

    #[test]
    fn remove_objects_at_indexes_removes_in_descending_order() {
        let arr = make_array_with(&[0xAA, 0xBB, 0xCC, 0xDD]);
        let indexes = nsindexset::shim_nsindexset_index_set(0, 0);
        nsindexset::shim_nsindexset_add_index(indexes, 0, 1);
        nsindexset::shim_nsindexset_add_index(indexes, 0, 3);

        shim_nsmutablearray_remove_objects_at_indexes(arr, 0, indexes);

        assert_eq!(read_field(arr, NSMUTABLEARRAY_COUNT_OFF), 2);
        let buf = read_field(arr, NSMUTABLEARRAY_ELEMS_OFF);
        assert_eq!(read_field(buf, 0), 0xAA);
        assert_eq!(read_field(buf, 8), 0xCC);
        free_obj(indexes);
        free_array(arr);
    }

    #[test]
    fn nil_remove_objects_at_indexes_is_noop() {
        assert_eq!(shim_nsmutablearray_remove_objects_at_indexes(0, 0, 0xABC), 0);
        assert_eq!(shim_nsmutablearray_remove_objects_at_indexes(0xABC, 0, 0), 0);
    }

    // ── removeObjectsInRange: tests ──────────────────────────────

    #[test]
    fn remove_objects_in_range_nil_safe() {
        assert_eq!(shim_nsmutablearray_remove_objects_in_range(0, 0, 0, 2), 0);
    }

    #[test]
    fn remove_objects_in_range_zero_length_is_noop() {
        let arr = make_array_with(&[0xAA, 0xBB, 0xCC]);
        shim_nsmutablearray_remove_objects_in_range(arr, 0, 1, 0);
        assert_eq!(read_field(arr, NSMUTABLEARRAY_COUNT_OFF), 3);
        free_array(arr);
    }

    #[test]
    fn remove_objects_in_range_middle() {
        let arr = make_array_with(&[0xAA, 0xBB, 0xCC, 0xDD]);
        shim_nsmutablearray_remove_objects_in_range(arr, 0, 1, 2);
        assert_eq!(read_field(arr, NSMUTABLEARRAY_COUNT_OFF), 2);
        let buf = read_field(arr, NSMUTABLEARRAY_ELEMS_OFF);
        assert_eq!(read_field(buf, 0), 0xAA);
        assert_eq!(read_field(buf, 8), 0xDD);
        free_array(arr);
    }

    #[test]
    fn remove_objects_in_range_all() {
        let arr = make_array_with(&[0xAA, 0xBB, 0xCC]);
        shim_nsmutablearray_remove_objects_in_range(arr, 0, 0, 3);
        assert_eq!(read_field(arr, NSMUTABLEARRAY_COUNT_OFF), 0);
        free_array(arr);
    }

    #[test]
    fn remove_objects_in_range_out_of_bounds_returns_zero() {
        let arr = make_array_with(&[0xAA, 0xBB]);
        let result = shim_nsmutablearray_remove_objects_in_range(arr, 0, 1, 5);
        assert_eq!(result, 0);
        // Count should be unchanged
        assert_eq!(read_field(arr, NSMUTABLEARRAY_COUNT_OFF), 2);
        free_array(arr);
    }

    // ── sortUsingDescriptors: tests ────────────────────────────

    #[test]
    fn sort_using_descriptors_nil_safe() {
        shim_nsmutablearray_sort_using_descriptors(0, 0, 0);
    }

    #[test]
    fn sort_using_descriptors_is_noop_stub() {
        let arr = make_array_with(&[0xCC, 0xAA, 0xBB]);
        shim_nsmutablearray_sort_using_descriptors(arr, 0, 0xDEAD);
        // Order should be unchanged (no-op)
        let buf = read_field(arr, NSMUTABLEARRAY_ELEMS_OFF);
        assert_eq!(read_field(buf, 0), 0xCC);
        assert_eq!(read_field(buf, 8), 0xAA);
        assert_eq!(read_field(buf, 16), 0xBB);
        free_array(arr);
    }

    #[test]
    fn remove_objects_in_range_from_start() {
        let arr = make_array_with(&[0xAA, 0xBB, 0xCC]);
        shim_nsmutablearray_remove_objects_in_range(arr, 0, 0, 2);
        assert_eq!(read_field(arr, NSMUTABLEARRAY_COUNT_OFF), 1);
        let buf = read_field(arr, NSMUTABLEARRAY_ELEMS_OFF);
        assert_eq!(read_field(buf, 0), 0xCC);
        free_array(arr);
    }
}
