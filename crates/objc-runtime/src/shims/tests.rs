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
    assert!(table.resolve("_objc_setProperty_nonatomic").is_some());
    assert!(table.resolve("_method_setImplementation").is_some());
    assert!(table.resolve("_method_exchangeImplementations").is_some());
    assert!(table.resolve("_class_replaceMethod").is_some());
    assert!(table.resolve("_objc_opt_class").is_some());
    assert!(table.resolve("_objc_opt_respondsToSelector").is_some());
    // Protocol shims
    assert!(table.resolve("_objc_getProtocol").is_some());
    assert!(table.resolve("_protocol_getName").is_some());
    assert!(table.resolve("_protocol_isEqual").is_some());
    assert!(table.resolve("_class_conformsToProtocol").is_some());
    assert!(table.resolve("_protocol_conformsToProtocol").is_some());
    assert!(table.resolve("_class_copyProtocolList").is_some());
    assert!(table.resolve("_protocol_copyProtocolList").is_some());
    // New protocol shims
    assert!(table.resolve("_objc_allocateProtocol").is_some());
    assert!(table.resolve("_objc_registerProtocol").is_some());
    assert!(table.resolve("_class_addProtocol").is_some());
    assert!(table.resolve("_protocol_copyMethodDescriptionList").is_some());
    assert!(table.resolve("_protocol_getMethodDescription").is_some());
    assert!(table.resolve("_protocol_copyPropertyList").is_some());
    assert!(table.resolve("_protocol_getProperty").is_some());
    // Dynamic class creation shims
    assert!(table.resolve("_objc_allocateClassPair").is_some());
    assert!(table.resolve("_objc_registerClassPair").is_some());
    assert!(table.resolve("_objc_disposeClassPair").is_some());
    assert!(table.resolve("_objc_duplicateClass").is_some());
    assert!(table.resolve("_class_addIvar").is_some());
    assert!(table.resolve("_class_addProperty").is_some());
    // Struct return / extended ABI shims (C3)
    assert!(table.resolve("_objc_msgSend_stret").is_some());
    assert!(table.resolve("_objc_msgSend_fp2ret").is_some());
    assert!(table.resolve("_objc_msgSendSuper_stret").is_some());
    assert!(table.resolve("_objc_msgSendSuper2_stret").is_some());
    assert_eq!(table.len(), 102);
}
