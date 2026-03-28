use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{
    nscountedset, nsfilehandle, nsmutablecharacterset, nsmutabledata, nsobj, nsorderedset,
    nsscanner,
    // Phase C9
    nsbytecountformatter, nshost, nsiso8601dateformatter, nspipe, nsprogress, nstask,
    nsurlcomponents, nsurlrequest,
};

/// Register `NSMutableData` with mutable byte buffer methods.
pub fn register_nsmutabledata_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutabledata_class_addr();
    let metaclass_addr = nsobj::nsmutabledata_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableData".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("length", nsmutabledata::shim_nsmutabledata_length as u64, "Q@:"),
            me("bytes", nsmutabledata::shim_nsmutabledata_bytes as u64, "^v@:"),
            me("mutableBytes", nsmutabledata::shim_nsmutabledata_bytes as u64, "^v@:"),
            me("appendBytes:length:", nsmutabledata::shim_nsmutabledata_append_bytes as u64, "v@:^vQ"),
            me("appendData:", nsmutabledata::shim_nsmutabledata_append_data as u64, "v@:@"),
            me("setLength:", nsmutabledata::shim_nsmutabledata_set_length as u64, "v@:Q"),
            me("resetBytesInRange:", nsmutabledata::shim_nsmutabledata_reset_bytes_in_range as u64, "v@:QQ"),
            me("replaceBytesInRange:withBytes:", nsmutabledata::shim_nsmutabledata_replace_bytes_in_range as u64, "v@:QQ^v"),
            me("description", nsmutabledata::shim_nsmutabledata_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("data", nsmutabledata::shim_nsmutabledata_data as u64, "@@:"),
            me("dataWithCapacity:", nsmutabledata::shim_nsmutabledata_data_with_capacity as u64, "@@:Q"),
            me("dataWithLength:", nsmutabledata::shim_nsmutabledata_data_with_length as u64, "@@:Q"),
            me("dataWithBytes:length:", nsmutabledata::shim_nsmutabledata_data_with_bytes as u64, "@@:^vQ"),
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
            me("fileDescriptor", nsfilehandle::shim_nsfilehandle_file_descriptor as u64, "i@:"),
            me("readDataOfLength:", nsfilehandle::shim_nsfilehandle_read_data_of_length as u64, "@@:Q"),
            me("readDataToEndOfFile", nsfilehandle::shim_nsfilehandle_read_data_to_end as u64, "@@:"),
            me("availableData", nsfilehandle::shim_nsfilehandle_available_data as u64, "@@:"),
            me("writeData:", nsfilehandle::shim_nsfilehandle_write_data as u64, "v@:@"),
            me("closeFile", nsfilehandle::shim_nsfilehandle_close_file as u64, "v@:"),
            me("seekToFileOffset:", nsfilehandle::shim_nsfilehandle_seek_to_file_offset as u64, "v@:Q"),
            me("offsetInFile", nsfilehandle::shim_nsfilehandle_offset_in_file as u64, "Q@:"),
            me("description", nsfilehandle::shim_nsfilehandle_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("fileHandleWithStandardInput", nsfilehandle::shim_nsfilehandle_stdin as u64, "@@:"),
            me("fileHandleWithStandardOutput", nsfilehandle::shim_nsfilehandle_stdout as u64, "@@:"),
            me("fileHandleWithStandardError", nsfilehandle::shim_nsfilehandle_stderr as u64, "@@:"),
            me("fileHandleForReadingAtPath:", nsfilehandle::shim_nsfilehandle_for_reading_at_path as u64, "@@:@"),
            me("fileHandleForWritingAtPath:", nsfilehandle::shim_nsfilehandle_for_writing_at_path as u64, "@@:@"),
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
            me("string", nsscanner::shim_nsscanner_string as u64, "@@:"),
            me("scanLocation", nsscanner::shim_nsscanner_scan_location as u64, "Q@:"),
            me("setScanLocation:", nsscanner::shim_nsscanner_set_scan_location as u64, "v@:Q"),
            me("isAtEnd", nsscanner::shim_nsscanner_is_at_end as u64, "B@:"),
            me("caseSensitive", nsscanner::shim_nsscanner_case_sensitive as u64, "B@:"),
            me("setCaseSensitive:", nsscanner::shim_nsscanner_set_case_sensitive as u64, "v@:B"),
            me("scanInt:", nsscanner::shim_nsscanner_scan_int as u64, "B@:^i"),
            me("scanDouble:", nsscanner::shim_nsscanner_scan_double as u64, "B@:^d"),
            me("scanString:intoString:", nsscanner::shim_nsscanner_scan_string as u64, "B@:@^@"),
            me("scanUpToString:intoString:", nsscanner::shim_nsscanner_scan_up_to_string as u64, "B@:@^@"),
            me("description", nsscanner::shim_nsscanner_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("scannerWithString:", nsscanner::shim_nsscanner_scanner_with_string as u64, "@@:@"),
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
            me("count", nsorderedset::shim_nsorderedset_count as u64, "Q@:"),
            me("objectAtIndex:", nsorderedset::shim_nsorderedset_object_at_index as u64, "@@:Q"),
            me("firstObject", nsorderedset::shim_nsorderedset_first_object as u64, "@@:"),
            me("lastObject", nsorderedset::shim_nsorderedset_last_object as u64, "@@:"),
            me("containsObject:", nsorderedset::shim_nsorderedset_contains_object as u64, "B@:@"),
            me("indexOfObject:", nsorderedset::shim_nsorderedset_index_of_object as u64, "Q@:@"),
            me("array", nsorderedset::shim_nsorderedset_array as u64, "@@:"),
            me("description", nsorderedset::shim_nsorderedset_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("orderedSetWithArray:", nsorderedset::shim_nsorderedset_ordered_set_with_array as u64, "@@:@"),
            me("orderedSet", nsorderedset::shim_nsorderedset_ordered_set as u64, "@@:"),
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
            me("addObject:", nsorderedset::shim_nsmutableorderedset_add_object as u64, "v@:@"),
            me("removeObject:", nsorderedset::shim_nsmutableorderedset_remove_object as u64, "v@:@"),
            me("removeAllObjects", nsorderedset::shim_nsmutableorderedset_remove_all as u64, "v@:"),
            me("insertObject:atIndex:", nsorderedset::shim_nsmutableorderedset_insert_object as u64, "v@:@Q"),
        ],
        class_methods: vec![
            me("orderedSet", nsorderedset::shim_nsmutableorderedset_ordered_set as u64, "@@:"),
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
            me("addObject:", nscountedset::shim_nscountedset_add_object as u64, "v@:@"),
            me("removeObject:", nscountedset::shim_nscountedset_remove_object as u64, "v@:@"),
            me("count", nscountedset::shim_nscountedset_count as u64, "Q@:"),
            me("countForObject:", nscountedset::shim_nscountedset_count_for_object as u64, "Q@:@"),
            me("containsObject:", nscountedset::shim_nscountedset_contains_object as u64, "B@:@"),
            me("allObjects", nscountedset::shim_nscountedset_all_objects as u64, "@@:"),
            me("description", nscountedset::shim_nscountedset_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("set", nscountedset::shim_nscountedset_set as u64, "@@:"),
            me("setWithArray:", nscountedset::shim_nscountedset_set_with_array as u64, "@@:@"),
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
            me("characterIsMember:", nsmutablecharacterset::shim_nsmutablecharset_character_is_member as u64, "B@:S"),
            me("addCharactersInString:", nsmutablecharacterset::shim_nsmutablecharset_add_characters_in_string as u64, "v@:@"),
            me("removeCharactersInString:", nsmutablecharacterset::shim_nsmutablecharset_remove_characters_in_string as u64, "v@:@"),
            me("formUnionWithCharacterSet:", nsmutablecharacterset::shim_nsmutablecharset_form_union as u64, "v@:@"),
            me("formIntersectionWithCharacterSet:", nsmutablecharacterset::shim_nsmutablecharset_form_intersection as u64, "v@:@"),
            me("invert", nsmutablecharacterset::shim_nsmutablecharset_invert as u64, "v@:"),
            me("description", nsmutablecharacterset::shim_nsmutablecharset_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("characterSetWithCharactersInString:", nsmutablecharacterset::shim_nsmutablecharset_with_characters_in_string as u64, "@@:@"),
            me("whitespaceCharacterSet", nsmutablecharacterset::shim_nsmutablecharset_whitespace as u64, "@@:"),
            me("alphanumericCharacterSet", nsmutablecharacterset::shim_nsmutablecharset_alphanumeric as u64, "@@:"),
            me("decimalDigitCharacterSet", nsmutablecharacterset::shim_nsmutablecharset_decimal_digit as u64, "@@:"),
            me("letterCharacterSet", nsmutablecharacterset::shim_nsmutablecharset_letter as u64, "@@:"),
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

// ═══════════════════════════════════════════════════════════════
// Phase C9 — remaining gap-analysis classes
// ═══════════════════════════════════════════════════════════════

/// Register `NSURLComponents` with URL builder/parser methods.
pub fn register_nsurlcomponents_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlcomponents_class_addr();
    let metaclass_addr = nsobj::nsurlcomponents_metaclass_addr();

    let info = ClassInfo {
        name: "NSURLComponents".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("scheme", nsurlcomponents::shim_nsurlcomponents_scheme as u64, "@@:"),
            me("host", nsurlcomponents::shim_nsurlcomponents_host as u64, "@@:"),
            me("port", nsurlcomponents::shim_nsurlcomponents_port as u64, "@@:"),
            me("path", nsurlcomponents::shim_nsurlcomponents_path as u64, "@@:"),
            me("query", nsurlcomponents::shim_nsurlcomponents_query as u64, "@@:"),
            me("fragment", nsurlcomponents::shim_nsurlcomponents_fragment as u64, "@@:"),
            me("setScheme:", nsurlcomponents::shim_nsurlcomponents_set_scheme as u64, "v@:@"),
            me("setHost:", nsurlcomponents::shim_nsurlcomponents_set_host as u64, "v@:@"),
            me("setPort:", nsurlcomponents::shim_nsurlcomponents_set_port as u64, "v@:@"),
            me("setPath:", nsurlcomponents::shim_nsurlcomponents_set_path as u64, "v@:@"),
            me("setQuery:", nsurlcomponents::shim_nsurlcomponents_set_query as u64, "v@:@"),
            me("setFragment:", nsurlcomponents::shim_nsurlcomponents_set_fragment as u64, "v@:@"),
            me("URL", nsurlcomponents::shim_nsurlcomponents_url as u64, "@@:"),
            me("string", nsurlcomponents::shim_nsurlcomponents_string as u64, "@@:"),
            me("queryItems", nsurlcomponents::shim_nsurlcomponents_query_items as u64, "@@:"),
            me("setQueryItems:", nsurlcomponents::shim_nsurlcomponents_set_query_items as u64, "v@:@"),
            me("description", nsurlcomponents::shim_nsurlcomponents_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("componentsWithString:", nsurlcomponents::shim_nsurlcomponents_with_string as u64, "@@:@"),
            me("new", nsurlcomponents::shim_nsurlcomponents_new as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 56,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSURLComponents");
}

/// Register `NSURLQueryItem` with name/value accessors.
pub fn register_nsurlqueryitem_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlqueryitem_class_addr();
    let metaclass_addr = nsobj::nsurlqueryitem_metaclass_addr();

    let info = ClassInfo {
        name: "NSURLQueryItem".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nsurlcomponents::shim_nsurlqueryitem_name as u64, "@@:"),
            me("value", nsurlcomponents::shim_nsurlqueryitem_value as u64, "@@:"),
            me("description", nsurlcomponents::shim_nsurlqueryitem_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("queryItemWithName:value:", nsurlcomponents::shim_nsurlqueryitem_with_name_value as u64, "@@:@@"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSURLQueryItem");
}

/// Register `NSByteCountFormatter` with byte formatting methods.
pub fn register_nsbytecountformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbytecountformatter_class_addr();
    let metaclass_addr = nsobj::nsbytecountformatter_metaclass_addr();

    let info = ClassInfo {
        name: "NSByteCountFormatter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsbytecountformatter::shim_bytecountformatter_init as u64, "@@:"),
            me("stringFromByteCount:", nsbytecountformatter::shim_bytecountformatter_string_from_instance as u64, "@@:q"),
            me("setCountStyle:", nsbytecountformatter::shim_bytecountformatter_set_count_style as u64, "v@:Q"),
            me("countStyle", nsbytecountformatter::shim_bytecountformatter_count_style as u64, "Q@:"),
            me("setAllowedUnits:", nsbytecountformatter::shim_bytecountformatter_set_allowed_units as u64, "v@:Q"),
            me("allowedUnits", nsbytecountformatter::shim_bytecountformatter_allowed_units as u64, "Q@:"),
            me("description", nsbytecountformatter::shim_bytecountformatter_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nsbytecountformatter::shim_bytecountformatter_alloc as u64, "@@:"),
            me("stringFromByteCount:countStyle:", nsbytecountformatter::shim_bytecountformatter_string_from as u64, "@@:qQ"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSByteCountFormatter");
}

/// Register `NSISO8601DateFormatter` with ISO 8601 formatting methods.
pub fn register_nsiso8601dateformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsiso8601dateformatter_class_addr();
    let metaclass_addr = nsobj::nsiso8601dateformatter_metaclass_addr();

    let info = ClassInfo {
        name: "NSISO8601DateFormatter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsiso8601dateformatter::shim_iso8601_init as u64, "@@:"),
            me("stringFromDate:", nsiso8601dateformatter::shim_iso8601_string_from_date as u64, "@@:@"),
            me("dateFromString:", nsiso8601dateformatter::shim_iso8601_date_from_string as u64, "@@:@"),
            me("setFormatOptions:", nsiso8601dateformatter::shim_iso8601_set_format_options as u64, "v@:Q"),
            me("formatOptions", nsiso8601dateformatter::shim_iso8601_format_options as u64, "Q@:"),
            me("setTimeZone:", nsiso8601dateformatter::shim_iso8601_set_timezone as u64, "v@:@"),
            me("timeZone", nsiso8601dateformatter::shim_iso8601_timezone as u64, "@@:"),
            me("description", nsiso8601dateformatter::shim_iso8601_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nsiso8601dateformatter::shim_iso8601_alloc as u64, "@@:"),
            me("stringFromDate:timeZone:formatOptions:", nsiso8601dateformatter::shim_iso8601_class_string_from_date as u64, "@@:@@Q"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSISO8601DateFormatter");
}

/// Register `NSProgress` with progress tracking methods.
pub fn register_nsprogress_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsprogress_class_addr();
    let metaclass_addr = nsobj::nsprogress_metaclass_addr();

    let info = ClassInfo {
        name: "NSProgress".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("totalUnitCount", nsprogress::shim_nsprogress_total_unit_count as u64, "q@:"),
            me("setTotalUnitCount:", nsprogress::shim_nsprogress_set_total_unit_count as u64, "v@:q"),
            me("completedUnitCount", nsprogress::shim_nsprogress_completed_unit_count as u64, "q@:"),
            me("setCompletedUnitCount:", nsprogress::shim_nsprogress_set_completed_unit_count as u64, "v@:q"),
            me("fractionCompleted", nsprogress::shim_nsprogress_fraction_completed as u64, "d@:"),
            me("isCancellable", nsprogress::shim_nsprogress_is_cancellable as u64, "B@:"),
            me("setCancellable:", nsprogress::shim_nsprogress_set_cancellable as u64, "v@:B"),
            me("isCancelled", nsprogress::shim_nsprogress_is_cancelled as u64, "B@:"),
            me("cancel", nsprogress::shim_nsprogress_cancel as u64, "v@:"),
            me("localizedDescription", nsprogress::shim_nsprogress_localized_description as u64, "@@:"),
            me("setLocalizedDescription:", nsprogress::shim_nsprogress_set_localized_description as u64, "v@:@"),
            me("description", nsprogress::shim_nsprogress_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("progressWithTotalUnitCount:", nsprogress::shim_nsprogress_with_total as u64, "@@:q"),
            me("discreteProgressWithTotalUnitCount:", nsprogress::shim_nsprogress_discrete as u64, "@@:q"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 48,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSProgress");
}

/// Register `NSHost` with hostname resolution methods.
pub fn register_nshost_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nshost_class_addr();
    let metaclass_addr = nsobj::nshost_metaclass_addr();

    let info = ClassInfo {
        name: "NSHost".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nshost::shim_nshost_name as u64, "@@:"),
            me("address", nshost::shim_nshost_address as u64, "@@:"),
            me("addresses", nshost::shim_nshost_addresses as u64, "@@:"),
            me("names", nshost::shim_nshost_names as u64, "@@:"),
            me("localizedName", nshost::shim_nshost_localized_name as u64, "@@:"),
            me("isEqualToHost:", nshost::shim_nshost_is_equal as u64, "B@:@"),
            me("description", nshost::shim_nshost_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("currentHost", nshost::shim_nshost_current_host as u64, "@@:"),
            me("hostWithName:", nshost::shim_nshost_with_name as u64, "@@:@"),
            me("hostWithAddress:", nshost::shim_nshost_with_address as u64, "@@:@"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSHost");
}

/// Register `NSPipe` with pipe creation methods.
pub fn register_nspipe_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nspipe_class_addr();
    let metaclass_addr = nsobj::nspipe_metaclass_addr();

    let info = ClassInfo {
        name: "NSPipe".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("fileHandleForReading", nspipe::shim_nspipe_file_handle_for_reading as u64, "@@:"),
            me("fileHandleForWriting", nspipe::shim_nspipe_file_handle_for_writing as u64, "@@:"),
            me("description", nspipe::shim_nspipe_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("pipe", nspipe::shim_nspipe_pipe as u64, "@@:"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSPipe");
}

/// Register `NSURLRequest` with URL request accessors.
pub fn register_nsurlrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlrequest_class_addr();
    let metaclass_addr = nsobj::nsurlrequest_metaclass_addr();

    let info = ClassInfo {
        name: "NSURLRequest".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("URL", nsurlrequest::shim_nsurlrequest_url as u64, "@@:"),
            me("HTTPMethod", nsurlrequest::shim_nsurlrequest_http_method as u64, "@@:"),
            me("allHTTPHeaderFields", nsurlrequest::shim_nsurlrequest_all_http_header_fields as u64, "@@:"),
            me("description", nsurlrequest::shim_nsurlrequest_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("requestWithURL:", nsurlrequest::shim_nsurlrequest_with_url as u64, "@@:@"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSURLRequest");
}

/// Register `NSMutableURLRequest` with mutable URL request methods.
pub fn register_nsmutableurlrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutableurlrequest_class_addr();
    let metaclass_addr = nsobj::nsmutableurlrequest_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableURLRequest".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("setHTTPMethod:", nsurlrequest::shim_nsmutableurlrequest_set_http_method as u64, "v@:@"),
            me("setURL:", nsurlrequest::shim_nsmutableurlrequest_set_url as u64, "v@:@"),
            me("setValue:forHTTPHeaderField:", nsurlrequest::shim_nsmutableurlrequest_set_value_for_header as u64, "v@:@@"),
        ],
        class_methods: vec![
            me("requestWithURL:", nsurlrequest::shim_nsmutableurlrequest_with_url as u64, "@@:@"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsurlrequest_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableURLRequest");
}

/// Register `NSURLResponse` with URL response accessors.
pub fn register_nsurlresponse_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlresponse_class_addr();
    let metaclass_addr = nsobj::nsurlresponse_metaclass_addr();

    let info = ClassInfo {
        name: "NSURLResponse".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("URL", nsurlrequest::shim_nsurlresponse_url as u64, "@@:"),
            me("MIMEType", nsurlrequest::shim_nsurlresponse_mime_type as u64, "@@:"),
            me("expectedContentLength", nsurlrequest::shim_nsurlresponse_expected_content_length as u64, "q@:"),
            me("description", nsurlrequest::shim_nsurlresponse_description as u64, "@@:"),
        ],
        class_methods: vec![],
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
    rt.register_metaclass_addr(metaclass_addr, "NSURLResponse");
}

/// Register `NSHTTPURLResponse` with HTTP-specific response methods.
pub fn register_nshttpurlresponse_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nshttpurlresponse_class_addr();
    let metaclass_addr = nsobj::nshttpurlresponse_metaclass_addr();

    let info = ClassInfo {
        name: "NSHTTPURLResponse".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("statusCode", nsurlrequest::shim_nshttpurlresponse_status_code as u64, "q@:"),
            me("description", nsurlrequest::shim_nshttpurlresponse_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("localizedStringForStatusCode:", nsurlrequest::shim_nshttpurlresponse_localized_string as u64, "@@:q"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsurlresponse_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSHTTPURLResponse");
}

/// Register `NSTask` with subprocess execution methods.
pub fn register_nstask_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstask_class_addr();
    let metaclass_addr = nsobj::nstask_metaclass_addr();

    let info = ClassInfo {
        name: "NSTask".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nstask::shim_nstask_init as u64, "@@:"),
            me("setLaunchPath:", nstask::shim_nstask_set_launch_path as u64, "v@:@"),
            me("setExecutableURL:", nstask::shim_nstask_set_executable_url as u64, "v@:@"),
            me("setArguments:", nstask::shim_nstask_set_arguments as u64, "v@:@"),
            me("setEnvironment:", nstask::shim_nstask_set_environment as u64, "v@:@"),
            me("setStandardOutput:", nstask::shim_nstask_set_stdout as u64, "v@:@"),
            me("launchPath", nstask::shim_nstask_launch_path as u64, "@@:"),
            me("arguments", nstask::shim_nstask_arguments as u64, "@@:"),
            me("environment", nstask::shim_nstask_environment as u64, "@@:"),
            me("terminationStatus", nstask::shim_nstask_termination_status as u64, "i@:"),
            me("processIdentifier", nstask::shim_nstask_process_identifier as u64, "i@:"),
            me("isRunning", nstask::shim_nstask_is_running as u64, "B@:"),
            me("launch", nstask::shim_nstask_launch as u64, "v@:"),
            me("waitUntilExit", nstask::shim_nstask_wait_until_exit as u64, "v@:"),
            me("description", nstask::shim_nstask_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nstask::shim_nstask_alloc as u64, "@@:"),
            me("launchedTaskWithLaunchPath:arguments:", nstask::shim_nstask_launched_with_path as u64, "@@:@@"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 48,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTask");
}
