//! XCStringsdataFile shim: represents a parsed .xcstringsdata file from
//! Apple's XCStringsParser.framework.
//!
//! Object layout: 56 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  source (NSString*)
//!   [16..24): version (NSString*)
//!   [24..32): tables (NSArray* of XCStringsdataTable)
//!   [32..40): path (NSString*)
//!   [40..48): encoding (u64 — NSStringEncoding value)
//!   [48..56): options (u64 — opaque bitmask)

const SOURCE_OFF: usize = 8;
const VERSION_OFF: usize = 16;
const TABLES_OFF: usize = 24;
const PATH_OFF: usize = 32;
const ENCODING_OFF: usize = 40;
const OPTIONS_OFF: usize = 48;

/// `-[XCStringsdataFile initWithSource:version:tables:]`
pub extern "C" fn shim_xcstringsdatafile_init_with_source_version_tables(
    obj: u64,
    _cmd: u64,
    source: u64,
    version: u64,
    tables: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + SOURCE_OFF) as *mut u64) = source;
        *((obj as usize + VERSION_OFF) as *mut u64) = version;
        *((obj as usize + TABLES_OFF) as *mut u64) = tables;
        *((obj as usize + ENCODING_OFF) as *mut u64) = 4; // NSUTF8StringEncoding
    }
    obj
}

/// `-[XCStringsdataFile initWithPath:options:]`
pub extern "C" fn shim_xcstringsdatafile_init_with_path_options(
    obj: u64,
    _cmd: u64,
    path: u64,
    options: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + PATH_OFF) as *mut u64) = path;
        *((obj as usize + OPTIONS_OFF) as *mut u64) = options;
        *((obj as usize + ENCODING_OFF) as *mut u64) = 4; // NSUTF8StringEncoding
    }
    obj
}

/// `-[XCStringsdataFile source]`
pub extern "C" fn shim_xcstringsdatafile_source(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + SOURCE_OFF) as *const u64) }
}

/// `-[XCStringsdataFile tables]`
pub extern "C" fn shim_xcstringsdatafile_tables(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + TABLES_OFF) as *const u64) }
}

/// `-[XCStringsdataFile encoding]`
pub extern "C" fn shim_xcstringsdatafile_encoding(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + ENCODING_OFF) as *const u64) }
}

/// `-[XCStringsdataFile options]`
pub extern "C" fn shim_xcstringsdatafile_options(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + OPTIONS_OFF) as *const u64) }
}

/// `-[XCStringsdataFile path]`
pub extern "C" fn shim_xcstringsdatafile_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + PATH_OFF) as *const u64) }
}

/// `-[XCStringsdataFile writeTo:error:]`
///
/// Serializes tables to `.strings` format and writes to the given path.
/// Returns 1 on success, 0 on failure.
pub extern "C" fn shim_xcstringsdatafile_write_to(
    obj: u64,
    _cmd: u64,
    dest_path: u64,
    error_out: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(path_str) = crate::cfstring::extract_cfstring_cstr(dest_path) else {
        if error_out != 0 {
            unsafe {
                *(error_out as *mut u64) = 0;
            }
        }
        return 0;
    };

    let mut output = String::new();
    let tables = unsafe { *((obj as usize + TABLES_OFF) as *const u64) };
    if tables != 0 {
        // NSArray layout: [ISA:8][count:8][items_ptr:8]
        let table_count = unsafe { *((tables as usize + 8) as *const u64) };
        let table_items = unsafe { *((tables as usize + 16) as *const u64) };
        if table_items != 0 {
            for t in 0..table_count as usize {
                let table = unsafe { *((table_items as usize + t * 8) as *const u64) };
                if table == 0 {
                    continue;
                }
                // XCStringsdataTable: [ISA:8][name:8][entries:8]
                let entries_arr = unsafe { *((table as usize + 16) as *const u64) };
                if entries_arr == 0 {
                    continue;
                }
                let entry_count = unsafe { *((entries_arr as usize + 8) as *const u64) };
                let entry_items = unsafe { *((entries_arr as usize + 16) as *const u64) };
                if entry_items == 0 {
                    continue;
                }
                for e in 0..entry_count as usize {
                    let entry = unsafe { *((entry_items as usize + e * 8) as *const u64) };
                    if entry == 0 {
                        continue;
                    }
                    // XCStringsdataEntry: [ISA:8][key:8][value:8][comment:8]
                    let key = unsafe { *((entry as usize + 8) as *const u64) };
                    let val = unsafe { *((entry as usize + 16) as *const u64) };
                    let key_s = crate::cfstring::extract_cfstring_cstr(key).unwrap_or_default();
                    let val_s = crate::cfstring::extract_cfstring_cstr(val).unwrap_or_default();
                    use std::fmt::Write;
                    let _ = writeln!(output, "\"{}\" = \"{}\";", key_s, val_s);
                }
            }
        }
    }

    match std::fs::write(path_str, output.as_bytes()) {
        Ok(()) => 1,
        Err(_) => {
            if error_out != 0 {
                unsafe {
                    *(error_out as *mut u64) = 0;
                }
            }
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const OBJ_SIZE: usize = 56;
    const FAKE_ISA: u64 = 0xDEAD_F11E;

    #[test]
    fn init_with_source_stores_fields() {
        let obj = alloc_obj(OBJ_SIZE, FAKE_ISA);
        let src = make_string("xcstrings");
        let ver = make_string("1.0");
        let tbl = 0xABCD_u64; // placeholder tables pointer

        let ret = shim_xcstringsdatafile_init_with_source_version_tables(obj, 0, src, ver, tbl);
        assert_eq!(ret, obj);

        assert_eq!(shim_xcstringsdatafile_source(obj, 0), src);
        assert_eq!(read_field(obj, VERSION_OFF), ver);
        assert_eq!(shim_xcstringsdatafile_tables(obj, 0), tbl);
        assert_eq!(shim_xcstringsdatafile_encoding(obj, 0), 4);

        free_obj(obj);
    }

    #[test]
    fn init_with_path_stores_fields() {
        let obj = alloc_obj(OBJ_SIZE, FAKE_ISA);
        let path = make_string("/tmp/test.xcstringsdata");

        let ret = shim_xcstringsdatafile_init_with_path_options(obj, 0, path, 0xFF);
        assert_eq!(ret, obj);

        assert_eq!(shim_xcstringsdatafile_path(obj, 0), path);
        assert_eq!(shim_xcstringsdatafile_options(obj, 0), 0xFF);
        assert_eq!(shim_xcstringsdatafile_encoding(obj, 0), 4);

        free_obj(obj);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(
            shim_xcstringsdatafile_init_with_source_version_tables(0, 0, 1, 2, 3),
            0
        );
        assert_eq!(shim_xcstringsdatafile_init_with_path_options(0, 0, 1, 2), 0);
        assert_eq!(shim_xcstringsdatafile_source(0, 0), 0);
        assert_eq!(shim_xcstringsdatafile_tables(0, 0), 0);
        assert_eq!(shim_xcstringsdatafile_encoding(0, 0), 0);
        assert_eq!(shim_xcstringsdatafile_options(0, 0), 0);
        assert_eq!(shim_xcstringsdatafile_path(0, 0), 0);
        assert_eq!(shim_xcstringsdatafile_write_to(0, 0, 1, 0), 0);
    }

    #[test]
    fn write_to_creates_file() {
        // Build an XCStringsdataEntry: [ISA:8][key:8][value:8][comment:8] = 32 bytes
        let key = make_string("greeting");
        let val = make_string("Hello");
        let entry = alloc_obj(32, 0xDEAD_E000);
        write_field(entry, 8, key);
        write_field(entry, 16, val);

        // Build NSArray for entries: [ISA:8][count:8][items_ptr:8][capacity:8] = 32 bytes
        let entry_items = alloc_u64_buf(1);
        write_field(entry_items, 0, entry);
        let entries_arr = alloc_obj(32, 0xDEAD_A000);
        write_field(entries_arr, 8, 1); // count
        write_field(entries_arr, 16, entry_items); // items_ptr

        // Build an XCStringsdataTable: [ISA:8][name:8][entries:8] = 24 bytes
        let table_name = make_string("Localizable");
        let table = alloc_obj(24, 0xDEAD_B000);
        write_field(table, 8, table_name);
        write_field(table, 16, entries_arr);

        // Build NSArray for tables
        let table_items = alloc_u64_buf(1);
        write_field(table_items, 0, table);
        let tables_arr = alloc_obj(32, 0xDEAD_A001);
        write_field(tables_arr, 8, 1); // count
        write_field(tables_arr, 16, table_items); // items_ptr

        // Build the file object
        let file_obj = alloc_obj(OBJ_SIZE, FAKE_ISA);
        let src = make_string("test");
        let ver = make_string("1.0");
        shim_xcstringsdatafile_init_with_source_version_tables(file_obj, 0, src, ver, tables_arr);

        // Write to a temp file
        let tmp = std::env::temp_dir().join("xcstringsdata_test_output.strings");
        let dest = make_string(tmp.to_str().unwrap());
        let result = shim_xcstringsdatafile_write_to(file_obj, 0, dest, 0);
        assert_eq!(result, 1);

        let contents = std::fs::read_to_string(&tmp).expect("failed to read output file");
        assert!(
            contents.contains("\"greeting\" = \"Hello\";"),
            "unexpected file contents: {contents}"
        );

        // Cleanup
        let _ = std::fs::remove_file(&tmp);
        free_buf(entry_items);
        free_buf(table_items);
        free_obj(entry);
        free_obj(entries_arr);
        free_obj(table);
        free_obj(tables_arr);
        free_obj(file_obj);
    }
}
