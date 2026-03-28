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

pub mod ops;

pub(crate) const NSMUTABLEARRAY_COUNT_OFF: usize = 8;
pub(crate) const NSMUTABLEARRAY_ELEMS_OFF: usize = 16;
const NSMUTABLEARRAY_CAP_OFF: usize = 24;

const INITIAL_CAPACITY: u64 = 4;

/// `+[NSMutableArray array]` — creates an empty mutable array.
pub extern "C" fn shim_nsmutablearray_array(cls: u64, _cmd: u64) -> u64 {
    shim_nsmutablearray_array_with_capacity(cls, _cmd, INITIAL_CAPACITY)
}

/// `+[NSMutableArray arrayWithArray:]` — creates a new mutable array
/// containing all elements from the source array.
pub extern "C" fn shim_nsmutablearray_array_with_array(cls: u64, _cmd: u64, source: u64) -> u64 {
    if source == 0 {
        return shim_nsmutablearray_array(cls, _cmd);
    }
    let count = crate::nsarray::shim_nsarray_count(source, 0);
    let cap = if count == 0 { INITIAL_CAPACITY } else { count };
    let new_arr = shim_nsmutablearray_array_with_capacity(cls, _cmd, cap);
    if new_arr == 0 {
        return 0;
    }
    for i in 0..count {
        let elem = crate::nsarray::shim_nsarray_object_at_index(source, 0, i);
        shim_nsmutablearray_add_object(new_arr, 0, elem);
    }
    new_arr
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

    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }

    // Set ISA = NSMutableArray class
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsmutablearray_class_addr();
    }

    // count = 0 (already zeroed by calloc)
    // Allocate elements buffer
    let buf = crate::pal_alloc().calloc(cap as usize, 8) as u64;
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
    let new_buf = crate::pal_alloc().realloc(old_buf as *mut u8, new_cap as usize * 8) as u64;
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

    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests;
