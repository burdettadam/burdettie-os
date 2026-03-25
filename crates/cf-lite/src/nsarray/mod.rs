//! NSArray shim: immutable ordered collection of objects.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa          (u64 — pointer to NSArray class)
//!   offset  8: count        (u64 — number of elements)
//!   offset 16: elements_ptr (u64 — pointer to malloc'd id[] array)
//!
//! Total: 24 bytes per array object.

use crate::cfstring;

pub mod ops;

pub(crate) const NSARRAY_COUNT_OFF: usize = 8;
pub(crate) const NSARRAY_ELEMS_OFF: usize = 16;

/// Create an NSArray from a slice of object pointers.
pub fn make_nsarray(items: &[u64]) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsarray_class_addr();
        *((obj as usize + NSARRAY_COUNT_OFF) as *mut u64) = items.len() as u64;
    }
    if !items.is_empty() {
        let buf = crate::pal_alloc().malloc(items.len() * 8) as *mut u64;
        if !buf.is_null() {
            for (i, &item) in items.iter().enumerate() {
                unsafe { buf.add(i).write(item); }
            }
        }
        unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *mut u64) = buf as u64; }
    }
    obj
}

/// `+[NSArray arrayWithObjects:count:]`
///
/// Class factory: creates immutable array from a C array of object pointers.
/// objects is a pointer to `id objects[]`, cnt is the element count.
pub extern "C" fn shim_nsarray_array_with_objects_count(
    cls: u64,
    _cmd: u64,
    objects: u64,
    cnt: u64,
) -> u64 {
    if cls == 0 {
        return 0;
    }

    // Allocate the NSArray struct (24 bytes)
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }

    // Set ISA = NSArray class
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsarray_class_addr();
    }

    // Set count
    unsafe {
        *((obj as usize + NSARRAY_COUNT_OFF) as *mut u64) = cnt;
    }

    if cnt > 0 && objects != 0 {
        // Allocate elements buffer and copy object pointers
        let buf = crate::pal_alloc().malloc(cnt as usize * 8) as u64;
        if buf != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    objects as *const u8,
                    buf as *mut u8,
                    cnt as usize * 8,
                );
            }
        }
        unsafe {
            *((obj as usize + NSARRAY_ELEMS_OFF) as *mut u64) = buf;
        }
    }

    log::debug!(
        "NSArray arrayWithObjects:count: → {:#x} ({} elements)",
        obj,
        cnt
    );
    obj
}

/// `-[NSArray count]` — returns the number of elements.
pub extern "C" fn shim_nsarray_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) }
}

/// `-[NSArray objectAtIndex:]` — returns the element at the given index.
///
/// Returns 0 (nil) if index is out of bounds (real NSArray throws
/// NSRangeException, but we can't throw ObjC exceptions yet).
pub extern "C" fn shim_nsarray_object_at_index(obj: u64, _cmd: u64, index: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    if index >= count {
        eprintln!(
            "NSArray objectAtIndex:{} beyond bounds [0..{})",
            index, count
        );
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe { *((elems as usize + index as usize * 8) as *const u64) }
}

/// `-[NSArray firstObject]` — returns the first element or nil.
pub extern "C" fn shim_nsarray_first_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    if count == 0 {
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe { *(elems as *const u64) }
}

/// `-[NSArray lastObject]` — returns the last element or nil.
pub extern "C" fn shim_nsarray_last_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    if count == 0 {
        return 0;
    }
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe { *((elems as usize + (count as usize - 1) * 8) as *const u64) }
}

/// `-[NSArray containsObject:]` — linear scan comparing pointers.
///
/// For NSString elements, we also compare via UTF8String if pointers differ.
pub extern "C" fn shim_nsarray_contains_object(obj: u64, _cmd: u64, target: u64) -> u8 {
    if obj == 0 || target == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };
    if elems == 0 || count == 0 {
        return 0;
    }

    // Get target's string representation for comparison
    let target_cstr = cfstring::extract_cfstring_cstr(target);

    for i in 0..count as usize {
        let elem = unsafe { *((elems + i as u64 * 8) as *const u64) };
        // Pointer equality
        if elem == target {
            return 1;
        }
        // String content equality if both are string-like
        if let Some(target_s) = &target_cstr {
            if let Some(elem_s) = cfstring::extract_cfstring_cstr(elem) {
                if elem_s == *target_s {
                    return 1;
                }
            }
        }
    }
    0
}

/// `-[NSArray description]` — returns a string representation.
///
/// We return a simple format like "(elem1, elem2, ...)".
pub extern "C" fn shim_nsarray_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };

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

    // Create an NSString from the description
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests;

