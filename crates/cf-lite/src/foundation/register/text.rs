use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nsattributedstring, nscharacterset, nsnotificationcenter, nsobj, nspredicate, nsregularexpression};

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
            me(
                "URLQueryAllowedCharacterSet",
                nscharacterset::shim_nscharacterset_url_query_allowed as u64,
                "@@:",
            ),
            me(
                "URLPathAllowedCharacterSet",
                nscharacterset::shim_nscharacterset_url_path_allowed as u64,
                "@@:",
            ),
            me(
                "URLHostAllowedCharacterSet",
                nscharacterset::shim_nscharacterset_url_host_allowed as u64,
                "@@:",
            ),
            me(
                "URLFragmentAllowedCharacterSet",
                nscharacterset::shim_nscharacterset_url_fragment_allowed as u64,
                "@@:",
            ),
            me(
                "URLUserAllowedCharacterSet",
                nscharacterset::shim_nscharacterset_url_user_allowed as u64,
                "@@:",
            ),
            me(
                "URLPasswordAllowedCharacterSet",
                nscharacterset::shim_nscharacterset_url_password_allowed as u64,
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
            me("postNotification:",
                nsnotificationcenter::shim_nsnotificationcenter_post_notification as u64, "v@:@"),
            me("addObserver:selector:name:object:",
                nsnotificationcenter::shim_nsnotificationcenter_add_observer as u64,               "v@:@:@@"),
            me("addObserverForName:object:queue:usingBlock:",
                nsnotificationcenter::shim_nsnotificationcenter_add_observer_for_name as u64,      "@@:@@@@?"),
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
        protocol_names: Vec::new(),
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSNotificationCenter");
}

/// Register `NSNotification` with name/object/userInfo accessors.
pub fn register_nsnotification_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsnotification_class_addr();
    let metaclass_addr = nsobj::nsnotification_metaclass_addr();

    let info = ClassInfo {
        name: "NSNotification".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nsnotificationcenter::shim_nsnotification_name as u64, "@@:"),
            me("object", nsnotificationcenter::shim_nsnotification_object as u64, "@@:"),
            me("userInfo", nsnotificationcenter::shim_nsnotification_userinfo as u64, "@@:"),
        ],
        class_methods: vec![
            me("notificationWithName:object:userInfo:",
                nsnotificationcenter::shim_nsnotification_with_name_object_userinfo as u64, "@@:@@@"),
            me("notificationWithName:object:",
                nsnotificationcenter::shim_nsnotification_with_name_object as u64, "@@:@@"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSNotification");
}

/// Register `NSRegularExpression`.
pub fn register_nsregularexpression_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsregularexpression_class_addr();
    let metaclass_addr = nsobj::nsregularexpression_metaclass_addr();

    let info = ClassInfo {
        name: "NSRegularExpression".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithPattern:options:error:", nsregularexpression::shim_regex_init_with_pattern as u64, "@@:@Q^@"),
            me("pattern", nsregularexpression::shim_regex_pattern as u64, "@@:"),
            me("options", nsregularexpression::shim_regex_options as u64, "Q@:"),
            me("numberOfMatchesInString:options:range:", nsregularexpression::shim_regex_number_of_matches as u64, "Q@:@Q{_NSRange=QQ}"),
            me("firstMatchInString:options:range:", nsregularexpression::shim_regex_first_match as u64, "@@:@Q{_NSRange=QQ}"),
            me("matchesInString:options:range:", nsregularexpression::shim_regex_matches_in_string as u64, "@@:@Q{_NSRange=QQ}"),
            me("enumerateMatchesInString:options:range:usingBlock:", nsregularexpression::shim_regex_enumerate_matches as u64, "v@:@Q{_NSRange=QQ}@?"),
            me("stringByReplacingMatchesInString:options:range:withTemplate:", nsregularexpression::shim_regex_replace as u64, "@@:@Q{_NSRange=QQ}@"),
            me("description", nsregularexpression::shim_regex_description as u64, "@@:"),
            me("replacementStringForResult:inString:offset:template:", nsregularexpression::shim_nsregex_replacement_string_for_result as u64, "@@:@@q@"),
        ],
        class_methods: vec![
            me("regularExpressionWithPattern:options:error:", nsregularexpression::shim_regex_with_pattern as u64, "@@:@Q^@"),
            me("escapedPatternForString:", nsregularexpression::shim_nsregex_escaped_pattern_for_string as u64, "@@:@"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsregularexpression::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSRegularExpression");
}

/// Register `NSTextCheckingResult`.
pub fn register_nstextcheckingresult_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstextcheckingresult_class_addr();
    let metaclass_addr = nsobj::nstextcheckingresult_metaclass_addr();

    let info = ClassInfo {
        name: "NSTextCheckingResult".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("range", nsregularexpression::shim_textcheckingresult_range as u64, "{_NSRange=QQ}@:"),
            me("rangeAtIndex:", nsregularexpression::shim_textcheckingresult_range_at_index as u64, "{_NSRange=QQ}@:Q"),
            me("numberOfRanges", nsregularexpression::shim_textcheckingresult_number_of_ranges as u64, "Q@:"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSTextCheckingResult");
}

/// Register `NSPredicate`.
pub fn register_nspredicate_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nspredicate_class_addr();
    let metaclass_addr = nsobj::nspredicate_metaclass_addr();

    let info = ClassInfo {
        name: "NSPredicate".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("evaluateWithObject:", nspredicate::shim_predicate_evaluate as u64, "B@:@"),
            me("predicateFormat", nspredicate::shim_predicate_format as u64, "@@:"),
            me("description", nspredicate::shim_predicate_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("predicateWithFormat:", nspredicate::shim_predicate_with_format as u64, "@@:@"),
            me("predicateWithValue:", nspredicate::shim_predicate_with_value as u64, "@@:B"),
            me("predicateWithBlock:", nspredicate::shim_predicate_with_block as u64, "@@:@?"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nspredicate::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSPredicate");
}

/// Register `NSAttributedString`.
pub fn register_nsattributedstring_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsattributedstring_class_addr();
    let metaclass_addr = nsobj::nsattributedstring_metaclass_addr();

    let info = ClassInfo {
        name: "NSAttributedString".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithString:", nsattributedstring::shim_attrstr_init_with_string as u64, "@@:@"),
            me("initWithString:attributes:", nsattributedstring::shim_attrstr_init_with_string_attributes as u64, "@@:@@"),
            me("string", nsattributedstring::shim_attrstr_string as u64, "@@:"),
            me("length", nsattributedstring::shim_attrstr_length as u64, "Q@:"),
            me("attributesAtIndex:effectiveRange:", nsattributedstring::shim_attrstr_attributes_at_index as u64, "@@:Q^{_NSRange=QQ}"),
            me("description", nsattributedstring::shim_attrstr_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nsattributedstring::shim_attrstr_alloc as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsattributedstring::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSAttributedString");
}

/// Register `NSMutableAttributedString`.
pub fn register_nsmutableattributedstring_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutableattributedstring_class_addr();
    let metaclass_addr = nsobj::nsmutableattributedstring_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableAttributedString".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithString:", nsattributedstring::shim_mutable_attrstr_init_with_string as u64, "@@:@"),
            me("string", nsattributedstring::shim_attrstr_string as u64, "@@:"),
            me("length", nsattributedstring::shim_attrstr_length as u64, "Q@:"),
            me("addAttribute:value:range:", nsattributedstring::shim_mutable_attrstr_add_attribute as u64, "v@:@@{_NSRange=QQ}"),
            me("replaceCharactersInRange:withString:", nsattributedstring::shim_mutable_attrstr_replace_chars as u64, "v@:{_NSRange=QQ}@"),
            me("description", nsattributedstring::shim_attrstr_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nsattributedstring::shim_mutable_attrstr_alloc as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsattributedstring::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsattributedstring_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableAttributedString");
}
