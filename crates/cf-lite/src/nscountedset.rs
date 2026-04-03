//! NSCountedSet shim: bag/multiset — set with per-element occurrence counts.
//!
//! Object layout (32 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: count        (u64 — number of unique elements)
//!   offset 16: entries_ptr  (u64 → malloc'd Entry[] array)
//!   offset 24: capacity     (u64)
//!
//! Entry layout (16 bytes each):
//!   offset  0: element      (u64)
//!   offset  8: occurrences  (u64)

use crate::cfstring;
use crate::nsobj;

const COUNT_OFF: usize = 8;
const ENTRIES_OFF: usize = 16;
const CAP_OFF: usize = 24;
const ENTRY_SIZE: usize = 16;
const INITIAL_CAP: u64 = 4;

fn elements_equal(a: u64, b: u64) -> bool {
    if a == b { return true; }
    if let (Some(sa), Some(sb)) = (
        cfstring::extract_cfstring_cstr(a),
        cfstring::extract_cfstring_cstr(b),
    ) {
        return sa == sb;
    }
    false
}

fn find_entry(entries: u64, count: u64, target: u64) -> Option<usize> {
    for i in 0..count as usize {
        let elem = unsafe { *((entries as usize + i * ENTRY_SIZE) as *const u64) };
        if elements_equal(elem, target) { return Some(i); }
    }
    None
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSCountedSet set]`
pub extern "C" fn shim_nscountedset_set(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nscountedset_class_addr(); }
    let buf = crate::pal_alloc().calloc(INITIAL_CAP as usize, ENTRY_SIZE) as u64;
    unsafe {
        *((obj as usize + ENTRIES_OFF) as *mut u64) = buf;
        *((obj as usize + CAP_OFF) as *mut u64) = INITIAL_CAP;
    }
    obj
}

/// `+[NSCountedSet setWithArray:]`
pub extern "C" fn shim_nscountedset_set_with_array(_cls: u64, _cmd: u64, array: u64) -> u64 {
    let obj = shim_nscountedset_set(_cls, _cmd);
    if obj == 0 || array == 0 { return obj; }
    let n = crate::nsarray::shim_nsarray_count(array, 0);
    for i in 0..n {
        let elem = crate::nsarray::shim_nsarray_object_at_index(array, 0, i);
        shim_nscountedset_add_object(obj, 0, elem);
    }
    obj
}

// ── Instance methods ───────────────────────────────────────────────

unsafe fn grow_if_needed(obj: u64) {
    let count = *((obj as usize + COUNT_OFF) as *const u64);
    let cap = *((obj as usize + CAP_OFF) as *const u64);
    if count < cap { return; }
    let new_cap = if cap == 0 { INITIAL_CAP } else { cap * 2 };
    let old_buf = *((obj as usize + ENTRIES_OFF) as *const u64);
    let new_buf = crate::pal_alloc().realloc(old_buf as *mut u8, new_cap as usize * ENTRY_SIZE) as u64;
    if new_buf != 0 {
        let start = (new_buf as usize + count as usize * ENTRY_SIZE) as *mut u8;
        std::ptr::write_bytes(start, 0, (new_cap - count) as usize * ENTRY_SIZE);
        *((obj as usize + ENTRIES_OFF) as *mut u64) = new_buf;
        *((obj as usize + CAP_OFF) as *mut u64) = new_cap;
    }
}

/// `-[NSCountedSet addObject:]`
pub extern "C" fn shim_nscountedset_add_object(obj: u64, _cmd: u64, element: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let entries = unsafe { *((obj as usize + ENTRIES_OFF) as *const u64) };

    if let Some(idx) = find_entry(entries, count, element) {
        // Increment occurrence count
        let occ_ptr = (entries as usize + idx * ENTRY_SIZE + 8) as *mut u64;
        unsafe { *occ_ptr += 1; }
    } else {
        // New entry
        unsafe { grow_if_needed(obj); }
        let entries = unsafe { *((obj as usize + ENTRIES_OFF) as *const u64) };
        let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
        unsafe {
            let base = entries as usize + count as usize * ENTRY_SIZE;
            *(base as *mut u64) = element;
            *((base + 8) as *mut u64) = 1;
            *((obj as usize + COUNT_OFF) as *mut u64) = count + 1;
        }
    }
    0
}

/// `-[NSCountedSet removeObject:]`
pub extern "C" fn shim_nscountedset_remove_object(obj: u64, _cmd: u64, element: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let entries = unsafe { *((obj as usize + ENTRIES_OFF) as *const u64) };

    if let Some(idx) = find_entry(entries, count, element) {
        let occ_ptr = (entries as usize + idx * ENTRY_SIZE + 8) as *mut u64;
        let occ = unsafe { *occ_ptr };
        if occ > 1 {
            unsafe { *occ_ptr = occ - 1; }
        } else {
            // Remove entry by shifting
            let remaining = count as usize - idx - 1;
            if remaining > 0 {
                unsafe {
                    let src = (entries as usize + (idx + 1) * ENTRY_SIZE) as *const u8;
                    let dst = (entries as usize + idx * ENTRY_SIZE) as *mut u8;
                    std::ptr::copy(src, dst, remaining * ENTRY_SIZE);
                }
            }
            unsafe { *((obj as usize + COUNT_OFF) as *mut u64) = count - 1; }
        }
    }
    0
}

/// `-[NSCountedSet count]` — number of unique elements.
pub extern "C" fn shim_nscountedset_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

/// `-[NSCountedSet countForObject:]`
pub extern "C" fn shim_nscountedset_count_for_object(obj: u64, _cmd: u64, element: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let entries = unsafe { *((obj as usize + ENTRIES_OFF) as *const u64) };
    if let Some(idx) = find_entry(entries, count, element) {
        unsafe { *((entries as usize + idx * ENTRY_SIZE + 8) as *const u64) }
    } else {
        0
    }
}

/// `-[NSCountedSet containsObject:]`
pub extern "C" fn shim_nscountedset_contains_object(obj: u64, _cmd: u64, element: u64) -> u8 {
    u8::from(shim_nscountedset_count_for_object(obj, 0, element) > 0)
}

/// `-[NSCountedSet allObjects]` — returns an NSArray of unique elements.
pub extern "C" fn shim_nscountedset_all_objects(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) } as usize;
    let entries = unsafe { *((obj as usize + ENTRIES_OFF) as *const u64) };
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        items.push(unsafe { *((entries as usize + i * ENTRY_SIZE) as *const u64) });
    }
    crate::nsarray::make_nsarray(&items)
}

/// `-[NSCountedSet description]`
pub extern "C" fn shim_nscountedset_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let desc = format!("<NSCountedSet: {} unique objects>", count);
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── Helper: create an empty NSCountedSet via the class method ───

    fn new_set() -> u64 {
        shim_nscountedset_set(0, 0)
    }

    // ── shim_nscountedset_set ───────────────────────────────────────

    #[test]
    fn set_creates_valid_object() {
        let s = new_set();
        assert_ne!(s, 0);
        assert_eq!(read_field(s, 0), nsobj::nscountedset_class_addr());
        assert_eq!(read_field(s, COUNT_OFF), 0); // count == 0
        assert_ne!(read_field(s, ENTRIES_OFF), 0); // entries buffer allocated
        assert_eq!(read_field(s, CAP_OFF), INITIAL_CAP);
        free_obj(s);
    }

    // ── shim_nscountedset_count ─────────────────────────────────────

    #[test]
    fn count_nil_returns_zero() {
        assert_eq!(shim_nscountedset_count(0, 0), 0);
    }

    #[test]
    fn count_empty_set() {
        let s = new_set();
        assert_eq!(shim_nscountedset_count(s, 0), 0);
        free_obj(s);
    }

    // ── shim_nscountedset_add_object ────────────────────────────────

    #[test]
    fn add_object_nil_set() {
        // Must not crash
        assert_eq!(shim_nscountedset_add_object(0, 0, 0x42), 0);
    }

    #[test]
    fn add_unique_elements_increments_count() {
        let s = new_set();
        let a = make_string("alpha");
        let b = make_string("beta");

        shim_nscountedset_add_object(s, 0, a);
        assert_eq!(shim_nscountedset_count(s, 0), 1);

        shim_nscountedset_add_object(s, 0, b);
        assert_eq!(shim_nscountedset_count(s, 0), 2);

        free_obj(s);
    }

    #[test]
    fn add_duplicate_does_not_increase_unique_count() {
        let s = new_set();
        let a = make_string("dup");

        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_add_object(s, 0, a);

        assert_eq!(shim_nscountedset_count(s, 0), 1);
        assert_eq!(shim_nscountedset_count_for_object(s, 0, a), 3);

        free_obj(s);
    }

    #[test]
    fn add_duplicate_via_equal_strings() {
        let s = new_set();
        let a = make_string("same");
        let b = make_string("same"); // different pointer, same content

        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_add_object(s, 0, b);

        assert_eq!(shim_nscountedset_count(s, 0), 1);
        assert_eq!(shim_nscountedset_count_for_object(s, 0, a), 2);

        free_obj(s);
    }

    #[test]
    fn add_beyond_initial_capacity_triggers_growth() {
        let s = new_set();
        // INITIAL_CAP is 4; add 6 unique elements
        for i in 0..6 {
            let elem = make_string(&format!("item{i}"));
            shim_nscountedset_add_object(s, 0, elem);
        }
        assert_eq!(shim_nscountedset_count(s, 0), 6);
        assert!(read_field(s, CAP_OFF) >= 6); // capacity grew
        free_obj(s);
    }

    // ── shim_nscountedset_remove_object ─────────────────────────────

    #[test]
    fn remove_object_nil_set() {
        assert_eq!(shim_nscountedset_remove_object(0, 0, 0x42), 0);
    }

    #[test]
    fn remove_decrements_occurrence() {
        let s = new_set();
        let a = make_string("hello");

        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_add_object(s, 0, a);
        assert_eq!(shim_nscountedset_count_for_object(s, 0, a), 3);

        shim_nscountedset_remove_object(s, 0, a);
        assert_eq!(shim_nscountedset_count_for_object(s, 0, a), 2);
        assert_eq!(shim_nscountedset_count(s, 0), 1); // still 1 unique

        free_obj(s);
    }

    #[test]
    fn remove_last_occurrence_removes_entry() {
        let s = new_set();
        let a = make_string("gone");

        shim_nscountedset_add_object(s, 0, a);
        assert_eq!(shim_nscountedset_count(s, 0), 1);

        shim_nscountedset_remove_object(s, 0, a);
        assert_eq!(shim_nscountedset_count(s, 0), 0);
        assert_eq!(shim_nscountedset_count_for_object(s, 0, a), 0);

        free_obj(s);
    }

    #[test]
    fn remove_nonexistent_element_is_noop() {
        let s = new_set();
        let a = make_string("exists");
        let b = make_string("ghost");

        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_remove_object(s, 0, b); // not in set

        assert_eq!(shim_nscountedset_count(s, 0), 1);
        free_obj(s);
    }

    #[test]
    fn remove_shifts_remaining_entries() {
        let s = new_set();
        let a = make_string("first");
        let b = make_string("second");
        let c = make_string("third");

        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_add_object(s, 0, b);
        shim_nscountedset_add_object(s, 0, c);

        // Remove the middle element
        shim_nscountedset_remove_object(s, 0, b);
        assert_eq!(shim_nscountedset_count(s, 0), 2);
        assert_eq!(shim_nscountedset_contains_object(s, 0, a), 1);
        assert_eq!(shim_nscountedset_contains_object(s, 0, b), 0);
        assert_eq!(shim_nscountedset_contains_object(s, 0, c), 1);

        free_obj(s);
    }

    // ── shim_nscountedset_count_for_object ──────────────────────────

    #[test]
    fn count_for_object_nil_set() {
        assert_eq!(shim_nscountedset_count_for_object(0, 0, 0x42), 0);
    }

    #[test]
    fn count_for_object_not_present() {
        let s = new_set();
        let a = make_string("nope");
        assert_eq!(shim_nscountedset_count_for_object(s, 0, a), 0);
        free_obj(s);
    }

    #[test]
    fn count_for_object_tracks_occurrences() {
        let s = new_set();
        let a = make_string("many");

        for _ in 0..5 {
            shim_nscountedset_add_object(s, 0, a);
        }
        assert_eq!(shim_nscountedset_count_for_object(s, 0, a), 5);

        free_obj(s);
    }

    // ── shim_nscountedset_contains_object ───────────────────────────

    #[test]
    fn contains_object_nil_set() {
        assert_eq!(shim_nscountedset_contains_object(0, 0, 0x42), 0);
    }

    #[test]
    fn contains_object_present_and_absent() {
        let s = new_set();
        let a = make_string("here");
        let b = make_string("absent");

        shim_nscountedset_add_object(s, 0, a);

        assert_eq!(shim_nscountedset_contains_object(s, 0, a), 1);
        assert_eq!(shim_nscountedset_contains_object(s, 0, b), 0);

        free_obj(s);
    }

    // ── shim_nscountedset_all_objects ────────────────────────────────

    #[test]
    fn all_objects_nil_set() {
        assert_eq!(shim_nscountedset_all_objects(0, 0), 0);
    }

    #[test]
    fn all_objects_empty_set() {
        let s = new_set();
        let arr = shim_nscountedset_all_objects(s, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
        free_obj(s);
    }

    #[test]
    fn all_objects_returns_unique_elements() {
        let s = new_set();
        let a = make_string("x");
        let b = make_string("y");

        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_add_object(s, 0, a); // dup
        shim_nscountedset_add_object(s, 0, b);

        let arr = shim_nscountedset_all_objects(s, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 2);

        free_obj(s);
    }

    // ── shim_nscountedset_description ───────────────────────────────

    #[test]
    fn description_nil_set() {
        assert_eq!(shim_nscountedset_description(0, 0), 0);
    }

    #[test]
    fn description_empty_set() {
        let s = new_set();
        let desc = shim_nscountedset_description(s, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSCountedSet: 0 unique objects>");
        free_obj(s);
    }

    #[test]
    fn description_with_elements() {
        let s = new_set();
        let a = make_string("a");
        let b = make_string("b");
        shim_nscountedset_add_object(s, 0, a);
        shim_nscountedset_add_object(s, 0, b);

        let desc = shim_nscountedset_description(s, 0);
        assert_string_eq(desc, "<NSCountedSet: 2 unique objects>");
        free_obj(s);
    }

    // ── shim_nscountedset_set_with_array ────────────────────────────

    #[test]
    fn set_with_array_nil_array() {
        let s = shim_nscountedset_set_with_array(0, 0, 0);
        assert_ne!(s, 0);
        assert_eq!(shim_nscountedset_count(s, 0), 0);
        free_obj(s);
    }

    #[test]
    fn set_with_array_populates_counts() {
        let a = make_string("k");
        let b = make_string("k"); // same content, different ptr
        let c = make_string("j");

        let arr = crate::nsarray::make_nsarray(&[a, b, c]);
        let s = shim_nscountedset_set_with_array(0, 0, arr);

        assert_eq!(shim_nscountedset_count(s, 0), 2); // "k" and "j"
        assert_eq!(shim_nscountedset_count_for_object(s, 0, a), 2); // "k" twice
        assert_eq!(shim_nscountedset_count_for_object(s, 0, c), 1);

        free_obj(s);
    }

    #[test]
    fn set_with_empty_array() {
        let arr = crate::nsarray::make_nsarray(&[]);
        let s = shim_nscountedset_set_with_array(0, 0, arr);

        assert_eq!(shim_nscountedset_count(s, 0), 0);
        free_obj(s);
    }
}
