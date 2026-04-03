use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::foundation::cfconstant_string_class_addr;
use crate::{cfstring, nsmutablestring, nsobj};

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
                "cStringUsingEncoding:",
                cfstring::shim_nsstring_cstring_using_encoding as u64,
                "r*:@Q",
            ),
            me(
                "description",
                cfstring::shim_nsstring_description as u64,
                "@@:",
            ),
            me("path", cfstring::shim_nsstring_path as u64, "@@:"),
            me(
                "dataUsingEncoding:",
                cfstring::shim_nsstring_data_using_encoding as u64,
                "@@:Q",
            ),
            me(
                "dataUsingEncoding:allowLossyConversion:",
                cfstring::shim_nsstring_data_using_encoding_lossy as u64,
                "@@:QB",
            ),
            me(
                "stringByResolvingSymlinksInPath",
                cfstring::shim_nsstring_string_by_resolving_symlinks_in_path as u64,
                "@@:",
            ),
            me(
                "stringByStandardizingPath",
                cfstring::shim_nsstring_string_by_standardizing_path as u64,
                "@@:",
            ),
            me(
                "stringByExpandingTildeInPath",
                cfstring::shim_nsstring_string_by_expanding_tilde_in_path as u64,
                "@@:",
            ),
            me(
                "stringByAppendingPathExtension:",
                cfstring::shim_nsstring_string_by_appending_path_extension as u64,
                "@@:@",
            ),
            me(
                "pathComponents",
                cfstring::shim_nsstring_path_components as u64,
                "@@:",
            ),
            me(
                "localizedCaseInsensitiveCompare:",
                cfstring::shim_nsstring_localized_case_insensitive_compare as u64,
                "q@:@",
            ),
            me(
                "localizedCompare:",
                cfstring::shim_nsstring_localized_compare as u64,
                "q@:@",
            ),
            me(
                "precomposedStringWithCanonicalMapping",
                cfstring::shim_nsstring_precomposed_string_with_canonical_mapping as u64,
                "@@:",
            ),
            me(
                "decomposedStringWithCanonicalMapping",
                cfstring::shim_nsstring_decomposed_string_with_canonical_mapping as u64,
                "@@:",
            ),
            me(
                "stringByAddingPercentEncodingWithAllowedCharacters:",
                cfstring::shim_nsstring_string_by_adding_percent_encoding as u64,
                "@@:@",
            ),
            me(
                "stringByRemovingPercentEncoding",
                cfstring::shim_nsstring_string_by_removing_percent_encoding as u64,
                "@@:",
            ),
            me(
                "lengthOfBytesUsingEncoding:",
                cfstring::shim_nsstring_length_of_bytes_using_encoding as u64,
                "Q@:Q",
            ),
            me(
                "maximumLengthOfBytesUsingEncoding:",
                cfstring::shim_nsstring_maximum_length_of_bytes_using_encoding as u64,
                "Q@:Q",
            ),
            // Mutation methods — shared 32-byte layout allows in-place
            // mutation of heap-allocated CFStrings (used by atos etc.).
            me(
                "appendString:",
                nsmutablestring::shim_nsmutablestring_append_string as u64,
                "v@:@",
            ),
            me(
                "appendFormat:",
                nsmutablestring::shim_nsmutablestring_append_format as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsstring_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
                "cStringUsingEncoding:",
                cfstring::shim_nsstring_cstring_using_encoding as u64,
                "r*:@Q",
            ),
            me(
                "description",
                cfstring::shim_nsstring_description as u64,
                "@@:",
            ),
            me("path", cfstring::shim_nsstring_path as u64, "@@:"),
            me(
                "initWithUTF8String:",
                cfstring::shim_nsstring_init_with_utf8string as u64,
                "@@:*",
            ),
            me(
                "initWithData:encoding:",
                cfstring::shim_nsstring_init_with_data_encoding as u64,
                "@@:@Q",
            ),
            me(
                "initWithContentsOfFile:encoding:error:",
                cfstring::shim_nsstring_init_with_contents_of_file_encoding_error as u64,
                "@@:@Q^@",
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
            me(
                "compare:options:",
                cfstring::shim_nsstring_compare_options as u64,
                "q@:@@Q",
            ),
            me("copy", cfstring::shim_nsstring_copy as u64, "@@:"),
            me("hash", cfstring::shim_nsstring_hash as u64, "Q@:"),
            me(
                "writeToFile:atomically:encoding:error:",
                cfstring::shim_nsstring_write_to_file as u64,
                "c@:@cQ^@",
            ),
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
                "rangeOfString:options:",
                cfstring::shim_nsstring_range_of_string_options as u64,
                "{_NSRange=QQ}@:@Q",
            ),
            me(
                "rangeOfString:options:range:",
                cfstring::shim_nsstring_range_of_string_options_range as u64,
                "{_NSRange=QQ}@:@Q{_NSRange=QQ}",
            ),
            me(
                "matchMemberOfArray:fromPosition:whichMember:",
                cfstring::shim_nsstring_match_member_of_array_from_position_which_member as u64,
                "{_NSRange=QQ}@:@Q^q",
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
                "stringByResolvingSymlinksInPath",
                cfstring::shim_nsstring_string_by_resolving_symlinks_in_path as u64,
                "@@:",
            ),
            me(
                "stringByStandardizingPath",
                cfstring::shim_nsstring_string_by_standardizing_path as u64,
                "@@:",
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
            me(
                "containsString:",
                cfstring::shim_nsstring_contains_string as u64,
                "B@:@",
            ),
            me(
                "substringWithRange:",
                cfstring::shim_nsstring_substring_with_range as u64,
                "@@:{_NSRange=QQ}",
            ),
            me(
                "characterAtIndex:",
                cfstring::shim_nsstring_character_at_index as u64,
                "S@:Q",
            ),
            me(
                "initWithString:",
                cfstring::shim_nsstring_init_with_string as u64,
                "@@:@",
            ),
            me(
                "capitalizedString",
                cfstring::shim_nsstring_capitalized_string as u64,
                "@@:",
            ),
            me(
                "caseInsensitiveCompare:",
                cfstring::shim_nsstring_case_insensitive_compare as u64,
                "q@:@",
            ),
            me(
                "isEqual:",
                cfstring::shim_nsstring_is_equal as u64,
                "B@:@",
            ),
            me(
                "dataUsingEncoding:",
                cfstring::shim_nsstring_data_using_encoding as u64,
                "@@:Q",
            ),
            me(
                "dataUsingEncoding:allowLossyConversion:",
                cfstring::shim_nsstring_data_using_encoding_lossy as u64,
                "@@:QB",
            ),
            me(
                "initWithFormat:",
                cfstring::shim_nsstring_init_with_format as u64,
                "@@:@",
            ),
            me(
                "initWithFormat:arguments:",
                cfstring::shim_nsstring_init_with_format_arguments as u64,
                "@@:@^v",
            ),
            me(
                "stringByReplacingUnicodeCharacterEscapeSequences:",
                cfstring::shim_nsstring_string_by_replacing_unicode_escapes as u64,
                "@@:B",
            ),
            me(
                "stringByExpandingTildeInPath",
                cfstring::shim_nsstring_string_by_expanding_tilde_in_path as u64,
                "@@:",
            ),
            me(
                "stringByAppendingPathExtension:",
                cfstring::shim_nsstring_string_by_appending_path_extension as u64,
                "@@:@",
            ),
            me(
                "pathComponents",
                cfstring::shim_nsstring_path_components as u64,
                "@@:",
            ),
            me(
                "localizedCaseInsensitiveCompare:",
                cfstring::shim_nsstring_localized_case_insensitive_compare as u64,
                "q@:@",
            ),
            me(
                "localizedCompare:",
                cfstring::shim_nsstring_localized_compare as u64,
                "q@:@",
            ),
            me(
                "precomposedStringWithCanonicalMapping",
                cfstring::shim_nsstring_precomposed_string_with_canonical_mapping as u64,
                "@@:",
            ),
            me(
                "decomposedStringWithCanonicalMapping",
                cfstring::shim_nsstring_decomposed_string_with_canonical_mapping as u64,
                "@@:",
            ),
            me(
                "stringByAddingPercentEncodingWithAllowedCharacters:",
                cfstring::shim_nsstring_string_by_adding_percent_encoding as u64,
                "@@:@",
            ),
            me(
                "stringByRemovingPercentEncoding",
                cfstring::shim_nsstring_string_by_removing_percent_encoding as u64,
                "@@:",
            ),
            me(
                "initWithBytes:length:encoding:",
                cfstring::shim_nsstring_init_with_bytes_length_encoding as u64,
                "@@:^vQQ",
            ),
            me(
                "lengthOfBytesUsingEncoding:",
                cfstring::shim_nsstring_length_of_bytes_using_encoding as u64,
                "Q@:Q",
            ),
            me(
                "maximumLengthOfBytesUsingEncoding:",
                cfstring::shim_nsstring_maximum_length_of_bytes_using_encoding as u64,
                "Q@:Q",
            ),
        ],
        class_methods: vec![
            me(
                "stringWithUTF8String:",
                cfstring::shim_nsstring_string_with_utf8string as u64,
                "@@:*",
            ),
            me(
                "stringWithString:",
                cfstring::shim_nsstring_string_with_string as u64,
                "@@:@",
            ),
            me(
                "stringWithCString:encoding:",
                cfstring::shim_nsstring_string_with_cstring_encoding as u64,
                "@@:*Q",
            ),
            me(
                "localizedNameOfStringEncoding:",
                cfstring::shim_nsstring_localized_name_of_string_encoding as u64,
                "@@:Q",
            ),
            me(
                "stringWithContentsOfFile:encoding:error:",
                cfstring::shim_nsstring_string_with_contents_of_file_encoding_error as u64,
                "@@:@Q^@",
            ),
            me(
                "stringWithContentsOfURL:usedEncoding:error:",
                cfstring::shim_nsstring_string_with_contents_of_url_used_encoding_error as u64,
                "@@:@@^@",
            ),
            me(
                "stringWithFormat:",
                cfstring::string_with_format_trampoline_addr(),
                "@@:@",
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
    rt.register_metaclass_addr(metaclass_addr, "NSString");
}

/// Register `NSMutableString` with mutation + inherited read methods.
pub fn register_nsmutablestring_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutablestring_class_addr();
    let metaclass_addr = nsobj::nsmutablestring_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableString".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithCapacity:",
                nsmutablestring::shim_nsmutablestring_init_with_capacity as u64,
                "@@:Q",
            ),
            me(
                "initWithFormat:",
                nsmutablestring::shim_nsmutablestring_init_with_format as u64,
                "@@:@",
            ),
            // Inherited read methods (same 32-byte layout as NSString)
            me("length", cfstring::shim_nsstring_length as u64, "Q@:"),
            me(
                "UTF8String",
                cfstring::shim_nsstring_utf8string as u64,
                "r*@:",
            ),
            me(
                "cStringUsingEncoding:",
                cfstring::shim_nsstring_cstring_using_encoding as u64,
                "r*:@Q",
            ),
            me(
                "description",
                cfstring::shim_nsstring_description as u64,
                "@@:",
            ),
            me("path", cfstring::shim_nsstring_path as u64, "@@:"),
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
            me(
                "writeToFile:atomically:encoding:error:",
                cfstring::shim_nsstring_write_to_file as u64,
                "c@:@cQ^@",
            ),
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
                "appendFormat:",
                nsmutablestring::shim_nsmutablestring_append_format as u64,
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
            me(
                "clear",
                nsmutablestring::shim_nsmutablestring_clear as u64,
                "v@:",
            ),
            me(
                "replaceOccurrencesOfString:withString:options:range:",
                nsmutablestring::shim_nsmutablestring_replace_occurrences as u64,
                "Q@:@@QQQ",
            ),
        ],
        class_methods: vec![
            me(
                "string",
                nsmutablestring::shim_nsmutablestring_string as u64,
                "@@:",
            ),
            me(
                "stringWithCapacity:",
                nsmutablestring::shim_nsmutablestring_string_with_capacity as u64,
                "@@:Q",
            ),
            me(
                "stringWithFormat:",
                nsmutablestring::string_with_format_trampoline_addr(),
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsstring_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableString");
}
