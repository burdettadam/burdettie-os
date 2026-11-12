//! Advanced ObjC runtime shim registration: methods, ivars, properties,
//! exceptions, associated objects, and weak references.

use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

use super::lifecycle::{
    shim_class_add_ivar,
    shim_class_add_method,
    shim_class_add_property,
    shim_class_add_protocol,
    shim_class_conforms_to_protocol,
    shim_class_copy_ivar_list,
    shim_class_copy_method_list,
    shim_class_copy_property_list,
    shim_class_copy_protocol_list,
    shim_class_get_class_method,
    shim_class_get_instance_method,
    shim_class_get_instance_variable,
    shim_class_get_method_implementation,
    shim_class_get_property,
    shim_class_replace_method,
    shim_ivar_get_name,
    shim_ivar_get_offset,
    shim_ivar_get_type_encoding,
    shim_method_copy_argument_type,
    // Method type introspection (Task 3)
    shim_method_copy_return_type,
    shim_method_exchange_implementations,
    shim_method_get_implementation,
    shim_method_get_name,
    shim_method_get_number_of_arguments,
    shim_method_get_type_encoding,
    shim_method_set_implementation,
    shim_ns_get_uncaught_exception_handler,
    shim_ns_set_uncaught_exception_handler,
    // Dynamic class creation shims
    shim_objc_allocate_class_pair,
    shim_objc_allocate_protocol,
    shim_objc_begin_catch,
    shim_objc_copy_weak,
    shim_objc_destroy_weak,
    shim_objc_dispose_class_pair,
    shim_objc_duplicate_class,
    shim_objc_end_catch,
    shim_objc_exception_extract,
    shim_objc_exception_match,
    shim_objc_exception_rethrow,
    shim_objc_exception_throw,
    // Legacy exception API
    shim_objc_exception_try_enter,
    shim_objc_exception_try_exit,
    shim_objc_get_associated_object,
    // Protocol shims
    shim_objc_get_protocol,
    shim_objc_init_weak,
    shim_objc_load_weak,
    shim_objc_load_weak_retained,
    shim_objc_move_weak,
    // Guest-side ObjC personality function (no host ObjC)
    shim_objc_personality_v0,
    shim_objc_register_class_pair,
    shim_objc_register_protocol,
    shim_objc_remove_associated_objects,
    shim_objc_set_associated_object,
    shim_objc_store_weak,
    shim_objc_sync_enter,
    shim_objc_sync_exit,
    shim_objc_terminate,
    shim_object_get_ivar,
    shim_object_set_ivar,
    shim_property_get_attributes,
    shim_property_get_name,
    shim_protocol_conforms_to_protocol,
    shim_protocol_copy_method_description_list,
    shim_protocol_copy_property_list,
    shim_protocol_copy_protocol_list,
    shim_protocol_get_method_description,
    shim_protocol_get_name,
    shim_protocol_get_property,
    shim_protocol_is_equal,
};

/// Method manipulation: add, replace, exchange, get implementation.
pub(super) fn install_method_shims(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_class_addMethod",
            shim_class_add_method as *const () as u64,
            "class_addMethod → add method to class"
        ),
        (
            "_class_getInstanceMethod",
            shim_class_get_instance_method as *const () as u64,
            "class_getInstanceMethod → Method struct lookup"
        ),
        (
            "_class_copyMethodList",
            shim_class_copy_method_list as *const () as u64,
            "class_copyMethodList → allocated Method array"
        ),
        (
            "_method_getName",
            shim_method_get_name as *const () as u64,
            "method_getName → selector of method"
        ),
        (
            "_method_getTypeEncoding",
            shim_method_get_type_encoding as *const () as u64,
            "method_getTypeEncoding → type encoding string"
        ),
        (
            "_method_getImplementation",
            shim_method_get_implementation as *const () as u64,
            "method_getImplementation → IMP function pointer"
        ),
        (
            "_method_setImplementation",
            shim_method_set_implementation as *const () as u64,
            "method_setImplementation → replace IMP, return old"
        ),
        (
            "_method_exchangeImplementations",
            shim_method_exchange_implementations as *const () as u64,
            "method_exchangeImplementations → swap two IMPs"
        ),
        (
            "_class_replaceMethod",
            shim_class_replace_method as *const () as u64,
            "class_replaceMethod → replace or add method"
        ),
        (
            "_method_getNumberOfArguments",
            shim_method_get_number_of_arguments as *const () as u64,
            "method_getNumberOfArguments → arg count"
        ),
        (
            "_class_getMethodImplementation",
            shim_class_get_method_implementation as *const () as u64,
            "class_getMethodImplementation → IMP via msg_lookup"
        ),
        (
            "_class_getClassMethod",
            shim_class_get_class_method as *const () as u64,
            "class_getClassMethod → lookup class method"
        ),
        (
            "_method_copyReturnType",
            shim_method_copy_return_type as *const () as u64,
            "method_copyReturnType → return type encoding string"
        ),
        (
            "_method_copyArgumentType",
            shim_method_copy_argument_type as *const () as u64,
            "method_copyArgumentType → argument type encoding string"
        ),
    );
}

/// Ivar and property introspection + object ivar access.
pub(super) fn install_ivar_property_shims(table: &mut ShimTable) {
    // Ivar introspection
    register_shims!(
        table,
        (
            "_class_copyIvarList",
            shim_class_copy_ivar_list as *const () as u64,
            "class_copyIvarList → allocated Ivar array"
        ),
        (
            "_class_getInstanceVariable",
            shim_class_get_instance_variable as *const () as u64,
            "class_getInstanceVariable → Ivar by name"
        ),
        (
            "_ivar_getName",
            shim_ivar_get_name as *const () as u64,
            "ivar_getName → ivar name C string"
        ),
        (
            "_ivar_getTypeEncoding",
            shim_ivar_get_type_encoding as *const () as u64,
            "ivar_getTypeEncoding → ivar type encoding"
        ),
        (
            "_ivar_getOffset",
            shim_ivar_get_offset as *const () as u64,
            "ivar_getOffset → ivar byte offset"
        ),
    );

    // Property introspection
    register_shims!(
        table,
        (
            "_class_copyPropertyList",
            shim_class_copy_property_list as *const () as u64,
            "class_copyPropertyList → allocated Property array"
        ),
        (
            "_class_getProperty",
            shim_class_get_property as *const () as u64,
            "class_getProperty → Property by name"
        ),
        (
            "_property_getName",
            shim_property_get_name as *const () as u64,
            "property_getName → property name C string"
        ),
        (
            "_property_getAttributes",
            shim_property_get_attributes as *const () as u64,
            "property_getAttributes → property attributes string"
        ),
    );

    // Object ivar access
    register_shims!(
        table,
        (
            "_object_getIvar",
            shim_object_get_ivar as *const () as u64,
            "object_getIvar → read ivar value"
        ),
        (
            "_object_setIvar",
            shim_object_set_ivar as *const () as u64,
            "object_setIvar → write ivar value"
        ),
    );
}

/// Exception handling + @synchronized support.
pub(super) fn install_exception_shims(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_objc_exception_throw",
            shim_objc_exception_throw as *const () as u64,
            "objc_exception_throw → throw ObjC exception"
        ),
        (
            "_objc_begin_catch",
            shim_objc_begin_catch as *const () as u64,
            "objc_begin_catch → enter @catch block"
        ),
        (
            "_objc_end_catch",
            shim_objc_end_catch as *const () as u64,
            "objc_end_catch → exit @catch block"
        ),
        (
            "_objc_exception_rethrow",
            shim_objc_exception_rethrow as *const () as u64,
            "objc_exception_rethrow → rethrow from @finally"
        ),
        (
            "_objc_terminate",
            shim_objc_terminate as *const () as u64,
            "objc_terminate → abort on uncaught exception"
        ),
        (
            "_objc_sync_enter",
            shim_objc_sync_enter as *const () as u64,
            "objc_sync_enter → @synchronized lock"
        ),
        (
            "_objc_sync_exit",
            shim_objc_sync_exit as *const () as u64,
            "objc_sync_exit → @synchronized unlock"
        ),
        (
            "_objc_exception_tryEnter",
            shim_objc_exception_try_enter as *const () as u64,
            "objc_exception_tryEnter → legacy @try enter"
        ),
        (
            "_objc_exception_tryExit",
            shim_objc_exception_try_exit as *const () as u64,
            "objc_exception_tryExit → legacy @try exit"
        ),
        (
            "_objc_exception_extract",
            shim_objc_exception_extract as *const () as u64,
            "objc_exception_extract → get thrown exception"
        ),
        (
            "_objc_exception_match",
            shim_objc_exception_match as *const () as u64,
            "objc_exception_match → @catch class matching"
        ),
        (
            "_NSSetUncaughtExceptionHandler",
            shim_ns_set_uncaught_exception_handler as *const () as u64,
            "NSSetUncaughtExceptionHandler"
        ),
        (
            "_NSGetUncaughtExceptionHandler",
            shim_ns_get_uncaught_exception_handler as *const () as u64,
            "NSGetUncaughtExceptionHandler"
        ),
        // Guest-side ObjC personality function — replaces host libobjc __objc_personality_v0.
        // Uses DWARF LSDA parsing + our class hierarchy; no host ObjC calls.
        (
            "___objc_personality_v0",
            shim_objc_personality_v0 as *const () as u64,
            "__objc_personality_v0 (guest LSDA parser, Linux-first)"
        )
    );
}

/// Protocol introspection: objc_getProtocol, protocol_getName, conformsToProtocol.
pub(super) fn install_protocol_shims(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_objc_getProtocol",
            shim_objc_get_protocol as *const () as u64,
            "objc_getProtocol → protocol lookup by name"
        ),
        (
            "_protocol_getName",
            shim_protocol_get_name as *const () as u64,
            "protocol_getName → protocol name C string"
        ),
        (
            "_protocol_isEqual",
            shim_protocol_is_equal as *const () as u64,
            "protocol_isEqual → protocol identity check"
        ),
        (
            "_class_conformsToProtocol",
            shim_class_conforms_to_protocol as *const () as u64,
            "class_conformsToProtocol → protocol conformance check"
        ),
        (
            "_protocol_conformsToProtocol",
            shim_protocol_conforms_to_protocol as *const () as u64,
            "protocol_conformsToProtocol → recursive protocol conformance"
        ),
        (
            "_class_copyProtocolList",
            shim_class_copy_protocol_list as *const () as u64,
            "class_copyProtocolList → adopted protocols array"
        ),
        (
            "_protocol_copyProtocolList",
            shim_protocol_copy_protocol_list as *const () as u64,
            "protocol_copyProtocolList → parent protocols array"
        ),
        (
            "_objc_allocateProtocol",
            shim_objc_allocate_protocol as *const () as u64,
            "objc_allocateProtocol → create new protocol"
        ),
        (
            "_objc_registerProtocol",
            shim_objc_register_protocol as *const () as u64,
            "objc_registerProtocol → finalize protocol creation"
        ),
        (
            "_class_addProtocol",
            shim_class_add_protocol as *const () as u64,
            "class_addProtocol → add protocol conformance"
        ),
        (
            "_protocol_copyMethodDescriptionList",
            shim_protocol_copy_method_description_list as *const () as u64,
            "protocol_copyMethodDescriptionList → method descriptions"
        ),
        (
            "_protocol_getMethodDescription",
            shim_protocol_get_method_description as *const () as u64,
            "protocol_getMethodDescription → single method description"
        ),
        (
            "_protocol_copyPropertyList",
            shim_protocol_copy_property_list as *const () as u64,
            "protocol_copyPropertyList → protocol properties"
        ),
        (
            "_protocol_getProperty",
            shim_protocol_get_property as *const () as u64,
            "protocol_getProperty → single protocol property"
        ),
    );
}

/// Dynamic class creation: allocate, register, dispose, duplicate, addIvar, addProperty.
pub(super) fn install_dynamic_class_shims(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_objc_allocateClassPair",
            shim_objc_allocate_class_pair as *const () as u64,
            "objc_allocateClassPair → create new class+metaclass"
        ),
        (
            "_objc_registerClassPair",
            shim_objc_register_class_pair as *const () as u64,
            "objc_registerClassPair → finalize dynamic class"
        ),
        (
            "_objc_disposeClassPair",
            shim_objc_dispose_class_pair as *const () as u64,
            "objc_disposeClassPair → destroy dynamic class"
        ),
        (
            "_objc_duplicateClass",
            shim_objc_duplicate_class as *const () as u64,
            "objc_duplicateClass → clone class for KVO"
        ),
        (
            "_class_addIvar",
            shim_class_add_ivar as *const () as u64,
            "class_addIvar → add ivar to unregistered class"
        ),
        (
            "_class_addProperty",
            shim_class_add_property as *const () as u64,
            "class_addProperty → add property to class"
        ),
    );
}

/// Associated objects + weak references.
pub(super) fn install_associated_shims(table: &mut ShimTable) {
    // Associated objects
    register_shims!(
        table,
        (
            "_objc_setAssociatedObject",
            shim_objc_set_associated_object as *const () as u64,
            "objc_setAssociatedObject → per-object key-value store"
        ),
        (
            "_objc_getAssociatedObject",
            shim_objc_get_associated_object as *const () as u64,
            "objc_getAssociatedObject → per-object key-value lookup"
        ),
        (
            "_objc_removeAssociatedObjects",
            shim_objc_remove_associated_objects as *const () as u64,
            "objc_removeAssociatedObjects → clear all associations"
        ),
    );

    // Weak references
    register_shims!(
        table,
        (
            "_objc_initWeak",
            shim_objc_init_weak as *const () as u64,
            "objc_initWeak → initialize weak variable"
        ),
        (
            "_objc_destroyWeak",
            shim_objc_destroy_weak as *const () as u64,
            "objc_destroyWeak → destroy weak variable"
        ),
        (
            "_objc_loadWeakRetained",
            shim_objc_load_weak_retained as *const () as u64,
            "objc_loadWeakRetained → load + retain weak ref"
        ),
        (
            "_objc_storeWeak",
            shim_objc_store_weak as *const () as u64,
            "objc_storeWeak → store weak reference"
        ),
        (
            "_objc_loadWeak",
            shim_objc_load_weak as *const () as u64,
            "objc_loadWeak → load weak reference"
        ),
        (
            "_objc_copyWeak",
            shim_objc_copy_weak as *const () as u64,
            "objc_copyWeak → copy weak variable"
        ),
        (
            "_objc_moveWeak",
            shim_objc_move_weak as *const () as u64,
            "objc_moveWeak → move weak variable"
        ),
    );
}
