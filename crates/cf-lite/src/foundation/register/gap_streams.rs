//! Gap-analysis class registrations: tasks, streams, XML, URLSession, archiving.

use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nskeyedarchiver, nsobj, nsstream, nstask, nsurlsession, nsxmlparser};

/// Register `NSCoder` — abstract keyed coding base class.
pub fn register_nscoder_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscoder_class_addr();
    let metaclass_addr = nsobj::nscoder_metaclass_addr();
    let info = ClassInfo {
        name: "NSCoder".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("allowsKeyedCoding", nskeyedarchiver::shim_nscoder_allows_keyed_coding as *const () as u64, "B@:"),
            me("containsValueForKey:", nskeyedarchiver::shim_nskeyedunarchiver_contains_value_for_key as *const () as u64, "B@:@"),
            me("description", nskeyedarchiver::shim_nscoder_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 8, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCoder");
}

/// Register `NSTask` with subprocess execution methods.
pub fn register_nstask_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstask_class_addr();
    let metaclass_addr = nsobj::nstask_metaclass_addr();

    let info = ClassInfo {
        name: "NSTask".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nstask::shim_nstask_init as *const () as u64, "@@:"),
            me("setLaunchPath:", nstask::shim_nstask_set_launch_path as *const () as u64, "v@:@"),
            me("setExecutableURL:", nstask::shim_nstask_set_executable_url as *const () as u64, "v@:@"),
            me("setArguments:", nstask::shim_nstask_set_arguments as *const () as u64, "v@:@"),
            me("setEnvironment:", nstask::shim_nstask_set_environment as *const () as u64, "v@:@"),
            me("setStandardOutput:", nstask::shim_nstask_set_stdout as *const () as u64, "v@:@"),
            me("setStandardInput:", nstask::shim_nstask_set_stdin as *const () as u64, "v@:@"),
            me("setStandardError:", nstask::shim_nstask_set_stderr as *const () as u64, "v@:@"),
            me("standardOutput", nstask::shim_nstask_get_stdout as *const () as u64, "@@:"),
            me("standardInput", nstask::shim_nstask_get_stdin as *const () as u64, "@@:"),
            me("standardError", nstask::shim_nstask_get_stderr as *const () as u64, "@@:"),
            me("setCurrentDirectoryPath:", nstask::shim_nstask_set_current_dir as *const () as u64, "v@:@"),
            me("setCurrentDirectoryURL:", nstask::shim_nstask_set_current_dir_url as *const () as u64, "v@:@"),
            me("currentDirectoryPath", nstask::shim_nstask_current_dir as *const () as u64, "@@:"),
            me("currentDirectoryURL", nstask::shim_nstask_current_directory_url as *const () as u64, "@@:"),
            me("launchPath", nstask::shim_nstask_launch_path as *const () as u64, "@@:"),
            me("executableURL", nstask::shim_nstask_executable_url as *const () as u64, "@@:"),
            me("arguments", nstask::shim_nstask_arguments as *const () as u64, "@@:"),
            me("environment", nstask::shim_nstask_environment as *const () as u64, "@@:"),
            me("terminationStatus", nstask::shim_nstask_termination_status as *const () as u64, "i@:"),
            me("terminationReason", nstask::shim_nstask_termination_reason as *const () as u64, "q@:"),
            me("processIdentifier", nstask::shim_nstask_process_identifier as *const () as u64, "i@:"),
            me("isRunning", nstask::shim_nstask_is_running as *const () as u64, "B@:"),
            me("launch", nstask::shim_nstask_launch as *const () as u64, "v@:"),
            me("launchAndReturnError:", nstask::shim_nstask_launch_and_return_error as *const () as u64, "B@:^@"),
            me("waitUntilExit", nstask::shim_nstask_wait_until_exit as *const () as u64, "v@:"),
            me("description", nstask::shim_nstask_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nstask::shim_nstask_alloc as *const () as u64, "@@:"),
            me("launchedTaskWithLaunchPath:arguments:", nstask::shim_nstask_launched_with_path as *const () as u64, "@@:@@"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 72,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTask");
}

/// Register `NSStream` — abstract stream base class.
pub fn register_nsstream_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsstream_class_addr();
    let metaclass_addr = nsobj::nsstream_metaclass_addr();
    let info = ClassInfo {
        name: "NSStream".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("open", nsstream::shim_nsstream_open as *const () as u64, "v@:"),
            me("close", nsstream::shim_nsstream_close as *const () as u64, "v@:"),
            me("streamStatus", nsstream::shim_nsstream_stream_status as *const () as u64, "Q@:"),
            me("streamError", nsstream::shim_nsstream_stream_error as *const () as u64, "@@:"),
            me("delegate", nsstream::shim_nsstream_delegate as *const () as u64, "@@:"),
            me("setDelegate:", nsstream::shim_nsstream_set_delegate as *const () as u64, "v@:@"),
            me("scheduleInRunLoop:forMode:", nsstream::shim_nsstream_schedule_in_run_loop as *const () as u64, "v@:@@"),
            me("removeFromRunLoop:forMode:", nsstream::shim_nsstream_remove_from_run_loop as *const () as u64, "v@:@@"),
            me("description", nsstream::shim_nsstream_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 24, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSStream");
}

/// Register `NSInputStream` — readable byte stream.
pub fn register_nsinputstream_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsinputstream_class_addr();
    let metaclass_addr = nsobj::nsinputstream_metaclass_addr();
    let info = ClassInfo {
        name: "NSInputStream".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("read:maxLength:", nsstream::shim_nsinputstream_read as *const () as u64, "l@:^vQ"),
            me("hasBytesAvailable", nsstream::shim_nsinputstream_has_bytes as *const () as u64, "B@:"),
            me("description", nsstream::shim_nsinputstream_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("inputStreamWithData:", nsstream::shim_nsinputstream_with_data as *const () as u64, "@@:@"),
            me("inputStreamWithFileAtPath:", nsstream::shim_nsinputstream_with_file as *const () as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 40, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsstream_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSInputStream");
}

/// Register `NSOutputStream` — writable byte stream.
pub fn register_nsoutputstream_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsoutputstream_class_addr();
    let metaclass_addr = nsobj::nsoutputstream_metaclass_addr();
    let info = ClassInfo {
        name: "NSOutputStream".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("write:maxLength:", nsstream::shim_nsoutputstream_write as *const () as u64, "l@:^vQ"),
            me("hasSpaceAvailable", nsstream::shim_nsoutputstream_has_space as *const () as u64, "B@:"),
            me("propertyForKey:", nsstream::shim_nsoutputstream_property_for_key as *const () as u64, "@@:@"),
            me("description", nsstream::shim_nsoutputstream_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("outputStreamToMemory", nsstream::shim_nsoutputstream_to_memory as *const () as u64, "@@:"),
            me("outputStreamToFileAtPath:append:", nsstream::shim_nsoutputstream_to_file as *const () as u64, "@@:@B"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsstream_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSOutputStream");
}

/// Register `NSXMLParser` — SAX-style XML parser.
pub fn register_nsxmlparser_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsxmlparser_class_addr();
    let metaclass_addr = nsobj::nsxmlparser_metaclass_addr();
    let info = ClassInfo {
        name: "NSXMLParser".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithData:", nsxmlparser::shim_nsxmlparser_init_with_data as *const () as u64, "@@:@"),
            me("initWithContentsOfURL:", nsxmlparser::shim_nsxmlparser_init_with_url as *const () as u64, "@@:@"),
            me("setDelegate:", nsxmlparser::shim_nsxmlparser_set_delegate as *const () as u64, "v@:@"),
            me("delegate", nsxmlparser::shim_nsxmlparser_delegate as *const () as u64, "@@:"),
            me("parse", nsxmlparser::shim_nsxmlparser_parse as *const () as u64, "B@:"),
            me("parserError", nsxmlparser::shim_nsxmlparser_parser_error as *const () as u64, "@@:"),
            me("abortParsing", nsxmlparser::shim_nsxmlparser_abort_parsing as *const () as u64, "v@:"),
            me("setShouldProcessNamespaces:", nsxmlparser::shim_nsxmlparser_set_should_process_ns as *const () as u64, "v@:B"),
            me("setShouldReportNamespacePrefixes:", nsxmlparser::shim_nsxmlparser_set_should_report_ns as *const () as u64, "v@:B"),
            me("shouldProcessNamespaces", nsxmlparser::shim_nsxmlparser_should_process_ns as *const () as u64, "B@:"),
            me("shouldReportNamespacePrefixes", nsxmlparser::shim_nsxmlparser_should_report_ns as *const () as u64, "B@:"),
            me("description", nsxmlparser::shim_nsxmlparser_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nsxmlparser::shim_nsxmlparser_alloc as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 40, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSXMLParser");
}

/// Register `NSURLSessionConfiguration` — session config object.
pub fn register_nsurlsessionconfiguration_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlsessionconfiguration_class_addr();
    let metaclass_addr = nsobj::nsurlsessionconfiguration_metaclass_addr();
    let info = ClassInfo {
        name: "NSURLSessionConfiguration".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("timeoutIntervalForRequest", nsurlsession::shim_nsurlsessionconfig_timeout as *const () as u64, "d@:"),
            me("setTimeoutIntervalForRequest:", nsurlsession::shim_nsurlsessionconfig_set_timeout as *const () as u64, "v@:d"),
            me("timeoutIntervalForResource", nsurlsession::shim_nsurlsessionconfig_timeout_resource as *const () as u64, "d@:"),
            me("setTimeoutIntervalForResource:", nsurlsession::shim_nsurlsessionconfig_set_timeout_resource as *const () as u64, "v@:d"),
            me("description", nsurlsession::shim_nsurlsessionconfig_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("defaultSessionConfiguration", nsurlsession::shim_nsurlsessionconfig_default as *const () as u64, "@@:"),
            me("ephemeralSessionConfiguration", nsurlsession::shim_nsurlsessionconfig_ephemeral as *const () as u64, "@@:"),
            me("backgroundSessionConfigurationWithIdentifier:", nsurlsession::shim_nsurlsessionconfig_background as *const () as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 16, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSURLSessionConfiguration");
}

/// Register `NSURLSession` — HTTP client stub.
pub fn register_nsurlsession_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlsession_class_addr();
    let metaclass_addr = nsobj::nsurlsession_metaclass_addr();
    let info = ClassInfo {
        name: "NSURLSession".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("configuration", nsurlsession::shim_nsurlsession_configuration as *const () as u64, "@@:"),
            me("dataTaskWithURL:", nsurlsession::shim_nsurlsession_data_task_with_url as *const () as u64, "@@:@"),
            me("dataTaskWithURL:completionHandler:", nsurlsession::shim_nsurlsession_data_task_with_url_completion as *const () as u64, "@@:@@"),
            me("dataTaskWithRequest:", nsurlsession::shim_nsurlsession_data_task_with_request as *const () as u64, "@@:@"),
            me("dataTaskWithRequest:completionHandler:", nsurlsession::shim_nsurlsession_data_task_with_request_completion as *const () as u64, "@@:@@"),
            me("uploadTaskWithRequest:fromData:", nsurlsession::shim_nsurlsession_upload_task_with_request as *const () as u64, "@@:@@"),
            me("downloadTaskWithRequest:", nsurlsession::shim_nsurlsession_download_task_with_request as *const () as u64, "@@:@"),
            me("downloadTaskWithURL:", nsurlsession::shim_nsurlsession_download_task_with_url as *const () as u64, "@@:@"),
            me("resetWithCompletionHandler:", nsurlsession::shim_nsurlsession_reset_with_completion_handler as *const () as u64, "v@:@?"),
            me("getAllTasksWithCompletionHandler:", nsurlsession::shim_nsurlsession_all_tasks_with_completion_handler as *const () as u64, "v@:@?"),
            me("invalidateAndCancel", nsurlsession::shim_nsurlsession_invalidate as *const () as u64, "v@:"),
            me("finishTasksAndInvalidate", nsurlsession::shim_nsurlsession_finish_tasks as *const () as u64, "v@:"),
            me("description", nsurlsession::shim_nsurlsession_description as *const () as u64, "@@:"),
            me("delegate", nsurlsession::shim_nsurlsession_delegate as *const () as u64, "@@:"),
            me("delegateQueue", nsurlsession::shim_nsurlsession_delegate_queue as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("sharedSession", nsurlsession::shim_nsurlsession_shared as *const () as u64, "@@:"),
            me("sessionWithConfiguration:", nsurlsession::shim_nsurlsession_with_config as *const () as u64, "@@:@"),
            me("sessionWithConfiguration:delegate:delegateQueue:", nsurlsession::shim_nsurlsession_with_config_delegate as *const () as u64, "@@:@@@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSURLSession");
}

/// Register `NSURLSessionDataTask` — data task stub.
pub fn register_nsurlsessiondatatask_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlsessiondatatask_class_addr();
    let metaclass_addr = nsobj::nsurlsessiondatatask_metaclass_addr();
    let info = ClassInfo {
        name: "NSURLSessionDataTask".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("resume", nsurlsession::shim_nsurlsessiondatatask_resume as *const () as u64, "v@:"),
            me("cancel", nsurlsession::shim_nsurlsessiondatatask_cancel as *const () as u64, "v@:"),
            me("suspend", nsurlsession::shim_nsurlsessiondatatask_suspend as *const () as u64, "v@:"),
            me("state", nsurlsession::shim_nsurlsessiondatatask_state as *const () as u64, "q@:"),
            me("originalRequest", nsurlsession::shim_nsurlsessiondatatask_original_request as *const () as u64, "@@:"),
            me("taskIdentifier", nsurlsession::shim_nsurlsessiondatatask_task_id as *const () as u64, "Q@:"),
            me("description", nsurlsession::shim_nsurlsessiondatatask_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 40, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSURLSessionDataTask");
}

/// Register `NSKeyedArchiver` — binary archiver (plist-backed).
pub fn register_nskeyedarchiver_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nskeyedarchiver_class_addr();
    let metaclass_addr = nsobj::nskeyedarchiver_metaclass_addr();
    let info = ClassInfo {
        name: "NSKeyedArchiver".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initRequiringSecureCoding:", nskeyedarchiver::shim_nskeyedarchiver_init_secure as *const () as u64, "@@:B"),
            me("encodedData", nskeyedarchiver::shim_nskeyedarchiver_encoded_data as *const () as u64, "@@:"),
            me("setRequiresSecureCoding:", nskeyedarchiver::shim_nskeyedarchiver_set_requires_secure as *const () as u64, "v@:B"),
            me("requiresSecureCoding", nskeyedarchiver::shim_nskeyedarchiver_requires_secure as *const () as u64, "B@:"),
            me("finishEncoding", nskeyedarchiver::shim_nskeyedarchiver_finish_encoding as *const () as u64, "v@:"),
            me("setOutputFormat:", nskeyedarchiver::shim_nskeyedarchiver_set_output_format as *const () as u64, "v@:Q"),
            me("outputFormat", nskeyedarchiver::shim_nskeyedarchiver_output_format as *const () as u64, "Q@:"),
            me("encodeObject:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_object_for_key as *const () as u64, "v@:@@"),
            me("encodeInteger:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_integer_for_key as *const () as u64, "v@:q@"),
            me("encodeBool:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_bool_for_key as *const () as u64, "v@:B@"),
            me("encodeDouble:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_double_for_key as *const () as u64, "v@:d@"),
            me("encodeFloat:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_float_for_key as *const () as u64, "v@:f@"),
            me("encodeInt:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_int_for_key as *const () as u64, "v@:i@"),
            me("encodeInt32:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_int32_for_key as *const () as u64, "v@:i@"),
            me("encodeInt64:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_int64_for_key as *const () as u64, "v@:q@"),
            me("encodeBytes:length:forKey:", nskeyedarchiver::shim_nskeyedarchiver_encode_bytes_for_key as *const () as u64, "v@:^vQ@"),
            me("description", nskeyedarchiver::shim_nskeyedarchiver_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nskeyedarchiver::shim_nskeyedarchiver_alloc as *const () as u64, "@@:"),
            me("archivedDataWithRootObject:requiringSecureCoding:error:", nskeyedarchiver::shim_nskeyedarchiver_archived_data as *const () as u64, "@@:@B^@"),
            me("archivedDataWithRootObject:", nskeyedarchiver::shim_nskeyedarchiver_archived_data_legacy as *const () as u64, "@@:@"),
            me("archiveRootObject:toFile:", nskeyedarchiver::shim_nskeyedarchiver_archive_root_to_file as *const () as u64, "B@:@@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 24, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscoder_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSKeyedArchiver");
}

/// Register `NSKeyedUnarchiver` — binary unarchiver (plist-backed).
pub fn register_nskeyedunarchiver_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nskeyedunarchiver_class_addr();
    let metaclass_addr = nsobj::nskeyedunarchiver_metaclass_addr();
    let info = ClassInfo {
        name: "NSKeyedUnarchiver".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initForReadingFromData:error:", nskeyedarchiver::shim_nskeyedunarchiver_init_from_data as *const () as u64, "@@:@^@"),
            me("setRequiresSecureCoding:", nskeyedarchiver::shim_nskeyedunarchiver_set_requires_secure as *const () as u64, "v@:B"),
            me("requiresSecureCoding", nskeyedarchiver::shim_nskeyedunarchiver_requires_secure as *const () as u64, "B@:"),
            me("finishDecoding", nskeyedarchiver::shim_nskeyedunarchiver_finish_decoding as *const () as u64, "v@:"),
            me("decodingFailurePolicy", nskeyedarchiver::shim_nskeyedunarchiver_decoding_failure_policy as *const () as u64, "q@:"),
            me("setDecodingFailurePolicy:", nskeyedarchiver::shim_nskeyedunarchiver_set_decoding_failure_policy as *const () as u64, "v@:q"),
            me("decodeObjectForKey:", nskeyedarchiver::shim_nskeyedunarchiver_decode_object_for_key as *const () as u64, "@@:@"),
            me("decodeIntegerForKey:", nskeyedarchiver::shim_nskeyedunarchiver_decode_integer_for_key as *const () as u64, "q@:@"),
            me("decodeBoolForKey:", nskeyedarchiver::shim_nskeyedunarchiver_decode_bool_for_key as *const () as u64, "B@:@"),
            me("decodeDoubleForKey:", nskeyedarchiver::shim_nskeyedunarchiver_decode_double_for_key as *const () as u64, "d@:@"),
            me("decodeFloatForKey:", nskeyedarchiver::shim_nskeyedunarchiver_decode_float_for_key as *const () as u64, "f@:@"),
            me("decodeInt32ForKey:", nskeyedarchiver::shim_nskeyedunarchiver_decode_int32_for_key as *const () as u64, "i@:@"),
            me("decodeInt64ForKey:", nskeyedarchiver::shim_nskeyedunarchiver_decode_int64_for_key as *const () as u64, "q@:@"),
            me("containsValueForKey:", nskeyedarchiver::shim_nskeyedunarchiver_contains_value_for_key as *const () as u64, "B@:@"),
            me("decodeBytesForKey:returnedLength:", nskeyedarchiver::shim_nskeyedunarchiver_decode_bytes_for_key as *const () as u64, "^v@:@^Q"),
            me("decodeObjectOfClass:forKey:", nskeyedarchiver::shim_nskeyedunarchiver_decode_object_of_class_for_key as *const () as u64, "@@:@@"),
            me("description", nskeyedarchiver::shim_nskeyedunarchiver_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nskeyedarchiver::shim_nskeyedunarchiver_alloc as *const () as u64, "@@:"),
            me("unarchivedObjectOfClass:fromData:error:", nskeyedarchiver::shim_nskeyedunarchiver_unarchived_object as *const () as u64, "@@:@@^@"),
            me("unarchiveObjectWithData:", nskeyedarchiver::shim_nskeyedunarchiver_unarchive_legacy as *const () as u64, "@@:@"),
            me("unarchiveObjectWithFile:", nskeyedarchiver::shim_nskeyedunarchiver_unarchive_from_file as *const () as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 24, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscoder_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSKeyedUnarchiver");
}
