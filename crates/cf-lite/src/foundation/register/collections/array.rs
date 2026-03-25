use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::super::me;
use crate::{nsarray, nsenumerator, nsmutablearray, nsobj};

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
                nsarray::ops::shim_nsarray_enumerate_objects_using_block as u64,
                "v@:@?",
            ),
            me(
                "sortedArrayUsingComparator:",
                nsarray::ops::shim_nsarray_sorted_array_using_comparator as u64,
                "@@:@?",
            ),
            me(
                "objectEnumerator",
                nsenumerator::shim_nsarray_object_enumerator as u64,
                "@@:",
            ),
            me(
                "indexOfObject:",
                nsarray::ops::shim_nsarray_index_of_object as u64,
                "Q@:@",
            ),
            me(
                "isEqualToArray:",
                nsarray::ops::shim_nsarray_is_equal_to_array as u64,
                "B@:@",
            ),
            me(
                "arrayByAddingObject:",
                nsarray::ops::shim_nsarray_array_by_adding_object as u64,
                "@@:@",
            ),
            me(
                "subarrayWithRange:",
                nsarray::ops::shim_nsarray_subarray_with_range as u64,
                "@@:{_NSRange=QQ}",
            ),
        ],
        class_methods: vec![me(
            "arrayWithObjects:count:",
            nsarray::shim_nsarray_array_with_objects_count as u64,
            "@@:^@Q",
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
    rt.register_metaclass_addr(metaclass_addr, "NSArray");
}

/// Register `NSMutableArray` with mutation + inherited read methods.
pub fn register_nsmutablearray_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutablearray_class_addr();
    let metaclass_addr = nsobj::nsmutablearray_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableArray".to_string(),
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
                nsarray::ops::shim_nsarray_enumerate_objects_using_block as u64,
                "v@:@?",
            ),
            me(
                "sortedArrayUsingComparator:",
                nsarray::ops::shim_nsarray_sorted_array_using_comparator as u64,
                "@@:@?",
            ),
            me(
                "sortUsingComparator:",
                nsmutablearray::ops::shim_nsmutablearray_sort_using_comparator as u64,
                "v@:@?",
            ),
            me(
                "objectEnumerator",
                nsenumerator::shim_nsarray_object_enumerator as u64,
                "@@:",
            ),
            me(
                "removeObject:",
                nsmutablearray::ops::shim_nsmutablearray_remove_object as u64,
                "v@:@",
            ),
            me(
                "addObjectsFromArray:",
                nsmutablearray::ops::shim_nsmutablearray_add_objects_from_array as u64,
                "v@:@",
            ),
            me(
                "exchangeObjectAtIndex:withObjectAtIndex:",
                nsmutablearray::ops::shim_nsmutablearray_exchange_object_at_index as u64,
                "v@:QQ",
            ),
            me(
                "indexOfObject:",
                nsarray::ops::shim_nsarray_index_of_object as u64,
                "Q@:@",
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
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsarray_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableArray");
}
