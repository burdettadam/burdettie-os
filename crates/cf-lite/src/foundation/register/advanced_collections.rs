use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{
    nscache, nshashtable, nsindexset, nsjsonserialization, nsmaptable, nsobj,
    nspropertylistserialization, nssortdescriptor, nsuuid,
};

/// Register `NSUUID` with UUID generation and comparison methods.
pub fn register_nsuuid_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsuuid_class_addr();
    let metaclass_addr = nsobj::nsuuid_metaclass_addr();

    let info = ClassInfo {
        name: "NSUUID".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsuuid::shim_nsuuid_init as u64, "@@:"),
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
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSUUID");
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
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsindexset_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSSortDescriptor");
}

/// Register `NSCache`.
pub fn register_nscache_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscache_class_addr();
    let metaclass_addr = nsobj::nscache_metaclass_addr();

    let info = ClassInfo {
        name: "NSCache".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nscache::shim_nscache_init as u64, "@@:"),
            me("objectForKey:", nscache::shim_nscache_object_for_key as u64, "@@:@"),
            me("setObject:forKey:", nscache::shim_nscache_set_object as u64, "v@:@@"),
            me("setObject:forKey:cost:", nscache::shim_nscache_set_object_cost as u64, "v@:@@Q"),
            me("removeObjectForKey:", nscache::shim_nscache_remove_object as u64, "v@:@"),
            me("removeAllObjects", nscache::shim_nscache_remove_all as u64, "v@:"),
            me("setCountLimit:", nscache::shim_nscache_set_count_limit as u64, "v@:Q"),
            me("countLimit", nscache::shim_nscache_count_limit as u64, "Q@:"),
            me("setName:", nscache::shim_nscache_set_name as u64, "v@:@"),
            me("name", nscache::shim_nscache_name as u64, "@@:"),
            me("description", nscache::shim_nscache_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nscache::shim_nscache_alloc as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nscache::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCache");
}

/// Register `NSMapTable`.
pub fn register_nsmaptable_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmaptable_class_addr();
    let metaclass_addr = nsobj::nsmaptable_metaclass_addr();

    let info = ClassInfo {
        name: "NSMapTable".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("objectForKey:", nsmaptable::shim_maptable_object_for_key as u64, "@@:@"),
            me("setObject:forKey:", nsmaptable::shim_maptable_set_object as u64, "v@:@@"),
            me("removeObjectForKey:", nsmaptable::shim_maptable_remove_object as u64, "v@:@"),
            me("removeAllObjects", nsmaptable::shim_maptable_remove_all as u64, "v@:"),
            me("count", nsmaptable::shim_maptable_count as u64, "Q@:"),
            me("description", nsmaptable::shim_maptable_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("mapTableWithKeyOptions:valueOptions:", nsmaptable::shim_maptable_with_options as u64, "@@:QQ"),
            me("strongToStrongObjectsMapTable", nsmaptable::shim_maptable_strong_strong as u64, "@@:"),
            me("weakToStrongObjectsMapTable", nsmaptable::shim_maptable_weak_strong as u64, "@@:"),
            me("strongToWeakObjectsMapTable", nsmaptable::shim_maptable_strong_weak as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsmaptable::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMapTable");
}

/// Register `NSHashTable`.
pub fn register_nshashtable_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nshashtable_class_addr();
    let metaclass_addr = nsobj::nshashtable_metaclass_addr();

    let info = ClassInfo {
        name: "NSHashTable".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("addObject:", nshashtable::shim_hashtable_add_object as u64, "v@:@"),
            me("removeObject:", nshashtable::shim_hashtable_remove_object as u64, "v@:@"),
            me("removeAllObjects", nshashtable::shim_hashtable_remove_all as u64, "v@:"),
            me("containsObject:", nshashtable::shim_hashtable_contains as u64, "B@:@"),
            me("count", nshashtable::shim_hashtable_count as u64, "Q@:"),
            me("allObjects", nshashtable::shim_hashtable_all_objects as u64, "@@:"),
            me("description", nshashtable::shim_hashtable_description as u64, "@@:"),
        ],
        class_methods: vec![
            me("hashTableWithOptions:", nshashtable::shim_hashtable_with_options as u64, "@@:Q"),
            me("weakObjectsHashTable", nshashtable::shim_hashtable_weak_objects as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nshashtable::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSHashTable");
}

/// Register `NSJSONSerialization`.
pub fn register_nsjsonserialization_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsjsonserialization_class_addr();
    let metaclass_addr = nsobj::nsjsonserialization_metaclass_addr();

    let info = ClassInfo {
        name: "NSJSONSerialization".to_string(),
        mapped_addr: class_addr,
        methods: vec![],
        class_methods: vec![
            me("JSONObjectWithData:options:error:", nsjsonserialization::shim_json_object_with_data as u64, "@@:@Q^@"),
            me("dataWithJSONObject:options:error:", nsjsonserialization::shim_data_with_json_object as u64, "@@:@Q^@"),
            me("isValidJSONObject:", nsjsonserialization::shim_is_valid_json_object as u64, "B@:@"),
        ],
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
    rt.register_metaclass_addr(metaclass_addr, "NSJSONSerialization");
}

/// Register `NSPropertyListSerialization`.
pub fn register_nspropertylistserialization_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nspropertylistserialization_class_addr();
    let metaclass_addr = nsobj::nspropertylistserialization_metaclass_addr();

    let info = ClassInfo {
        name: "NSPropertyListSerialization".to_string(),
        mapped_addr: class_addr,
        methods: vec![],
        class_methods: vec![
            me("dataWithPropertyList:format:options:error:", nspropertylistserialization::shim_plist_data_with_object as u64, "@@:@QQ^@"),
            me("propertyListWithData:options:format:error:", nspropertylistserialization::shim_plist_with_data as u64, "@@:@Q^Q^@"),
            me("propertyList:isValidForFormat:", nspropertylistserialization::shim_plist_is_valid as u64, "B@:@Q"),
        ],
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
    rt.register_metaclass_addr(metaclass_addr, "NSPropertyListSerialization");
}
