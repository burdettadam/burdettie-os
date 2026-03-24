//! ObjC runtime shims: objc_msgSend, objc_getClass, etc.
//!
//! These are `extern "C"` functions registered in the `ShimTable`.
//! They reference the global `OBJC_RUNTIME` (populated during image
//! registration) to perform class lookup and method dispatch.
//!
//! On aarch64-apple, `objc_msgSend` uses an ARM64 assembly trampoline
//! (`trampoline.s`) that preserves the entire argument register file
//! (x0-x7, d0-d7) across the class-table lookup, then tail-calls the
//! IMP.

mod dispatch;
mod lifecycle;

use libsystem_shims::table::ShimTable;

use dispatch::{objc_msg_send_addr, objc_msg_send_super2_addr, objc_msg_send_super_addr};
use lifecycle::{
    shim_class_add_method, shim_class_copy_ivar_list, shim_class_copy_method_list,
    shim_class_copy_property_list, shim_class_create_instance, shim_class_get_instance_method,
    shim_class_get_instance_size, shim_class_get_instance_variable,
    shim_class_get_method_implementation, shim_class_get_name, shim_class_get_property,
    shim_class_get_superclass, shim_class_is_meta_class, shim_class_responds_to_selector,
    shim_ivar_get_name, shim_ivar_get_offset, shim_ivar_get_type_encoding,
    shim_method_get_implementation, shim_method_get_name, shim_method_get_number_of_arguments,
    shim_method_get_type_encoding, shim_objc_alloc, shim_objc_alloc_init, shim_objc_autorelease,
    shim_objc_autorelease_pool_pop, shim_objc_autorelease_pool_push,
    shim_objc_autorelease_return_value, shim_objc_begin_catch, shim_objc_end_catch,
    shim_objc_exception_rethrow, shim_objc_get_class, shim_objc_lookup_class, shim_objc_opt_self,
    shim_objc_release, shim_objc_retain, shim_objc_retain_autorelease,
    shim_objc_retain_autoreleased_return_value, shim_objc_store_strong, shim_objc_sync_enter,
    shim_objc_sync_exit, shim_objc_terminate, shim_objc_unsafe_claim_autoreleased_return_value,
    shim_object_get_class, shim_object_get_ivar, shim_object_set_ivar,
    shim_property_get_attributes, shim_property_get_name, shim_sel_get_name, shim_sel_is_equal,
    shim_sel_register_name,
};

// Re-export NSObject memory-management method shims for Foundation registration.
pub use lifecycle::{
    shim_nsobject_autorelease, shim_nsobject_dealloc, shim_nsobject_release, shim_nsobject_retain,
    shim_nsobject_retain_count,
};

// Re-export exception shim for NSException's -raise method.
pub use lifecycle::shim_objc_exception_throw;

/// A zero-filled 16 bytes that serves as the empty method cache.
static EMPTY_CACHE: [u8; 16] = [0u8; 16];

/// Install ObjC runtime shims into a `ShimTable`.
pub struct ObjcShims;

impl ObjcShims {
    pub fn install(table: &mut ShimTable) {
        // Message dispatch
        table.register(
            "_objc_msgSend",
            objc_msg_send_addr(),
            "objc_msgSend → ARM64 trampoline dispatch",
        );
        table.register(
            "_objc_msgSendSuper",
            objc_msg_send_super_addr(),
            "objc_msgSendSuper → super dispatch trampoline",
        );
        table.register(
            "_objc_msgSendSuper2",
            objc_msg_send_super2_addr(),
            "objc_msgSendSuper2 → super2 dispatch trampoline",
        );

        // Runtime queries
        table.register(
            "_objc_getClass",
            shim_objc_get_class as u64,
            "objc_getClass → class table lookup",
        );
        table.register(
            "_class_createInstance",
            shim_class_create_instance as u64,
            "class_createInstance → allocate + set isa",
        );
        table.register(
            "_sel_registerName",
            shim_sel_register_name as u64,
            "sel_registerName → interned selector",
        );
        table.register(
            "_class_getName",
            shim_class_get_name as u64,
            "class_getName → class name string",
        );
        table.register(
            "__objc_empty_cache",
            EMPTY_CACHE.as_ptr() as u64,
            "objc empty cache data symbol",
        );

        // ARC / memory management
        table.register(
            "_objc_autoreleasePoolPush",
            shim_objc_autorelease_pool_push as u64,
            "objc_autoreleasePoolPush → sentinel token",
        );
        table.register(
            "_objc_autoreleasePoolPop",
            shim_objc_autorelease_pool_pop as u64,
            "objc_autoreleasePoolPop → noop",
        );
        table.register(
            "_objc_retain",
            shim_objc_retain as u64,
            "objc_retain → pass-through",
        );
        table.register(
            "_objc_release",
            shim_objc_release as u64,
            "objc_release → noop",
        );
        table.register(
            "_objc_storeStrong",
            shim_objc_store_strong as u64,
            "objc_storeStrong → direct store",
        );
        table.register(
            "_objc_alloc",
            shim_objc_alloc as u64,
            "objc_alloc → allocate via guest runtime",
        );
        table.register(
            "_objc_alloc_init",
            shim_objc_alloc_init as u64,
            "objc_alloc_init → allocate + init via guest runtime",
        );
        table.register(
            "_objc_opt_self",
            shim_objc_opt_self as u64,
            "objc_opt_self → return self",
        );
        table.register(
            "_objc_autorelease",
            shim_objc_autorelease as u64,
            "objc_autorelease → add to current pool",
        );
        table.register(
            "_objc_autoreleaseReturnValue",
            shim_objc_autorelease_return_value as u64,
            "objc_autoreleaseReturnValue → autorelease for return value",
        );
        table.register(
            "_objc_retainAutoreleasedReturnValue",
            shim_objc_retain_autoreleased_return_value as u64,
            "objc_retainAutoreleasedReturnValue → retain returned value",
        );
        table.register(
            "_objc_unsafeClaimAutoreleasedReturnValue",
            shim_objc_unsafe_claim_autoreleased_return_value as u64,
            "objc_unsafeClaimAutoreleasedReturnValue → claim without retain",
        );
        table.register(
            "_objc_retainAutorelease",
            shim_objc_retain_autorelease as u64,
            "objc_retainAutorelease → retain + autorelease combined",
        );

        // Blocks runtime
        table.register(
            "__NSConcreteGlobalBlock",
            crate::blocks::global_block_class_addr(),
            "_NSConcreteGlobalBlock class reference",
        );
        table.register(
            "__NSConcreteStackBlock",
            crate::blocks::stack_block_class_addr(),
            "_NSConcreteStackBlock class reference",
        );
        table.register(
            "__Block_copy",
            crate::blocks::shim_block_copy as u64,
            "_Block_copy → copy stack block to heap",
        );
        table.register(
            "__Block_release",
            crate::blocks::shim_block_release as u64,
            "_Block_release → release heap block",
        );
        table.register(
            "_objc_retainBlock",
            crate::blocks::shim_objc_retain_block as u64,
            "objc_retainBlock → Block_copy for ARC",
        );

        // Introspection
        table.register(
            "_object_getClass",
            shim_object_get_class as u64,
            "object_getClass → read ISA",
        );
        table.register(
            "_objc_lookUpClass",
            shim_objc_lookup_class as u64,
            "objc_lookUpClass → class table lookup",
        );
        table.register(
            "_class_getSuperclass",
            shim_class_get_superclass as u64,
            "class_getSuperclass → superclass addr",
        );
        table.register(
            "_class_respondsToSelector",
            shim_class_responds_to_selector as u64,
            "class_respondsToSelector → msg_lookup check",
        );
        table.register(
            "_sel_isEqual",
            shim_sel_is_equal as u64,
            "sel_isEqual → selector string comparison",
        );
        table.register(
            "_sel_getName",
            shim_sel_get_name as u64,
            "sel_getName → selector C string",
        );

        // Method / class introspection
        table.register(
            "_class_addMethod",
            shim_class_add_method as u64,
            "class_addMethod → add method to class",
        );
        table.register(
            "_class_getInstanceMethod",
            shim_class_get_instance_method as u64,
            "class_getInstanceMethod → Method struct lookup",
        );
        table.register(
            "_class_copyMethodList",
            shim_class_copy_method_list as u64,
            "class_copyMethodList → allocated Method array",
        );
        table.register(
            "_method_getName",
            shim_method_get_name as u64,
            "method_getName → selector of method",
        );
        table.register(
            "_method_getTypeEncoding",
            shim_method_get_type_encoding as u64,
            "method_getTypeEncoding → type encoding string",
        );
        table.register(
            "_method_getImplementation",
            shim_method_get_implementation as u64,
            "method_getImplementation → IMP function pointer",
        );
        table.register(
            "_method_getNumberOfArguments",
            shim_method_get_number_of_arguments as u64,
            "method_getNumberOfArguments → arg count",
        );
        table.register(
            "_class_getMethodImplementation",
            shim_class_get_method_implementation as u64,
            "class_getMethodImplementation → IMP via msg_lookup",
        );
        table.register(
            "_class_getInstanceSize",
            shim_class_get_instance_size as u64,
            "class_getInstanceSize → instance size",
        );
        table.register(
            "_class_isMetaClass",
            shim_class_is_meta_class as u64,
            "class_isMetaClass → meta class check",
        );

        // Ivar introspection
        table.register(
            "_class_copyIvarList",
            shim_class_copy_ivar_list as u64,
            "class_copyIvarList → allocated Ivar array",
        );
        table.register(
            "_class_getInstanceVariable",
            shim_class_get_instance_variable as u64,
            "class_getInstanceVariable → Ivar by name",
        );
        table.register(
            "_ivar_getName",
            shim_ivar_get_name as u64,
            "ivar_getName → ivar name C string",
        );
        table.register(
            "_ivar_getTypeEncoding",
            shim_ivar_get_type_encoding as u64,
            "ivar_getTypeEncoding → ivar type encoding",
        );
        table.register(
            "_ivar_getOffset",
            shim_ivar_get_offset as u64,
            "ivar_getOffset → ivar byte offset",
        );

        // Property introspection
        table.register(
            "_class_copyPropertyList",
            shim_class_copy_property_list as u64,
            "class_copyPropertyList → allocated Property array",
        );
        table.register(
            "_class_getProperty",
            shim_class_get_property as u64,
            "class_getProperty → Property by name",
        );
        table.register(
            "_property_getName",
            shim_property_get_name as u64,
            "property_getName → property name C string",
        );
        table.register(
            "_property_getAttributes",
            shim_property_get_attributes as u64,
            "property_getAttributes → property attributes string",
        );

        // Object ivar access
        table.register(
            "_object_getIvar",
            shim_object_get_ivar as u64,
            "object_getIvar → read ivar value",
        );
        table.register(
            "_object_setIvar",
            shim_object_set_ivar as u64,
            "object_setIvar → write ivar value",
        );

        // Exception handling
        table.register(
            "_objc_exception_throw",
            shim_objc_exception_throw as u64,
            "objc_exception_throw → throw ObjC exception",
        );
        table.register(
            "_objc_begin_catch",
            shim_objc_begin_catch as u64,
            "objc_begin_catch → enter @catch block",
        );
        table.register(
            "_objc_end_catch",
            shim_objc_end_catch as u64,
            "objc_end_catch → exit @catch block",
        );
        table.register(
            "_objc_exception_rethrow",
            shim_objc_exception_rethrow as u64,
            "objc_exception_rethrow → rethrow from @finally",
        );
        table.register(
            "_objc_terminate",
            shim_objc_terminate as u64,
            "objc_terminate → abort on uncaught exception",
        );

        // @synchronized support
        table.register(
            "_objc_sync_enter",
            shim_objc_sync_enter as u64,
            "objc_sync_enter → @synchronized lock",
        );
        table.register(
            "_objc_sync_exit",
            shim_objc_sync_exit as u64,
            "objc_sync_exit → @synchronized unlock",
        );

        log::debug!("ObjcShims: registered 60 ObjC runtime shims");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objc_shims_registered() {
        let mut table = ShimTable::new();
        ObjcShims::install(&mut table);
        assert!(table.resolve("_objc_msgSend").is_some());
        assert!(table.resolve("_objc_msgSendSuper").is_some());
        assert!(table.resolve("_objc_msgSendSuper2").is_some());
        assert!(table.resolve("_objc_getClass").is_some());
        assert!(table.resolve("_class_createInstance").is_some());
        assert!(table.resolve("_sel_registerName").is_some());
        assert!(table.resolve("_class_getName").is_some());
        assert!(table.resolve("__objc_empty_cache").is_some());
        assert!(table.resolve("_objc_autoreleasePoolPush").is_some());
        assert!(table.resolve("_objc_autoreleasePoolPop").is_some());
        assert!(table.resolve("_objc_retain").is_some());
        assert!(table.resolve("_objc_release").is_some());
        assert!(table.resolve("_objc_storeStrong").is_some());
        assert!(table.resolve("_objc_alloc").is_some());
        assert!(table.resolve("_objc_alloc_init").is_some());
        assert!(table.resolve("_objc_opt_self").is_some());
        assert!(table.resolve("_objc_autorelease").is_some());
        assert!(table.resolve("_objc_autoreleaseReturnValue").is_some());
        assert!(table
            .resolve("_objc_retainAutoreleasedReturnValue")
            .is_some());
        assert!(table
            .resolve("_objc_unsafeClaimAutoreleasedReturnValue")
            .is_some());
        assert!(table.resolve("_objc_retainAutorelease").is_some());
        assert!(table.resolve("__NSConcreteGlobalBlock").is_some());
        assert!(table.resolve("__NSConcreteStackBlock").is_some());
        assert!(table.resolve("__Block_copy").is_some());
        assert!(table.resolve("__Block_release").is_some());
        assert!(table.resolve("_objc_retainBlock").is_some());
        assert!(table.resolve("_object_getClass").is_some());
        assert!(table.resolve("_objc_lookUpClass").is_some());
        assert!(table.resolve("_class_getSuperclass").is_some());
        assert!(table.resolve("_class_respondsToSelector").is_some());
        assert!(table.resolve("_sel_isEqual").is_some());
        assert!(table.resolve("_sel_getName").is_some());
        assert!(table.resolve("_class_addMethod").is_some());
        assert!(table.resolve("_class_getInstanceMethod").is_some());
        assert!(table.resolve("_class_copyMethodList").is_some());
        assert!(table.resolve("_method_getName").is_some());
        assert!(table.resolve("_method_getTypeEncoding").is_some());
        assert!(table.resolve("_method_getImplementation").is_some());
        assert!(table.resolve("_method_getNumberOfArguments").is_some());
        assert!(table.resolve("_class_getMethodImplementation").is_some());
        assert!(table.resolve("_class_getInstanceSize").is_some());
        assert!(table.resolve("_class_isMetaClass").is_some());
        assert!(table.resolve("_class_copyIvarList").is_some());
        assert!(table.resolve("_class_getInstanceVariable").is_some());
        assert!(table.resolve("_ivar_getName").is_some());
        assert!(table.resolve("_ivar_getTypeEncoding").is_some());
        assert!(table.resolve("_ivar_getOffset").is_some());
        assert!(table.resolve("_class_copyPropertyList").is_some());
        assert!(table.resolve("_class_getProperty").is_some());
        assert!(table.resolve("_property_getName").is_some());
        assert!(table.resolve("_property_getAttributes").is_some());
        assert!(table.resolve("_object_getIvar").is_some());
        assert!(table.resolve("_object_setIvar").is_some());
        assert!(table.resolve("_objc_exception_throw").is_some());
        assert!(table.resolve("_objc_begin_catch").is_some());
        assert!(table.resolve("_objc_end_catch").is_some());
        assert!(table.resolve("_objc_exception_rethrow").is_some());
        assert!(table.resolve("_objc_terminate").is_some());
        assert_eq!(table.len(), 60);
    }
}
