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
