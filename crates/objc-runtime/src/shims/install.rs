//! Core ObjC runtime shim registration: dispatch, ARC, introspection, selectors.

use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

use super::dispatch::{
    objc_msg_send_addr, objc_msg_send_fp2ret_addr, objc_msg_send_stret_addr,
    objc_msg_send_super2_addr, objc_msg_send_super2_stret_addr, objc_msg_send_super_addr,
    objc_msg_send_super_stret_addr,
};
use super::lifecycle::{
    shim_class_create_instance, shim_class_get_instance_size, shim_class_get_name,
    shim_class_get_superclass, shim_class_is_meta_class, shim_class_responds_to_selector,
    shim_objc_alloc, shim_objc_alloc_init, shim_objc_alloc_with_zone, shim_objc_autorelease,
    shim_objc_autorelease_pool_pop, shim_objc_autorelease_pool_push,
    shim_objc_autorelease_return_value, shim_objc_copy_struct,
    shim_objc_enumeration_mutation, shim_objc_get_class,
    shim_objc_get_property, shim_objc_lookup_class,
    shim_objc_opt_class, shim_objc_opt_is_kind_of_class, shim_objc_opt_new,
    shim_objc_opt_responds_to_selector, shim_objc_opt_self,
    shim_objc_release, shim_objc_retain, shim_objc_retain_autorelease,
    shim_objc_retain_autoreleased_return_value,
    shim_objc_set_property_atomic, shim_objc_set_property_atomic_copy,
    shim_objc_set_property_nonatomic, shim_objc_set_property_nonatomic_copy,
    shim_objc_store_strong, shim_objc_unsafe_claim_autoreleased_return_value,
    shim_object_get_class, shim_object_set_class, shim_sel_get_name, shim_sel_is_equal,
    shim_sel_register_name,
    // Class enumeration (Task 1)
    shim_objc_get_class_list, shim_objc_copy_class_list, shim_class_get_image_name,
    // Additional introspection
    shim_objc_get_meta_class, shim_object_get_class_name,
};
use super::EMPTY_CACHE;

/// Message dispatch: objc_msgSend and all ABI variants (stret, fp2ret, super).
pub(super) fn install_dispatch_shims(table: &mut ShimTable) {
    register_shims!(table,
        ("_objc_msgSend", objc_msg_send_addr(), "objc_msgSend → ARM64 trampoline dispatch"),
        ("_objc_msgSend_stret", objc_msg_send_stret_addr(), "objc_msgSend_stret → struct return dispatch (alias on ARM64)"),
        ("_objc_msgSend_fp2ret", objc_msg_send_fp2ret_addr(), "objc_msgSend_fp2ret → fp return dispatch (alias on ARM64)"),
        ("_objc_msgSendSuper", objc_msg_send_super_addr(), "objc_msgSendSuper → super dispatch trampoline"),
        ("_objc_msgSendSuper_stret", objc_msg_send_super_stret_addr(), "objc_msgSendSuper_stret → super struct return dispatch"),
        ("_objc_msgSendSuper2", objc_msg_send_super2_addr(), "objc_msgSendSuper2 → super2 dispatch trampoline"),
        ("_objc_msgSendSuper2_stret", objc_msg_send_super2_stret_addr(), "objc_msgSendSuper2_stret → super2 struct return dispatch"),
    );
}

/// ARC / memory management: retain, release, autorelease, alloc, blocks.
pub(super) fn install_arc_shims(table: &mut ShimTable) {
    // ARC functions
    register_shims!(table,
        ("_objc_autoreleasePoolPush", shim_objc_autorelease_pool_push as u64, "objc_autoreleasePoolPush → sentinel token"),
        ("__objc_autoreleasePoolPush", shim_objc_autorelease_pool_push as u64, "__objc_autoreleasePoolPush → sentinel token"),
        ("_objc_autoreleasePoolPop", shim_objc_autorelease_pool_pop as u64, "objc_autoreleasePoolPop → noop"),
        ("__objc_autoreleasePoolPop", shim_objc_autorelease_pool_pop as u64, "__objc_autoreleasePoolPop → noop"),
        ("_objc_retain", shim_objc_retain as u64, "objc_retain → pass-through"),
        ("_objc_release", shim_objc_release as u64, "objc_release → noop"),
        ("_objc_storeStrong", shim_objc_store_strong as u64, "objc_storeStrong → direct store"),
        ("_objc_setProperty_nonatomic", shim_objc_set_property_nonatomic as u64, "objc_setProperty_nonatomic → direct ivar store"),
        ("_objc_setProperty_nonatomic_copy", shim_objc_set_property_nonatomic_copy as u64, "objc_setProperty_nonatomic_copy → direct ivar store (copy elided)"),
        ("_objc_setProperty_atomic", shim_objc_set_property_atomic as u64, "objc_setProperty_atomic → retain-swap property setter"),
        ("_objc_setProperty_atomic_copy", shim_objc_set_property_atomic_copy as u64, "objc_setProperty_atomic_copy → retain-swap property setter (copy elided)"),
        ("_objc_getProperty", shim_objc_get_property as u64, "objc_getProperty → retain+autorelease property getter"),
        ("_objc_copyStruct", shim_objc_copy_struct as u64, "objc_copyStruct → memcpy struct between ivars"),
        ("_objc_enumerationMutation", shim_objc_enumeration_mutation as u64, "objc_enumerationMutation → abort on mutation during enumeration"),
        ("_objc_alloc", shim_objc_alloc as u64, "objc_alloc → allocate via guest runtime"),
        ("_objc_allocWithZone", shim_objc_alloc_with_zone as u64, "objc_allocWithZone → allocate via guest runtime (zone ignored)"),
        ("_objc_alloc_init", shim_objc_alloc_init as u64, "objc_alloc_init → allocate + init via guest runtime"),
        ("_objc_opt_self", shim_objc_opt_self as u64, "objc_opt_self → return self"),
        ("_objc_opt_new", shim_objc_opt_new as u64, "objc_opt_new → fast [cls new] (alloc+init)"),
        ("_objc_opt_class", shim_objc_opt_class as u64, "objc_opt_class → return ISA (fast [obj class])"),
        ("_objc_opt_isKindOfClass", shim_objc_opt_is_kind_of_class as u64, "objc_opt_isKindOfClass → fast [obj isKindOfClass:]"),
        ("_objc_opt_respondsToSelector", shim_objc_opt_responds_to_selector as u64, "objc_opt_respondsToSelector → fast respondsToSelector:"),
        ("_objc_autorelease", shim_objc_autorelease as u64, "objc_autorelease → add to current pool"),
        ("_objc_autoreleaseReturnValue", shim_objc_autorelease_return_value as u64, "objc_autoreleaseReturnValue → autorelease for return value"),
        ("_objc_retainAutoreleasedReturnValue", shim_objc_retain_autoreleased_return_value as u64, "objc_retainAutoreleasedReturnValue → retain returned value"),
        ("_objc_unsafeClaimAutoreleasedReturnValue", shim_objc_unsafe_claim_autoreleased_return_value as u64, "objc_unsafeClaimAutoreleasedReturnValue → claim without retain"),
        ("_objc_retainAutorelease", shim_objc_retain_autorelease as u64, "objc_retainAutorelease → retain + autorelease combined"),
    );

    // Blocks runtime
    register_shims!(table,
        ("__NSConcreteGlobalBlock", crate::blocks::global_block_class_addr(), "_NSConcreteGlobalBlock class reference"),
        ("__NSConcreteStackBlock", crate::blocks::stack_block_class_addr(), "_NSConcreteStackBlock class reference"),
        ("__Block_copy", crate::blocks::shim_block_copy as u64, "_Block_copy → copy stack block to heap"),
        ("__Block_release", crate::blocks::shim_block_release as u64, "_Block_release → release heap block"),
        ("__Block_object_assign", crate::blocks::shim_block_object_assign as u64, "_Block_object_assign → retain/copy captured objects"),
        ("__Block_object_dispose", crate::blocks::shim_block_object_dispose as u64, "_Block_object_dispose → release captured objects"),
        ("_objc_retainBlock", crate::blocks::shim_objc_retain_block as u64, "objc_retainBlock → Block_copy for ARC"),
        ("_imp_implementationWithBlock", crate::blocks::shim_imp_implementation_with_block as u64, "imp_implementationWithBlock → IMP from block"),
        ("_imp_getBlock", crate::blocks::shim_imp_get_block as u64, "imp_getBlock → block for IMP"),
        ("_imp_removeBlock", crate::blocks::shim_imp_remove_block as u64, "imp_removeBlock → disassociate block from IMP"),
    );
}

/// Introspection: class lookup, superclass, meta-class, instance size.
pub(super) fn install_introspection_shims(table: &mut ShimTable) {
    register_shims!(table,
        ("_objc_getClass", shim_objc_get_class as u64, "objc_getClass → class table lookup"),
        ("_class_createInstance", shim_class_create_instance as u64, "class_createInstance → allocate + set isa"),
        ("_class_getName", shim_class_get_name as u64, "class_getName → class name string"),
        ("__objc_empty_cache", EMPTY_CACHE.as_ptr() as u64, "objc empty cache data symbol"),
        ("_object_getClass", shim_object_get_class as u64, "object_getClass → read ISA"),
        ("_objc_lookUpClass", shim_objc_lookup_class as u64, "objc_lookUpClass → class table lookup"),
        ("_class_getSuperclass", shim_class_get_superclass as u64, "class_getSuperclass → superclass addr"),
        ("_class_respondsToSelector", shim_class_responds_to_selector as u64, "class_respondsToSelector → msg_lookup check"),
        ("_class_getInstanceSize", shim_class_get_instance_size as u64, "class_getInstanceSize → instance size"),
        ("_class_isMetaClass", shim_class_is_meta_class as u64, "class_isMetaClass → meta class check"),
        ("_object_setClass", shim_object_set_class as u64, "object_setClass → change object ISA"),
        ("_objc_getClassList", shim_objc_get_class_list as u64, "objc_getClassList → enumerate registered classes"),
        ("_objc_copyClassList", shim_objc_copy_class_list as u64, "objc_copyClassList → allocated class array"),
        ("_class_getImageName", shim_class_get_image_name as u64, "class_getImageName → image name for class"),
        ("_objc_getMetaClass", shim_objc_get_meta_class as u64, "objc_getMetaClass → metaclass table lookup"),
        ("_object_getClassName", shim_object_get_class_name as u64, "object_getClassName → class name for object"),
    );
}

/// Selector operations: register, compare, get name.
pub(super) fn install_selector_shims(table: &mut ShimTable) {
    register_shims!(table,
        ("_sel_registerName", shim_sel_register_name as u64, "sel_registerName → interned selector"),
        ("_sel_isEqual", shim_sel_is_equal as u64, "sel_isEqual → selector string comparison"),
        ("_sel_getName", shim_sel_get_name as u64, "sel_getName → selector C string"),
    );
}


