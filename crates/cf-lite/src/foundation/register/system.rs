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
                "operatingSystemVersion",
                nsprocessinfo::shim_nsprocessinfo_operating_system_version as u64,
                "{_NSOperatingSystemVersion=qqq}@:",
            ),
            me(
                "isOperatingSystemAtLeastVersion:",
                nsprocessinfo::shim_nsprocessinfo_is_operating_system_at_least_version as u64,
                "B@:{_NSOperatingSystemVersion=qqq}",
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
            me(
                "globallyUniqueString",
                nsprocessinfo::shim_nsprocessinfo_globally_unique_string as u64,
                "@@:",
            ),
            me(
                "activeProcessorCount",
                nsprocessinfo::shim_nsprocessinfo_active_processor_count as u64,
                "Q@:",
            ),
            me(
                "physicalMemory",
                nsprocessinfo::shim_nsprocessinfo_physical_memory as u64,
                "Q@:",
            ),
            me(
                "systemUptime",
                nsprocessinfo::shim_nsprocessinfo_system_uptime as u64,
                "d@:",
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
            me("bundlePath", nsbundle::shim_nsbundle_bundle_path as u64, "@@:"),
            me("bundleURL", nsbundle::shim_nsbundle_bundle_url as u64, "@@:"),
            me("resourcePath", nsbundle::shim_nsbundle_resource_path as u64, "@@:"),
            me("executablePath", nsbundle::shim_nsbundle_executable_path as u64, "@@:"),
            me("executableURL", nsbundle::shim_nsbundle_executable_url as u64, "@@:"),
            me("bundleIdentifier", nsbundle::shim_nsbundle_bundle_identifier as u64, "@@:"),
            me("infoDictionary", nsbundle::shim_nsbundle_info_dictionary as u64, "@@:"),
            me("objectForInfoDictionaryKey:", nsbundle::shim_nsbundle_object_for_info_key as u64, "@@:@"),
            me("pathForResource:ofType:", nsbundle::shim_nsbundle_path_for_resource as u64, "@@:@@"),
            me("loadNibNamed:owner:topLevelObjects:", nsbundle::shim_nsbundle_load_nib_named as u64, "B@:@@^@"),
            me("URLForResource:withExtension:", nsbundle::shim_nsbundle_url_for_resource as u64, "@@:@@"),
            me("localizedStringForKey:value:table:", nsbundle::shim_nsbundle_localized_string as u64, "@@:@@@"),
        ],
        class_methods: vec![
            me("mainBundle", nsbundle::shim_nsbundle_main_bundle as u64, "@@:"),
            me("allFrameworks", nsbundle::shim_nsbundle_all_frameworks as u64, "@@:"),
            me("bundleWithPath:", nsbundle::shim_nsbundle_bundle_with_path as u64, "@@:@"),
            me("bundleForClass:", nsbundle::shim_nsbundle_bundle_for_class as u64, "@@:@"),
        ],
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
                "changeCurrentDirectoryPath:",
                nsfilemanager::shim_nsfilemanager_change_current_directory_path as u64,
                "B@:@",
            ),
            me(
                "contentsOfDirectoryAtPath:error:",
                nsfilemanager::shim_nsfilemanager_contents_of_directory_at_path as u64,
                "@@:@^@",
            ),
            me(
                "copyItemAtPath:toPath:error:",
                nsfilemanager::shim_nsfilemanager_copy_item as u64,
                "B@:@@^@",
            ),
            me(
                "moveItemAtPath:toPath:error:",
                nsfilemanager::shim_nsfilemanager_move_item as u64,
                "B@:@@^@",
            ),
            me(
                "removeItemAtPath:error:",
                nsfilemanager::shim_nsfilemanager_remove_item as u64,
                "B@:@^@",
            ),
            me(
                "createDirectoryAtPath:withIntermediateDirectories:attributes:error:",
                nsfilemanager::shim_nsfilemanager_create_directory as u64,
                "B@:@B@@^@",
            ),
            me(
                "attributesOfItemAtPath:error:",
                nsfilemanager::shim_nsfilemanager_attributes_of_item as u64,
                "@@:@^@",
            ),
            me(
                "createFileAtPath:contents:attributes:",
                nsfilemanager::shim_nsfilemanager_create_file as u64,
                "c@:@@@",
            ),
            me(
                "contentsAtPath:",
                nsfilemanager::shim_nsfilemanager_contents_at_path as u64,
                "@@:@",
            ),
            me(
                "isReadableFileAtPath:",
                nsfilemanager::shim_nsfilemanager_is_readable_file as u64,
                "c@:@",
            ),
            me(
                "isWritableFileAtPath:",
                nsfilemanager::shim_nsfilemanager_is_writable_file as u64,
                "c@:@",
            ),
            me(
                "isDeletableFileAtPath:",
                nsfilemanager::shim_nsfilemanager_is_deletable_file as u64,
                "c@:@",
            ),
            me(
                "subpathsOfDirectoryAtPath:error:",
                nsfilemanager::shim_nsfilemanager_subpaths_of_directory as u64,
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
            me("init", nsuserdefaults::shim_nsuserdefaults_init as u64, "@@:"),
            me(
                "initWithSuiteName:",
                nsuserdefaults::shim_nsuserdefaults_init_with_suite_name as u64,
                "@@:@",
            ),
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
                "registerDefaults:",
                nsuserdefaults::shim_nsuserdefaults_register_defaults as u64,
                "v@:@",
            ),
            me(
                "addSuiteNamed:",
                nsuserdefaults::shim_nsuserdefaults_add_suite_named as u64,
                "v@:@",
            ),
            me(
                "removeSuiteNamed:",
                nsuserdefaults::shim_nsuserdefaults_remove_suite_named as u64,
                "v@:@",
            ),
            me(
                "volatileDomainForName:",
                nsuserdefaults::shim_nsuserdefaults_volatile_domain_for_name as u64,
                "@@:@",
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
            me(
                "URLForKey:",
                nsuserdefaults::shim_nsuserdefaults_url_for_key as u64,
                "@@:@",
            ),
            me(
                "dictionaryRepresentation",
                nsuserdefaults::shim_nsuserdefaults_dictionary_representation as u64,
                "@@:",
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
