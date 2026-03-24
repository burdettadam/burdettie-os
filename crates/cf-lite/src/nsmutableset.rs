//! NSMutableSet shim: mutable unordered collection of unique objects.
//!
//! Object layout — same as NSSet (24 bytes):
//!
//!   offset  0: isa          (u64 — pointer to NSMutableSet class)
//!   offset  8: count        (u64 — number of elements)
//!   offset 16: elements_ptr (u64 — pointer to malloc'd id[] array)
//!
//! Mutation methods reallocate the backing array as needed.

use crate::cfstring;
use crate::nsobj;

const COUNT_OFF: usize = 8;
const ELEMS_OFF: usize = 16;

fn set_count(obj: u64) -> u64 {
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

fn set_elems(obj: u64) -> u64 {
    unsafe { *((obj as usize + ELEMS_OFF) as *const u64) }
}

/// Check if needle exists in the set.
fn contains(obj: u64, needle: u64) -> bool {
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return false;
    }
    let needle_cstr = cfstring::extract_cfstring_cstr(needle);
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if e == needle {
            return true;
        }
        if let Some(ns) = &needle_cstr {
            if let Some(es) = cfstring::extract_cfstring_cstr(e) {
                if es == *ns {
                    return true;
                }
            }
        }
    }
    false
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSMutableSet set]` — empty mutable set.
pub extern "C" fn shim_nsmutableset_set(_cls: u64, _cmd: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, 24) } as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsmutableset_class_addr();
    }
    obj
}

/// `+[NSMutableSet setWithCapacity:]` — same as `set` (we don't pre-allocate).
pub extern "C" fn shim_nsmutableset_set_with_capacity(_cls: u64, _cmd: u64, _cap: u64) -> u64 {
    shim_nsmutableset_set(0, 0)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSMutableSet addObject:]` — adds the object if not already present.
pub extern "C" fn shim_nsmutableset_add_object(obj: u64, _cmd: u64, element: u64) {
    if obj == 0 || element == 0 {
        return;
    }
    if contains(obj, element) {
        return;
    }

    let count = set_count(obj) as usize;
    let old_elems = set_elems(obj);
    let new_count = count + 1;
    let new_buf = unsafe { libc::malloc(new_count * 8) } as u64;
    if new_buf == 0 {
        return;
    }

    // Copy existing elements
    if count > 0 && old_elems != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(old_elems as *const u8, new_buf as *mut u8, count * 8);
        }
    }
    // Append new element
    unsafe {
        *((new_buf as usize + count * 8) as *mut u64) = element;
    }

    // Free old buffer
    if old_elems != 0 {
        unsafe {
            libc::free(old_elems as *mut libc::c_void);
        }
    }

    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = new_count as u64;
        *((obj as usize + ELEMS_OFF) as *mut u64) = new_buf;
    }
}

/// `-[NSMutableSet removeObject:]` — removes the object if present.
pub extern "C" fn shim_nsmutableset_remove_object(obj: u64, _cmd: u64, element: u64) {
    if obj == 0 || element == 0 {
        return;
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return;
    }

    let element_cstr = cfstring::extract_cfstring_cstr(element);

    // Find the index to remove
    let mut found_idx = None;
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if e == element {
            found_idx = Some(i);
            break;
        }
        if let Some(ns) = &element_cstr {
            if let Some(es) = cfstring::extract_cfstring_cstr(e) {
                if es == *ns {
                    found_idx = Some(i);
                    break;
                }
            }
        }
    }

    if let Some(idx) = found_idx {
        // Shift remaining elements down
        for i in idx..count - 1 {
            let next = unsafe { *((elems as usize + (i + 1) * 8) as *const u64) };
            unsafe {
                *((elems as usize + i * 8) as *mut u64) = next;
            }
        }
        unsafe {
            *((obj as usize + COUNT_OFF) as *mut u64) = (count - 1) as u64;
        }
    }
}

/// `-[NSMutableSet removeAllObjects]`
pub extern "C" fn shim_nsmutableset_remove_all_objects(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let elems = set_elems(obj);
    if elems != 0 {
        unsafe {
            libc::free(elems as *mut libc::c_void);
        }
    }
    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = 0;
        *((obj as usize + ELEMS_OFF) as *mut u64) = 0;
    }
}

/// `-[NSMutableSet unionSet:]` — adds all objects from another set.
pub extern "C" fn shim_nsmutableset_union_set(obj: u64, _cmd: u64, other: u64) {
    if obj == 0 || other == 0 {
        return;
    }
    let other_count = set_count(other) as usize;
    let other_elems = set_elems(other);
    if other_count == 0 || other_elems == 0 {
        return;
    }
    for i in 0..other_count {
        let e = unsafe { *((other_elems as usize + i * 8) as *const u64) };
        shim_nsmutableset_add_object(obj, 0, e);
    }
}

/// `-[NSMutableSet intersectSet:]` — removes objects not in `other`.
pub extern "C" fn shim_nsmutableset_intersect_set(obj: u64, _cmd: u64, other: u64) {
    if obj == 0 {
        return;
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return;
    }

    // Collect elements to keep
    let mut keep: Vec<u64> = Vec::new();
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if other != 0 && crate::nsset::contains(other, e) {
            keep.push(e);
        }
    }

    // Rebuild the elements buffer
    if elems != 0 {
        unsafe {
            libc::free(elems as *mut libc::c_void);
        }
    }
    if keep.is_empty() {
        unsafe {
            *((obj as usize + COUNT_OFF) as *mut u64) = 0;
            *((obj as usize + ELEMS_OFF) as *mut u64) = 0;
        }
    } else {
        let buf = unsafe { libc::malloc(keep.len() * 8) } as u64;
        if buf != 0 {
            for (i, &e) in keep.iter().enumerate() {
                unsafe {
                    *((buf as usize + i * 8) as *mut u64) = e;
                }
            }
        }
        unsafe {
            *((obj as usize + COUNT_OFF) as *mut u64) = keep.len() as u64;
            *((obj as usize + ELEMS_OFF) as *mut u64) = buf;
        }
    }
}

/// `-[NSMutableSet minusSet:]` — removes objects present in `other`.
pub extern "C" fn shim_nsmutableset_minus_set(obj: u64, _cmd: u64, other: u64) {
    if obj == 0 || other == 0 {
        return;
    }
    let other_count = set_count(other) as usize;
    let other_elems = set_elems(other);
    if other_count == 0 || other_elems == 0 {
        return;
    }
    for i in 0..other_count {
        let e = unsafe { *((other_elems as usize + i * 8) as *const u64) };
        shim_nsmutableset_remove_object(obj, 0, e);
    }
}
