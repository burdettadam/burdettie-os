use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nsdecimalnumber, nsnull, nsnumber, nsnumberformatter, nsobj, nsvalue};

/// Register `NSNumber` with numeric accessors and factory methods.
pub fn register_nsnumber_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsnumber_class_addr();
    let metaclass_addr = nsobj::nsnumber_metaclass_addr();

    let info = ClassInfo {
        name: "NSNumber".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("intValue", nsnumber::shim_nsnumber_int_value as *const () as u64, "i@:"),
            me(
                "doubleValue",
                nsnumber::shim_nsnumber_double_value as *const () as u64,
                "d@:",
            ),
            me(
                "boolValue",
                nsnumber::shim_nsnumber_bool_value as *const () as u64,
                "B@:",
            ),
            me(
                "integerValue",
                nsnumber::shim_nsnumber_integer_value as *const () as u64,
                "q@:",
            ),
            me(
                "longLongValue",
                nsnumber::shim_nsnumber_long_long_value as *const () as u64,
                "q@:",
            ),
            me(
                "longValue",
                nsnumber::shim_nsnumber_long_value as *const () as u64,
                "q@:",
            ),
            me(
                "floatValue",
                nsnumber::shim_nsnumber_float_value as *const () as u64,
                "f@:",
            ),
            me(
                "unsignedIntegerValue",
                nsnumber::shim_nsnumber_unsigned_integer_value as *const () as u64,
                "Q@:",
            ),
            me(
                "unsignedIntValue",
                nsnumber::shim_nsnumber_unsigned_int_value as *const () as u64,
                "I@:",
            ),
            me(
                "shortValue",
                nsnumber::shim_nsnumber_short_value as *const () as u64,
                "s@:",
            ),
            me(
                "charValue",
                nsnumber::shim_nsnumber_char_value as *const () as u64,
                "c@:",
            ),
            me(
                "unsignedCharValue",
                nsnumber::shim_nsnumber_unsigned_char_value as *const () as u64,
                "C@:",
            ),
            me(
                "stringValue",
                nsnumber::shim_nsnumber_string_value as *const () as u64,
                "@@:",
            ),
            me(
                "isEqualToNumber:",
                nsnumber::shim_nsnumber_is_equal_to_number as *const () as u64,
                "B@:@",
            ),
            me(
                "isEqual:",
                nsnumber::shim_nsnumber_is_equal as *const () as u64,
                "B@:@",
            ),
            me("compare:", nsnumber::shim_nsnumber_compare as *const () as u64, "q@:@"),
            me(
                "description",
                nsnumber::shim_nsnumber_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "numberWithInt:",
                nsnumber::shim_nsnumber_number_with_int as *const () as u64,
                "@@:i",
            ),
            me(
                "numberWithDouble:",
                nsnumber::shim_nsnumber_number_with_double as *const () as u64,
                "@@:d",
            ),
            me(
                "numberWithBool:",
                nsnumber::shim_nsnumber_number_with_bool as *const () as u64,
                "@@:B",
            ),
            me(
                "numberWithInteger:",
                nsnumber::shim_nsnumber_number_with_integer as *const () as u64,
                "@@:q",
            ),
            me(
                "numberWithLong:",
                nsnumber::shim_nsnumber_number_with_long as *const () as u64,
                "@@:q",
            ),
            me(
                "numberWithLongLong:",
                nsnumber::shim_nsnumber_number_with_long_long as *const () as u64,
                "@@:q",
            ),
            me(
                "numberWithFloat:",
                nsnumber::shim_nsnumber_number_with_float as *const () as u64,
                "@@:f",
            ),
            me(
                "numberWithUnsignedInt:",
                nsnumber::shim_nsnumber_number_with_unsigned_int as *const () as u64,
                "@@:I",
            ),
            me(
                "numberWithUnsignedInteger:",
                nsnumber::shim_nsnumber_number_with_unsigned_integer as *const () as u64,
                "@@:Q",
            ),
            me(
                "numberWithShort:",
                nsnumber::shim_nsnumber_number_with_short as *const () as u64,
                "@@:s",
            ),
            me(
                "numberWithChar:",
                nsnumber::shim_nsnumber_number_with_char as *const () as u64,
                "@@:c",
            ),
            me(
                "numberWithUnsignedChar:",
                nsnumber::shim_nsnumber_number_with_unsigned_char as *const () as u64,
                "@@:C",
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
    rt.register_metaclass_addr(metaclass_addr, "NSNumber");
}

/// Register `NSConstantIntegerNumber` — compiler-embedded number literals (@(N)).
/// Reuses all NSNumber method implementations; the `read_number()` helper
/// in nsnumber.rs detects the constant layout automatically.
pub fn register_nsconstantintegernumber_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsconstantintegernumber_class_addr();
    let metaclass_addr = nsobj::nsconstantintegernumber_metaclass_addr();

    let info = ClassInfo {
        name: "NSConstantIntegerNumber".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("intValue", nsnumber::shim_nsnumber_int_value as *const () as u64, "i@:"),
            me("doubleValue", nsnumber::shim_nsnumber_double_value as *const () as u64, "d@:"),
            me("boolValue", nsnumber::shim_nsnumber_bool_value as *const () as u64, "B@:"),
            me("integerValue", nsnumber::shim_nsnumber_integer_value as *const () as u64, "q@:"),
            me("longLongValue", nsnumber::shim_nsnumber_long_long_value as *const () as u64, "q@:"),
            me("longValue", nsnumber::shim_nsnumber_long_value as *const () as u64, "q@:"),
            me("floatValue", nsnumber::shim_nsnumber_float_value as *const () as u64, "f@:"),
            me("unsignedIntegerValue", nsnumber::shim_nsnumber_unsigned_integer_value as *const () as u64, "Q@:"),
            me("unsignedIntValue", nsnumber::shim_nsnumber_unsigned_int_value as *const () as u64, "I@:"),
            me("stringValue", nsnumber::shim_nsnumber_string_value as *const () as u64, "@@:"),
            me("isEqualToNumber:", nsnumber::shim_nsnumber_is_equal_to_number as *const () as u64, "B@:@"),
            me("isEqual:", nsnumber::shim_nsnumber_is_equal as *const () as u64, "B@:@"),
            me("compare:", nsnumber::shim_nsnumber_compare as *const () as u64, "q@:@"),
            me("description", nsnumber::shim_nsnumber_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsnumber_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSConstantIntegerNumber");
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
                nsvalue::shim_nsvalue_range_value as *const () as u64,
                "Q@:",
            ),
            me(
                "pointerValue",
                nsvalue::shim_nsvalue_pointer_value as *const () as u64,
                "^v@:",
            ),
            me(
                "pointValue",
                nsvalue::shim_nsvalue_point_value as *const () as u64,
                "{CGPoint=dd}@:",
            ),
            me(
                "sizeValue",
                nsvalue::shim_nsvalue_size_value as *const () as u64,
                "{CGSize=dd}@:",
            ),
            me(
                "rectValue",
                nsvalue::shim_nsvalue_rect_value as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:",
            ),
            me("objCType", nsvalue::shim_nsvalue_objc_type as *const () as u64, "r*@:"),
            me(
                "description",
                nsvalue::shim_nsvalue_description as *const () as u64,
                "@@:",
            ),
            me(
                "initWithBytes:objCType:",
                nsvalue::shim_nsvalue_init_with_bytes_objctype as *const () as u64,
                "@@:^v*",
            ),
            me(
                "isEqualToValue:",
                nsvalue::shim_nsvalue_is_equal_to_value as *const () as u64,
                "c@:@",
            ),
        ],
        class_methods: vec![
            me(
                "valueWithBytes:objCType:",
                nsvalue::shim_nsvalue_value_with_bytes as *const () as u64,
                "@@:r^v r*",
            ),
            me(
                "valueWithRange:",
                nsvalue::shim_nsvalue_value_with_range as *const () as u64,
                "@@:QQ",
            ),
            me(
                "valueWithPointer:",
                nsvalue::shim_nsvalue_value_with_pointer as *const () as u64,
                "@@:^v",
            ),
            me(
                "valueWithPoint:",
                nsvalue::shim_nsvalue_value_with_point as *const () as u64,
                "@@:{CGPoint=dd}",
            ),
            me(
                "valueWithSize:",
                nsvalue::shim_nsvalue_value_with_size as *const () as u64,
                "@@:{CGSize=dd}",
            ),
            me(
                "valueWithRect:",
                nsvalue::shim_nsvalue_value_with_rect as *const () as u64,
                "@@:{CGRect={CGPoint=dd}{CGSize=dd}}",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 64,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSValue");
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
            nsnull::shim_nsnull_description as *const () as u64,
            "@@:",
        )],
        class_methods: vec![me("null", nsnull::shim_nsnull_null as *const () as u64, "@@:")],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSNull");
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
                nsnumberformatter::shim_nsnumberformatter_init as *const () as u64,
                "@@:",
            ),
            me(
                "setNumberStyle:",
                nsnumberformatter::shim_nsnumberformatter_set_number_style as *const () as u64,
                "v@:q",
            ),
            me(
                "numberStyle",
                nsnumberformatter::shim_nsnumberformatter_number_style as *const () as u64,
                "q@:",
            ),
            me(
                "setMinimumFractionDigits:",
                nsnumberformatter::shim_nsnumberformatter_set_min_frac as *const () as u64,
                "v@:Q",
            ),
            me(
                "setMaximumFractionDigits:",
                nsnumberformatter::shim_nsnumberformatter_set_max_frac as *const () as u64,
                "v@:Q",
            ),
            me(
                "minimumFractionDigits",
                nsnumberformatter::shim_nsnumberformatter_min_frac as *const () as u64,
                "Q@:",
            ),
            me(
                "maximumFractionDigits",
                nsnumberformatter::shim_nsnumberformatter_max_frac as *const () as u64,
                "Q@:",
            ),
            me(
                "locale",
                nsnumberformatter::shim_nsnumberformatter_locale as *const () as u64,
                "@@:",
            ),
            me(
                "setLocale:",
                nsnumberformatter::shim_nsnumberformatter_set_locale as *const () as u64,
                "v@:@",
            ),
            me(
                "stringFromNumber:",
                nsnumberformatter::shim_nsnumberformatter_string_from_number as *const () as u64,
                "@@:@",
            ),
            me(
                "numberFromString:",
                nsnumberformatter::shim_nsnumberformatter_number_from_string as *const () as u64,
                "@@:@",
            ),
            me(
                "description",
                nsnumberformatter::shim_nsnumberformatter_description as *const () as u64,
                "@@:",
            ),
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
    rt.register_metaclass_addr(metaclass_addr, "NSNumberFormatter");
}

/// Register `NSDecimalNumber` — decimal arithmetic subclass of NSNumber.
pub fn register_nsdecimalnumber_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdecimalnumber_class_addr();
    let metaclass_addr = nsobj::nsdecimalnumber_metaclass_addr();
    let info = ClassInfo {
        name: "NSDecimalNumber".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithString:", nsdecimalnumber::shim_nsdecimalnumber_init_with_string as *const () as u64, "@@:@"),
            me("initWithMantissa:exponent:isNegative:", nsdecimalnumber::shim_nsdecimalnumber_init_with_mantissa as *const () as u64, "@@:Qsc"),
            me("decimalNumberByAdding:", nsdecimalnumber::shim_nsdecimalnumber_by_adding as *const () as u64, "@@:@"),
            me("decimalNumberBySubtracting:", nsdecimalnumber::shim_nsdecimalnumber_by_subtracting as *const () as u64, "@@:@"),
            me("decimalNumberByMultiplyingBy:", nsdecimalnumber::shim_nsdecimalnumber_by_multiplying as *const () as u64, "@@:@"),
            me("decimalNumberByDividingBy:", nsdecimalnumber::shim_nsdecimalnumber_by_dividing as *const () as u64, "@@:@"),
            me("decimalNumberByRaisingToPower:", nsdecimalnumber::shim_nsdecimalnumber_by_raising_to_power as *const () as u64, "@@:Q"),
            me("compare:", nsdecimalnumber::shim_nsdecimalnumber_compare as *const () as u64, "q@:@"),
            me("doubleValue", nsdecimalnumber::shim_nsdecimalnumber_double_value as *const () as u64, "d@:"),
            me("stringValue", nsdecimalnumber::shim_nsdecimalnumber_string_value as *const () as u64, "@@:"),
            me("descriptionWithLocale:", nsdecimalnumber::shim_nsdecimalnumber_description_with_locale as *const () as u64, "@@:@"),
            me("description", nsdecimalnumber::shim_nsdecimalnumber_description as *const () as u64, "@@:"),
            me("objCType", nsdecimalnumber::shim_nsdecimalnumber_objc_type as *const () as u64, "r*@:"),
            me("isEqual:", nsdecimalnumber::shim_nsdecimalnumber_is_equal as *const () as u64, "B@:@"),
        ],
        class_methods: vec![
            me("alloc", nsdecimalnumber::shim_nsdecimalnumber_alloc as *const () as u64, "@@:"),
            me("decimalNumberWithString:", nsdecimalnumber::shim_nsdecimalnumber_with_string as *const () as u64, "@@:@"),
            me("decimalNumberWithMantissa:exponent:isNegative:", nsdecimalnumber::shim_nsdecimalnumber_with_mantissa as *const () as u64, "@@:Qsc"),
            me("zero", nsdecimalnumber::shim_nsdecimalnumber_zero as *const () as u64, "@@:"),
            me("one", nsdecimalnumber::shim_nsdecimalnumber_one as *const () as u64, "@@:"),
            me("notANumber", nsdecimalnumber::shim_nsdecimalnumber_nan as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 24, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsnumber_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDecimalNumber");
}
