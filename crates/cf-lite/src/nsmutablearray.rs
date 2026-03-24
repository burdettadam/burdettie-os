//! NSMutableArray shim: mutable ordered collection of objects.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa          (u64 — pointer to NSMutableArray class)
//!   offset  8: count        (u64 — number of elements)
//!   offset 16: elements_ptr (u64 — pointer to malloc'd id[] array)
//!   offset 24: capacity     (u64 — allocated slots in elements array)
//!
//! Total: 32 bytes per mutable array object.
//!
//! NSMutableArray reuses the same offsets for count and elements_ptr
//! as NSArray, so all immutable read methods (count, objectAtIndex:,
//! firstObject, lastObject, containsObject:, description) can be
//! registered with the NSArray function pointers directly.

use crate::cfstring;

const NSMUTABLEARRAY_COUNT_OFF: usize = 8;
const NSMUTABLEARRAY_ELEMS_OFF: usize = 16;
const NSMUTABLEARRAY_CAP_OFF: usize = 24;

const INITIAL_CAPACITY: u64 = 4;

/// `+[NSMutableArray array]` — creates an empty mutable array.
pub extern "C" fn shim_nsmutablearray_array(cls: u64, _cmd: u64) -> u64 {
    shim_nsmutablearray_array_with_capacity(cls, _cmd, INITIAL_CAPACITY)
}

/// `+[NSMutableArray arrayWithCapacity:]` — creates an empty mutable array
/// with pre-allocated capacity.
pub extern "C" fn shim_nsmutablearray_array_with_capacity(
    cls: u64,
    _cmd: u64,
    capacity: u64,
) -> u64 {
    if cls == 0 {
        return 0;
    }

    let cap = if capacity == 0 {
        INITIAL_CAPACITY
    } else {
        capacity
    };

    let obj = unsafe { libc::calloc(1, 32) } as u64;
    if obj == 0 {
        return 0;
    }

    // Set ISA = NSMutableArray class
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsmutablearray_class_addr();
    }

    // count = 0 (already zeroed by calloc)
    // Allocate elements buffer
    let buf = unsafe { libc::calloc(cap as usize, 8) } as u64;
    if buf != 0 {
        unsafe {
            *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *mut u64) = buf;
        }
    }
    unsafe {
        *((obj as usize + NSMUTABLEARRAY_CAP_OFF) as *mut u64) = cap;
    }

    log::debug!("NSMutableArray array → {:#x} (capacity {})", obj, cap);
    obj
}

/// Grow the elements buffer when count == capacity.
unsafe fn grow_if_needed(obj: u64) {
    let count = *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64);
    let cap = *((obj as usize + NSMUTABLEARRAY_CAP_OFF) as *const u64);
    if count < cap {
        return;
    }

    let new_cap = if cap == 0 { INITIAL_CAPACITY } else { cap * 2 };
    let old_buf = *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64);
    let new_buf = libc::realloc(old_buf as *mut libc::c_void, new_cap as usize * 8) as u64;
    if new_buf != 0 {
        // Zero newly allocated slots
        let start = (new_buf as usize + count as usize * 8) as *mut u8;
        std::ptr::write_bytes(start, 0, (new_cap - count) as usize * 8);
        *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *mut u64) = new_buf;
        *((obj as usize + NSMUTABLEARRAY_CAP_OFF) as *mut u64) = new_cap;
    }
}

/// `-[NSMutableArray addObject:]` — appends an object to the end.
pub extern "C" fn shim_nsmutablearray_add_object(obj: u64, _cmd: u64, element: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    unsafe {
        grow_if_needed(obj);
    }

    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }

    unsafe {
        *((elems as usize + count as usize * 8) as *mut u64) = element;
        *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *mut u64) = count + 1;
    }
    0 // void return
}

/// `-[NSMutableArray insertObject:atIndex:]` — inserts at the given index,
/// shifting subsequent elements right.
pub extern "C" fn shim_nsmutablearray_insert_object_at_index(
    obj: u64,
    _cmd: u64,
    element: u64,
    index: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    if index > count {
        eprintln!(
            "NSMutableArray insertObject:atIndex:{} beyond bounds [0..{}]",
            index, count
        );
        return 0;
    }

    unsafe {
        grow_if_needed(obj);
    }

    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }

    // Shift elements [index..count) right by one slot
    if index < count {
        unsafe {
            let src = (elems as usize + index as usize * 8) as *const u8;
            let dst = (elems as usize + (index as usize + 1) * 8) as *mut u8;
            std::ptr::copy(src, dst, (count - index) as usize * 8);
        }
    }

    unsafe {
        *((elems as usize + index as usize * 8) as *mut u64) = element;
        *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *mut u64) = count + 1;
    }
    0
}

/// `-[NSMutableArray removeObjectAtIndex:]` — removes the element at the
/// given index, shifting subsequent elements left.
pub extern "C" fn shim_nsmutablearray_remove_object_at_index(
    obj: u64,
    _cmd: u64,
    index: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    if index >= count {
        eprintln!(
            "NSMutableArray removeObjectAtIndex:{} beyond bounds [0..{})",
            index, count
        );
        return 0;
    }

    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }

    // Shift elements [index+1..count) left by one slot
    if index + 1 < count {
        unsafe {
            let src = (elems as usize + (index as usize + 1) * 8) as *const u8;
            let dst = (elems as usize + index as usize * 8) as *mut u8;
            std::ptr::copy(src, dst, (count - index - 1) as usize * 8);
        }
    }

    unsafe {
        *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *mut u64) = count - 1;
    }
    0
}

/// `-[NSMutableArray removeLastObject]` — removes the last element.
pub extern "C" fn shim_nsmutablearray_remove_last_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    if count == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *mut u64) = count - 1;
    }
    0
}

/// `-[NSMutableArray replaceObjectAtIndex:withObject:]` — replaces the
/// element at the given index with a new object.
pub extern "C" fn shim_nsmutablearray_replace_object_at_index(
    obj: u64,
    _cmd: u64,
    index: u64,
    new_obj: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    if index >= count {
        eprintln!(
            "NSMutableArray replaceObjectAtIndex:{} beyond bounds [0..{})",
            index, count
        );
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe {
        *((elems as usize + index as usize * 8) as *mut u64) = new_obj;
    }
    0
}

/// `-[NSMutableArray removeAllObjects]` — sets count to 0 (keeps capacity).
pub extern "C" fn shim_nsmutablearray_remove_all_objects(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *mut u64) = 0;
    }
    0
}

/// `-[NSMutableArray description]` — returns a string representation.
///
/// Delegates to the same format as NSArray description.
pub extern "C" fn shim_nsmutablearray_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSMUTABLEARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSMUTABLEARRAY_ELEMS_OFF) as *const u64) };

    let mut parts = Vec::new();
    for i in 0..count as usize {
        if elems == 0 {
            parts.push("(null)".to_string());
        } else {
            let elem = unsafe { *((elems + i as u64 * 8) as *const u64) };
            if let Some(s) = cfstring::extract_cfstring_cstr(elem) {
                parts.push(format!("\"{}\"", s));
            } else {
                parts.push(format!("{:#x}", elem));
            }
        }
    }
    let desc = format!("({})", parts.join(", "));

    let c_str = std::ffi::CString::new(desc).unwrap();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsarray;

    /// Helper: create a bare mutable array object with the given capacity.
    fn make_empty(cap: u64) -> u64 {
        let obj = unsafe { libc::calloc(1, 32) } as u64;
        unsafe {
            *(obj as *mut u64) = 0xDEAD; // fake ISA
            let buf = libc::calloc(cap as usize, 8) as u64;
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
            unsafe {
                libc::free(buf as *mut libc::c_void);
            }
        }
        unsafe {
            libc::free(obj as *mut libc::c_void);
        }
    }

    #[test]
    fn add_and_count() {
        let arr = make_empty(4);
        shim_nsmutablearray_add_object(arr, 0, 0xAAA);
        shim_nsmutablearray_add_object(arr, 0, 0xBBB);
        shim_nsmutablearray_add_object(arr, 0, 0xCCC);
        assert_eq!(get_count(arr), 3);
        // Use NSArray read functions (same layout)
        assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
        assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
        assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCCC);
        free_array(arr);
    }

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
    fn remove_at_index() {
        let arr = make_empty(8);
        shim_nsmutablearray_add_object(arr, 0, 0xAAA);
        shim_nsmutablearray_add_object(arr, 0, 0xBBB);
        shim_nsmutablearray_add_object(arr, 0, 0xCCC);
        // Remove middle element (index 1)
        shim_nsmutablearray_remove_object_at_index(arr, 0, 1);
        assert_eq!(get_count(arr), 2);
        assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
        assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xCCC);
        free_array(arr);
    }

    #[test]
    fn insert_at_index() {
        let arr = make_empty(8);
        shim_nsmutablearray_add_object(arr, 0, 0xAAA);
        shim_nsmutablearray_add_object(arr, 0, 0xCCC);
        // Insert at index 1 (between AAA and CCC)
        shim_nsmutablearray_insert_object_at_index(arr, 0, 0xBBB, 1);
        assert_eq!(get_count(arr), 3);
        assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 0), 0xAAA);
        assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 1), 0xBBB);
        assert_eq!(nsarray::shim_nsarray_object_at_index(arr, 0, 2), 0xCCC);
        free_array(arr);
    }

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
    fn remove_all() {
        let arr = make_empty(4);
        shim_nsmutablearray_add_object(arr, 0, 0xAAA);
        shim_nsmutablearray_add_object(arr, 0, 0xBBB);
        shim_nsmutablearray_remove_all_objects(arr, 0);
        assert_eq!(get_count(arr), 0);
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
}
