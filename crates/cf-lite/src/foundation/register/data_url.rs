use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nsdata, nsobj, nsurl};

/// Register `NSData` with byte buffer methods and class factories.
pub fn register_nsdata_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdata_class_addr();
    let metaclass_addr = nsobj::nsdata_metaclass_addr();

    let info = ClassInfo {
        name: "NSData".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("length", nsdata::shim_nsdata_length as u64, "Q@:"),
            me("bytes", nsdata::shim_nsdata_bytes as u64, "^v@:"),
            me("description", nsdata::shim_nsdata_description as u64, "@@:"),
            me(
                "isEqualToData:",
                nsdata::shim_nsdata_is_equal_to_data as u64,
                "B@:@",
            ),
            me(
                "writeToFile:atomically:",
                nsdata::shim_nsdata_write_to_file_atomically as u64,
                "B@:@B",
            ),
            me(
                "base64EncodedStringWithOptions:",
                nsdata::shim_nsdata_base64_encoded_string_with_options as u64,
                "@@:Q",
            ),
            me(
                "initWithBase64EncodedString:options:",
                nsdata::shim_nsdata_init_with_base64_encoded_string as u64,
                "@@:@Q",
            ),
            me(
                "base64EncodedDataWithOptions:",
                nsdata::shim_nsdata_base64_encoded_data_with_options as u64,
                "@@:Q",
            ),
            me(
                "subdataWithRange:",
                nsdata::shim_nsdata_subdata_with_range as u64,
                "@@:QQ",
            ),
            me(
                "getBytes:length:",
                nsdata::shim_nsdata_get_bytes_length as u64,
                "v@:^vQ",
            ),
            me(
                "getBytes:range:",
                nsdata::shim_nsdata_get_bytes_range as u64,
                "v@:^vQQ",
            ),
            me(
                "writeToFile:options:error:",
                nsdata::shim_nsdata_write_to_file_options_error as u64,
                "c@:@QQ",
            ),
            me(
                "writeToURL:atomically:",
                nsdata::shim_nsdata_write_to_url_atomically as u64,
                "c@:@c",
            ),
        ],
        class_methods: vec![
            me(
                "dataWithBytes:length:",
                nsdata::shim_nsdata_data_with_bytes_length as u64,
                "@@:^vQ",
            ),
            me("data", nsdata::shim_nsdata_data as u64, "@@:"),
            me(
                "dataWithContentsOfFile:",
                nsdata::shim_nsdata_data_with_contents_of_file as u64,
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
    rt.register_metaclass_addr(metaclass_addr, "NSData");
}

/// Register `NSURL` with URL methods.
pub fn register_nsurl_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurl_class_addr();
    let metaclass_addr = nsobj::nsurl_metaclass_addr();

    let info = ClassInfo {
        name: "NSURL".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "absoluteString",
                nsurl::shim_nsurl_absolute_string as u64,
                "@@:",
            ),
            me("path", nsurl::shim_nsurl_path as u64, "@@:"),
            me(
                "lastPathComponent",
                nsurl::shim_nsurl_last_path_component as u64,
                "@@:",
            ),
            me(
                "pathExtension",
                nsurl::shim_nsurl_path_extension as u64,
                "@@:",
            ),
            me(
                "URLByAppendingPathComponent:",
                nsurl::shim_nsurl_url_by_appending_path_component as u64,
                "@@:@",
            ),
            me("isFileURL", nsurl::shim_nsurl_is_file_url as u64, "B@:"),
            me("scheme", nsurl::shim_nsurl_scheme as u64, "@@:"),
            me("standardizedURL", nsurl::shim_nsurl_standardized_url as u64, "@@:"),
            me("description", nsurl::shim_nsurl_description as u64, "@@:"),
            me("absoluteURL", nsurl::shim_nsurl_absolute_url as u64, "@@:"),
            me(
                "URLByDeletingLastPathComponent",
                nsurl::shim_nsurl_url_by_deleting_last_path_component as u64,
                "@@:",
            ),
            me(
                "URLByDeletingPathExtension",
                nsurl::shim_nsurl_url_by_deleting_path_extension as u64,
                "@@:",
            ),
            me(
                "URLByAppendingPathExtension:",
                nsurl::shim_nsurl_url_by_appending_path_extension as u64,
                "@@:@",
            ),
            me(
                "pathComponents",
                nsurl::shim_nsurl_path_components as u64,
                "@@:",
            ),
            me(
                "checkResourceIsReachableAndReturnError:",
                nsurl::shim_nsurl_check_resource_is_reachable as u64,
                "c@:^@",
            ),
            me(
                "initWithString:",
                nsurl::shim_nsurl_init_with_string as u64,
                "@@:@",
            ),
            me(
                "initWithString:relativeToURL:",
                nsurl::shim_nsurl_init_with_string_relative as u64,
                "@@:@@",
            ),
            me(
                "resourceSpecifier",
                nsurl::shim_nsurl_resource_specifier as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "fileURLWithPath:",
                nsurl::shim_nsurl_file_url_with_path as u64,
                "@@:@",
            ),
            me(
                "fileURLWithPath:isDirectory:",
                nsurl::shim_nsurl_file_url_with_path_is_directory as u64,
                "@@:@B",
            ),
            me(
                "URLWithString:",
                nsurl::shim_nsurl_url_with_string as u64,
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
    rt.register_metaclass_addr(metaclass_addr, "NSURL");
}
