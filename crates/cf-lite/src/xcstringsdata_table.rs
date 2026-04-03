//! XCStringsdataTable shim: table of localization string entries.
//!
//! Object layout: 32 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  name (NSString*)
//!   [16..24): entries (NSArray* of XCStringsdataEntry)
//!   [24..32): reserved (u64)

const NAME_OFF: usize = 8;
const ENTRIES_OFF: usize = 16;
const _RESERVED_OFF: usize = 24;

/// `-[XCStringsdataTable initWithName:entries:]`
pub extern "C" fn shim_xcstringsdatatable_init_with_name_entries(
    obj: u64,
    _cmd: u64,
    name: u64,
    entries: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + NAME_OFF) as *mut u64) = name;
        *((obj as usize + ENTRIES_OFF) as *mut u64) = entries;
    }
    obj
}

/// `-[XCStringsdataTable name]`
pub extern "C" fn shim_xcstringsdatatable_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NAME_OFF) as *const u64) }
}

/// `-[XCStringsdataTable entries]`
pub extern "C" fn shim_xcstringsdatatable_entries(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + ENTRIES_OFF) as *const u64) }
}

/// `-[XCStringsdataTable findEntryInTable:key:value:comments:unicode:attributed:]`
pub extern "C" fn shim_xcstringsdatatable_find_entry(
    obj: u64,
    _cmd: u64,
    _table: u64,
    key: u64,
    value: u64,
    comment: u64,
    _unicode: u64,
    _attributed: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Read existing entries array
    let entries_arr = unsafe { *((obj as usize + ENTRIES_OFF) as *const u64) };
    if entries_arr != 0 {
        // NSArray layout: [ISA:8][count:8][items_ptr:8]
        let count = unsafe { *((entries_arr as usize + 8) as *const u64) };
        let items = unsafe { *((entries_arr as usize + 16) as *const u64) };
        if items != 0 {
            for i in 0..count as usize {
                let entry = unsafe { *((items as usize + i * 8) as *const u64) };
                if entry == 0 {
                    continue;
                }
                let entry_key = unsafe { *((entry as usize + 8) as *const u64) };
                if entry_key != 0 && key != 0 {
                    // Compare keys by string content
                    let k1 = crate::cfstring::extract_cfstring_cstr(entry_key);
                    let k2 = crate::cfstring::extract_cfstring_cstr(key);
                    if k1 == k2 {
                        // Update value and comment
                        unsafe {
                            *((entry as usize + 16) as *mut u64) = value;
                            *((entry as usize + 24) as *mut u64) = comment;
                        }
                        return entry;
                    }
                }
            }
        }
    }
    // Not found — create new entry
    let entry = crate::pal_alloc().calloc(1, 56) as u64;
    if entry != 0 {
        unsafe {
            *((entry as usize + 8) as *mut u64) = key;
            *((entry as usize + 16) as *mut u64) = value;
            *((entry as usize + 24) as *mut u64) = comment;
        }
    }
    entry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn init_stores_name_and_entries() {
        let obj = alloc_obj(32, 0xDEAD_7AB0);
        let name = make_string("MyTable");
        let entries = alloc_obj_fake(24); // stand-in NSArray

        let ret = shim_xcstringsdatatable_init_with_name_entries(obj, 0, name, entries);
        assert_eq!(ret, obj);
        assert_eq!(shim_xcstringsdatatable_name(obj, 0), name);
        assert_eq!(shim_xcstringsdatatable_entries(obj, 0), entries);

        free_obj(entries);
        free_obj(obj);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_xcstringsdatatable_init_with_name_entries(0, 0, 1, 2), 0);
        assert_eq!(shim_xcstringsdatatable_name(0, 0), 0);
        assert_eq!(shim_xcstringsdatatable_entries(0, 0), 0);
        assert_eq!(shim_xcstringsdatatable_find_entry(0, 0, 0, 1, 2, 3, 0, 0), 0);
    }

    #[test]
    fn find_entry_creates_new() {
        let obj = alloc_obj(32, 0xDEAD_7AB0);
        // entries is 0 (nil), so find_entry won't find a match
        let key = make_string("greeting");
        let val = make_string("Hello");
        let cmt = make_string("A greeting");

        let entry = shim_xcstringsdatatable_find_entry(obj, 0, 0, key, val, cmt, 0, 0);
        assert_ne!(entry, 0);
        assert_eq!(read_field(entry, 8), key);
        assert_eq!(read_field(entry, 16), val);
        assert_eq!(read_field(entry, 24), cmt);

        free_obj(entry);
        free_obj(obj);
    }

    #[test]
    fn entries_initially_nil_if_passed_zero() {
        let obj = alloc_obj(32, 0xDEAD_7AB0);
        shim_xcstringsdatatable_init_with_name_entries(obj, 0, 0, 0);
        assert_eq!(shim_xcstringsdatatable_entries(obj, 0), 0);

        free_obj(obj);
    }
}
