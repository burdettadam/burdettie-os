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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn empty_set_count_zero() {
        let set = shim_nsset_set(0, 0);
        assert_ne!(set, 0);
        assert_eq!(shim_nsset_count(set, 0), 0);
        free_obj(set);
    }

    #[test]
    fn set_with_objects_count_basic() {
        let a = make_string("alpha");
        let b = make_string("beta");
        let c = make_string("gamma");
        let buf = make_u64_array(&[a, b, c]);
        let set = shim_nsset_with_objects_count(0, 0, buf, 3);
        assert_ne!(set, 0);
        assert_eq!(shim_nsset_count(set, 0), 3);
        free_buf(buf);
        free_obj(set);
    }

    #[test]
    fn deduplication_removes_string_duplicates() {
        let a = make_string("dup");
        let b = make_string("dup"); // different pointer, same content
        let buf = make_u64_array(&[a, b]);
        let set = shim_nsset_with_objects_count(0, 0, buf, 2);
        assert_eq!(shim_nsset_count(set, 0), 1);
        free_buf(buf);
        free_obj(set);
    }

    #[test]
    fn contains_object_true_and_false() {
        let a = make_string("apple");
        let b = make_string("banana");
        let c = make_string("cherry");
        let buf = make_u64_array(&[a, b]);
        let set = shim_nsset_with_objects_count(0, 0, buf, 2);
        assert_eq!(shim_nsset_contains_object(set, 0, a), 1);
        assert_eq!(shim_nsset_contains_object(set, 0, b), 1);
        assert_eq!(shim_nsset_contains_object(set, 0, c), 0);
        free_buf(buf);
        free_obj(set);
    }

    #[test]
    fn member_returns_element_or_nil() {
        let a = make_string("hello");
        let b = make_string("world");
        let buf = make_u64_array(&[a]);
        let set = shim_nsset_with_objects_count(0, 0, buf, 1);
        assert_ne!(shim_nsset_member(set, 0, a), 0);
        assert_eq!(shim_nsset_member(set, 0, b), 0);
        free_buf(buf);
        free_obj(set);
    }

    #[test]
    fn any_object_returns_element_or_nil() {
        let a = make_string("item");
        let buf = make_u64_array(&[a]);
        let set = shim_nsset_with_objects_count(0, 0, buf, 1);
        assert_ne!(shim_nsset_any_object(set, 0), 0);
        let empty = shim_nsset_set(0, 0);
        assert_eq!(shim_nsset_any_object(empty, 0), 0);
        free_buf(buf);
        free_obj(set);
        free_obj(empty);
    }

    #[test]
    fn is_subset_of_set() {
        let x = make_string("x");
        let y = make_string("y");
        let z = make_string("z");
        let buf_xy = make_u64_array(&[x, y]);
        let buf_xyz = make_u64_array(&[x, y, z]);
        let sub = shim_nsset_with_objects_count(0, 0, buf_xy, 2);
        let sup = shim_nsset_with_objects_count(0, 0, buf_xyz, 3);
        assert_eq!(shim_nsset_is_subset_of_set(sub, 0, sup), 1);
        assert_eq!(shim_nsset_is_subset_of_set(sup, 0, sub), 0);
        free_buf(buf_xy);
        free_buf(buf_xyz);
        free_obj(sub);
        free_obj(sup);
    }

    #[test]
    fn intersects_set_shared_element() {
        let shared = make_string("shared");
        let left = make_string("left-only");
        let right = make_string("right-only");
        let buf1 = make_u64_array(&[shared, left]);
        let buf2 = make_u64_array(&[shared, right]);
        let s1 = shim_nsset_with_objects_count(0, 0, buf1, 2);
        let s2 = shim_nsset_with_objects_count(0, 0, buf2, 2);
        assert_eq!(shim_nsset_intersects_set(s1, 0, s2), 1);
        free_buf(buf1);
        free_buf(buf2);
        free_obj(s1);
        free_obj(s2);
    }

    #[test]
    fn is_equal_to_set_same_pointers() {
        let a = make_string("foo");
        let b = make_string("bar");
        let buf1 = make_u64_array(&[a, b]);
        let buf2 = make_u64_array(&[a, b]);
        let s1 = shim_nsset_with_objects_count(0, 0, buf1, 2);
        let s2 = shim_nsset_with_objects_count(0, 0, buf2, 2);
        assert_eq!(shim_nsset_is_equal_to_set(s1, 0, s2), 1);
        free_buf(buf1);
        free_buf(buf2);
        free_obj(s1);
        free_obj(s2);
    }

    #[test]
    fn description_returns_nonzero() {
        let a = make_string("item1");
        let buf = make_u64_array(&[a]);
        let set = shim_nsset_with_objects_count(0, 0, buf, 1);
        assert_ne!(shim_nsset_description(set, 0), 0);
        free_buf(buf);
        free_obj(set);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsset_count(0, 0), 0);
        assert_eq!(shim_nsset_contains_object(0, 0, 0), 0);
        assert_eq!(shim_nsset_any_object(0, 0), 0);
        assert_eq!(shim_nsset_description(0, 0), 0);
    }
}
