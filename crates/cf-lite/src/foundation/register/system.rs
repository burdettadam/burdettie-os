use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nsbundle, nsfilemanager, nsobj, nsprocessinfo, nsuserdefaults};

/// Register `NSProcessInfo` with process info accessors and singleton factory.
pub fn register_nsprocessinfo_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsprocessinfo_class_addr();
    let metaclass_addr = nsobj::nsprocessinfo_metaclass_addr();

    let info = ClassInfo {
        name: "NSProcessInfo".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "processName",
                nsprocessinfo::shim_nsprocessinfo_process_name as u64,
                "@@:",
            ),
            me(
                "processIdentifier",
                nsprocessinfo::shim_nsprocessinfo_process_identifier as u64,
                "i@:",
            ),
            me(
                "operatingSystemVersionString",
                nsprocessinfo::shim_nsprocessinfo_os_version_string as u64,
                "@@:",
            ),
            me(
                "arguments",
                nsprocessinfo::shim_nsprocessinfo_arguments as u64,
                "@@:",
            ),
            me(
                "environment",
                nsprocessinfo::shim_nsprocessinfo_environment as u64,
                "@@:",
            ),
            me(
                "hostName",
                nsprocessinfo::shim_nsprocessinfo_host_name as u64,
                "@@:",
            ),
            me(
                "description",
                nsprocessinfo::shim_nsprocessinfo_description as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "processInfo",
            nsprocessinfo::shim_nsprocessinfo_process_info as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSProcessInfo");
}

/// Register `NSBundle` with bundle accessors and singleton factory.
pub fn register_nsbundle_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbundle_class_addr();
    let metaclass_addr = nsobj::nsbundle_metaclass_addr();

    let info = ClassInfo {
        name: "NSBundle".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "bundlePath",
                nsbundle::shim_nsbundle_bundle_path as u64,
                "@@:",
            ),
            me(
                "resourcePath",
                nsbundle::shim_nsbundle_resource_path as u64,
                "@@:",
            ),
            me(
                "executablePath",
                nsbundle::shim_nsbundle_executable_path as u64,
                "@@:",
            ),
            me(
                "bundleIdentifier",
                nsbundle::shim_nsbundle_bundle_identifier as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "mainBundle",
            nsbundle::shim_nsbundle_main_bundle as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSBundle");
}

/// Register `NSFileManager` with file system query methods and singleton factory.
pub fn register_nsfilemanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsfilemanager_class_addr();
    let metaclass_addr = nsobj::nsfilemanager_metaclass_addr();

    let info = ClassInfo {
        name: "NSFileManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "fileExistsAtPath:",
                nsfilemanager::shim_nsfilemanager_file_exists_at_path as u64,
                "B@:@",
            ),
            me(
                "fileExistsAtPath:isDirectory:",
                nsfilemanager::shim_nsfilemanager_file_exists_at_path_is_directory as u64,
                "B@:@^B",
            ),
            me(
                "currentDirectoryPath",
                nsfilemanager::shim_nsfilemanager_current_directory_path as u64,
                "@@:",
            ),
            me(
                "contentsOfDirectoryAtPath:error:",
                nsfilemanager::shim_nsfilemanager_contents_of_directory_at_path as u64,
                "@@:@^@",
            ),
        ],
        class_methods: vec![me(
            "defaultManager",
            nsfilemanager::shim_nsfilemanager_default_manager as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSFileManager");
}

/// Register `NSUserDefaults` with key-value storage methods.
pub fn register_nsuserdefaults_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsuserdefaults_class_addr();
    let metaclass_addr = nsobj::nsuserdefaults_metaclass_addr();

    let info = ClassInfo {
        name: "NSUserDefaults".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "objectForKey:",
                nsuserdefaults::shim_nsuserdefaults_object_for_key as u64,
                "@@:@",
            ),
            me(
                "stringForKey:",
                nsuserdefaults::shim_nsuserdefaults_string_for_key as u64,
                "@@:@",
            ),
            me(
                "integerForKey:",
                nsuserdefaults::shim_nsuserdefaults_integer_for_key as u64,
                "q@:@",
            ),
            me(
                "boolForKey:",
                nsuserdefaults::shim_nsuserdefaults_bool_for_key as u64,
                "B@:@",
            ),
            me(
                "doubleForKey:",
                nsuserdefaults::shim_nsuserdefaults_double_for_key as u64,
                "d@:@",
            ),
            me(
                "setObject:forKey:",
                nsuserdefaults::shim_nsuserdefaults_set_object as u64,
                "v@:@@",
            ),
            me(
                "removeObjectForKey:",
                nsuserdefaults::shim_nsuserdefaults_remove_object as u64,
                "v@:@",
            ),
            me(
                "synchronize",
                nsuserdefaults::shim_nsuserdefaults_synchronize as u64,
                "B@:",
            ),
            me(
                "setInteger:forKey:",
                nsuserdefaults::shim_nsuserdefaults_set_integer as u64,
                "v@:q@",
            ),
            me(
                "setBool:forKey:",
                nsuserdefaults::shim_nsuserdefaults_set_bool as u64,
                "v@:B@",
            ),
            me(
                "setDouble:forKey:",
                nsuserdefaults::shim_nsuserdefaults_set_double as u64,
                "v@:d@",
            ),
            me(
                "setFloat:forKey:",
                nsuserdefaults::shim_nsuserdefaults_set_float as u64,
                "v@:f@",
            ),
            me(
                "floatForKey:",
                nsuserdefaults::shim_nsuserdefaults_float_for_key as u64,
                "f@:@",
            ),
            me(
                "arrayForKey:",
                nsuserdefaults::shim_nsuserdefaults_array_for_key as u64,
                "@@:@",
            ),
            me(
                "dictionaryForKey:",
                nsuserdefaults::shim_nsuserdefaults_dictionary_for_key as u64,
                "@@:@",
            ),
        ],
        class_methods: vec![me(
            "standardUserDefaults",
            nsuserdefaults::shim_nsuserdefaults_standard as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSUserDefaults");
}
