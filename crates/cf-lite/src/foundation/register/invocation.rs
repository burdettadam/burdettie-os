use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nsinvocation, nsmethodsignature, nsobj};

/// Register `NSMethodSignature` with type encoding queries.
pub fn register_nsmethodsignature_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmethodsignature_class_addr();
    let metaclass_addr = nsobj::nsmethodsignature_metaclass_addr();

    let info = ClassInfo {
        name: "NSMethodSignature".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "numberOfArguments",
                nsmethodsignature::shim_number_of_arguments as *const () as u64,
                "Q@:",
            ),
            me(
                "methodReturnType",
                nsmethodsignature::shim_method_return_type as *const () as u64,
                "r*@:",
            ),
            me(
                "methodReturnLength",
                nsmethodsignature::shim_method_return_length as *const () as u64,
                "Q@:",
            ),
            me(
                "getArgumentTypeAtIndex:",
                nsmethodsignature::shim_get_argument_type_at_index as *const () as u64,
                "r*@:Q",
            ),
            me(
                "frameLength",
                nsmethodsignature::shim_frame_length as *const () as u64,
                "Q@:",
            ),
            me(
                "dealloc",
                nsmethodsignature::shim_dealloc as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![me(
            "signatureWithObjCTypes:",
            nsmethodsignature::shim_signature_with_objc_types as *const () as u64,
            "@@:r*",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsmethodsignature::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMethodSignature");
}

/// Register `NSInvocation` with argument/invoke/return value methods.
pub fn register_nsinvocation_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsinvocation_class_addr();
    let metaclass_addr = nsobj::nsinvocation_metaclass_addr();

    let info = ClassInfo {
        name: "NSInvocation".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "methodSignature",
                nsinvocation::shim_method_signature as *const () as u64,
                "@@:",
            ),
            me("target", nsinvocation::shim_target as *const () as u64, "@@:"),
            me("setTarget:", nsinvocation::shim_set_target as *const () as u64, "v@:@"),
            me("selector", nsinvocation::shim_selector as *const () as u64, ":@:"),
            me(
                "setSelector:",
                nsinvocation::shim_set_selector as *const () as u64,
                "v@::",
            ),
            me(
                "setArgument:atIndex:",
                nsinvocation::shim_set_argument_at_index as *const () as u64,
                "v@:^vq",
            ),
            me(
                "getArgument:atIndex:",
                nsinvocation::shim_get_argument_at_index as *const () as u64,
                "v@:^vq",
            ),
            me("invoke", nsinvocation::shim_invoke as *const () as u64, "v@:"),
            me(
                "invokeWithTarget:",
                nsinvocation::shim_invoke_with_target as *const () as u64,
                "v@:@",
            ),
            me(
                "getReturnValue:",
                nsinvocation::shim_get_return_value as *const () as u64,
                "v@:^v",
            ),
            me(
                "setReturnValue:",
                nsinvocation::shim_set_return_value as *const () as u64,
                "v@:^v",
            ),
            me(
                "retainArguments",
                nsinvocation::shim_retain_arguments as *const () as u64,
                "v@:",
            ),
            me(
                "argumentsRetained",
                nsinvocation::shim_arguments_retained as *const () as u64,
                "B@:",
            ),
            me("dealloc", nsinvocation::shim_dealloc as *const () as u64, "v@:"),
        ],
        class_methods: vec![me(
            "invocationWithMethodSignature:",
            nsinvocation::shim_invocation_with_method_signature as *const () as u64,
            "@@:@",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsinvocation::INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSInvocation");
}
