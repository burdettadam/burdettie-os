//! NSArray block-based and extended operations.

use crate::cfstring;

use super::{
    make_nsarray, shim_nsarray_count, shim_nsarray_object_at_index, NSARRAY_COUNT_OFF,
    NSARRAY_ELEMS_OFF,
};

fn collect_items(obj: u64) -> Vec<u64> {
    let count = shim_nsarray_count(obj, 0) as usize;
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if obj == 0 || count == 0 || elems == 0 {
        return Vec::new();
    }

    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        items.push(unsafe { *((elems as usize + i * 8) as *const u64) });
    }
    items
}

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

    // Build new immutable array from sorted elements.
    make_nsarray(&items)
}

/// `-[NSArray sortedArrayUsingSelector:]`
///
/// Returns a new NSArray sorted by sending the given selector to each pair.
/// The selector must return NSComparisonResult (e.g., `compare:`).
/// For string elements, we compare via extracted UTF-8 representations.
pub extern "C" fn shim_nsarray_sorted_array_using_selector(obj: u64, _cmd: u64, sel: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let mut items = collect_items(obj);
    if items.is_empty() {
        return make_nsarray(&[]);
    }

    // Try using the selector via objc_msgSend dispatch.
    // The selector is a SEL pointer (C-string). Get the IMP for each element.
    // For safety and performance, first try string-based comparison.
    items.sort_by(|a, b| {
        // Attempt ObjC dispatch: [a compare:b]
        let imp = objc_runtime::shims::dispatch::lookup::objc_msg_lookup_imp(*a, sel);
        if imp != 0 {
            let f: extern "C" fn(u64, u64, u64) -> i64 = unsafe { std::mem::transmute(imp) };
            let result = f(*a, sel, *b);
            return match result {
                r if r < 0 => std::cmp::Ordering::Less,
                0 => std::cmp::Ordering::Equal,
                _ => std::cmp::Ordering::Greater,
            };
        }
        // Fallback: string comparison
        let sa = cfstring::extract_cfstring_cstr(*a).unwrap_or("");
        let sb = cfstring::extract_cfstring_cstr(*b).unwrap_or("");
        sa.cmp(sb)
    });

    make_nsarray(&items)
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

/// `-[NSArray indexOfObjectPassingTest:]`
///
/// Returns the index of the first element for which the block returns YES.
/// Block signature: `BOOL (^)(id obj, NSUInteger idx, BOOL *stop)`
pub extern "C" fn shim_nsarray_index_of_object_passing_test(
    obj: u64,
    _cmd: u64,
    block: u64,
) -> u64 {
    const NS_NOT_FOUND: u64 = u64::MAX;
    if obj == 0 || block == 0 {
        return NS_NOT_FOUND;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if count == 0 || elems == 0 {
        return NS_NOT_FOUND;
    }
    // Block invoke: BOOL (^)(void *block, id obj, NSUInteger idx, BOOL *stop)
    let invoke: extern "C" fn(u64, u64, u64, *mut u8) -> u8 =
        unsafe { std::mem::transmute(*((block as usize + 16) as *const u64)) };
    let mut stop: u8 = 0;
    for i in 0..count {
        let elem = unsafe { *((elems as usize + i as usize * 8) as *const u64) };
        let result = invoke(block, elem, i, &mut stop);
        if result != 0 {
            return i;
        }
        if stop != 0 {
            break;
        }
    }
    NS_NOT_FOUND
}

/// `-[NSArray isEqualToArray:]`
pub extern "C" fn shim_nsarray_is_equal_to_array(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == other {
        return 1;
    }
    if obj == 0 || other == 0 {
        return 0;
    }
    let count_a = shim_nsarray_count(obj, 0);
    let count_b = shim_nsarray_count(other, 0);
    if count_a != count_b {
        return 0;
    }
    for i in 0..count_a {
        let a = shim_nsarray_object_at_index(obj, 0, i);
        let b = shim_nsarray_object_at_index(other, 0, i);
        if a == b {
            continue;
        }
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
    if obj == 0 {
        return 0;
    }
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
    if obj == 0 {
        return 0;
    }
    let count = shim_nsarray_count(obj, 0);
    let loc = location as usize;
    let len = length as usize;
    if loc + len > count as usize {
        log::warn!(
            "subarrayWithRange: range {},{} out of bounds for count={}",
            loc,
            len,
            count
        );
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    let mut items = Vec::with_capacity(len);
    for i in loc..loc + len {
        items.push(unsafe { *((elems as usize + i * 8) as *const u64) });
    }
    make_nsarray(&items)
}

/// `-[NSArray filteredArrayUsingPredicate:]`
///
/// Calls the predicate's stored closure (offset 16 in our NSPredicate layout)
/// on each element and collects those that pass.
pub extern "C" fn shim_nsarray_filtered_using_predicate(
    obj: u64,
    _cmd: u64,
    predicate: u64,
) -> u64 {
    if obj == 0 || predicate == 0 {
        return make_nsarray(&[]);
    }

    let count = shim_nsarray_count(obj, 0) as usize;
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if count == 0 || elems == 0 {
        return make_nsarray(&[]);
    }

    // Our NSPredicate stores a closure at offset 16
    let fn_ptr = unsafe { *((predicate + 16) as *const u64) };
    if fn_ptr == 0 {
        return make_nsarray(&[]);
    }

    let predicate_fn: &(dyn Fn(u64) -> bool + Send + Sync) =
        unsafe { &*(fn_ptr as *const Box<dyn Fn(u64) -> bool + Send + Sync>) };

    let mut results = Vec::new();
    for i in 0..count {
        let elem = unsafe { *((elems as usize + i * 8) as *const u64) };
        if predicate_fn(elem) {
            results.push(elem);
        }
    }
    make_nsarray(&results)
}

/// `-[NSArray copy]` — immutable arrays return self.
pub extern "C" fn shim_nsarray_copy(obj: u64, _cmd: u64) -> u64 {
    objc_runtime::shim_nsobject_retain(obj, 0)
}

/// `-[NSArray mutableCopy]` — creates an NSMutableArray copy.
pub extern "C" fn shim_nsarray_mutable_copy(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::nsmutablearray::shim_nsmutablearray_array_with_array(
        crate::nsobj::nsmutablearray_class_addr(),
        0,
        obj,
    )
}

/// `-[NSMutableArray copy]` — creates an immutable NSArray copy.
pub extern "C" fn shim_nsmutablearray_copy(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    make_nsarray(&collect_items(obj))
}

/// `-[NSMutableArray mutableCopy]` — creates a new mutable NSArray copy.
pub extern "C" fn shim_nsmutablearray_mutable_copy(obj: u64, _cmd: u64) -> u64 {
    shim_nsarray_mutable_copy(obj, _cmd)
}

/// `-[NSArray componentsJoinedByString:]` — joins string elements with a separator.
pub extern "C" fn shim_nsarray_components_joined_by_string(
    obj: u64,
    _cmd: u64,
    separator: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let sep = cfstring::extract_cfstring_cstr(separator).unwrap_or("");
    let items = collect_items(obj);
    let parts: Vec<&str> = items
        .iter()
        .map(|&e| cfstring::extract_cfstring_cstr(e).unwrap_or(""))
        .collect();
    let joined = parts.join(sep);
    let c_str = std::ffi::CString::new(joined).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsarray::{
        make_nsarray, shim_nsarray_count, shim_nsarray_object_at_index, NSARRAY_ELEMS_OFF,
    };
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_enumerate_is_noop() {
        shim_nsarray_enumerate_objects_using_block(0, 0, 0);
        let arr = make_nsarray(&[0xAAA]);
        shim_nsarray_enumerate_objects_using_block(arr, 0, 0);
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
    }

    #[test]
    fn index_of_nil_returns_notfound() {
        assert_eq!(shim_nsarray_index_of_object(0, 0, 0), u64::MAX);
    }

    #[test]
    fn index_of_object_found_by_pointer() {
        let s1 = make_string("alpha");
        let s2 = make_string("beta");
        let arr = make_nsarray(&[s1, s2]);
        assert_eq!(shim_nsarray_index_of_object(arr, 0, s1), 0);
        assert_eq!(shim_nsarray_index_of_object(arr, 0, s2), 1);
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
    }

    #[test]
    fn nsarray_mutable_copy_returns_mutable_array() {
        let arr = make_nsarray(&[0xAA, 0xBB]);
        let copy = shim_nsarray_mutable_copy(arr, 0);
        assert_ne!(copy, 0);
        assert_eq!(
            read_field(copy, 0),
            crate::nsobj::nsmutablearray_class_addr()
        );
        assert_eq!(crate::nsarray::shim_nsarray_count(copy, 0), 2);
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(copy, 0, 1),
            0xBB
        );
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
        free_buf(read_field(
            copy,
            crate::nsmutablearray::NSMUTABLEARRAY_ELEMS_OFF,
        ));
        free_obj(copy);
    }

    #[test]
    fn nsarray_copy_returns_self_with_plus_one() {
        let arr = make_nsarray(&[]);
        let before = objc_runtime::shim_nsobject_retain_count(arr, 0);

        let copy = shim_nsarray_copy(arr, 0);
        assert_eq!(copy, arr);
        assert_eq!(objc_runtime::shim_nsobject_retain_count(arr, 0), before + 1);

        objc_runtime::shim_nsobject_release(copy, 0);
        objc_runtime::shim_nsobject_release(arr, 0);
    }

    #[test]
    fn nsmutablearray_copy_returns_immutable_array() {
        let arr = crate::nsmutablearray::shim_nsmutablearray_array(
            crate::nsobj::nsmutablearray_class_addr(),
            0,
        );
        crate::nsmutablearray::shim_nsmutablearray_add_object(arr, 0, 0xAA);
        crate::nsmutablearray::shim_nsmutablearray_add_object(arr, 0, 0xBB);

        let copy = shim_nsmutablearray_copy(arr, 0);
        assert_ne!(copy, 0);
        assert_eq!(read_field(copy, 0), crate::nsobj::nsarray_class_addr());
        assert_eq!(crate::nsarray::shim_nsarray_count(copy, 0), 2);
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(copy, 0, 0),
            0xAA
        );

        free_buf(read_field(
            arr,
            crate::nsmutablearray::NSMUTABLEARRAY_ELEMS_OFF,
        ));
        free_obj(arr);
        free_buf(read_field(copy, NSARRAY_ELEMS_OFF));
        free_obj(copy);
    }

    #[test]
    fn index_of_object_not_found() {
        let s1 = make_string("hello");
        let s_miss = make_string("world");
        let arr = make_nsarray(&[s1]);
        assert_eq!(shim_nsarray_index_of_object(arr, 0, s_miss), u64::MAX);
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
    }

    #[test]
    fn is_equal_to_array_nil_obj_returns_zero() {
        // nil obj vs non-nil other → 0
        let other = make_nsarray(&[]);
        assert_eq!(shim_nsarray_is_equal_to_array(0, 0, other), 0);
        free_obj(other);
    }

    #[test]
    fn is_equal_to_array_same_pointer() {
        let arr = make_nsarray(&[]);
        assert_eq!(shim_nsarray_is_equal_to_array(arr, 0, arr), 1);
        free_obj(arr);
    }

    #[test]
    fn is_equal_to_array_different_counts_not_equal() {
        let a = make_nsarray(&[0x100]);
        let b = make_nsarray(&[0x100, 0x200]);
        assert_eq!(shim_nsarray_is_equal_to_array(a, 0, b), 0);
        free_buf(read_field(a, NSARRAY_ELEMS_OFF));
        free_buf(read_field(b, NSARRAY_ELEMS_OFF));
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn array_by_adding_object_nil_safe() {
        assert_eq!(shim_nsarray_array_by_adding_object(0, 0, 0xAAA), 0);
    }

    #[test]
    fn array_by_adding_object_appends() {
        let arr = make_nsarray(&[0x111]);
        let new_arr = shim_nsarray_array_by_adding_object(arr, 0, 0x222);
        assert_ne!(new_arr, 0);
        assert_eq!(shim_nsarray_count(new_arr, 0), 2);
        assert_eq!(shim_nsarray_object_at_index(new_arr, 0, 1), 0x222);
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_buf(read_field(new_arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
        free_obj(new_arr);
    }

    #[test]
    fn subarray_nil_safe() {
        assert_eq!(shim_nsarray_subarray_with_range(0, 0, 0, 2), 0);
    }

    #[test]
    fn subarray_valid_range() {
        let arr = make_nsarray(&[0xA, 0xB, 0xC, 0xD]);
        let sub = shim_nsarray_subarray_with_range(arr, 0, 1, 2);
        assert_ne!(sub, 0);
        assert_eq!(shim_nsarray_count(sub, 0), 2);
        assert_eq!(shim_nsarray_object_at_index(sub, 0, 0), 0xB);
        assert_eq!(shim_nsarray_object_at_index(sub, 0, 1), 0xC);
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_buf(read_field(sub, NSARRAY_ELEMS_OFF));
        free_obj(arr);
        free_obj(sub);
    }

    #[test]
    fn sorted_array_nil_obj_returns_zero() {
        assert_eq!(shim_nsarray_sorted_array_using_comparator(0, 0, 0xBEEF), 0);
    }

    #[test]
    fn sorted_array_nil_comparator_returns_zero() {
        let arr = make_nsarray(&[0x1]);
        assert_eq!(shim_nsarray_sorted_array_using_comparator(arr, 0, 0), 0);
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
    }

    #[test]
    fn filtered_nil_predicate_returns_empty_array() {
        let arr = make_nsarray(&[0x1, 0x2]);
        let result = shim_nsarray_filtered_using_predicate(arr, 0, 0);
        assert_ne!(result, 0);
        assert_eq!(shim_nsarray_count(result, 0), 0);
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
        free_obj(result);
    }

    // ── componentsJoinedByString: tests ──────────────────────────────

    #[test]
    fn components_joined_nil_safe() {
        assert_eq!(shim_nsarray_components_joined_by_string(0, 0, 0), 0);
    }

    #[test]
    fn components_joined_basic() {
        let s1 = make_string("hello");
        let s2 = make_string("world");
        let arr = make_nsarray(&[s1, s2]);
        let sep = make_string(", ");
        let result = shim_nsarray_components_joined_by_string(arr, 0, sep);
        assert_ne!(result, 0);
        assert_string_eq(result, "hello, world");
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
    }

    #[test]
    fn components_joined_empty_array() {
        let arr = make_nsarray(&[]);
        let sep = make_string(",");
        let result = shim_nsarray_components_joined_by_string(arr, 0, sep);
        assert_ne!(result, 0);
        assert_string_eq(result, "");
        free_obj(arr);
    }

    #[test]
    fn components_joined_single_element() {
        let s1 = make_string("only");
        let arr = make_nsarray(&[s1]);
        let sep = make_string("-");
        let result = shim_nsarray_components_joined_by_string(arr, 0, sep);
        assert_ne!(result, 0);
        assert_string_eq(result, "only");
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
    }

    #[test]
    fn components_joined_empty_separator() {
        let s1 = make_string("a");
        let s2 = make_string("b");
        let s3 = make_string("c");
        let arr = make_nsarray(&[s1, s2, s3]);
        let sep = make_string("");
        let result = shim_nsarray_components_joined_by_string(arr, 0, sep);
        assert_ne!(result, 0);
        assert_string_eq(result, "abc");
        free_buf(read_field(arr, NSARRAY_ELEMS_OFF));
        free_obj(arr);
    }
}
