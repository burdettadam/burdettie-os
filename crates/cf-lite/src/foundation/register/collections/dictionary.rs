use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::super::me;
use crate::{nsdictionary, nsenumerator, nskeyvaluecoding, nsmutabledict, nsobj};

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
            me(
                "valueForKey:",
                nsdictionary::shim_nsdictionary_object_for_key as u64,
                "@@:@",
            ),
            me(
                "valueForKeyPath:",
                nskeyvaluecoding::shim_dict_value_for_key_path as u64,
                "@@:@",
            ),
        ],
        class_methods: vec![me(
            "dictionaryWithObjects:forKeys:count:",
            nsdictionary::shim_nsdictionary_with_objects_for_keys_count as u64,
            "@@:^@^@Q",
        )],
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
    rt.register_metaclass_addr(metaclass_addr, "NSDictionary");
}

/// Register `NSConstantDictionary` — compiler-embedded dict literals (@{}).
/// Same method implementations as NSDictionary; the layout-aware helpers
/// in nsdictionary/helpers.rs handle the different field offsets.
pub fn register_nsconstantdictionary_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsconstantdictionary_class_addr();
    let metaclass_addr = nsobj::nsconstantdictionary_metaclass_addr();

    let info = ClassInfo {
        name: "NSConstantDictionary".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("count", nsdictionary::shim_nsdictionary_count as u64, "Q@:"),
            me("objectForKey:", nsdictionary::shim_nsdictionary_object_for_key as u64, "@@:@"),
            me("objectForKeyedSubscript:", nsdictionary::shim_nsdictionary_object_for_keyed_subscript as u64, "@@:@"),
            me("allKeys", nsdictionary::shim_nsdictionary_all_keys as u64, "@@:"),
            me("allValues", nsdictionary::shim_nsdictionary_all_values as u64, "@@:"),
            me("description", nsdictionary::shim_nsdictionary_description as u64, "@@:"),
            me("enumerateKeysAndObjectsUsingBlock:", nsdictionary::shim_nsdictionary_enumerate_keys_and_objects_using_block as u64, "v@:@?"),
            me("keyEnumerator", nsenumerator::shim_nsdictionary_key_enumerator as u64, "@@:"),
            me("valueForKey:", nsdictionary::shim_nsdictionary_object_for_key as u64, "@@:@"),
            me("valueForKeyPath:", nskeyvaluecoding::shim_dict_value_for_key_path as u64, "@@:@"),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsdictionary_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSConstantDictionary");
}

/// Register `NSMutableDictionary` with mutation + inherited read methods.
pub fn register_nsmutabledictionary_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutabledictionary_class_addr();
    let metaclass_addr = nsobj::nsmutabledictionary_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableDictionary".to_string(),
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
            me(
                "valueForKey:",
                nsdictionary::shim_nsdictionary_object_for_key as u64,
                "@@:@",
            ),
            me(
                "valueForKeyPath:",
                nskeyvaluecoding::shim_dict_value_for_key_path as u64,
                "@@:@",
            ),
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
            me(
                "addEntriesFromDictionary:",
                nsmutabledict::shim_nsmutabledict_add_entries_from_dictionary as u64,
                "v@:@",
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
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsdictionary_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableDictionary");
}
