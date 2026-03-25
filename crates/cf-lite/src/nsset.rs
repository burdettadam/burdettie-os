//! NSSet shim: immutable unordered collection of unique objects.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa          (u64 — pointer to NSSet class)
//!   offset  8: count        (u64 — number of elements)
//!   offset 16: elements_ptr (u64 — pointer to malloc'd id[] array, deduplicated)
//!
//! Total: 24 bytes per set object.
//!
//! Uniqueness is determined by pointer equality first, then string
//! content equality for NSString/CFString objects (same as NSDictionary key lookup).

use crate::cfstring;
use crate::nsarray;
use crate::nsobj;

const COUNT_OFF: usize = 8;
const ELEMS_OFF: usize = 16;

// ── Helpers ────────────────────────────────────────────────────────

fn set_count(obj: u64) -> u64 {
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

fn set_elems(obj: u64) -> u64 {
    unsafe { *((obj as usize + ELEMS_OFF) as *const u64) }
}

/// Check if `needle` is contained in the set's elements.
pub(crate) fn contains(obj: u64, needle: u64) -> bool {
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
        if let Some(target_s) = &needle_cstr {
            if let Some(elem_s) = cfstring::extract_cfstring_cstr(e) {
                if elem_s == *target_s {
                    return true;
                }
            }
        }
    }
    false
}

/// Allocate a set from a C array, deduplicating entries.
fn alloc_set(isa: u64, objects: u64, cnt: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = isa;
    }

    if cnt == 0 || objects == 0 {
        unsafe {
            *((obj as usize + COUNT_OFF) as *mut u64) = 0;
        }
        return obj;
    }

    // Deduplicate: collect unique elements
    let mut unique: Vec<u64> = Vec::with_capacity(cnt as usize);
    for i in 0..cnt as usize {
        let element = unsafe { *((objects as usize + i * 8) as *const u64) };
        let mut dup = false;
        let elem_cstr = cfstring::extract_cfstring_cstr(element);
        for &existing in &unique {
            if existing == element {
                dup = true;
                break;
            }
            if let Some(es) = &elem_cstr {
                if let Some(xs) = cfstring::extract_cfstring_cstr(existing) {
                    if xs == *es {
                        dup = true;
                        break;
                    }
                }
            }
        }
        if !dup {
            unique.push(element);
        }
    }

    let unique_count = unique.len();
    let buf = crate::pal_alloc().malloc(unique_count * 8) as u64;
    if buf != 0 {
        for (i, &elem) in unique.iter().enumerate() {
            unsafe {
                *((buf as usize + i * 8) as *mut u64) = elem;
            }
        }
    }
    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = unique_count as u64;
        *((obj as usize + ELEMS_OFF) as *mut u64) = buf;
    }

    log::debug!(
        "NSSet alloc → {:#x} ({} unique from {} input)",
        obj,
        unique_count,
        cnt
    );
    obj
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSSet setWithObjects:count:]`
pub extern "C" fn shim_nsset_with_objects_count(
    _cls: u64,
    _cmd: u64,
    objects: u64,
    cnt: u64,
) -> u64 {
    alloc_set(nsobj::nsset_class_addr(), objects, cnt)
}

/// `+[NSSet set]` — empty set.
pub extern "C" fn shim_nsset_set(_cls: u64, _cmd: u64) -> u64 {
    alloc_set(nsobj::nsset_class_addr(), 0, 0)
}

/// `+[NSSet setWithArray:]` — create set from NSArray (deduplicating).
pub extern "C" fn shim_nsset_set_with_array(_cls: u64, _cmd: u64, array: u64) -> u64 {
    if array == 0 {
        return alloc_set(nsobj::nsset_class_addr(), 0, 0);
    }
    let count = unsafe { *((array as usize + 8) as *const u64) };
    let elems = unsafe { *((array as usize + 16) as *const u64) };
    alloc_set(nsobj::nsset_class_addr(), elems, count)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSSet count]`
pub extern "C" fn shim_nsset_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    set_count(obj)
}

/// `-[NSSet containsObject:]`
pub extern "C" fn shim_nsset_contains_object(obj: u64, _cmd: u64, needle: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    contains(obj, needle) as u64
}

/// `-[NSSet member:]` — returns the object if found, nil otherwise.
pub extern "C" fn shim_nsset_member(obj: u64, _cmd: u64, needle: u64) -> u64 {
    if obj == 0 || needle == 0 {
        return 0;
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return 0;
    }

    let needle_cstr = cfstring::extract_cfstring_cstr(needle);
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if e == needle {
            return e;
        }
        if let Some(target_s) = &needle_cstr {
            if let Some(elem_s) = cfstring::extract_cfstring_cstr(e) {
                if elem_s == *target_s {
                    return e;
                }
            }
        }
    }
    0
}

/// `-[NSSet allObjects]` — returns an NSArray of all elements.
pub extern "C" fn shim_nsset_all_objects(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = set_count(obj);
    let elems = set_elems(obj);
    nsarray::shim_nsarray_array_with_objects_count(nsobj::nsarray_class_addr(), 0, elems, count)
}

/// `-[NSSet anyObject]` — returns an arbitrary element, or nil if empty.
pub extern "C" fn shim_nsset_any_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = set_count(obj);
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return 0;
    }
    unsafe { *(elems as *const u64) }
}

/// `-[NSSet isSubsetOfSet:]`
pub extern "C" fn shim_nsset_is_subset_of_set(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 {
        return 1;
    } // empty set is subset of everything
    if other == 0 {
        return if set_count(obj) == 0 { 1 } else { 0 };
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 {
        return 1;
    }
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if !contains(other, e) {
            return 0;
        }
    }
    1
}

/// `-[NSSet intersectsSet:]`
pub extern "C" fn shim_nsset_intersects_set(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 || other == 0 {
        return 0;
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 {
        return 0;
    }
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        if contains(other, e) {
            return 1;
        }
    }
    0
}

/// `-[NSSet isEqualToSet:]`
pub extern "C" fn shim_nsset_is_equal_to_set(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == other {
        return 1;
    }
    if obj == 0 || other == 0 {
        return 0;
    }
    if set_count(obj) != set_count(other) {
        return 0;
    }
    // Every element in self is in other (counts equal, so this suffices)
    shim_nsset_is_subset_of_set(obj, 0, other)
}

/// `-[NSSet description]`
pub extern "C" fn shim_nsset_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    let mut parts = Vec::new();
    for i in 0..count {
        let e = if elems != 0 {
            unsafe { *((elems as usize + i * 8) as *const u64) }
        } else {
            0
        };
        let s = cfstring::extract_cfstring_cstr(e)
            .map(|s| format!("\"{}\"", s))
            .unwrap_or_else(|| format!("{:#x}", e));
        parts.push(s);
    }
    let desc = format!("{{({})}}", parts.join(", "));
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSSet enumerateObjectsUsingBlock:]`
pub extern "C" fn shim_nsset_enumerate_objects_using_block(obj: u64, _cmd: u64, block: u64) {
    if obj == 0 || block == 0 {
        return;
    }
    let count = set_count(obj) as usize;
    let elems = set_elems(obj);
    if count == 0 || elems == 0 {
        return;
    }

    let invoke: extern "C" fn(u64, u64, *mut u8) =
        unsafe { std::mem::transmute(*((block as usize + 16) as *const u64)) };

    let mut stop: u8 = 0;
    for i in 0..count {
        let e = unsafe { *((elems as usize + i * 8) as *const u64) };
        invoke(block, e, &mut stop);
        if stop != 0 {
            break;
        }
    }
}
