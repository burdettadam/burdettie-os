//! Synthetic Foundation class registration.
//!
//! Each `register_*_class()` function builds a `ClassInfo` with the
//! correct method table and registers it in the ObjC runtime.

use objc_runtime::{ClassInfo, MethodEntry};

use super::cfconstant_string_class_addr;
use crate::{
    cfstring, nsarray, nsbundle, nscalendar, nscharacterset, nsdata, nsdate, nsdateformatter,
    nsdictionary, nsenumerator, nserror, nsexception, nsfilemanager, nsindexset, nslocale, nslock,
    nsmutablearray, nsmutabledict, nsmutableset, nsmutablestring, nsnotificationcenter, nsnull,
    nsnumber, nsnumberformatter, nsobj, nsprocessinfo, nsset, nssortdescriptor, nsthread, nstimer,
    nstimezone, nsurl, nsuserdefaults, nsuuid, nsvalue,
};

/// Concise constructor for `MethodEntry`.
fn me(sel: &str, imp: u64, enc: &str) -> MethodEntry {
    MethodEntry {
        selector: sel.to_string(),
        imp,
        type_encoding: enc.to_string(),
    }
}

/// Register `__NSCFConstantString` so `objc_msgSend` can dispatch
/// `length`, `UTF8String`, and `description` on @"..." literals.
pub fn register_nsstring_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = cfconstant_string_class_addr();
    let nsstring_addr = nsobj::nsstring_class_addr();

    // Write superclass pointer at offset 8 of the static class data
    // SAFETY: class_addr points to the 40-byte CFCONSTANT_STRING_CLASS.
    unsafe {
        let ptr = class_addr as *mut u64;
        ptr.add(1).write(nsstring_addr);
    }

    let info = ClassInfo {
        name: "__NSCFConstantString".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("length", cfstring::shim_nsstring_length as u64, "Q@:"),
            me(
                "UTF8String",
                cfstring::shim_nsstring_utf8string as u64,
                "r*@:",
            ),
            me(
                "description",
                cfstring::shim_nsstring_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsstring_class_addr()),
    };
    rt.register_synthetic_class(info);
}

/// Register `NSString` with string methods and class factory methods.
pub fn register_nsstring_proper_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsstring_class_addr();
    let metaclass_addr = nsobj::nsstring_metaclass_addr();

    let info = ClassInfo {
        name: "NSString".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("length", cfstring::shim_nsstring_length as u64, "Q@:"),
            me(
                "UTF8String",
                cfstring::shim_nsstring_utf8string as u64,
                "r*@:",
            ),
            me(
                "description",
                cfstring::shim_nsstring_description as u64,
                "@@:",
            ),
            me(
                "initWithUTF8String:",
                cfstring::shim_nsstring_init_with_utf8string as u64,
                "@@:*",
            ),
            me(
                "isEqualToString:",
                cfstring::shim_nsstring_is_equal_to_string as u64,
                "B@:@",
            ),
            me(
                "hasPrefix:",
                cfstring::shim_nsstring_has_prefix as u64,
                "B@:@",
            ),
            me(
                "hasSuffix:",
                cfstring::shim_nsstring_has_suffix as u64,
                "B@:@",
            ),
            me(
                "stringByAppendingString:",
                cfstring::shim_nsstring_string_by_appending as u64,
                "@@:@",
            ),
            me(
                "substringFromIndex:",
                cfstring::shim_nsstring_substring_from_index as u64,
                "@@:Q",
            ),
            me(
                "substringToIndex:",
                cfstring::shim_nsstring_substring_to_index as u64,
                "@@:Q",
            ),
            me(
                "lowercaseString",
                cfstring::shim_nsstring_lowercase as u64,
                "@@:",
            ),
            me(
                "uppercaseString",
                cfstring::shim_nsstring_uppercase as u64,
                "@@:",
            ),
            me("compare:", cfstring::shim_nsstring_compare as u64, "q@:@"),
            me("copy", cfstring::shim_nsstring_copy as u64, "@@:"),
            me("hash", cfstring::shim_nsstring_hash as u64, "Q@:"),
            me(
                "stringByReplacingOccurrencesOfString:withString:",
                cfstring::shim_nsstring_string_by_replacing as u64,
                "@@:@@",
            ),
            me(
                "componentsSeparatedByString:",
                cfstring::shim_nsstring_components_separated_by_string as u64,
                "@@:@",
            ),
            me(
                "rangeOfString:",
                cfstring::shim_nsstring_range_of_string as u64,
                "{_NSRange=QQ}@:@",
            ),
            me(
                "integerValue",
                cfstring::shim_nsstring_integer_value as u64,
                "q@:",
            ),
            me(
                "doubleValue",
                cfstring::shim_nsstring_double_value as u64,
                "d@:",
            ),
            me(
                "mutableCopy",
                cfstring::shim_nsstring_mutable_copy as u64,
                "@@:",
            ),
            me(
                "pathExtension",
                cfstring::shim_nsstring_path_extension as u64,
                "@@:",
            ),
            me(
                "lastPathComponent",
                cfstring::shim_nsstring_last_path_component as u64,
                "@@:",
            ),
            me(
                "stringByDeletingLastPathComponent",
                cfstring::shim_nsstring_string_by_deleting_last_path_component as u64,
                "@@:",
            ),
            me(
                "stringByDeletingPathExtension",
                cfstring::shim_nsstring_string_by_deleting_path_extension as u64,
                "@@:",
            ),
            me(
                "stringByAppendingPathComponent:",
                cfstring::shim_nsstring_string_by_appending_path_component as u64,
                "@@:@",
            ),
            me(
                "isAbsolutePath",
                cfstring::shim_nsstring_is_absolute_path as u64,
                "B@:",
            ),
            me(
                "stringByTrimmingCharactersInSet:",
                cfstring::shim_nsstring_string_by_trimming_characters_in_set as u64,
                "@@:@",
            ),
            me(
                "componentsSeparatedByCharactersInSet:",
                cfstring::shim_nsstring_components_separated_by_characters_in_set as u64,
                "@@:@",
            ),
            me(
                "rangeOfCharacterFromSet:",
                cfstring::shim_nsstring_range_of_character_from_set as u64,
                "{_NSRange=QQ}@:@",
            ),
        ],
        class_methods: vec![
            me(
                "stringWithUTF8String:",
                cfstring::shim_nsstring_string_with_utf8string as u64,
                "@@:*",
            ),
            me(
                "stringWithFormat:",
                cfstring::string_with_format_trampoline_addr(),
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSString");
}

/// Register `NSObject` as root class with basic instance and class methods.
pub fn register_nsobject_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsobject_class_addr();
    let metaclass_addr = nsobj::nsobject_metaclass_addr();

    let info = ClassInfo {
        name: "NSObject".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsobj::shim_nsobject_init as u64, "@@:"),
            me("self", nsobj::shim_nsobject_self as u64, "@@:"),
            me("class", nsobj::shim_nsobject_class as u64, "#@:"),
            me(
                "respondsToSelector:",
                nsobj::shim_nsobject_responds_to_selector as u64,
                "B@::",
            ),
            me(
                "isKindOfClass:",
                nsobj::shim_nsobject_is_kind_of_class as u64,
                "B@:#",
            ),
            me("retain", objc_runtime::shim_nsobject_retain as u64, "@@:"),
            me("release", objc_runtime::shim_nsobject_release as u64, "v@:"),
            me(
                "autorelease",
                objc_runtime::shim_nsobject_autorelease as u64,
                "@@:",
            ),
            me(
                "retainCount",
                objc_runtime::shim_nsobject_retain_count as u64,
                "Q@:",
            ),
            me("dealloc", objc_runtime::shim_nsobject_dealloc as u64, "v@:"),
        ],
        class_methods: vec![
            me("alloc", nsobj::shim_nsobject_class_alloc as u64, "@@:"),
            me("new", nsobj::shim_nsobject_class_new as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 8,
        is_root: true,
        is_meta: false,
        superclass_addr: None,
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSObject");
}

/// Register `NSArray` with array methods and class factory.
pub fn register_nsarray_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsarray_class_addr();
    let metaclass_addr = nsobj::nsarray_metaclass_addr();

    let info = ClassInfo {
        name: "NSArray".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("count", nsarray::shim_nsarray_count as u64, "Q@:"),
            me(
                "objectAtIndex:",
                nsarray::shim_nsarray_object_at_index as u64,
                "@@:Q",
            ),
            me(
                "firstObject",
                nsarray::shim_nsarray_first_object as u64,
                "@@:",
            ),
            me(
                "lastObject",
                nsarray::shim_nsarray_last_object as u64,
                "@@:",
            ),
            me(
                "containsObject:",
                nsarray::shim_nsarray_contains_object as u64,
                "B@:@",
            ),
            me(
                "description",
                nsarray::shim_nsarray_description as u64,
                "@@:",
            ),
            me(
                "enumerateObjectsUsingBlock:",
                nsarray::shim_nsarray_enumerate_objects_using_block as u64,
                "v@:@?",
            ),
            me(
                "sortedArrayUsingComparator:",
                nsarray::shim_nsarray_sorted_array_using_comparator as u64,
                "@@:@?",
            ),
            me(
                "objectEnumerator",
                nsenumerator::shim_nsarray_object_enumerator as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "arrayWithObjects:count:",
            nsarray::shim_nsarray_array_with_objects_count as u64,
            "@@:^@Q",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSArray");
}

/// Register `NSNumber` with numeric accessors and factory methods.
pub fn register_nsnumber_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsnumber_class_addr();
    let metaclass_addr = nsobj::nsnumber_metaclass_addr();

    let info = ClassInfo {
        name: "NSNumber".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("intValue", nsnumber::shim_nsnumber_int_value as u64, "i@:"),
            me(
                "doubleValue",
                nsnumber::shim_nsnumber_double_value as u64,
                "d@:",
            ),
            me(
                "boolValue",
                nsnumber::shim_nsnumber_bool_value as u64,
                "B@:",
            ),
            me(
                "integerValue",
                nsnumber::shim_nsnumber_integer_value as u64,
                "q@:",
            ),
            me(
                "isEqualToNumber:",
                nsnumber::shim_nsnumber_is_equal_to_number as u64,
                "B@:@",
            ),
            me("compare:", nsnumber::shim_nsnumber_compare as u64, "q@:@"),
            me(
                "description",
                nsnumber::shim_nsnumber_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "numberWithInt:",
                nsnumber::shim_nsnumber_number_with_int as u64,
                "@@:i",
            ),
            me(
                "numberWithDouble:",
                nsnumber::shim_nsnumber_number_with_double as u64,
                "@@:d",
            ),
            me(
                "numberWithBool:",
                nsnumber::shim_nsnumber_number_with_bool as u64,
                "@@:B",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSNumber");
}

/// Register `NSDictionary` with dictionary methods and class factory.
pub fn register_nsdictionary_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdictionary_class_addr();
    let metaclass_addr = nsobj::nsdictionary_metaclass_addr();

    let info = ClassInfo {
        name: "NSDictionary".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("count", nsdictionary::shim_nsdictionary_count as u64, "Q@:"),
            me(
                "objectForKey:",
                nsdictionary::shim_nsdictionary_object_for_key as u64,
                "@@:@",
            ),
            me(
                "objectForKeyedSubscript:",
                nsdictionary::shim_nsdictionary_object_for_keyed_subscript as u64,
                "@@:@",
            ),
            me(
                "allKeys",
                nsdictionary::shim_nsdictionary_all_keys as u64,
                "@@:",
            ),
            me(
                "allValues",
                nsdictionary::shim_nsdictionary_all_values as u64,
                "@@:",
            ),
            me(
                "description",
                nsdictionary::shim_nsdictionary_description as u64,
                "@@:",
            ),
            me(
                "enumerateKeysAndObjectsUsingBlock:",
                nsdictionary::shim_nsdictionary_enumerate_keys_and_objects_using_block as u64,
                "v@:@?",
            ),
            me(
                "keyEnumerator",
                nsenumerator::shim_nsdictionary_key_enumerator as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "dictionaryWithObjects:forKeys:count:",
            nsdictionary::shim_nsdictionary_with_objects_for_keys_count as u64,
            "@@:^@^@Q",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDictionary");
}

/// Register `NSMutableArray` with mutation + inherited read methods.
pub fn register_nsmutablearray_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutablearray_class_addr();
    let metaclass_addr = nsobj::nsmutablearray_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableArray".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            // Inherited read methods (same layout as NSArray)
            me("count", nsarray::shim_nsarray_count as u64, "Q@:"),
            me(
                "objectAtIndex:",
                nsarray::shim_nsarray_object_at_index as u64,
                "@@:Q",
            ),
            me(
                "firstObject",
                nsarray::shim_nsarray_first_object as u64,
                "@@:",
            ),
            me(
                "lastObject",
                nsarray::shim_nsarray_last_object as u64,
                "@@:",
            ),
            me(
                "containsObject:",
                nsarray::shim_nsarray_contains_object as u64,
                "B@:@",
            ),
            // Mutation methods
            me(
                "addObject:",
                nsmutablearray::shim_nsmutablearray_add_object as u64,
                "v@:@",
            ),
            me(
                "insertObject:atIndex:",
                nsmutablearray::shim_nsmutablearray_insert_object_at_index as u64,
                "v@:@Q",
            ),
            me(
                "removeObjectAtIndex:",
                nsmutablearray::shim_nsmutablearray_remove_object_at_index as u64,
                "v@:Q",
            ),
            me(
                "removeLastObject",
                nsmutablearray::shim_nsmutablearray_remove_last_object as u64,
                "v@:",
            ),
            me(
                "replaceObjectAtIndex:withObject:",
                nsmutablearray::shim_nsmutablearray_replace_object_at_index as u64,
                "v@:Q@",
            ),
            me(
                "removeAllObjects",
                nsmutablearray::shim_nsmutablearray_remove_all_objects as u64,
                "v@:",
            ),
            me(
                "description",
                nsmutablearray::shim_nsmutablearray_description as u64,
                "@@:",
            ),
            me(
                "enumerateObjectsUsingBlock:",
                nsarray::shim_nsarray_enumerate_objects_using_block as u64,
                "v@:@?",
            ),
            me(
                "sortedArrayUsingComparator:",
                nsarray::shim_nsarray_sorted_array_using_comparator as u64,
                "@@:@?",
            ),
            me(
                "sortUsingComparator:",
                nsmutablearray::shim_nsmutablearray_sort_using_comparator as u64,
                "v@:@?",
            ),
            me(
                "objectEnumerator",
                nsenumerator::shim_nsarray_object_enumerator as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "array",
                nsmutablearray::shim_nsmutablearray_array as u64,
                "@@:",
            ),
            me(
                "arrayWithCapacity:",
                nsmutablearray::shim_nsmutablearray_array_with_capacity as u64,
                "@@:Q",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsarray_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableArray");
}

/// Register `NSMutableDictionary` with mutation + inherited read methods.
pub fn register_nsmutabledictionary_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutabledictionary_class_addr();
    let metaclass_addr = nsobj::nsmutabledictionary_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableDictionary".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            // Inherited read methods (same layout as NSDictionary)
            me("count", nsdictionary::shim_nsdictionary_count as u64, "Q@:"),
            me(
                "objectForKey:",
                nsdictionary::shim_nsdictionary_object_for_key as u64,
                "@@:@",
            ),
            me(
                "objectForKeyedSubscript:",
                nsdictionary::shim_nsdictionary_object_for_keyed_subscript as u64,
                "@@:@",
            ),
            me(
                "allKeys",
                nsdictionary::shim_nsdictionary_all_keys as u64,
                "@@:",
            ),
            me(
                "allValues",
                nsdictionary::shim_nsdictionary_all_values as u64,
                "@@:",
            ),
            me(
                "description",
                nsdictionary::shim_nsdictionary_description as u64,
                "@@:",
            ),
            me(
                "enumerateKeysAndObjectsUsingBlock:",
                nsdictionary::shim_nsdictionary_enumerate_keys_and_objects_using_block as u64,
                "v@:@?",
            ),
            me(
                "keyEnumerator",
                nsenumerator::shim_nsdictionary_key_enumerator as u64,
                "@@:",
            ),
            // Mutation methods
            me(
                "setObject:forKey:",
                nsmutabledict::shim_nsmutabledict_set_object_for_key as u64,
                "v@:@@",
            ),
            me(
                "setObject:forKeyedSubscript:",
                nsmutabledict::shim_nsmutabledict_set_object_for_keyed_subscript as u64,
                "v@:@@",
            ),
            me(
                "removeObjectForKey:",
                nsmutabledict::shim_nsmutabledict_remove_object_for_key as u64,
                "v@:@",
            ),
            me(
                "removeAllObjects",
                nsmutabledict::shim_nsmutabledict_remove_all_objects as u64,
                "v@:",
            ),
        ],
        class_methods: vec![
            me(
                "dictionary",
                nsmutabledict::shim_nsmutabledict_dictionary as u64,
                "@@:",
            ),
            me(
                "dictionaryWithCapacity:",
                nsmutabledict::shim_nsmutabledict_dictionary_with_capacity as u64,
                "@@:Q",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsdictionary_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableDictionary");
}

/// Register `NSMutableString` with mutation + inherited read methods.
pub fn register_nsmutablestring_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutablestring_class_addr();
    let metaclass_addr = nsobj::nsmutablestring_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableString".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            // Inherited read methods (same 32-byte layout as NSString)
            me("length", cfstring::shim_nsstring_length as u64, "Q@:"),
            me(
                "UTF8String",
                cfstring::shim_nsstring_utf8string as u64,
                "r*@:",
            ),
            me(
                "description",
                cfstring::shim_nsstring_description as u64,
                "@@:",
            ),
            me(
                "isEqualToString:",
                cfstring::shim_nsstring_is_equal_to_string as u64,
                "B@:@",
            ),
            me(
                "hasPrefix:",
                cfstring::shim_nsstring_has_prefix as u64,
                "B@:@",
            ),
            me(
                "hasSuffix:",
                cfstring::shim_nsstring_has_suffix as u64,
                "B@:@",
            ),
            me("hash", cfstring::shim_nsstring_hash as u64, "Q@:"),
            // Mutation methods
            me(
                "initWithString:",
                nsmutablestring::shim_nsmutablestring_init_with_string as u64,
                "@@:@",
            ),
            me(
                "appendString:",
                nsmutablestring::shim_nsmutablestring_append_string as u64,
                "v@:@",
            ),
            me(
                "insertString:atIndex:",
                nsmutablestring::shim_nsmutablestring_insert_string_at_index as u64,
                "v@:@Q",
            ),
            me(
                "deleteCharactersInRange:",
                nsmutablestring::shim_nsmutablestring_delete_characters_in_range as u64,
                "v@:{_NSRange=QQ}",
            ),
            me(
                "replaceCharactersInRange:withString:",
                nsmutablestring::shim_nsmutablestring_replace_characters_in_range as u64,
                "v@:{_NSRange=QQ}@",
            ),
            me(
                "setString:",
                nsmutablestring::shim_nsmutablestring_set_string as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![me(
            "string",
            nsmutablestring::shim_nsmutablestring_string as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsstring_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableString");
}

/// Register `NSData` with byte buffer methods and class factories.
pub fn register_nsdata_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdata_class_addr();
    let metaclass_addr = nsobj::nsdata_metaclass_addr();

    let info = ClassInfo {
        name: "NSData".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("length", nsdata::shim_nsdata_length as u64, "Q@:"),
            me("bytes", nsdata::shim_nsdata_bytes as u64, "^v@:"),
            me("description", nsdata::shim_nsdata_description as u64, "@@:"),
            me(
                "isEqualToData:",
                nsdata::shim_nsdata_is_equal_to_data as u64,
                "B@:@",
            ),
        ],
        class_methods: vec![
            me(
                "dataWithBytes:length:",
                nsdata::shim_nsdata_data_with_bytes_length as u64,
                "@@:^vQ",
            ),
            me("data", nsdata::shim_nsdata_data as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSData");
}

/// Register `NSProcessInfo` with process info accessors and singleton factory.
pub fn register_nsprocessinfo_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsprocessinfo_class_addr();
    let metaclass_addr = nsobj::nsprocessinfo_metaclass_addr();

    let info = ClassInfo {
        name: "NSProcessInfo".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "processName",
                nsprocessinfo::shim_nsprocessinfo_process_name as u64,
                "@@:",
            ),
            me(
                "processIdentifier",
                nsprocessinfo::shim_nsprocessinfo_process_identifier as u64,
                "i@:",
            ),
            me(
                "operatingSystemVersionString",
                nsprocessinfo::shim_nsprocessinfo_os_version_string as u64,
                "@@:",
            ),
            me(
                "arguments",
                nsprocessinfo::shim_nsprocessinfo_arguments as u64,
                "@@:",
            ),
            me(
                "environment",
                nsprocessinfo::shim_nsprocessinfo_environment as u64,
                "@@:",
            ),
            me(
                "hostName",
                nsprocessinfo::shim_nsprocessinfo_host_name as u64,
                "@@:",
            ),
            me(
                "description",
                nsprocessinfo::shim_nsprocessinfo_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "processInfo",
            nsprocessinfo::shim_nsprocessinfo_process_info as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSProcessInfo");
}

/// Register `NSError` with error accessors and factory methods.
pub fn register_nserror_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nserror_class_addr();
    let metaclass_addr = nsobj::nserror_metaclass_addr();

    let info = ClassInfo {
        name: "NSError".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("domain", nserror::shim_nserror_domain as u64, "@@:"),
            me("code", nserror::shim_nserror_code as u64, "q@:"),
            me("userInfo", nserror::shim_nserror_user_info as u64, "@@:"),
            me(
                "localizedDescription",
                nserror::shim_nserror_localized_description as u64,
                "@@:",
            ),
            me(
                "description",
                nserror::shim_nserror_description as u64,
                "@@:",
            ),
            me(
                "initWithDomain:code:userInfo:",
                nserror::shim_nserror_init_with_domain_code_userinfo as u64,
                "@@:@q@",
            ),
        ],
        class_methods: vec![me(
            "errorWithDomain:code:userInfo:",
            nserror::shim_nserror_error_with_domain_code_userinfo as u64,
            "@@:@q@",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSError");
}

/// Register `NSBundle` with bundle accessors and singleton factory.
pub fn register_nsbundle_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbundle_class_addr();
    let metaclass_addr = nsobj::nsbundle_metaclass_addr();

    let info = ClassInfo {
        name: "NSBundle".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "bundlePath",
                nsbundle::shim_nsbundle_bundle_path as u64,
                "@@:",
            ),
            me(
                "resourcePath",
                nsbundle::shim_nsbundle_resource_path as u64,
                "@@:",
            ),
            me(
                "executablePath",
                nsbundle::shim_nsbundle_executable_path as u64,
                "@@:",
            ),
            me(
                "bundleIdentifier",
                nsbundle::shim_nsbundle_bundle_identifier as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "mainBundle",
            nsbundle::shim_nsbundle_main_bundle as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSBundle");
}

/// Register `NSFileManager` with file system query methods and singleton factory.
pub fn register_nsfilemanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsfilemanager_class_addr();
    let metaclass_addr = nsobj::nsfilemanager_metaclass_addr();

    let info = ClassInfo {
        name: "NSFileManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "fileExistsAtPath:",
                nsfilemanager::shim_nsfilemanager_file_exists_at_path as u64,
                "B@:@",
            ),
            me(
                "fileExistsAtPath:isDirectory:",
                nsfilemanager::shim_nsfilemanager_file_exists_at_path_is_directory as u64,
                "B@:@^B",
            ),
            me(
                "currentDirectoryPath",
                nsfilemanager::shim_nsfilemanager_current_directory_path as u64,
                "@@:",
            ),
            me(
                "contentsOfDirectoryAtPath:error:",
                nsfilemanager::shim_nsfilemanager_contents_of_directory_at_path as u64,
                "@@:@^@",
            ),
        ],
        class_methods: vec![me(
            "defaultManager",
            nsfilemanager::shim_nsfilemanager_default_manager as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSFileManager");
}

/// Register `NSException` with exception accessors and factory methods.
pub fn register_nsexception_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsexception_class_addr();
    let metaclass_addr = nsobj::nsexception_metaclass_addr();

    let info = ClassInfo {
        name: "NSException".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nsexception::shim_nsexception_name as u64, "@@:"),
            me("reason", nsexception::shim_nsexception_reason as u64, "@@:"),
            me(
                "userInfo",
                nsexception::shim_nsexception_user_info as u64,
                "@@:",
            ),
            me(
                "description",
                nsexception::shim_nsexception_description as u64,
                "@@:",
            ),
            me("raise", nsexception::shim_nsexception_raise as u64, "v@:"),
            me(
                "initWithName:reason:userInfo:",
                nsexception::shim_nsexception_init_with_name_reason_userinfo as u64,
                "@@:@@@",
            ),
        ],
        class_methods: vec![me(
            "exceptionWithName:reason:userInfo:",
            nsexception::shim_nsexception_exception_with_name_reason_userinfo as u64,
            "@@:@@@",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSException");
}

/// Register `NSEnumerator` with iteration methods.
pub fn register_nsenumerator_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsenumerator_class_addr();
    let metaclass_addr = nsobj::nsenumerator_metaclass_addr();

    let info = ClassInfo {
        name: "NSEnumerator".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "nextObject",
                nsenumerator::shim_nsenumerator_next_object as u64,
                "@@:",
            ),
            me(
                "allObjects",
                nsenumerator::shim_nsenumerator_all_objects as u64,
                "@@:",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSEnumerator");
}

/// Register `NSDate` with date creation and comparison methods.
pub fn register_nsdate_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdate_class_addr();
    let metaclass_addr = nsobj::nsdate_metaclass_addr();

    let info = ClassInfo {
        name: "NSDate".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "timeIntervalSince1970",
                nsdate::shim_nsdate_ti_since_1970 as u64,
                "d@:",
            ),
            me(
                "timeIntervalSinceReferenceDate",
                nsdate::shim_nsdate_ti_since_ref as u64,
                "d@:",
            ),
            me(
                "timeIntervalSinceNow",
                nsdate::shim_nsdate_ti_since_now as u64,
                "d@:",
            ),
            me(
                "timeIntervalSinceDate:",
                nsdate::shim_nsdate_ti_since_date as u64,
                "d@:@",
            ),
            me(
                "dateByAddingTimeInterval:",
                nsdate::shim_nsdate_date_by_adding as u64,
                "@@:d",
            ),
            me(
                "isEqualToDate:",
                nsdate::shim_nsdate_is_equal_to_date as u64,
                "B@:@",
            ),
            me("compare:", nsdate::shim_nsdate_compare as u64, "q@:@"),
            me("description", nsdate::shim_nsdate_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("date", nsdate::shim_nsdate_date as u64, "@@:"),
            me(
                "dateWithTimeIntervalSince1970:",
                nsdate::shim_nsdate_date_with_ti_since_1970 as u64,
                "@@:d",
            ),
            me(
                "dateWithTimeIntervalSinceReferenceDate:",
                nsdate::shim_nsdate_date_with_ti_since_ref as u64,
                "@@:d",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDate");
}

/// Register `NSUserDefaults` with key-value storage methods.
pub fn register_nsuserdefaults_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsuserdefaults_class_addr();
    let metaclass_addr = nsobj::nsuserdefaults_metaclass_addr();

    let info = ClassInfo {
        name: "NSUserDefaults".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "objectForKey:",
                nsuserdefaults::shim_nsuserdefaults_object_for_key as u64,
                "@@:@",
            ),
            me(
                "stringForKey:",
                nsuserdefaults::shim_nsuserdefaults_string_for_key as u64,
                "@@:@",
            ),
            me(
                "integerForKey:",
                nsuserdefaults::shim_nsuserdefaults_integer_for_key as u64,
                "q@:@",
            ),
            me(
                "boolForKey:",
                nsuserdefaults::shim_nsuserdefaults_bool_for_key as u64,
                "B@:@",
            ),
            me(
                "doubleForKey:",
                nsuserdefaults::shim_nsuserdefaults_double_for_key as u64,
                "d@:@",
            ),
            me(
                "setObject:forKey:",
                nsuserdefaults::shim_nsuserdefaults_set_object as u64,
                "v@:@@",
            ),
            me(
                "removeObjectForKey:",
                nsuserdefaults::shim_nsuserdefaults_remove_object as u64,
                "v@:@",
            ),
            me(
                "synchronize",
                nsuserdefaults::shim_nsuserdefaults_synchronize as u64,
                "B@:",
            ),
        ],
        class_methods: vec![me(
            "standardUserDefaults",
            nsuserdefaults::shim_nsuserdefaults_standard as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSUserDefaults");
}

/// Register `NSSet` with immutable set methods.
pub fn register_nsset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsset_class_addr();
    let metaclass_addr = nsobj::nsset_metaclass_addr();

    let info = ClassInfo {
        name: "NSSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("count", nsset::shim_nsset_count as u64, "Q@:"),
            me(
                "containsObject:",
                nsset::shim_nsset_contains_object as u64,
                "B@:@",
            ),
            me("member:", nsset::shim_nsset_member as u64, "@@:@"),
            me("allObjects", nsset::shim_nsset_all_objects as u64, "@@:"),
            me("anyObject", nsset::shim_nsset_any_object as u64, "@@:"),
            me(
                "isSubsetOfSet:",
                nsset::shim_nsset_is_subset_of_set as u64,
                "B@:@",
            ),
            me(
                "intersectsSet:",
                nsset::shim_nsset_intersects_set as u64,
                "B@:@",
            ),
            me(
                "isEqualToSet:",
                nsset::shim_nsset_is_equal_to_set as u64,
                "B@:@",
            ),
            me("description", nsset::shim_nsset_description as u64, "@@:"),
            me(
                "enumerateObjectsUsingBlock:",
                nsset::shim_nsset_enumerate_objects_using_block as u64,
                "v@:@?",
            ),
        ],
        class_methods: vec![
            me(
                "setWithObjects:count:",
                nsset::shim_nsset_with_objects_count as u64,
                "@@:^@Q",
            ),
            me("set", nsset::shim_nsset_set as u64, "@@:"),
            me(
                "setWithArray:",
                nsset::shim_nsset_set_with_array as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSSet");
}

/// Register `NSMutableSet` with mutable set methods.
pub fn register_nsmutableset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutableset_class_addr();
    let metaclass_addr = nsobj::nsmutableset_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("count", nsset::shim_nsset_count as u64, "Q@:"),
            me(
                "containsObject:",
                nsset::shim_nsset_contains_object as u64,
                "B@:@",
            ),
            me("member:", nsset::shim_nsset_member as u64, "@@:@"),
            me("allObjects", nsset::shim_nsset_all_objects as u64, "@@:"),
            me("anyObject", nsset::shim_nsset_any_object as u64, "@@:"),
            me(
                "isSubsetOfSet:",
                nsset::shim_nsset_is_subset_of_set as u64,
                "B@:@",
            ),
            me(
                "intersectsSet:",
                nsset::shim_nsset_intersects_set as u64,
                "B@:@",
            ),
            me(
                "isEqualToSet:",
                nsset::shim_nsset_is_equal_to_set as u64,
                "B@:@",
            ),
            me("description", nsset::shim_nsset_description as u64, "@@:"),
            me(
                "enumerateObjectsUsingBlock:",
                nsset::shim_nsset_enumerate_objects_using_block as u64,
                "v@:@?",
            ),
            me(
                "addObject:",
                nsmutableset::shim_nsmutableset_add_object as u64,
                "v@:@",
            ),
            me(
                "removeObject:",
                nsmutableset::shim_nsmutableset_remove_object as u64,
                "v@:@",
            ),
            me(
                "removeAllObjects",
                nsmutableset::shim_nsmutableset_remove_all_objects as u64,
                "v@:",
            ),
            me(
                "unionSet:",
                nsmutableset::shim_nsmutableset_union_set as u64,
                "v@:@",
            ),
            me(
                "intersectSet:",
                nsmutableset::shim_nsmutableset_intersect_set as u64,
                "v@:@",
            ),
            me(
                "minusSet:",
                nsmutableset::shim_nsmutableset_minus_set as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![
            me("set", nsmutableset::shim_nsmutableset_set as u64, "@@:"),
            me(
                "setWithCapacity:",
                nsmutableset::shim_nsmutableset_set_with_capacity as u64,
                "@@:Q",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsset_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableSet");
}

/// Register `NSURL` with URL methods.
pub fn register_nsurl_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurl_class_addr();
    let metaclass_addr = nsobj::nsurl_metaclass_addr();

    let info = ClassInfo {
        name: "NSURL".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "absoluteString",
                nsurl::shim_nsurl_absolute_string as u64,
                "@@:",
            ),
            me("path", nsurl::shim_nsurl_path as u64, "@@:"),
            me(
                "lastPathComponent",
                nsurl::shim_nsurl_last_path_component as u64,
                "@@:",
            ),
            me(
                "pathExtension",
                nsurl::shim_nsurl_path_extension as u64,
                "@@:",
            ),
            me(
                "URLByAppendingPathComponent:",
                nsurl::shim_nsurl_url_by_appending_path_component as u64,
                "@@:@",
            ),
            me("isFileURL", nsurl::shim_nsurl_is_file_url as u64, "B@:"),
            me("scheme", nsurl::shim_nsurl_scheme as u64, "@@:"),
            me("description", nsurl::shim_nsurl_description as u64, "@@:"),
        ],
        class_methods: vec![
            me(
                "fileURLWithPath:",
                nsurl::shim_nsurl_file_url_with_path as u64,
                "@@:@",
            ),
            me(
                "URLWithString:",
                nsurl::shim_nsurl_url_with_string as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSURL");
}

/// Register `NSNull` with the singleton null placeholder.
pub fn register_nsnull_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsnull_class_addr();
    let metaclass_addr = nsobj::nsnull_metaclass_addr();

    let info = ClassInfo {
        name: "NSNull".to_string(),
        mapped_addr: class_addr,
        methods: vec![me(
            "description",
            nsnull::shim_nsnull_description as u64,
            "@@:",
        )],
        class_methods: vec![me("null", nsnull::shim_nsnull_null as u64, "@@:")],
        ivars: vec![],
        properties: vec![],
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSNull");
}

/// Register `NSDateFormatter` with date formatting methods.
pub fn register_nsdateformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdateformatter_class_addr();
    let metaclass_addr = nsobj::nsdateformatter_metaclass_addr();

    let info = ClassInfo {
        name: "NSDateFormatter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nsdateformatter::shim_nsdateformatter_init as u64,
                "@@:",
            ),
            me(
                "setDateFormat:",
                nsdateformatter::shim_nsdateformatter_set_date_format as u64,
                "v@:@",
            ),
            me(
                "dateFormat",
                nsdateformatter::shim_nsdateformatter_date_format as u64,
                "@@:",
            ),
            me(
                "setDateStyle:",
                nsdateformatter::shim_nsdateformatter_set_date_style as u64,
                "v@:Q",
            ),
            me(
                "dateStyle",
                nsdateformatter::shim_nsdateformatter_date_style as u64,
                "Q@:",
            ),
            me(
                "setTimeStyle:",
                nsdateformatter::shim_nsdateformatter_set_time_style as u64,
                "v@:Q",
            ),
            me(
                "timeStyle",
                nsdateformatter::shim_nsdateformatter_time_style as u64,
                "Q@:",
            ),
            me(
                "stringFromDate:",
                nsdateformatter::shim_nsdateformatter_string_from_date as u64,
                "@@:@",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDateFormatter");
}

/// Register `NSCharacterSet` with character membership methods.
pub fn register_nscharacterset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscharacterset_class_addr();
    let metaclass_addr = nsobj::nscharacterset_metaclass_addr();

    let info = ClassInfo {
        name: "NSCharacterSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![me(
            "characterIsMember:",
            nscharacterset::shim_nscharacterset_character_is_member as u64,
            "B@:S",
        )],
        class_methods: vec![
            me(
                "whitespaceCharacterSet",
                nscharacterset::shim_nscharacterset_whitespace as u64,
                "@@:",
            ),
            me(
                "whitespaceAndNewlineCharacterSet",
                nscharacterset::shim_nscharacterset_whitespace_and_newline as u64,
                "@@:",
            ),
            me(
                "decimalDigitCharacterSet",
                nscharacterset::shim_nscharacterset_decimal_digit as u64,
                "@@:",
            ),
            me(
                "letterCharacterSet",
                nscharacterset::shim_nscharacterset_letter as u64,
                "@@:",
            ),
            me(
                "lowercaseLetterCharacterSet",
                nscharacterset::shim_nscharacterset_lowercase_letter as u64,
                "@@:",
            ),
            me(
                "uppercaseLetterCharacterSet",
                nscharacterset::shim_nscharacterset_uppercase_letter as u64,
                "@@:",
            ),
            me(
                "alphanumericCharacterSet",
                nscharacterset::shim_nscharacterset_alphanumeric as u64,
                "@@:",
            ),
            me(
                "punctuationCharacterSet",
                nscharacterset::shim_nscharacterset_punctuation as u64,
                "@@:",
            ),
            me(
                "newlineCharacterSet",
                nscharacterset::shim_nscharacterset_newline as u64,
                "@@:",
            ),
            me(
                "characterSetWithCharactersInString:",
                nscharacterset::shim_nscharacterset_with_characters_in_string as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCharacterSet");
}

/// Register `NSNotificationCenter` with notification dispatch methods.
pub fn register_nsnotificationcenter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsnotificationcenter_class_addr();
    let metaclass_addr = nsobj::nsnotificationcenter_metaclass_addr();

    let info = ClassInfo {
        name: "NSNotificationCenter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("postNotificationName:object:",
                nsnotificationcenter::shim_nsnotificationcenter_post_notification_name_object as u64, "v@:@@"),
            me("postNotificationName:object:userInfo:",
                nsnotificationcenter::shim_nsnotificationcenter_post_notification_name_object_userinfo as u64, "v@:@@@"),
            me("addObserver:selector:name:object:",
                nsnotificationcenter::shim_nsnotificationcenter_add_observer as u64,               "v@:@:@@"),
            me("removeObserver:",
                nsnotificationcenter::shim_nsnotificationcenter_remove_observer as u64,            "v@:@"),
            me("removeObserver:name:object:",
                nsnotificationcenter::shim_nsnotificationcenter_remove_observer_name_object as u64, "v@:@@@"),
        ],
        class_methods: vec![
            me("defaultCenter", nsnotificationcenter::shim_nsnotificationcenter_default_center as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSNotificationCenter");
}

/// Register `NSValue` with value wrapping methods.
pub fn register_nsvalue_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsvalue_class_addr();
    let metaclass_addr = nsobj::nsvalue_metaclass_addr();

    let info = ClassInfo {
        name: "NSValue".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "rangeValue",
                nsvalue::shim_nsvalue_range_value as u64,
                "Q@:",
            ),
            me(
                "pointerValue",
                nsvalue::shim_nsvalue_pointer_value as u64,
                "^v@:",
            ),
            me("objCType", nsvalue::shim_nsvalue_objc_type as u64, "r*@:"),
            me(
                "description",
                nsvalue::shim_nsvalue_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "valueWithBytes:objCType:",
                nsvalue::shim_nsvalue_value_with_bytes as u64,
                "@@:r^v r*",
            ),
            me(
                "valueWithRange:",
                nsvalue::shim_nsvalue_value_with_range as u64,
                "@@:QQ",
            ),
            me(
                "valueWithPointer:",
                nsvalue::shim_nsvalue_value_with_pointer as u64,
                "@@:^v",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSValue");
}

/// Register `NSUUID` with UUID generation and comparison methods.
pub fn register_nsuuid_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsuuid_class_addr();
    let metaclass_addr = nsobj::nsuuid_metaclass_addr();

    let info = ClassInfo {
        name: "NSUUID".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithUUIDString:",
                nsuuid::shim_nsuuid_init_with_uuid_string as u64,
                "@@:@",
            ),
            me("UUIDString", nsuuid::shim_nsuuid_uuid_string as u64, "@@:"),
            me("isEqual:", nsuuid::shim_nsuuid_is_equal as u64, "B@:@"),
            me("description", nsuuid::shim_nsuuid_description as u64, "@@:"),
        ],
        class_methods: vec![me("UUID", nsuuid::shim_nsuuid_uuid as u64, "@@:")],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSUUID");
}

/// Register `NSLocale` with locale information methods.
pub fn register_nslocale_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nslocale_class_addr();
    let metaclass_addr = nsobj::nslocale_metaclass_addr();

    let info = ClassInfo {
        name: "NSLocale".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "localeIdentifier",
                nslocale::shim_nslocale_locale_identifier as u64,
                "@@:",
            ),
            me(
                "languageCode",
                nslocale::shim_nslocale_language_code as u64,
                "@@:",
            ),
            me(
                "countryCode",
                nslocale::shim_nslocale_country_code as u64,
                "@@:",
            ),
            me(
                "objectForKey:",
                nslocale::shim_nslocale_object_for_key as u64,
                "@@:@",
            ),
            me(
                "displayNameForKey:value:",
                nslocale::shim_nslocale_display_name_for_key_value as u64,
                "@@:@@",
            ),
            me(
                "description",
                nslocale::shim_nslocale_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "currentLocale",
            nslocale::shim_nslocale_current_locale as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSLocale");
}

/// Register `NSTimeZone` with timezone query methods.
pub fn register_nstimezone_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstimezone_class_addr();
    let metaclass_addr = nsobj::nstimezone_metaclass_addr();

    let info = ClassInfo {
        name: "NSTimeZone".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nstimezone::shim_nstimezone_name as u64, "@@:"),
            me(
                "abbreviation",
                nstimezone::shim_nstimezone_abbreviation as u64,
                "@@:",
            ),
            me(
                "secondsFromGMT",
                nstimezone::shim_nstimezone_seconds_from_gmt as u64,
                "q@:",
            ),
            me(
                "description",
                nstimezone::shim_nstimezone_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "systemTimeZone",
                nstimezone::shim_nstimezone_system_timezone as u64,
                "@@:",
            ),
            me(
                "localTimeZone",
                nstimezone::shim_nstimezone_local_timezone as u64,
                "@@:",
            ),
            me(
                "timeZoneWithName:",
                nstimezone::shim_nstimezone_with_name as u64,
                "@@:@",
            ),
            me(
                "timeZoneWithAbbreviation:",
                nstimezone::shim_nstimezone_with_abbreviation as u64,
                "@@:@",
            ),
            me(
                "timeZoneForSecondsFromGMT:",
                nstimezone::shim_nstimezone_for_seconds_from_gmt as u64,
                "@@:q",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTimeZone");
}

/// Register `NSCalendar` with calendar computation methods.
pub fn register_nscalendar_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscalendar_class_addr();
    let metaclass_addr = nsobj::nscalendar_metaclass_addr();

    let info = ClassInfo {
        name: "NSCalendar".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "calendarIdentifier",
                nscalendar::shim_nscalendar_identifier as u64,
                "@@:",
            ),
            me(
                "components:fromDate:",
                nscalendar::shim_nscalendar_components_from_date as u64,
                "@@:Q@",
            ),
            me(
                "component:fromDate:",
                nscalendar::shim_nscalendar_component_from_date as u64,
                "q@:Q@",
            ),
            me(
                "dateFromComponents:",
                nscalendar::shim_nscalendar_date_from_components as u64,
                "@@:@",
            ),
            me(
                "dateByAddingComponents:toDate:options:",
                nscalendar::shim_nscalendar_date_by_adding_components as u64,
                "@@:@@Q",
            ),
            me(
                "description",
                nscalendar::shim_nscalendar_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "currentCalendar",
                nscalendar::shim_nscalendar_current_calendar as u64,
                "@@:",
            ),
            me(
                "calendarWithIdentifier:",
                nscalendar::shim_nscalendar_with_identifier as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCalendar");
}

/// Register `NSNumberFormatter` with number formatting/parsing methods.
pub fn register_nsnumberformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsnumberformatter_class_addr();
    let metaclass_addr = nsobj::nsnumberformatter_metaclass_addr();

    let info = ClassInfo {
        name: "NSNumberFormatter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nsnumberformatter::shim_nsnumberformatter_init as u64,
                "@@:",
            ),
            me(
                "setNumberStyle:",
                nsnumberformatter::shim_nsnumberformatter_set_number_style as u64,
                "v@:q",
            ),
            me(
                "numberStyle",
                nsnumberformatter::shim_nsnumberformatter_number_style as u64,
                "q@:",
            ),
            me(
                "setMinimumFractionDigits:",
                nsnumberformatter::shim_nsnumberformatter_set_min_frac as u64,
                "v@:Q",
            ),
            me(
                "setMaximumFractionDigits:",
                nsnumberformatter::shim_nsnumberformatter_set_max_frac as u64,
                "v@:Q",
            ),
            me(
                "stringFromNumber:",
                nsnumberformatter::shim_nsnumberformatter_string_from_number as u64,
                "@@:@",
            ),
            me(
                "numberFromString:",
                nsnumberformatter::shim_nsnumberformatter_number_from_string as u64,
                "@@:@",
            ),
            me(
                "description",
                nsnumberformatter::shim_nsnumberformatter_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSNumberFormatter");
}

/// Register `NSIndexSet` with index collection methods.
pub fn register_nsindexset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsindexset_class_addr();
    let metaclass_addr = nsobj::nsindexset_metaclass_addr();

    let info = ClassInfo {
        name: "NSIndexSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("count", nsindexset::shim_nsindexset_count as u64, "Q@:"),
            me(
                "containsIndex:",
                nsindexset::shim_nsindexset_contains_index as u64,
                "B@:Q",
            ),
            me(
                "firstIndex",
                nsindexset::shim_nsindexset_first_index as u64,
                "Q@:",
            ),
            me(
                "lastIndex",
                nsindexset::shim_nsindexset_last_index as u64,
                "Q@:",
            ),
            me(
                "containsIndexesInRange:",
                nsindexset::shim_nsindexset_contains_indexes_in_range as u64,
                "B@:QQ",
            ),
            me(
                "description",
                nsindexset::shim_nsindexset_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "indexSet",
                nsindexset::shim_nsindexset_index_set as u64,
                "@@:",
            ),
            me(
                "indexSetWithIndex:",
                nsindexset::shim_nsindexset_with_index as u64,
                "@@:Q",
            ),
            me(
                "indexSetWithIndexesInRange:",
                nsindexset::shim_nsindexset_with_range as u64,
                "@@:QQ",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSIndexSet");
}

/// Register `NSMutableIndexSet` with mutable index collection methods.
pub fn register_nsmutableindexset_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutableindexset_class_addr();
    let metaclass_addr = nsobj::nsmutableindexset_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableIndexSet".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nsindexset::shim_nsmutableindexset_init as u64,
                "@@:",
            ),
            me(
                "addIndex:",
                nsindexset::shim_nsmutableindexset_add_index as u64,
                "v@:Q",
            ),
            me(
                "addIndexesInRange:",
                nsindexset::shim_nsmutableindexset_add_indexes_in_range as u64,
                "v@:QQ",
            ),
            me(
                "removeIndex:",
                nsindexset::shim_nsmutableindexset_remove_index as u64,
                "v@:Q",
            ),
            me(
                "removeAllIndexes",
                nsindexset::shim_nsmutableindexset_remove_all as u64,
                "v@:",
            ),
        ],
        class_methods: vec![me(
            "indexSet",
            nsindexset::shim_nsmutableindexset_index_set as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsindexset_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableIndexSet");
}

/// Register `NSSortDescriptor` with key-path sorting methods.
pub fn register_nssortdescriptor_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nssortdescriptor_class_addr();
    let metaclass_addr = nsobj::nssortdescriptor_metaclass_addr();

    let info = ClassInfo {
        name: "NSSortDescriptor".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "key",
                nssortdescriptor::shim_nssortdescriptor_key as u64,
                "@@:",
            ),
            me(
                "ascending",
                nssortdescriptor::shim_nssortdescriptor_ascending as u64,
                "B@:",
            ),
            me(
                "compareObject:toObject:",
                nssortdescriptor::shim_nssortdescriptor_compare_object as u64,
                "q@:@@",
            ),
            me(
                "reversedSortDescriptor",
                nssortdescriptor::shim_nssortdescriptor_reversed as u64,
                "@@:",
            ),
            me(
                "description",
                nssortdescriptor::shim_nssortdescriptor_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "sortDescriptorWithKey:ascending:",
            nssortdescriptor::shim_nssortdescriptor_with_key_ascending as u64,
            "@@:@B",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSSortDescriptor");
}

/// Register `NSTimer` with timer lifecycle methods.
pub fn register_nstimer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstimer_class_addr();
    let metaclass_addr = nsobj::nstimer_metaclass_addr();

    let info = ClassInfo {
        name: "NSTimer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("fire", nstimer::shim_nstimer_fire as u64, "v@:"),
            me("invalidate", nstimer::shim_nstimer_invalidate as u64, "v@:"),
            me("isValid", nstimer::shim_nstimer_is_valid as u64, "B@:"),
            me(
                "timeInterval",
                nstimer::shim_nstimer_time_interval as u64,
                "d@:",
            ),
            me(
                "description",
                nstimer::shim_nstimer_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "scheduledTimerWithTimeInterval:repeats:block:",
                nstimer::shim_nstimer_scheduled_with_block as u64,
                "@@:dB@?",
            ),
            me(
                "timerWithTimeInterval:repeats:block:",
                nstimer::shim_nstimer_timer_with_block as u64,
                "@@:dB@?",
            ),
            me(
                "scheduledTimerWithTimeInterval:target:selector:userInfo:repeats:",
                nstimer::shim_nstimer_scheduled_with_target as u64,
                "@@:d@:@B",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTimer");
}

/// Register `NSThread` with thread management methods.
pub fn register_nsthread_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsthread_class_addr();
    let metaclass_addr = nsobj::nsthread_metaclass_addr();

    let info = ClassInfo {
        name: "NSThread".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "isMainThread",
                nsthread::shim_nsthread_is_main_thread as u64,
                "B@:",
            ),
            me("name", nsthread::shim_nsthread_name as u64, "@@:"),
            me("setName:", nsthread::shim_nsthread_set_name as u64, "v@:@"),
            me(
                "description",
                nsthread::shim_nsthread_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "currentThread",
                nsthread::shim_nsthread_current_thread as u64,
                "@@:",
            ),
            me(
                "isMainThread",
                nsthread::shim_nsthread_is_main_thread_class as u64,
                "B@:",
            ),
            me(
                "isMultiThreaded",
                nsthread::shim_nsthread_is_multi_threaded as u64,
                "B@:",
            ),
            me(
                "sleepForTimeInterval:",
                nsthread::shim_nsthread_sleep_for_time_interval as u64,
                "v@:d",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSThread");
}

/// Register `NSLock` with locking methods.
pub fn register_nslock_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nslock_class_addr();
    let metaclass_addr = nsobj::nslock_metaclass_addr();

    let info = ClassInfo {
        name: "NSLock".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nslock::shim_nslock_init as u64, "@@:"),
            me("lock", nslock::shim_nslock_lock as u64, "v@:"),
            me("unlock", nslock::shim_nslock_unlock as u64, "v@:"),
            me("tryLock", nslock::shim_nslock_try_lock as u64, "B@:"),
            me("name", nslock::shim_nslock_name as u64, "@@:"),
            me("setName:", nslock::shim_nslock_set_name as u64, "v@:@"),
            me("dealloc", nslock::shim_nslock_dealloc as u64, "v@:"),
            me("description", nslock::shim_nslock_description as u64, "@@:"),
        ],
        class_methods: vec![me("alloc", nslock::shim_nslock_alloc as u64, "@@:")],
        ivars: vec![],
        properties: vec![],
        instance_size: 80,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSLock");
}

/// Register `NSRecursiveLock`.
pub fn register_nsrecursivelock_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsrecursivelock_class_addr();
    let metaclass_addr = nsobj::nsrecursivelock_metaclass_addr();

    let info = ClassInfo {
        name: "NSRecursiveLock".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nslock::shim_nsrecursivelock_init as u64, "@@:"),
            me("lock", nslock::shim_nsrecursivelock_lock as u64, "v@:"),
            me("unlock", nslock::shim_nsrecursivelock_unlock as u64, "v@:"),
            me(
                "tryLock",
                nslock::shim_nsrecursivelock_try_lock as u64,
                "B@:",
            ),
            me(
                "dealloc",
                nslock::shim_nsrecursivelock_dealloc as u64,
                "v@:",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nslock::shim_nsrecursivelock_alloc as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 80,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSRecursiveLock");
}

/// Register `NSCondition`.
pub fn register_nscondition_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscondition_class_addr();
    let metaclass_addr = nsobj::nscondition_metaclass_addr();

    let info = ClassInfo {
        name: "NSCondition".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nslock::shim_nscondition_init as u64, "@@:"),
            me("lock", nslock::shim_nscondition_lock as u64, "v@:"),
            me("unlock", nslock::shim_nscondition_unlock as u64, "v@:"),
            me("wait", nslock::shim_nscondition_wait as u64, "v@:"),
            me("signal", nslock::shim_nscondition_signal as u64, "v@:"),
            me(
                "broadcast",
                nslock::shim_nscondition_broadcast as u64,
                "v@:",
            ),
            me("name", nslock::shim_nscondition_name as u64, "@@:"),
            me("setName:", nslock::shim_nscondition_set_name as u64, "v@:@"),
            me("dealloc", nslock::shim_nscondition_dealloc as u64, "v@:"),
        ],
        class_methods: vec![me("alloc", nslock::shim_nscondition_alloc as u64, "@@:")],
        ivars: vec![],
        properties: vec![],
        instance_size: 128,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCondition");
}

/// Register `NSConditionLock`.
pub fn register_nsconditionlock_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsconditionlock_class_addr();
    let metaclass_addr = nsobj::nsconditionlock_metaclass_addr();

    let info = ClassInfo {
        name: "NSConditionLock".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nslock::shim_nsconditionlock_init as u64, "@@:"),
            me(
                "initWithCondition:",
                nslock::shim_nsconditionlock_init_with_condition as u64,
                "@@:q",
            ),
            me(
                "condition",
                nslock::shim_nsconditionlock_condition as u64,
                "q@:",
            ),
            me("lock", nslock::shim_nsconditionlock_lock as u64, "v@:"),
            me("unlock", nslock::shim_nsconditionlock_unlock as u64, "v@:"),
            me(
                "lockWhenCondition:",
                nslock::shim_nsconditionlock_lock_when_condition as u64,
                "v@:q",
            ),
            me(
                "unlockWithCondition:",
                nslock::shim_nsconditionlock_unlock_with_condition as u64,
                "v@:q",
            ),
            me(
                "tryLock",
                nslock::shim_nsconditionlock_try_lock as u64,
                "B@:",
            ),
            me("name", nslock::shim_nsconditionlock_name as u64, "@@:"),
            me(
                "setName:",
                nslock::shim_nsconditionlock_set_name as u64,
                "v@:@",
            ),
            me(
                "dealloc",
                nslock::shim_nsconditionlock_dealloc as u64,
                "v@:",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nslock::shim_nsconditionlock_alloc as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        instance_size: 136,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSConditionLock");
}
