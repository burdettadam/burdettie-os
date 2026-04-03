use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nserror, nsexception, nsobj};

/// Register `NSError` with error accessors and factory methods.
pub fn register_nserror_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nserror_class_addr();
    let metaclass_addr = nsobj::nserror_metaclass_addr();

    let info = ClassInfo {
        name: "NSError".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("domain", nserror::shim_nserror_domain as u64, "@@:"),
            me("code", nserror::shim_nserror_code as u64, "q@:"),
            me("userInfo", nserror::shim_nserror_user_info as u64, "@@:"),
            me(
                "localizedDescription",
                nserror::shim_nserror_localized_description as u64,
                "@@:",
            ),
            me(
                "description",
                nserror::shim_nserror_description as u64,
                "@@:",
            ),
            me(
                "localizedFailureReason",
                nserror::shim_nserror_localized_failure_reason as u64,
                "@@:",
            ),
            me(
                "localizedRecoverySuggestion",
                nserror::shim_nserror_localized_recovery_suggestion as u64,
                "@@:",
            ),
            me(
                "initWithDomain:code:userInfo:",
                nserror::shim_nserror_init_with_domain_code_userinfo as u64,
                "@@:@q@",
            ),
        ],
        class_methods: vec![me(
            "errorWithDomain:code:userInfo:",
            nserror::shim_nserror_error_with_domain_code_userinfo as u64,
            "@@:@q@",
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
    rt.register_metaclass_addr(metaclass_addr, "NSError");
}

/// Register `NSException` with exception accessors and factory methods.
pub fn register_nsexception_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsexception_class_addr();
    let metaclass_addr = nsobj::nsexception_metaclass_addr();

    let info = ClassInfo {
        name: "NSException".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nsexception::shim_nsexception_name as u64, "@@:"),
            me("reason", nsexception::shim_nsexception_reason as u64, "@@:"),
            me(
                "userInfo",
                nsexception::shim_nsexception_user_info as u64,
                "@@:",
            ),
            me(
                "description",
                nsexception::shim_nsexception_description as u64,
                "@@:",
            ),
            me("raise", nsexception::shim_nsexception_raise as u64, "v@:"),
            me(
                "initWithName:reason:userInfo:",
                nsexception::shim_nsexception_init_with_name_reason_userinfo as u64,
                "@@:@@@",
            ),
        ],
        class_methods: vec![me(
            "exceptionWithName:reason:userInfo:",
            nsexception::shim_nsexception_exception_with_name_reason_userinfo as u64,
            "@@:@@@",
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
    rt.register_metaclass_addr(metaclass_addr, "NSException");
}
