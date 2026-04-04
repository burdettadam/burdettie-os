//! NSIndexSet / NSMutableIndexSet shim: sorted collections of unique unsigned integers.
//!
//! Object layout: 24 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  count (u64) — number of indexes stored
//!   [16..24): data_ptr (u64) — pointer to heap-allocated sorted u64 array
//!
//! Stores indexes in a sorted `Vec<u64>` on the heap. For our use cases
//! (typically small sets), linear storage is sufficient.

use crate::cfstring;
use crate::nsobj;

const NSINDEXSET_SIZE: usize = 24;

/// Allocate an NSIndexSet from a sorted slice of unique indexes.
fn alloc_nsindexset(indexes: &[u64], mutable: bool) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSINDEXSET_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    let isa = if mutable {
        nsobj::nsmutableindexset_class_addr()
    } else {
        nsobj::nsindexset_class_addr()
    };
    let count = indexes.len() as u64;
    let data_ptr = if indexes.is_empty() {
        std::ptr::null_mut::<u64>()
    } else {
        let buf = crate::pal_alloc().malloc(indexes.len() * 8) as *mut u64;
        if !buf.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(indexes.as_ptr(), buf, indexes.len());
            }
        }
        buf
    };
    unsafe {
        *obj = isa;
        *obj.add(1) = count;
        *obj.add(2) = data_ptr as u64;
    }
    obj as u64
}

/// Read stored indexes as a slice.
fn read_indexes(obj: u64) -> &'static [u64] {
    let count = unsafe { *((obj as *const u64).add(1)) } as usize;
    let data = unsafe { *((obj as *const u64).add(2)) } as *const u64;
    if count == 0 || data.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, count) }
    }
}

/// Read stored indexes as a mutable Vec, modify, and write back.
fn modify_indexes<F: FnOnce(&mut Vec<u64>)>(obj: u64, f: F) {
    let mut indexes: Vec<u64> = read_indexes(obj).to_vec();
    f(&mut indexes);
    indexes.sort_unstable();
    indexes.dedup();

    // Free old buffer
    let old_ptr = unsafe { *((obj as *const u64).add(2)) } as *mut u8;
    if !old_ptr.is_null() {
        crate::pal_alloc().free(old_ptr);
    }

    let count = indexes.len() as u64;
    let new_ptr = if indexes.is_empty() {
        std::ptr::null_mut::<u64>()
    } else {
        let buf = crate::pal_alloc().malloc(indexes.len() * 8) as *mut u64;
        if !buf.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(indexes.as_ptr(), buf, indexes.len());
            }
        }
        buf
    };
    unsafe {
        *((obj as *mut u64).add(1)) = count;
        *((obj as *mut u64).add(2)) = new_ptr as u64;
    }
}

// ── NSIndexSet class methods ───────────────────────────────────────

/// `+[NSIndexSet indexSet]` — empty index set.
pub extern "C" fn shim_nsindexset_index_set(_cls: u64, _cmd: u64) -> u64 {
    alloc_nsindexset(&[], false)
}

/// `+[NSIndexSet indexSetWithIndex:]`
pub extern "C" fn shim_nsindexset_with_index(_cls: u64, _cmd: u64, index: u64) -> u64 {
    alloc_nsindexset(&[index], false)
}

/// `+[NSIndexSet indexSetWithIndexesInRange:]`
///
/// NSRange passed as two u64 args: location, length.
pub extern "C" fn shim_nsindexset_with_range(
    _cls: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) -> u64 {
    let indexes: Vec<u64> = (location..location.saturating_add(length)).collect();
    alloc_nsindexset(&indexes, false)
}

// ── NSIndexSet instance methods ────────────────────────────────────

/// `-[NSIndexSet count]`
pub extern "C" fn shim_nsindexset_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(1)) }
}

/// `-[NSIndexSet containsIndex:]`
pub extern "C" fn shim_nsindexset_contains_index(obj: u64, _cmd: u64, index: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let idxs = read_indexes(obj);
    let result = u8::from(idxs.binary_search(&index).is_ok());
    eprintln!(
        "[containsIndex] obj={:#x} index={} → {} (set has {} entries: {:?})",
        obj, index, result, idxs.len(), &idxs[..idxs.len().min(8)]
    );
    result
}

/// `-[NSIndexSet firstIndex]` — returns `NSNotFound` (== `NSIntegerMax`) if empty.
pub extern "C" fn shim_nsindexset_first_index(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return u64::MAX;
    }
    let idxs = read_indexes(obj);
    if idxs.is_empty() {
        u64::MAX
    } else {
        idxs[0]
    }
}

/// `-[NSIndexSet lastIndex]`
pub extern "C" fn shim_nsindexset_last_index(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return u64::MAX;
    }
    let idxs = read_indexes(obj);
    if idxs.is_empty() {
        u64::MAX
    } else {
        idxs[idxs.len() - 1]
    }
}

/// `-[NSIndexSet containsIndexesInRange:]` — range passed as (location, length).
pub extern "C" fn shim_nsindexset_contains_indexes_in_range(
    obj: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) -> u8 {
    if obj == 0 || length == 0 {
        return 0;
    }
    let idxs = read_indexes(obj);
    for i in location..location.saturating_add(length) {
        if idxs.binary_search(&i).is_err() {
            return 0;
        }
    }
    1
}

/// `-[NSIndexSet description]`
pub extern "C" fn shim_nsindexset_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let idxs = read_indexes(obj);
    let count = idxs.len();
    let desc = if count <= 10 {
        let items: Vec<String> = idxs.iter().map(|i| i.to_string()).collect();
        format!(
            "[number of indexes: {count}, indexes: ({})]\0",
            items.join(" ")
        )
    } else {
        format!("[number of indexes: {count}]\0")
    };
    let c_str = std::ffi::CString::new(desc.trim_end_matches('\0')).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSIndexSet addIndex:]`
///
/// Foundation frequently uses private mutable subclasses behind the public
/// `NSIndexSet` API surface. Allowing the base class to accept the mutable
/// selector keeps those call paths working with our simplified class model.
pub extern "C" fn shim_nsindexset_add_index(obj: u64, _cmd: u64, index: u64) {
    shim_nsmutableindexset_add_index(obj, _cmd, index);
}

/// `-[NSIndexSet addIndexesInRange:]`
pub extern "C" fn shim_nsindexset_add_indexes_in_range(
    obj: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) {
    shim_nsmutableindexset_add_indexes_in_range(obj, _cmd, location, length);
}

/// `-[NSIndexSet removeIndex:]`
pub extern "C" fn shim_nsindexset_remove_index(obj: u64, _cmd: u64, index: u64) {
    shim_nsmutableindexset_remove_index(obj, _cmd, index);
}

/// `-[NSIndexSet removeAllIndexes]`
pub extern "C" fn shim_nsindexset_remove_all(obj: u64, _cmd: u64) {
    shim_nsmutableindexset_remove_all(obj, _cmd);
}

// ── NSMutableIndexSet class methods ────────────────────────────────

/// `+[NSMutableIndexSet indexSet]` — empty mutable index set.
pub extern "C" fn shim_nsmutableindexset_index_set(_cls: u64, _cmd: u64) -> u64 {
    alloc_nsindexset(&[], true)
}

// ── NSMutableIndexSet instance methods ─────────────────────────────

/// `-[NSMutableIndexSet addIndex:]`
pub extern "C" fn shim_nsmutableindexset_add_index(obj: u64, _cmd: u64, index: u64) {
    if obj == 0 {
        return;
    }
    modify_indexes(obj, |v| {
        if v.binary_search(&index).is_err() {
            v.push(index);
        }
    });
}

/// `-[NSMutableIndexSet addIndexesInRange:]` — range as (location, length).
pub extern "C" fn shim_nsmutableindexset_add_indexes_in_range(
    obj: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) {
    if obj == 0 {
        return;
    }
    modify_indexes(obj, |v| {
        for i in location..location.saturating_add(length) {
            if v.binary_search(&i).is_err() {
                v.push(i);
            }
        }
    });
}

/// `-[NSMutableIndexSet removeIndex:]`
pub extern "C" fn shim_nsmutableindexset_remove_index(obj: u64, _cmd: u64, index: u64) {
    if obj == 0 {
        return;
    }
    modify_indexes(obj, |v| {
        if let Ok(pos) = v.binary_search(&index) {
            v.remove(pos);
        }
    });
}

/// `-[NSMutableIndexSet removeAllIndexes]`
pub extern "C" fn shim_nsmutableindexset_remove_all(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    modify_indexes(obj, |v| v.clear());
}

/// `-[NSMutableIndexSet init]` — returns self.
pub extern "C" fn shim_nsmutableindexset_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSIndexSet enumerateIndexesUsingBlock:]`
///
/// Block signature: `void (^)(NSUInteger idx, BOOL *stop)`
pub extern "C" fn shim_nsindexset_enumerate_indexes_using_block(obj: u64, _cmd: u64, block: u64) {
    if obj == 0 || block == 0 {
        return;
    }
    let invoke = unsafe { *((block as *const u64).add(2)) };
    if invoke == 0 {
        return;
    }
    let invoke_fn: extern "C" fn(u64, u64, *mut u8) =
        unsafe { std::mem::transmute(invoke) };
    let idxs = read_indexes(obj);
    let mut stop: u8 = 0;
    for &idx in idxs {
        invoke_fn(block, idx, &mut stop);
        if stop != 0 {
            break;
        }
    }
}

/// `-[NSIndexSet indexesPassingTest:]`
///
/// Block signature: `BOOL (^)(NSUInteger idx, BOOL *stop)`
/// Returns an NSIndexSet containing the indexes for which the block returns true.
pub extern "C" fn shim_nsindexset_indexes_passing_test(obj: u64, _cmd: u64, block: u64) -> u64 {
    if obj == 0 || block == 0 {
        return alloc_nsindexset(&[], false);
    }
    let invoke = unsafe { *((block as *const u64).add(2)) };
    if invoke == 0 {
        return alloc_nsindexset(&[], false);
    }
    let invoke_fn: extern "C" fn(u64, u64, *mut u8) -> u8 =
        unsafe { std::mem::transmute(invoke) };
    let idxs = read_indexes(obj);
    let mut result = Vec::new();
    let mut stop: u8 = 0;
    for &idx in idxs {
        let passes = invoke_fn(block, idx, &mut stop);
        if passes != 0 {
            result.push(idx);
        }
        if stop != 0 {
            break;
        }
    }
    alloc_nsindexset(&result, false)
}

/// `-[NSIndexSet countOfIndexesInRange:]`
///
/// Counts how many indexes fall within the given range (location, length).
pub extern "C" fn shim_nsindexset_count_of_indexes_in_range(
    obj: u64,
    _cmd: u64,
    range_loc: u64,
    range_len: u64,
) -> u64 {
    if obj == 0 || range_len == 0 {
        return 0;
    }
    let idxs = read_indexes(obj);
    let end = range_loc.saturating_add(range_len);
    idxs.iter().filter(|&&i| i >= range_loc && i < end).count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn empty_index_set() {
        let set = shim_nsindexset_index_set(0, 0);
        assert_ne!(set, 0);
        assert_eq!(shim_nsindexset_count(set, 0), 0);
        assert_eq!(shim_nsindexset_first_index(set, 0), u64::MAX);
        assert_eq!(shim_nsindexset_last_index(set, 0), u64::MAX);
        free_obj(set);
    }

    #[test]
    fn single_index() {
        let set = shim_nsindexset_with_index(0, 0, 42);
        assert_eq!(shim_nsindexset_count(set, 0), 1);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 42), 1);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 99), 0);
        assert_eq!(shim_nsindexset_first_index(set, 0), 42);
        assert_eq!(shim_nsindexset_last_index(set, 0), 42);
        free_obj(set);
    }

    #[test]
    fn range() {
        let set = shim_nsindexset_with_range(0, 0, 5, 3); // {5,6,7}
        assert_eq!(shim_nsindexset_count(set, 0), 3);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 4), 0);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 5), 1);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 7), 1);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 8), 0);
        assert_eq!(shim_nsindexset_first_index(set, 0), 5);
        assert_eq!(shim_nsindexset_last_index(set, 0), 7);
        free_obj(set);
    }

    #[test]
    fn contains_indexes_in_range() {
        let set = shim_nsindexset_with_range(0, 0, 10, 5); // {10,11,12,13,14}
        assert_eq!(shim_nsindexset_contains_indexes_in_range(set, 0, 10, 3), 1);
        assert_eq!(shim_nsindexset_contains_indexes_in_range(set, 0, 10, 5), 1);
        assert_eq!(shim_nsindexset_contains_indexes_in_range(set, 0, 10, 6), 0);
        assert_eq!(shim_nsindexset_contains_indexes_in_range(set, 0, 9, 2), 0);
        free_obj(set);
    }

    #[test]
    fn mutable_add_remove() {
        let set = shim_nsmutableindexset_index_set(0, 0);
        shim_nsmutableindexset_add_index(set, 0, 10);
        shim_nsmutableindexset_add_index(set, 0, 5);
        shim_nsmutableindexset_add_index(set, 0, 20);
        assert_eq!(shim_nsindexset_count(set, 0), 3);
        assert_eq!(shim_nsindexset_first_index(set, 0), 5);
        assert_eq!(shim_nsindexset_last_index(set, 0), 20);

        shim_nsmutableindexset_remove_index(set, 0, 10);
        assert_eq!(shim_nsindexset_count(set, 0), 2);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 10), 0);
        free_obj(set);
    }

    #[test]
    fn mutable_add_range() {
        let set = shim_nsmutableindexset_index_set(0, 0);
        shim_nsmutableindexset_add_indexes_in_range(set, 0, 100, 3);
        assert_eq!(shim_nsindexset_count(set, 0), 3);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 100), 1);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 102), 1);
        free_obj(set);
    }

    #[test]
    fn mutable_remove_all() {
        let set = shim_nsmutableindexset_index_set(0, 0);
        shim_nsmutableindexset_add_index(set, 0, 1);
        shim_nsmutableindexset_add_index(set, 0, 2);
        shim_nsmutableindexset_remove_all(set, 0);
        assert_eq!(shim_nsindexset_count(set, 0), 0);
        free_obj(set);
    }

    #[test]
    fn mutable_dedup() {
        let set = shim_nsmutableindexset_index_set(0, 0);
        shim_nsmutableindexset_add_index(set, 0, 42);
        shim_nsmutableindexset_add_index(set, 0, 42);
        assert_eq!(shim_nsindexset_count(set, 0), 1);
        free_obj(set);
    }

    #[test]
    fn immutable_indexset_accepts_mutating_selectors_for_cluster_compat() {
        let set = shim_nsindexset_index_set(0, 0);
        shim_nsindexset_add_index(set, 0, 7);
        shim_nsindexset_add_indexes_in_range(set, 0, 10, 2);
        assert_eq!(shim_nsindexset_count(set, 0), 3);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 7), 1);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 10), 1);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 11), 1);

        shim_nsindexset_remove_index(set, 0, 10);
        assert_eq!(shim_nsindexset_contains_index(set, 0, 10), 0);

        shim_nsindexset_remove_all(set, 0);
        assert_eq!(shim_nsindexset_count(set, 0), 0);
        free_obj(set);
    }

    #[test]
    fn nil_operations_safe() {
        assert_eq!(shim_nsindexset_count(0, 0), 0);
        assert_eq!(shim_nsindexset_first_index(0, 0), u64::MAX);
        assert_eq!(shim_nsindexset_contains_index(0, 0, 1), 0);
    }

    #[test]
    fn count_of_indexes_in_range_basic() {
        let set = shim_nsindexset_with_range(0, 0, 10, 5); // {10,11,12,13,14}
        assert_eq!(shim_nsindexset_count_of_indexes_in_range(set, 0, 10, 3), 3);
        assert_eq!(shim_nsindexset_count_of_indexes_in_range(set, 0, 10, 5), 5);
        assert_eq!(shim_nsindexset_count_of_indexes_in_range(set, 0, 12, 10), 3);
        assert_eq!(shim_nsindexset_count_of_indexes_in_range(set, 0, 0, 5), 0);
        free_obj(set);
    }

    #[test]
    fn count_of_indexes_in_range_nil_safe() {
        assert_eq!(shim_nsindexset_count_of_indexes_in_range(0, 0, 0, 10), 0);
    }

    #[test]
    fn enumerate_indexes_using_block() {
        let set = shim_nsindexset_with_range(0, 0, 5, 3); // {5,6,7}
        // We'll use a fake block struct to verify the enumerate logic.
        // Block layout: isa(0), flags(8), invoke(16), ...
        // We need a real callable function pointer at offset 16.
        static mut COLLECTED: Vec<u64> = Vec::new();

        extern "C" fn test_block_invoke(block: u64, idx: u64, stop: *mut u8) {
            let _ = block;
            let _ = stop;
            unsafe {
                COLLECTED.push(idx);
            }
        }

        let block = crate::pal_alloc().calloc(1, 24) as *mut u64;
        unsafe {
            *block.add(2) = test_block_invoke as *const () as u64;
            COLLECTED.clear();
        }
        shim_nsindexset_enumerate_indexes_using_block(set, 0, block as u64);
        let collected = unsafe { COLLECTED.clone() };
        assert_eq!(collected, vec![5, 6, 7]);
        free_obj(set);
    }

    #[test]
    fn enumerate_indexes_nil_safe() {
        shim_nsindexset_enumerate_indexes_using_block(0, 0, 0); // must not crash
    }

    #[test]
    fn indexes_passing_test() {
        // Build a set {1, 2, 3, 4, 5}
        let set = shim_nsindexset_with_range(0, 0, 1, 5);

        // Block that returns true for even indexes
        extern "C" fn even_block(_block: u64, idx: u64, _stop: *mut u8) -> u8 {
            u8::from(idx % 2 == 0)
        }

        let block = crate::pal_alloc().calloc(1, 24) as *mut u64;
        unsafe {
            *block.add(2) = even_block as *const () as u64;
        }

        let result = shim_nsindexset_indexes_passing_test(set, 0, block as u64);
        assert_eq!(shim_nsindexset_count(result, 0), 2);
        assert_eq!(shim_nsindexset_contains_index(result, 0, 2), 1);
        assert_eq!(shim_nsindexset_contains_index(result, 0, 4), 1);
        assert_eq!(shim_nsindexset_contains_index(result, 0, 1), 0);
        free_obj(set);
        free_obj(result);
    }

    #[test]
    fn indexes_passing_test_nil_returns_empty() {
        let result = shim_nsindexset_indexes_passing_test(0, 0, 0);
        assert_eq!(shim_nsindexset_count(result, 0), 0);
        free_obj(result);
    }
}
