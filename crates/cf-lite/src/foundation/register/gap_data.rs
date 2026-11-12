//! Gap-analysis class registrations: data structures, collections, scanning.

use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{
    nscountedset, nsfilehandle, nsmutablecharacterset, nsmutabledata, nsobj, nsorderedset,
    nsscanner,
};

/// Register `NSMutableData` with mutable byte buffer methods.
pub fn register_nsmutabledata_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutabledata_class_addr();
    let metaclass_addr = nsobj::nsmutabledata_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableData".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "length",
                nsmutabledata::shim_nsmutabledata_length as *const () as u64,
                "Q@:",
            ),
            me(
                "bytes",
                nsmutabledata::shim_nsmutabledata_bytes as *const () as u64,
                "^v@:",
            ),
            me(
                "mutableBytes",
                nsmutabledata::shim_nsmutabledata_bytes as *const () as u64,
                "^v@:",
            ),
            me(
                "appendBytes:length:",
                nsmutabledata::shim_nsmutabledata_append_bytes as *const () as u64,
                "v@:^vQ",
            ),
            me(
                "appendData:",
                nsmutabledata::shim_nsmutabledata_append_data as *const () as u64,
                "v@:@",
            ),
            me(
                "setLength:",
                nsmutabledata::shim_nsmutabledata_set_length as *const () as u64,
                "v@:Q",
            ),
            me(
                "resetBytesInRange:",
                nsmutabledata::shim_nsmutabledata_reset_bytes_in_range as *const () as u64,
                "v@:QQ",
            ),
            me(
                "replaceBytesInRange:withBytes:",
                nsmutabledata::shim_nsmutabledata_replace_bytes_in_range as *const () as u64,
                "v@:QQ^v",
            ),
            me(
                "description",
                nsmutabledata::shim_nsmutabledata_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "data",
                nsmutabledata::shim_nsmutabledata_data as *const () as u64,
                "@@:",
            ),
            me(
                "dataWithCapacity:",
                nsmutabledata::shim_nsmutabledata_data_with_capacity as *const () as u64,
                "@@:Q",
            ),
            me(
                "dataWithLength:",
                nsmutabledata::shim_nsmutabledata_data_with_length as *const () as u64,
                "@@:Q",
            ),
            me(
                "dataWithBytes:length:",
                nsmutabledata::shim_nsmutabledata_data_with_bytes as *const () as u64,
                "@@:^vQ",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsdata_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableData");
}

/// Register `NSFileHandle` with file descriptor methods.
pub fn register_nsfilehandle_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsfilehandle_class_addr();
    let metaclass_addr = nsobj::nsfilehandle_metaclass_addr();

    let info = ClassInfo {
        name: "NSFileHandle".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "fileDescriptor",
                nsfilehandle::shim_nsfilehandle_file_descriptor as *const () as u64,
                "i@:",
            ),
            me(
                "readDataOfLength:",
                nsfilehandle::shim_nsfilehandle_read_data_of_length as *const () as u64,
                "@@:Q",
            ),
            me(
                "readDataToEndOfFile",
                nsfilehandle::shim_nsfilehandle_read_data_to_end as *const () as u64,
                "@@:",
            ),
            me(
                "availableData",
                nsfilehandle::shim_nsfilehandle_available_data as *const () as u64,
                "@@:",
            ),
            me(
                "writeData:",
                nsfilehandle::shim_nsfilehandle_write_data as *const () as u64,
                "v@:@",
            ),
            me(
                "closeFile",
                nsfilehandle::shim_nsfilehandle_close_file as *const () as u64,
                "v@:",
            ),
            me(
                "seekToFileOffset:",
                nsfilehandle::shim_nsfilehandle_seek_to_file_offset as *const () as u64,
                "v@:Q",
            ),
            me(
                "seekToEndOfFile",
                nsfilehandle::shim_nsfilehandle_seek_to_end_of_file as *const () as u64,
                "Q@:",
            ),
            me(
                "truncateFileAtOffset:",
                nsfilehandle::shim_nsfilehandle_truncate_at_offset as *const () as u64,
                "v@:Q",
            ),
            me(
                "offsetInFile",
                nsfilehandle::shim_nsfilehandle_offset_in_file as *const () as u64,
                "Q@:",
            ),
            me(
                "description",
                nsfilehandle::shim_nsfilehandle_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "fileHandleWithStandardInput",
                nsfilehandle::shim_nsfilehandle_stdin as *const () as u64,
                "@@:",
            ),
            me(
                "fileHandleWithStandardOutput",
                nsfilehandle::shim_nsfilehandle_stdout as *const () as u64,
                "@@:",
            ),
            me(
                "fileHandleWithStandardError",
                nsfilehandle::shim_nsfilehandle_stderr as *const () as u64,
                "@@:",
            ),
            me(
                "fileHandleForReadingAtPath:",
                nsfilehandle::shim_nsfilehandle_for_reading_at_path as *const () as u64,
                "@@:@",
            ),
            me(
                "fileHandleForWritingAtPath:",
                nsfilehandle::shim_nsfilehandle_for_writing_at_path as *const () as u64,
                "@@:@",
            ),
            me(
                "fileHandleForUpdatingAtPath:",
                nsfilehandle::shim_nsfilehandle_for_updating_at_path as *const () as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSFileHandle");
}

/// Register `NSScanner` with string scanning methods.
pub fn register_nsscanner_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsscanner_class_addr();
    let metaclass_addr = nsobj::nsscanner_metaclass_addr();

    let info = ClassInfo {
        name: "NSScanner".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "string",
                nsscanner::shim_nsscanner_string as *const () as u64,
                "@@:",
            ),
            me(
                "scanLocation",
                nsscanner::shim_nsscanner_scan_location as *const () as u64,
                "Q@:",
            ),
            me(
                "setScanLocation:",
                nsscanner::shim_nsscanner_set_scan_location as *const () as u64,
                "v@:Q",
            ),
            me(
                "isAtEnd",
                nsscanner::shim_nsscanner_is_at_end as *const () as u64,
                "B@:",
            ),
            me(
                "caseSensitive",
                nsscanner::shim_nsscanner_case_sensitive as *const () as u64,
                "B@:",
            ),
            me(
                "setCaseSensitive:",
                nsscanner::shim_nsscanner_set_case_sensitive as *const () as u64,
                "v@:B",
            ),
            me(
                "setCharactersToBeSkipped:",
                nsscanner::shim_nsscanner_set_characters_to_be_skipped as *const () as u64,
                "v@:@",
            ),
            me(
                "charactersToBeSkipped",
                nsscanner::shim_nsscanner_characters_to_be_skipped as *const () as u64,
                "@@:",
            ),
            me(
                "scanInt:",
                nsscanner::shim_nsscanner_scan_int as *const () as u64,
                "B@:^i",
            ),
            me(
                "scanDouble:",
                nsscanner::shim_nsscanner_scan_double as *const () as u64,
                "B@:^d",
            ),
            me(
                "scanString:intoString:",
                nsscanner::shim_nsscanner_scan_string as *const () as u64,
                "B@:@^@",
            ),
            me(
                "scanUpToString:intoString:",
                nsscanner::shim_nsscanner_scan_up_to_string as *const () as u64,
                "B@:@^@",
            ),
            me(
                "scanUpToCharactersFromSet:intoString:",
                nsscanner::shim_nsscanner_scan_up_to_characters_from_set as *const () as u64,
                "B@:@^@",
            ),
            me(
                "scanCharactersFromSet:intoString:",
                nsscanner::shim_nsscanner_scan_characters_from_set as *const () as u64,
                "B@:@^@",
            ),
            me(
                "description",
                nsscanner::shim_nsscanner_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "scannerWithString:",
            nsscanner::shim_nsscanner_scanner_with_string as *const () as u64,
            "@@:@",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSScanner");
}

/// Register `NSOrderedSet` with ordered unique collection methods.
pub fn register_nsorderedset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsorderedset_class_addr();
    let metaclass_addr = nsobj::nsorderedset_metaclass_addr();

    let info = ClassInfo {
        name: "NSOrderedSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "count",
                nsorderedset::shim_nsorderedset_count as *const () as u64,
                "Q@:",
            ),
            me(
                "objectAtIndex:",
                nsorderedset::shim_nsorderedset_object_at_index as *const () as u64,
                "@@:Q",
            ),
            me(
                "firstObject",
                nsorderedset::shim_nsorderedset_first_object as *const () as u64,
                "@@:",
            ),
            me(
                "lastObject",
                nsorderedset::shim_nsorderedset_last_object as *const () as u64,
                "@@:",
            ),
            me(
                "containsObject:",
                nsorderedset::shim_nsorderedset_contains_object as *const () as u64,
                "B@:@",
            ),
            me(
                "indexOfObject:",
                nsorderedset::shim_nsorderedset_index_of_object as *const () as u64,
                "Q@:@",
            ),
            me(
                "array",
                nsorderedset::shim_nsorderedset_array as *const () as u64,
                "@@:",
            ),
            me(
                "set",
                nsorderedset::shim_nsorderedset_set as *const () as u64,
                "@@:",
            ),
            me(
                "reverseObjectEnumerator",
                nsorderedset::shim_nsorderedset_reverse_object_enumerator as *const () as u64,
                "@@:",
            ),
            me(
                "description",
                nsorderedset::shim_nsorderedset_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "orderedSetWithArray:",
                nsorderedset::shim_nsorderedset_ordered_set_with_array as *const () as u64,
                "@@:@",
            ),
            me(
                "orderedSet",
                nsorderedset::shim_nsorderedset_ordered_set as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSOrderedSet");
}

/// Register `NSMutableOrderedSet` with mutable ordered unique collection methods.
pub fn register_nsmutableorderedset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutableorderedset_class_addr();
    let metaclass_addr = nsobj::nsmutableorderedset_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableOrderedSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "addObject:",
                nsorderedset::shim_nsmutableorderedset_add_object as *const () as u64,
                "v@:@",
            ),
            me(
                "removeObject:",
                nsorderedset::shim_nsmutableorderedset_remove_object as *const () as u64,
                "v@:@",
            ),
            me(
                "removeAllObjects",
                nsorderedset::shim_nsmutableorderedset_remove_all as *const () as u64,
                "v@:",
            ),
            me(
                "insertObject:atIndex:",
                nsorderedset::shim_nsmutableorderedset_insert_object as *const () as u64,
                "v@:@Q",
            ),
        ],
        class_methods: vec![
            me(
                "orderedSet",
                nsorderedset::shim_nsmutableorderedset_ordered_set as *const () as u64,
                "@@:",
            ),
            me(
                "orderedSetWithCapacity:",
                nsorderedset::shim_nsmutableorderedset_ordered_set_with_capacity as *const ()
                    as u64,
                "@@:Q",
            ),
            me(
                "orderedSetWithArray:",
                nsorderedset::shim_nsmutableorderedset_ordered_set_with_array as *const () as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsorderedset_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableOrderedSet");
}

/// Register `NSCountedSet` with bag/multiset methods.
pub fn register_nscountedset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscountedset_class_addr();
    let metaclass_addr = nsobj::nscountedset_metaclass_addr();

    let info = ClassInfo {
        name: "NSCountedSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "addObject:",
                nscountedset::shim_nscountedset_add_object as *const () as u64,
                "v@:@",
            ),
            me(
                "removeObject:",
                nscountedset::shim_nscountedset_remove_object as *const () as u64,
                "v@:@",
            ),
            me(
                "count",
                nscountedset::shim_nscountedset_count as *const () as u64,
                "Q@:",
            ),
            me(
                "countForObject:",
                nscountedset::shim_nscountedset_count_for_object as *const () as u64,
                "Q@:@",
            ),
            me(
                "containsObject:",
                nscountedset::shim_nscountedset_contains_object as *const () as u64,
                "B@:@",
            ),
            me(
                "allObjects",
                nscountedset::shim_nscountedset_all_objects as *const () as u64,
                "@@:",
            ),
            me(
                "description",
                nscountedset::shim_nscountedset_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "set",
                nscountedset::shim_nscountedset_set as *const () as u64,
                "@@:",
            ),
            me(
                "setWithArray:",
                nscountedset::shim_nscountedset_set_with_array as *const () as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsmutableset_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCountedSet");
}

/// Register `NSMutableCharacterSet` with mutable character set methods.
pub fn register_nsmutablecharacterset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutablecharacterset_class_addr();
    let metaclass_addr = nsobj::nsmutablecharacterset_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableCharacterSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "characterIsMember:",
                nsmutablecharacterset::shim_nsmutablecharset_character_is_member as *const ()
                    as u64,
                "B@:S",
            ),
            me(
                "addCharactersInString:",
                nsmutablecharacterset::shim_nsmutablecharset_add_characters_in_string as *const ()
                    as u64,
                "v@:@",
            ),
            me(
                "removeCharactersInString:",
                nsmutablecharacterset::shim_nsmutablecharset_remove_characters_in_string
                    as *const () as u64,
                "v@:@",
            ),
            me(
                "formUnionWithCharacterSet:",
                nsmutablecharacterset::shim_nsmutablecharset_form_union as *const () as u64,
                "v@:@",
            ),
            me(
                "formIntersectionWithCharacterSet:",
                nsmutablecharacterset::shim_nsmutablecharset_form_intersection as *const () as u64,
                "v@:@",
            ),
            me(
                "invert",
                nsmutablecharacterset::shim_nsmutablecharset_invert as *const () as u64,
                "v@:",
            ),
            me(
                "description",
                nsmutablecharacterset::shim_nsmutablecharset_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "characterSetWithCharactersInString:",
                nsmutablecharacterset::shim_nsmutablecharset_with_characters_in_string as *const ()
                    as u64,
                "@@:@",
            ),
            me(
                "whitespaceCharacterSet",
                nsmutablecharacterset::shim_nsmutablecharset_whitespace as *const () as u64,
                "@@:",
            ),
            me(
                "alphanumericCharacterSet",
                nsmutablecharacterset::shim_nsmutablecharset_alphanumeric as *const () as u64,
                "@@:",
            ),
            me(
                "decimalDigitCharacterSet",
                nsmutablecharacterset::shim_nsmutablecharset_decimal_digit as *const () as u64,
                "@@:",
            ),
            me(
                "letterCharacterSet",
                nsmutablecharacterset::shim_nsmutablecharset_letter as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nscharacterset_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableCharacterSet");
}
