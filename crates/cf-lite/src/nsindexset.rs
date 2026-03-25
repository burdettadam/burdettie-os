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
    u8::from(read_indexes(obj).binary_search(&index).is_ok())
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
