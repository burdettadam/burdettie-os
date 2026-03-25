//! NSArray block-based and extended operations.

use crate::cfstring;

use super::{
    make_nsarray, shim_nsarray_count, shim_nsarray_object_at_index, NSARRAY_COUNT_OFF,
    NSARRAY_ELEMS_OFF,
};

/// `-[NSArray enumerateObjectsUsingBlock:]`
///
/// Calls `block(obj, idx, &stop)` for each element. The block is an
/// ObjC block literal with invoke at offset 16. The invoke signature is:
///   `void (^)(id obj, NSUInteger idx, BOOL *stop)`
pub extern "C" fn shim_nsarray_enumerate_objects_using_block(obj: u64, _cmd: u64, block: u64) {
    if obj == 0 || block == 0 {
        return;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if count == 0 || elems == 0 {
        return;
    }

    // Block invoke signature: void (^)(void *block, id obj, NSUInteger idx, BOOL *stop)
    let invoke: extern "C" fn(u64, u64, u64, *mut u8) =
        unsafe { std::mem::transmute(*((block as usize + 16) as *const u64)) };

    let mut stop: u8 = 0;
    for i in 0..count {
        let elem = unsafe { *((elems as usize + i as usize * 8) as *const u64) };
        invoke(block, elem, i, &mut stop);
        if stop != 0 {
            break;
        }
    }
}

/// `-[NSArray sortedArrayUsingComparator:]`
///
/// Returns a new NSArray sorted using the given comparator block.
/// Block signature: `NSComparisonResult (^)(id obj1, id obj2)`
/// Returns NSOrderedAscending (-1), NSOrderedSame (0), or NSOrderedDescending (1).
pub extern "C" fn shim_nsarray_sorted_array_using_comparator(
    obj: u64,
    _cmd: u64,
    comparator: u64,
) -> u64 {
    if obj == 0 || comparator == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) } as usize;
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if count == 0 {
        // Return empty array
        return super::shim_nsarray_array_with_objects_count(
            crate::nsobj::nsarray_class_addr(),
            0,
            0,
            0,
        );
    }

    // Copy elements into a Vec for sorting
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

    // Build new immutable array from sorted elements
    let buf = crate::pal_alloc().malloc(count * 8) as u64;
    if buf == 0 {
        return 0;
    }
    for (i, item) in items.iter().enumerate() {
        unsafe {
            *((buf as usize + i * 8) as *mut u64) = *item;
        }
    }

    let result = crate::pal_alloc().calloc(1, 24) as u64;
    if result == 0 {
        crate::pal_alloc().free(buf as *mut u8);
        return 0;
    }
    unsafe {
        *(result as *mut u64) = crate::nsobj::nsarray_class_addr();
        *((result as usize + NSARRAY_COUNT_OFF) as *mut u64) = count as u64;
        *((result as usize + NSARRAY_ELEMS_OFF) as *mut u64) = buf;
    }
    result
}

/// `-[NSArray indexOfObject:]` — linear scan, returns NSNotFound if not found.
pub extern "C" fn shim_nsarray_index_of_object(obj: u64, _cmd: u64, target: u64) -> u64 {
    const NS_NOT_FOUND: u64 = u64::MAX; // NSNotFound = NSIntegerMax
    if obj == 0 || target == 0 {
        return NS_NOT_FOUND;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 || count == 0 {
        return NS_NOT_FOUND;
    }
    let target_cstr = cfstring::extract_cfstring_cstr(target);
    for i in 0..count as usize {
        let elem = unsafe { *((elems + i as u64 * 8) as *const u64) };
        if elem == target {
            return i as u64;
        }
        if let Some(target_s) = &target_cstr {
            if let Some(elem_s) = cfstring::extract_cfstring_cstr(elem) {
                if elem_s == *target_s {
                    return i as u64;
                }
            }
        }
    }
    NS_NOT_FOUND
}

/// `-[NSArray isEqualToArray:]`
pub extern "C" fn shim_nsarray_is_equal_to_array(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == other { return 1; }
    if obj == 0 || other == 0 { return 0; }
    let count_a = shim_nsarray_count(obj, 0);
    let count_b = shim_nsarray_count(other, 0);
    if count_a != count_b { return 0; }
    for i in 0..count_a {
        let a = shim_nsarray_object_at_index(obj, 0, i);
        let b = shim_nsarray_object_at_index(other, 0, i);
        if a == b { continue; }
        // Try string comparison
        let a_s = cfstring::extract_cfstring_cstr(a);
        let b_s = cfstring::extract_cfstring_cstr(b);
        match (a_s, b_s) {
            (Some(sa), Some(sb)) if sa == sb => continue,
            _ => return 0,
        }
    }
    1
}

/// `-[NSArray arrayByAddingObject:]` — returns new array with element appended.
pub extern "C" fn shim_nsarray_array_by_adding_object(obj: u64, _cmd: u64, element: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = shim_nsarray_count(obj, 0) as usize;
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    let mut items = Vec::with_capacity(count + 1);
    for i in 0..count {
        items.push(unsafe { *((elems as usize + i * 8) as *const u64) });
    }
    items.push(element);
    make_nsarray(&items)
}

/// `-[NSArray subarrayWithRange:]`
///
/// On aarch64, NSRange is passed in x2+x3.
pub extern "C" fn shim_nsarray_subarray_with_range(
    obj: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) -> u64 {
    if obj == 0 { return 0; }
    let count = shim_nsarray_count(obj, 0);
    let loc = location as usize;
    let len = length as usize;
    if loc + len > count as usize {
        log::warn!("subarrayWithRange: range {},{} out of bounds for count={}", loc, len, count);
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    let mut items = Vec::with_capacity(len);
    for i in loc..loc + len {
        items.push(unsafe { *((elems as usize + i * 8) as *const u64) });
    }
    make_nsarray(&items)
}
