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
            me(
                "initWithObjects:",
                nsarray::shim_nsarray_init_with_objects as *const () as u64,
                "@@:@",
            ),
            me("count", nsarray::shim_nsarray_count as *const () as u64, "Q@:"),
            me(
                "objectAtIndex:",
                nsarray::shim_nsarray_object_at_index as *const () as u64,
                "@@:Q",
            ),
            me(
                "objectAtIndexedSubscript:",
                nsarray::shim_nsarray_object_at_index as *const () as u64,
                "@@:Q",
            ),
            me(
                "firstObject",
                nsarray::shim_nsarray_first_object as *const () as u64,
                "@@:",
            ),
            me(
                "lastObject",
                nsarray::shim_nsarray_last_object as *const () as u64,
                "@@:",
            ),
            me(
                "containsObject:",
                nsarray::shim_nsarray_contains_object as *const () as u64,
                "B@:@",
            ),
            me(
                "description",
                nsarray::shim_nsarray_description as *const () as u64,
                "@@:",
            ),
            me(
                "enumerateObjectsUsingBlock:",
                nsarray::ops::shim_nsarray_enumerate_objects_using_block as *const () as u64,
                "v@:@?",
            ),
            me(
                "sortedArrayUsingComparator:",
                nsarray::ops::shim_nsarray_sorted_array_using_comparator as *const () as u64,
                "@@:@?",
            ),
            me(
                "sortedArrayUsingSelector:",
                nsarray::ops::shim_nsarray_sorted_array_using_selector as *const () as u64,
                "@@::",
            ),
            me(
                "objectEnumerator",
                nsenumerator::shim_nsarray_object_enumerator as *const () as u64,
                "@@:",
            ),
            me(
                "indexOfObject:",
                nsarray::ops::shim_nsarray_index_of_object as *const () as u64,
                "Q@:@",
            ),
            me(
                "indexOfObjectPassingTest:",
                nsarray::ops::shim_nsarray_index_of_object_passing_test as *const () as u64,
                "Q@:@?",
            ),
            me(
                "isEqualToArray:",
                nsarray::ops::shim_nsarray_is_equal_to_array as *const () as u64,
                "B@:@",
            ),
            me(
                "arrayByAddingObject:",
                nsarray::ops::shim_nsarray_array_by_adding_object as *const () as u64,
                "@@:@",
            ),
            me(
                "subarrayWithRange:",
                nsarray::ops::shim_nsarray_subarray_with_range as *const () as u64,
                "@@:{_NSRange=QQ}",
            ),
            me(
                "filteredArrayUsingPredicate:",
                nsarray::ops::shim_nsarray_filtered_using_predicate as *const () as u64,
                "@@:@",
            ),
            me("copy", nsarray::ops::shim_nsarray_copy as *const () as u64, "@@:"),
            me(
                "mutableCopy",
                nsarray::ops::shim_nsarray_mutable_copy as *const () as u64,
                "@@:",
            ),
            me(
                "countByEnumeratingWithState:objects:count:",
                nsarray::shim_nsarray_count_by_enumerating_with_state as *const () as u64,
                "Q@:^Q^@Q",
            ),
            me(
                "componentsJoinedByString:",
                nsarray::ops::shim_nsarray_components_joined_by_string as *const () as u64,
                "@@:@",
            ),
            me(
                "reverseObjectEnumerator",
                nsenumerator::shim_nsarray_reverse_object_enumerator as *const () as u64,
                "@@:",
            ),
            me(
                "initWithArray:",
                nsarray::shim_nsarray_init_with_array as *const () as u64,
                "@@:@",
            ),
        ],
        class_methods: vec![
            me(
                "arrayWithObject:",
                nsarray::shim_nsarray_array_with_object as *const () as u64,
                "@@:@",
            ),
            me(
                "arrayWithObjects:count:",
                nsarray::shim_nsarray_array_with_objects_count as *const () as u64,
                "@@:^@Q",
            ),
            me(
                "arrayWithObjects:",
                nsarray::shim_nsarray_array_with_objects as *const () as u64,
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
            me(
                "initWithCapacity:",
                nsmutablearray::shim_nsmutablearray_init_with_capacity as *const () as u64,
                "@@:Q",
            ),
            me(
                "initWithObjects:",
                nsmutablearray::shim_nsmutablearray_init_with_objects as *const () as u64,
                "@@:@",
            ),
            me(
                "initWithArray:",
                nsmutablearray::shim_nsmutablearray_init_with_array as *const () as u64,
                "@@:@",
            ),
            me("count", nsarray::shim_nsarray_count as *const () as u64, "Q@:"),
            me(
                "objectAtIndex:",
                nsarray::shim_nsarray_object_at_index as *const () as u64,
                "@@:Q",
            ),
            me(
                "objectAtIndexedSubscript:",
                nsarray::shim_nsarray_object_at_index as *const () as u64,
                "@@:Q",
            ),
            me(
                "firstObject",
                nsarray::shim_nsarray_first_object as *const () as u64,
                "@@:",
            ),
            me(
                "lastObject",
                nsarray::shim_nsarray_last_object as *const () as u64,
                "@@:",
            ),
            me(
                "containsObject:",
                nsarray::shim_nsarray_contains_object as *const () as u64,
                "B@:@",
            ),
            me(
                "addObject:",
                nsmutablearray::shim_nsmutablearray_add_object as *const () as u64,
                "v@:@",
            ),
            me(
                "insertObject:atIndex:",
                nsmutablearray::shim_nsmutablearray_insert_object_at_index as *const () as u64,
                "v@:@Q",
            ),
            me(
                "removeObjectAtIndex:",
                nsmutablearray::shim_nsmutablearray_remove_object_at_index as *const () as u64,
                "v@:Q",
            ),
            me(
                "removeLastObject",
                nsmutablearray::shim_nsmutablearray_remove_last_object as *const () as u64,
                "v@:",
            ),
            me(
                "replaceObjectAtIndex:withObject:",
                nsmutablearray::shim_nsmutablearray_replace_object_at_index as *const () as u64,
                "v@:Q@",
            ),
            me(
                "setObject:atIndexedSubscript:",
                nsmutablearray::shim_nsmutablearray_set_object_at_indexed_subscript as *const () as u64,
                "v@:@Q",
            ),
            me(
                "removeAllObjects",
                nsmutablearray::shim_nsmutablearray_remove_all_objects as *const () as u64,
                "v@:",
            ),
            me(
                "description",
                nsmutablearray::shim_nsmutablearray_description as *const () as u64,
                "@@:",
            ),
            me(
                "enumerateObjectsUsingBlock:",
                nsarray::ops::shim_nsarray_enumerate_objects_using_block as *const () as u64,
                "v@:@?",
            ),
            me(
                "sortedArrayUsingComparator:",
                nsarray::ops::shim_nsarray_sorted_array_using_comparator as *const () as u64,
                "@@:@?",
            ),
            me(
                "sortedArrayUsingSelector:",
                nsarray::ops::shim_nsarray_sorted_array_using_selector as *const () as u64,
                "@@::",
            ),
            me(
                "sortUsingComparator:",
                nsmutablearray::ops::shim_nsmutablearray_sort_using_comparator as *const () as u64,
                "v@:@?",
            ),
            me(
                "sortUsingDescriptors:",
                nsmutablearray::ops::shim_nsmutablearray_sort_using_descriptors as *const () as u64,
                "v@:@",
            ),
            me(
                "objectEnumerator",
                nsenumerator::shim_nsarray_object_enumerator as *const () as u64,
                "@@:",
            ),
            me(
                "removeObject:",
                nsmutablearray::ops::shim_nsmutablearray_remove_object as *const () as u64,
                "v@:@",
            ),
            me(
                "removeObjectsAtIndexes:",
                nsmutablearray::ops::shim_nsmutablearray_remove_objects_at_indexes as *const () as u64,
                "v@:@",
            ),
            me(
                "addObjectsFromArray:",
                nsmutablearray::ops::shim_nsmutablearray_add_objects_from_array as *const () as u64,
                "v@:@",
            ),
            me(
                "exchangeObjectAtIndex:withObjectAtIndex:",
                nsmutablearray::ops::shim_nsmutablearray_exchange_object_at_index as *const () as u64,
                "v@:QQ",
            ),
            me(
                "removeObjectsInRange:",
                nsmutablearray::ops::shim_nsmutablearray_remove_objects_in_range as *const () as u64,
                "v@:QQ",
            ),
            me(
                "indexOfObject:",
                nsarray::ops::shim_nsarray_index_of_object as *const () as u64,
                "Q@:@",
            ),
            me(
                "indexOfObjectPassingTest:",
                nsarray::ops::shim_nsarray_index_of_object_passing_test as *const () as u64,
                "Q@:@?",
            ),
            me(
                "countByEnumeratingWithState:objects:count:",
                nsarray::shim_nsarray_count_by_enumerating_with_state as *const () as u64,
                "Q@:^Q^@Q",
            ),
            me("copy", nsarray::ops::shim_nsmutablearray_copy as *const () as u64, "@@:"),
            me(
                "mutableCopy",
                nsarray::ops::shim_nsmutablearray_mutable_copy as *const () as u64,
                "@@:",
            ),
            me(
                "componentsJoinedByString:",
                nsarray::ops::shim_nsarray_components_joined_by_string as *const () as u64,
                "@@:@",
            ),
            me(
                "reverseObjectEnumerator",
                nsenumerator::shim_nsarray_reverse_object_enumerator as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "array",
                nsmutablearray::shim_nsmutablearray_array as *const () as u64,
                "@@:",
            ),
            me(
                "arrayWithCapacity:",
                nsmutablearray::shim_nsmutablearray_array_with_capacity as *const () as u64,
                "@@:Q",
            ),
            me(
                "arrayWithArray:",
                nsmutablearray::shim_nsmutablearray_array_with_array as *const () as u64,
                "@@:@",
            ),
            me(
                "arrayWithObjects:",
                nsmutablearray::shim_nsmutablearray_array_with_objects as *const () as u64,
                "@@:@",
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
