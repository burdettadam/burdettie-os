//! NSHashTable shim: set with configurable memory semantics.
//!
//! Object layout (24 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: options      (u64 → NSPointerFunctionsOptions)
//!   offset 16: set_ptr      (u64 → leaked Box<HashSet<u64>>)
//!
//! In the lab environment, weak references are treated as strong.

use std::collections::HashSet;

use crate::cfstring;
use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 24;

type SetStorage = HashSet<u64>;

unsafe fn get_set(obj: u64) -> *mut SetStorage {
    let ptr = *((obj + 16) as *const u64);
    ptr as *mut SetStorage
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSHashTable hashTableWithOptions:]`
pub extern "C" fn shim_hashtable_with_options(_cls: u64, _cmd: u64, options: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nshashtable_class_addr();
        *((obj + 8) as *mut u64) = options;
        *((obj + 16) as *mut u64) = Box::into_raw(Box::new(SetStorage::new())) as u64;
    }
    obj
}

/// `+[NSHashTable weakObjectsHashTable]`
pub extern "C" fn shim_hashtable_weak_objects(_cls: u64, _cmd: u64) -> u64 {
    shim_hashtable_with_options(0, 0, 1) // options=1 → weak (treated as strong)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSHashTable addObject:]`
pub extern "C" fn shim_hashtable_add_object(obj: u64, _cmd: u64, member: u64) {
    if obj == 0 {
        return;
    }
    let set = unsafe { &mut *get_set(obj) };
    set.insert(member);
}

/// `-[NSHashTable removeObject:]`
pub extern "C" fn shim_hashtable_remove_object(obj: u64, _cmd: u64, member: u64) {
    if obj == 0 {
        return;
    }
    let set = unsafe { &mut *get_set(obj) };
    set.remove(&member);
}

/// `-[NSHashTable removeAllObjects]`
pub extern "C" fn shim_hashtable_remove_all(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let set = unsafe { &mut *get_set(obj) };
    set.clear();
}

/// `-[NSHashTable containsObject:]`
pub extern "C" fn shim_hashtable_contains(obj: u64, _cmd: u64, member: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let set = unsafe { &mut *get_set(obj) };
    u8::from(set.contains(&member))
}

/// `-[NSHashTable count]`
pub extern "C" fn shim_hashtable_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let set = unsafe { &mut *get_set(obj) };
    set.len() as u64
}

/// `-[NSHashTable allObjects]`
pub extern "C" fn shim_hashtable_all_objects(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let set = unsafe { &mut *get_set(obj) };
    let items: Vec<u64> = set.iter().copied().collect();
    crate::nsarray::make_nsarray(&items)
}

/// `-[NSHashTable anyObject]` — returns any object from the hash table, or 0 if empty.
pub extern "C" fn shim_hashtable_any_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let set = unsafe { &*get_set(obj) };
    set.iter().next().copied().unwrap_or(0)
}

/// `-[NSHashTable setRepresentation]` — returns an NSSet containing all objects.
pub extern "C" fn shim_hashtable_set_representation(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let set = unsafe { &*get_set(obj) };
    let items: Vec<u64> = set.iter().copied().collect();
    // Build an NSSet from a temporary buffer
    if items.is_empty() {
        return crate::nsset::shim_nsset_set(0, 0);
    }
    let buf = crate::pal_alloc().malloc(items.len() * 8) as u64;
    if buf == 0 {
        return 0;
    }
    for (i, &item) in items.iter().enumerate() {
        unsafe {
            *((buf as usize + i * 8) as *mut u64) = item;
        }
    }
    let result = crate::nsset::shim_nsset_with_objects_count(0, 0, buf, items.len() as u64);
    crate::pal_alloc().free(buf as *mut u8);
    result
}

/// `-[NSHashTable description]`
pub extern "C" fn shim_hashtable_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let set = unsafe { &mut *get_set(obj) };
    let desc = format!("<NSHashTable: {} objects>", set.len());
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    /// Free a hashtable created by shim_hashtable_with_options / shim_hashtable_weak_objects.
    /// Drops the internal HashSet and then frees the object.
    fn free_hashtable(obj: u64) {
        if obj != 0 {
            unsafe {
                let set_ptr = *((obj + 16) as *const u64) as *mut SetStorage;
                if !set_ptr.is_null() {
                    drop(Box::from_raw(set_ptr));
                }
            }
            free_obj(obj);
        }
    }

    // ── Construction ───────────────────────────────────────────────

    #[test]
    fn with_options_returns_valid_obj() {
        let ht = shim_hashtable_with_options(0, 0, 42);
        assert_ne!(ht, 0);
        // ISA should be nshashtable_class_addr
        assert_eq!(read_field(ht, 0), nsobj::nshashtable_class_addr());
        // options stored at offset 8
        assert_eq!(read_field(ht, 8), 42);
        // count starts at 0
        assert_eq!(shim_hashtable_count(ht, 0), 0);
        free_hashtable(ht);
    }

    #[test]
    fn weak_objects_returns_valid_obj() {
        let ht = shim_hashtable_weak_objects(0, 0);
        assert_ne!(ht, 0);
        assert_eq!(read_field(ht, 0), nsobj::nshashtable_class_addr());
        // weak → options = 1
        assert_eq!(read_field(ht, 8), 1);
        assert_eq!(shim_hashtable_count(ht, 0), 0);
        free_hashtable(ht);
    }

    // ── add / contains / count ─────────────────────────────────────

    #[test]
    fn add_and_contains() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 100);
        assert_eq!(shim_hashtable_contains(ht, 0, 100), 1);
        assert_eq!(shim_hashtable_contains(ht, 0, 200), 0);
        assert_eq!(shim_hashtable_count(ht, 0), 1);
        free_hashtable(ht);
    }

    #[test]
    fn add_multiple_objects() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 10);
        shim_hashtable_add_object(ht, 0, 20);
        shim_hashtable_add_object(ht, 0, 30);
        assert_eq!(shim_hashtable_count(ht, 0), 3);
        assert_eq!(shim_hashtable_contains(ht, 0, 10), 1);
        assert_eq!(shim_hashtable_contains(ht, 0, 20), 1);
        assert_eq!(shim_hashtable_contains(ht, 0, 30), 1);
        free_hashtable(ht);
    }

    #[test]
    fn duplicate_add_does_not_increase_count() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 99);
        shim_hashtable_add_object(ht, 0, 99);
        assert_eq!(shim_hashtable_count(ht, 0), 1);
        free_hashtable(ht);
    }

    // ── remove ─────────────────────────────────────────────────────

    #[test]
    fn remove_object() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 1);
        shim_hashtable_add_object(ht, 0, 2);
        shim_hashtable_remove_object(ht, 0, 1);
        assert_eq!(shim_hashtable_contains(ht, 0, 1), 0);
        assert_eq!(shim_hashtable_contains(ht, 0, 2), 1);
        assert_eq!(shim_hashtable_count(ht, 0), 1);
        free_hashtable(ht);
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 5);
        shim_hashtable_remove_object(ht, 0, 999);
        assert_eq!(shim_hashtable_count(ht, 0), 1);
        free_hashtable(ht);
    }

    #[test]
    fn remove_all() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 1);
        shim_hashtable_add_object(ht, 0, 2);
        shim_hashtable_add_object(ht, 0, 3);
        shim_hashtable_remove_all(ht, 0);
        assert_eq!(shim_hashtable_count(ht, 0), 0);
        assert_eq!(shim_hashtable_contains(ht, 0, 1), 0);
        free_hashtable(ht);
    }

    #[test]
    fn remove_all_on_empty() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_remove_all(ht, 0); // should not crash
        assert_eq!(shim_hashtable_count(ht, 0), 0);
        free_hashtable(ht);
    }

    // ── allObjects ─────────────────────────────────────────────────

    #[test]
    fn all_objects_empty() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        let arr = shim_hashtable_all_objects(ht, 0);
        assert_ne!(arr, 0);
        // NSArray count should be 0
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
        free_obj(arr);
        free_hashtable(ht);
    }

    #[test]
    fn all_objects_returns_all_members() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 0xA);
        shim_hashtable_add_object(ht, 0, 0xB);
        let arr = shim_hashtable_all_objects(ht, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 2);
        free_obj(arr);
        free_hashtable(ht);
    }

    // ── description ────────────────────────────────────────────────

    #[test]
    fn description_empty() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        let desc = shim_hashtable_description(ht, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSHashTable: 0 objects>");
        free_obj(desc);
        free_hashtable(ht);
    }

    #[test]
    fn description_with_items() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 1);
        shim_hashtable_add_object(ht, 0, 2);
        let desc = shim_hashtable_description(ht, 0);
        assert_string_eq(desc, "<NSHashTable: 2 objects>");
        free_obj(desc);
        free_hashtable(ht);
    }

    // ── Nil safety ─────────────────────────────────────────────────

    #[test]
    fn nil_add_object() {
        shim_hashtable_add_object(0, 0, 42); // must not crash
    }

    #[test]
    fn nil_remove_object() {
        shim_hashtable_remove_object(0, 0, 42);
    }

    #[test]
    fn nil_remove_all() {
        shim_hashtable_remove_all(0, 0);
    }

    #[test]
    fn nil_contains() {
        assert_eq!(shim_hashtable_contains(0, 0, 42), 0);
    }

    #[test]
    fn nil_count() {
        assert_eq!(shim_hashtable_count(0, 0), 0);
    }

    #[test]
    fn nil_all_objects() {
        assert_eq!(shim_hashtable_all_objects(0, 0), 0);
    }

    #[test]
    fn nil_description() {
        assert_eq!(shim_hashtable_description(0, 0), 0);
    }

    // ── Edge cases ─────────────────────────────────────────────────

    #[test]
    fn add_zero_as_member() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 0); // member value 0 (nil pointer)
        assert_eq!(shim_hashtable_count(ht, 0), 1);
        assert_eq!(shim_hashtable_contains(ht, 0, 0), 1);
        free_hashtable(ht);
    }

    #[test]
    fn add_and_remove_then_re_add() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 7);
        shim_hashtable_remove_object(ht, 0, 7);
        assert_eq!(shim_hashtable_contains(ht, 0, 7), 0);
        shim_hashtable_add_object(ht, 0, 7);
        assert_eq!(shim_hashtable_contains(ht, 0, 7), 1);
        assert_eq!(shim_hashtable_count(ht, 0), 1);
        free_hashtable(ht);
    }

    #[test]
    fn count_on_empty_table() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        assert_eq!(shim_hashtable_count(ht, 0), 0);
        free_hashtable(ht);
    }

    // ── anyObject ──────────────────────────────────────────────────

    #[test]
    fn any_object_empty() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        assert_eq!(shim_hashtable_any_object(ht, 0), 0);
        free_hashtable(ht);
    }

    #[test]
    fn any_object_returns_member() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 42);
        assert_eq!(shim_hashtable_any_object(ht, 0), 42);
        free_hashtable(ht);
    }

    #[test]
    fn any_object_nil_receiver() {
        assert_eq!(shim_hashtable_any_object(0, 0), 0);
    }

    // ── setRepresentation ──────────────────────────────────────────

    #[test]
    fn set_representation_empty() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        let s = shim_hashtable_set_representation(ht, 0);
        assert_ne!(s, 0);
        assert_eq!(crate::nsset::shim_nsset_count(s, 0), 0);
        free_obj(s);
        free_hashtable(ht);
    }

    #[test]
    fn set_representation_with_items() {
        let ht = shim_hashtable_with_options(0, 0, 0);
        shim_hashtable_add_object(ht, 0, 0xA);
        shim_hashtable_add_object(ht, 0, 0xB);
        let s = shim_hashtable_set_representation(ht, 0);
        assert_ne!(s, 0);
        assert_eq!(crate::nsset::shim_nsset_count(s, 0), 2);
        free_obj(s);
        free_hashtable(ht);
    }

    #[test]
    fn set_representation_nil_receiver() {
        assert_eq!(shim_hashtable_set_representation(0, 0), 0);
    }
}
