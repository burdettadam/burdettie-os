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
    if a == b { return true; }
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
        if elements_equal(elem, target) { return true; }
    }
    false
}

// ── NSOrderedSet ───────────────────────────────────────────────────

/// `+[NSOrderedSet orderedSetWithArray:]`
pub extern "C" fn shim_nsorderedset_ordered_set_with_array(
    _cls: u64, _cmd: u64, array: u64,
) -> u64 {
    let count = if array != 0 {
        crate::nsarray::shim_nsarray_count(array, 0)
    } else { 0 };
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nsorderedset_class_addr(); }

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
            unsafe { *((buf as usize + n as usize * 8) as *mut u64) = elem; }
            n += 1;
        }
    }
    unsafe { *((obj as usize + COUNT_OFF) as *mut u64) = n; }
    obj
}

/// `+[NSOrderedSet orderedSet]`
pub extern "C" fn shim_nsorderedset_ordered_set(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nsorderedset_class_addr(); }
    let buf = crate::pal_alloc().calloc(INITIAL_CAP as usize, 8) as u64;
    unsafe {
        *((obj as usize + ELEMS_OFF) as *mut u64) = buf;
        *((obj as usize + CAP_OFF) as *mut u64) = INITIAL_CAP;
    }
    obj
}

/// `-[NSOrderedSet count]`
pub extern "C" fn shim_nsorderedset_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

/// `-[NSOrderedSet objectAtIndex:]`
pub extern "C" fn shim_nsorderedset_object_at_index(obj: u64, _cmd: u64, idx: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    if idx >= count { return 0; }
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if elems == 0 { return 0; }
    unsafe { *((elems as usize + idx as usize * 8) as *const u64) }
}

/// `-[NSOrderedSet firstObject]`
pub extern "C" fn shim_nsorderedset_first_object(obj: u64, _cmd: u64) -> u64 {
    shim_nsorderedset_object_at_index(obj, _cmd, 0)
}

/// `-[NSOrderedSet lastObject]`
pub extern "C" fn shim_nsorderedset_last_object(obj: u64, _cmd: u64) -> u64 {
    let count = shim_nsorderedset_count(obj, 0);
    if count == 0 { return 0; }
    shim_nsorderedset_object_at_index(obj, 0, count - 1)
}

/// `-[NSOrderedSet containsObject:]`
pub extern "C" fn shim_nsorderedset_contains_object(obj: u64, _cmd: u64, target: u64) -> u8 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    u8::from(contains_element(elems, count, target))
}

/// `-[NSOrderedSet indexOfObject:]`
pub extern "C" fn shim_nsorderedset_index_of_object(obj: u64, _cmd: u64, target: u64) -> u64 {
    if obj == 0 { return u64::MAX; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    for i in 0..count as usize {
        let elem = unsafe { *((elems as usize + i * 8) as *const u64) };
        if elements_equal(elem, target) { return i as u64; }
    }
    u64::MAX // NSNotFound
}

/// `-[NSOrderedSet array]` — returns an NSArray of all elements in order.
pub extern "C" fn shim_nsorderedset_array(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
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

/// `-[NSOrderedSet description]`
pub extern "C" fn shim_nsorderedset_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let desc = format!("<NSOrderedSet: {} objects>", count);
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

// ── NSMutableOrderedSet ────────────────────────────────────────────

unsafe fn grow_ordered_set(obj: u64) {
    let count = *((obj as usize + COUNT_OFF) as *const u64);
    let cap = *((obj as usize + CAP_OFF) as *const u64);
    if count < cap { return; }
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
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nsmutableorderedset_class_addr(); }
    let buf = crate::pal_alloc().calloc(INITIAL_CAP as usize, 8) as u64;
    unsafe {
        *((obj as usize + ELEMS_OFF) as *mut u64) = buf;
        *((obj as usize + CAP_OFF) as *mut u64) = INITIAL_CAP;
    }
    obj
}

/// `-[NSMutableOrderedSet addObject:]`
pub extern "C" fn shim_nsmutableorderedset_add_object(obj: u64, _cmd: u64, element: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if contains_element(elems, count, element) { return 0; } // already present
    unsafe { grow_ordered_set(obj); }
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
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) } as usize;
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if elems == 0 || count == 0 { return 0; }
    let mut write = 0usize;
    for read in 0..count {
        let elem = unsafe { *((elems as usize + read * 8) as *const u64) };
        if !elements_equal(elem, target) {
            if write != read {
                unsafe { *((elems as usize + write * 8) as *mut u64) = elem; }
            }
            write += 1;
        }
    }
    unsafe { *((obj as usize + COUNT_OFF) as *mut u64) = write as u64; }
    0
}

/// `-[NSMutableOrderedSet removeAllObjects]`
pub extern "C" fn shim_nsmutableorderedset_remove_all(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + COUNT_OFF) as *mut u64) = 0; }
    0
}

/// `-[NSMutableOrderedSet insertObject:atIndex:]`
pub extern "C" fn shim_nsmutableorderedset_insert_object(
    obj: u64, _cmd: u64, element: u64, idx: u64,
) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + ELEMS_OFF) as *const u64) };
    if contains_element(elems, count, element) { return 0; }
    if idx > count { return 0; }
    unsafe { grow_ordered_set(obj); }
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
