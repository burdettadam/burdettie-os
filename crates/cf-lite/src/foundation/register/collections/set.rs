use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::super::me;
use crate::{nsenumerator, nsmutableset, nsobj, nsset};

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
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSEnumerator");
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
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsset_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableSet");
}
