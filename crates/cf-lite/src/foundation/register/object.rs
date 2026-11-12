use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nskeyvaluecoding, nskeyvalueobserving, nsobj};

/// Register `NSObject` as root class with basic instance and class methods.
pub fn register_nsobject_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsobject_class_addr();
    let metaclass_addr = nsobj::nsobject_metaclass_addr();

    let info = ClassInfo {
        name: "NSObject".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsobj::shim_nsobject_init as *const () as u64, "@@:"),
            me("self", nsobj::shim_nsobject_self as *const () as u64, "@@:"),
            me(
                "class",
                nsobj::shim_nsobject_class as *const () as u64,
                "#@:",
            ),
            me(
                "respondsToSelector:",
                nsobj::shim_nsobject_responds_to_selector as *const () as u64,
                "B@::",
            ),
            me(
                "isKindOfClass:",
                nsobj::shim_nsobject_is_kind_of_class as *const () as u64,
                "B@:#",
            ),
            me(
                "conformsToProtocol:",
                nsobj::shim_nsobject_conforms_to_protocol as *const () as u64,
                "B@:@",
            ),
            me(
                "retain",
                objc_runtime::shim_nsobject_retain as *const () as u64,
                "@@:",
            ),
            me(
                "release",
                objc_runtime::shim_nsobject_release as *const () as u64,
                "v@:",
            ),
            me(
                "autorelease",
                objc_runtime::shim_nsobject_autorelease as *const () as u64,
                "@@:",
            ),
            me(
                "retainCount",
                objc_runtime::shim_nsobject_retain_count as *const () as u64,
                "Q@:",
            ),
            me(
                "dealloc",
                objc_runtime::shim_nsobject_dealloc as *const () as u64,
                "v@:",
            ),
            // KVO methods
            me(
                "addObserver:forKeyPath:options:context:",
                nskeyvalueobserving::shim_nsobject_add_observer as *const () as u64,
                "v@:@@Q^v",
            ),
            me(
                "removeObserver:forKeyPath:",
                nskeyvalueobserving::shim_nsobject_remove_observer as *const () as u64,
                "v@:@@",
            ),
            me(
                "willChangeValueForKey:",
                nskeyvalueobserving::shim_nsobject_will_change_value_for_key as *const () as u64,
                "v@:@",
            ),
            me(
                "didChangeValueForKey:",
                nskeyvalueobserving::shim_nsobject_did_change_value_for_key as *const () as u64,
                "v@:@",
            ),
            me(
                "observeValueForKeyPath:ofObject:change:context:",
                nskeyvalueobserving::shim_nsobject_observe_value as *const () as u64,
                "v@:@@@^v",
            ),
            // KVC methods
            me(
                "valueForKey:",
                nskeyvaluecoding::shim_value_for_key as *const () as u64,
                "@@:@",
            ),
            me(
                "setValue:forKey:",
                nskeyvaluecoding::shim_set_value_for_key as *const () as u64,
                "v@:@@",
            ),
            me(
                "valueForKeyPath:",
                nskeyvaluecoding::shim_value_for_key_path as *const () as u64,
                "@@:@",
            ),
            me(
                "setValue:forKeyPath:",
                nskeyvaluecoding::shim_set_value_for_key_path as *const () as u64,
                "v@:@@",
            ),
            me(
                "valueForUndefinedKey:",
                nskeyvaluecoding::shim_value_for_undefined_key_method as *const () as u64,
                "@@:@",
            ),
            me(
                "setValue:forUndefinedKey:",
                nskeyvaluecoding::shim_set_value_for_undefined_key_method as *const () as u64,
                "v@:@@",
            ),
            // Forwarding methods
            me(
                "methodSignatureForSelector:",
                nsobj::shim_nsobject_method_signature_for_selector as *const () as u64,
                "@@::",
            ),
            me(
                "forwardInvocation:",
                nsobj::shim_nsobject_forward_invocation as *const () as u64,
                "v@:@",
            ),
            me(
                "doesNotRecognizeSelector:",
                nsobj::shim_nsobject_does_not_recognize_selector as *const () as u64,
                "v@::",
            ),
            me(
                "superclass",
                nsobj::shim_nsobject_superclass as *const () as u64,
                "#@:",
            ),
            me(
                "className",
                nsobj::shim_nsobject_class_name as *const () as u64,
                "@@:",
            ),
            me(
                "debugDescription",
                nsobj::shim_nsobject_debug_description as *const () as u64,
                "@@:",
            ),
            // performSelector: family
            me(
                "performSelector:",
                nsobj::shim_nsobject_perform_selector as *const () as u64,
                "@@::",
            ),
            me(
                "performSelector:withObject:",
                nsobj::shim_nsobject_perform_selector_with_object as *const () as u64,
                "@@::@",
            ),
            me(
                "performSelector:withObject:afterDelay:",
                nsobj::shim_nsobject_perform_selector_after_delay as *const () as u64,
                "v@::@d",
            ),
        ],
        class_methods: vec![
            me(
                "alloc",
                nsobj::shim_nsobject_class_alloc as *const () as u64,
                "@@:",
            ),
            me(
                "new",
                nsobj::shim_nsobject_class_new as *const () as u64,
                "@@:",
            ),
            me(
                "isSubclassOfClass:",
                nsobj::shim_nsobject_is_subclass_of_class as *const () as u64,
                "B@:#",
            ),
            me(
                "resolveInstanceMethod:",
                nsobj::shim_nsobject_resolve_instance_method as *const () as u64,
                "B@::",
            ),
            me(
                "resolveClassMethod:",
                nsobj::shim_nsobject_resolve_class_method as *const () as u64,
                "B@::",
            ),
            me(
                "methodSignatureForSelector:",
                nsobj::shim_nsobject_class_method_signature_for_selector as *const () as u64,
                "@@::",
            ),
            me(
                "superclass",
                nsobj::shim_nsobject_class_superclass as *const () as u64,
                "#@:",
            ),
            me(
                "cancelPreviousPerformRequestsWithTarget:",
                nsobj::shim_nsobject_cancel_previous_perform as *const () as u64,
                "v@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 8,
        is_root: true,
        is_meta: false,
        superclass_addr: None,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSObject");
}
