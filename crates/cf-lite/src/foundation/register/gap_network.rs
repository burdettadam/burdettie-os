//! Gap-analysis class registrations: URL components, networking, system utilities.

use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{
    nsbytecountformatter, nshost, nsiso8601dateformatter, nspipe, nsprogress, nsurlcomponents,
    nsurlrequest, nscredential, nsobj, pkbundlecomponentversion,
};

/// Register `NSURLComponents` with URL builder/parser methods.
pub fn register_nsurlcomponents_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlcomponents_class_addr();
    let metaclass_addr = nsobj::nsurlcomponents_metaclass_addr();

    let info = ClassInfo {
        name: "NSURLComponents".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("scheme", nsurlcomponents::shim_nsurlcomponents_scheme as *const () as u64, "@@:"),
            me("host", nsurlcomponents::shim_nsurlcomponents_host as *const () as u64, "@@:"),
            me("port", nsurlcomponents::shim_nsurlcomponents_port as *const () as u64, "@@:"),
            me("path", nsurlcomponents::shim_nsurlcomponents_path as *const () as u64, "@@:"),
            me("query", nsurlcomponents::shim_nsurlcomponents_query as *const () as u64, "@@:"),
            me("fragment", nsurlcomponents::shim_nsurlcomponents_fragment as *const () as u64, "@@:"),
            me("setScheme:", nsurlcomponents::shim_nsurlcomponents_set_scheme as *const () as u64, "v@:@"),
            me("setHost:", nsurlcomponents::shim_nsurlcomponents_set_host as *const () as u64, "v@:@"),
            me("setPort:", nsurlcomponents::shim_nsurlcomponents_set_port as *const () as u64, "v@:@"),
            me("setPath:", nsurlcomponents::shim_nsurlcomponents_set_path as *const () as u64, "v@:@"),
            me("setQuery:", nsurlcomponents::shim_nsurlcomponents_set_query as *const () as u64, "v@:@"),
            me("setFragment:", nsurlcomponents::shim_nsurlcomponents_set_fragment as *const () as u64, "v@:@"),
            me("percentEncodedQuery", nsurlcomponents::shim_nsurlcomponents_percent_encoded_query as *const () as u64, "@@:"),
            me("setPercentEncodedQuery:", nsurlcomponents::shim_nsurlcomponents_set_percent_encoded_query as *const () as u64, "v@:@"),
            me("percentEncodedPath", nsurlcomponents::shim_nsurlcomponents_percent_encoded_path as *const () as u64, "@@:"),
            me("setPercentEncodedPath:", nsurlcomponents::shim_nsurlcomponents_set_percent_encoded_path as *const () as u64, "v@:@"),
            me("URL", nsurlcomponents::shim_nsurlcomponents_url as *const () as u64, "@@:"),
            me("string", nsurlcomponents::shim_nsurlcomponents_string as *const () as u64, "@@:"),
            me("queryItems", nsurlcomponents::shim_nsurlcomponents_query_items as *const () as u64, "@@:"),
            me("setQueryItems:", nsurlcomponents::shim_nsurlcomponents_set_query_items as *const () as u64, "v@:@"),
            me("description", nsurlcomponents::shim_nsurlcomponents_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("componentsWithString:", nsurlcomponents::shim_nsurlcomponents_with_string as *const () as u64, "@@:@"),
            me("new", nsurlcomponents::shim_nsurlcomponents_new as *const () as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 56,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSURLComponents");
}

/// Register `NSURLQueryItem` with name/value accessors.
pub fn register_nsurlqueryitem_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlqueryitem_class_addr();
    let metaclass_addr = nsobj::nsurlqueryitem_metaclass_addr();

    let info = ClassInfo {
        name: "NSURLQueryItem".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nsurlcomponents::shim_nsurlqueryitem_name as *const () as u64, "@@:"),
            me("value", nsurlcomponents::shim_nsurlqueryitem_value as *const () as u64, "@@:"),
            me("description", nsurlcomponents::shim_nsurlqueryitem_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("queryItemWithName:value:", nsurlcomponents::shim_nsurlqueryitem_with_name_value as *const () as u64, "@@:@@"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSURLQueryItem");
}

/// Register `NSByteCountFormatter` with byte formatting methods.
pub fn register_nsbytecountformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbytecountformatter_class_addr();
    let metaclass_addr = nsobj::nsbytecountformatter_metaclass_addr();

    let info = ClassInfo {
        name: "NSByteCountFormatter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsbytecountformatter::shim_bytecountformatter_init as *const () as u64, "@@:"),
            me("stringFromByteCount:", nsbytecountformatter::shim_bytecountformatter_string_from_instance as *const () as u64, "@@:q"),
            me("setCountStyle:", nsbytecountformatter::shim_bytecountformatter_set_count_style as *const () as u64, "v@:Q"),
            me("countStyle", nsbytecountformatter::shim_bytecountformatter_count_style as *const () as u64, "Q@:"),
            me("setAllowedUnits:", nsbytecountformatter::shim_bytecountformatter_set_allowed_units as *const () as u64, "v@:Q"),
            me("allowedUnits", nsbytecountformatter::shim_bytecountformatter_allowed_units as *const () as u64, "Q@:"),
            me("description", nsbytecountformatter::shim_bytecountformatter_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nsbytecountformatter::shim_bytecountformatter_alloc as *const () as u64, "@@:"),
            me("stringFromByteCount:countStyle:", nsbytecountformatter::shim_bytecountformatter_string_from as *const () as u64, "@@:qQ"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSByteCountFormatter");
}

/// Register `NSISO8601DateFormatter` with ISO 8601 formatting methods.
pub fn register_nsiso8601dateformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsiso8601dateformatter_class_addr();
    let metaclass_addr = nsobj::nsiso8601dateformatter_metaclass_addr();

    let info = ClassInfo {
        name: "NSISO8601DateFormatter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsiso8601dateformatter::shim_iso8601_init as *const () as u64, "@@:"),
            me("stringFromDate:", nsiso8601dateformatter::shim_iso8601_string_from_date as *const () as u64, "@@:@"),
            me("dateFromString:", nsiso8601dateformatter::shim_iso8601_date_from_string as *const () as u64, "@@:@"),
            me("setFormatOptions:", nsiso8601dateformatter::shim_iso8601_set_format_options as *const () as u64, "v@:Q"),
            me("formatOptions", nsiso8601dateformatter::shim_iso8601_format_options as *const () as u64, "Q@:"),
            me("setTimeZone:", nsiso8601dateformatter::shim_iso8601_set_timezone as *const () as u64, "v@:@"),
            me("timeZone", nsiso8601dateformatter::shim_iso8601_timezone as *const () as u64, "@@:"),
            me("description", nsiso8601dateformatter::shim_iso8601_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", nsiso8601dateformatter::shim_iso8601_alloc as *const () as u64, "@@:"),
            me("stringFromDate:timeZone:formatOptions:", nsiso8601dateformatter::shim_iso8601_class_string_from_date as *const () as u64, "@@:@@Q"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSISO8601DateFormatter");
}

/// Register `NSProgress` with progress tracking methods.
pub fn register_nsprogress_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsprogress_class_addr();
    let metaclass_addr = nsobj::nsprogress_metaclass_addr();

    let info = ClassInfo {
        name: "NSProgress".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("totalUnitCount", nsprogress::shim_nsprogress_total_unit_count as *const () as u64, "q@:"),
            me("setTotalUnitCount:", nsprogress::shim_nsprogress_set_total_unit_count as *const () as u64, "v@:q"),
            me("completedUnitCount", nsprogress::shim_nsprogress_completed_unit_count as *const () as u64, "q@:"),
            me("setCompletedUnitCount:", nsprogress::shim_nsprogress_set_completed_unit_count as *const () as u64, "v@:q"),
            me("fractionCompleted", nsprogress::shim_nsprogress_fraction_completed as *const () as u64, "d@:"),
            me("isCancellable", nsprogress::shim_nsprogress_is_cancellable as *const () as u64, "B@:"),
            me("setCancellable:", nsprogress::shim_nsprogress_set_cancellable as *const () as u64, "v@:B"),
            me("isCancelled", nsprogress::shim_nsprogress_is_cancelled as *const () as u64, "B@:"),
            me("cancel", nsprogress::shim_nsprogress_cancel as *const () as u64, "v@:"),
            me("localizedDescription", nsprogress::shim_nsprogress_localized_description as *const () as u64, "@@:"),
            me("setLocalizedDescription:", nsprogress::shim_nsprogress_set_localized_description as *const () as u64, "v@:@"),
            me("description", nsprogress::shim_nsprogress_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("progressWithTotalUnitCount:", nsprogress::shim_nsprogress_with_total as *const () as u64, "@@:q"),
            me("discreteProgressWithTotalUnitCount:", nsprogress::shim_nsprogress_discrete as *const () as u64, "@@:q"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 48,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSProgress");
}

/// Register `NSHost` with hostname resolution methods.
pub fn register_nshost_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nshost_class_addr();
    let metaclass_addr = nsobj::nshost_metaclass_addr();

    let info = ClassInfo {
        name: "NSHost".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nshost::shim_nshost_name as *const () as u64, "@@:"),
            me("address", nshost::shim_nshost_address as *const () as u64, "@@:"),
            me("addresses", nshost::shim_nshost_addresses as *const () as u64, "@@:"),
            me("names", nshost::shim_nshost_names as *const () as u64, "@@:"),
            me("localizedName", nshost::shim_nshost_localized_name as *const () as u64, "@@:"),
            me("isEqualToHost:", nshost::shim_nshost_is_equal as *const () as u64, "B@:@"),
            me("description", nshost::shim_nshost_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("currentHost", nshost::shim_nshost_current_host as *const () as u64, "@@:"),
            me("hostWithName:", nshost::shim_nshost_with_name as *const () as u64, "@@:@"),
            me("hostWithAddress:", nshost::shim_nshost_with_address as *const () as u64, "@@:@"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSHost");
}

/// Register `NSPipe` with pipe creation methods.
pub fn register_nspipe_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nspipe_class_addr();
    let metaclass_addr = nsobj::nspipe_metaclass_addr();

    let info = ClassInfo {
        name: "NSPipe".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("fileHandleForReading", nspipe::shim_nspipe_file_handle_for_reading as *const () as u64, "@@:"),
            me("fileHandleForWriting", nspipe::shim_nspipe_file_handle_for_writing as *const () as u64, "@@:"),
            me("description", nspipe::shim_nspipe_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("pipe", nspipe::shim_nspipe_pipe as *const () as u64, "@@:"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSPipe");
}

/// Register `PKBundleComponentVersion` with minimal Xcode version accessors.
pub fn register_pkbundlecomponentversion_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::pkbundlecomponentversion_class_addr();
    let metaclass_addr = nsobj::pkbundlecomponentversion_metaclass_addr();

    let info = ClassInfo {
        name: "PKBundleComponentVersion".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "currentMarketingVersion",
                pkbundlecomponentversion::shim_pkbundlecomponentversion_current_marketing_version as *const () as u64,
                "@@:",
            ),
            me(
                "bundleVersion",
                pkbundlecomponentversion::shim_pkbundlecomponentversion_bundle_version as *const () as u64,
                "@@:",
            ),
            me(
                "description",
                pkbundlecomponentversion::shim_pkbundlecomponentversion_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "bundleComponentVersionWithOnDiskCFBundle:",
            pkbundlecomponentversion::shim_pkbundlecomponentversion_with_on_disk_cfbundle as *const () as u64,
            "@@:@",
        )],
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
    rt.register_metaclass_addr(metaclass_addr, "PKBundleComponentVersion");
}

/// Register `NSURLRequest` with URL request accessors.
pub fn register_nsurlrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlrequest_class_addr();
    let metaclass_addr = nsobj::nsurlrequest_metaclass_addr();

    let info = ClassInfo {
        name: "NSURLRequest".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("URL", nsurlrequest::shim_nsurlrequest_url as *const () as u64, "@@:"),
            me("HTTPMethod", nsurlrequest::shim_nsurlrequest_http_method as *const () as u64, "@@:"),
            me("allHTTPHeaderFields", nsurlrequest::shim_nsurlrequest_all_http_header_fields as *const () as u64, "@@:"),
            me("HTTPBody", nsurlrequest::shim_nsurlrequest_http_body as *const () as u64, "@@:"),
            me("description", nsurlrequest::shim_nsurlrequest_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("requestWithURL:", nsurlrequest::shim_nsurlrequest_with_url as *const () as u64, "@@:@"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSURLRequest");
}

/// Register `NSMutableURLRequest` with mutable URL request methods.
pub fn register_nsmutableurlrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmutableurlrequest_class_addr();
    let metaclass_addr = nsobj::nsmutableurlrequest_metaclass_addr();

    let info = ClassInfo {
        name: "NSMutableURLRequest".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("setHTTPMethod:", nsurlrequest::shim_nsmutableurlrequest_set_http_method as *const () as u64, "v@:@"),
            me("setURL:", nsurlrequest::shim_nsmutableurlrequest_set_url as *const () as u64, "v@:@"),
            me("setValue:forHTTPHeaderField:", nsurlrequest::shim_nsmutableurlrequest_set_value_for_header as *const () as u64, "v@:@@"),
            me("setHTTPBody:", nsurlrequest::shim_nsmutableurlrequest_set_http_body as *const () as u64, "v@:@"),
            me("HTTPBody", nsurlrequest::shim_nsurlrequest_http_body as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("requestWithURL:", nsurlrequest::shim_nsmutableurlrequest_with_url as *const () as u64, "@@:@"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsurlrequest_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMutableURLRequest");
}

/// Register `NSURLResponse` with URL response accessors.
pub fn register_nsurlresponse_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlresponse_class_addr();
    let metaclass_addr = nsobj::nsurlresponse_metaclass_addr();

    let info = ClassInfo {
        name: "NSURLResponse".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("URL", nsurlrequest::shim_nsurlresponse_url as *const () as u64, "@@:"),
            me("MIMEType", nsurlrequest::shim_nsurlresponse_mime_type as *const () as u64, "@@:"),
            me("expectedContentLength", nsurlrequest::shim_nsurlresponse_expected_content_length as *const () as u64, "q@:"),
            me("description", nsurlrequest::shim_nsurlresponse_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
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
    rt.register_metaclass_addr(metaclass_addr, "NSURLResponse");
}

/// Register `NSHTTPURLResponse` with HTTP-specific response methods.
pub fn register_nshttpurlresponse_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nshttpurlresponse_class_addr();
    let metaclass_addr = nsobj::nshttpurlresponse_metaclass_addr();

    let info = ClassInfo {
        name: "NSHTTPURLResponse".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("statusCode", nsurlrequest::shim_nshttpurlresponse_status_code as *const () as u64, "q@:"),
            me("description", nsurlrequest::shim_nshttpurlresponse_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("localizedStringForStatusCode:", nsurlrequest::shim_nshttpurlresponse_localized_string as *const () as u64, "@@:q"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsurlresponse_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSHTTPURLResponse");
}

/// Register `NSURLCredential` with credential accessors.
pub fn register_nsurlcredential_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlcredential_class_addr();
    let metaclass_addr = nsobj::nsurlcredential_metaclass_addr();
    let info = ClassInfo {
        name: "NSURLCredential".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("user", nscredential::shim_nsurlcredential_user as *const () as u64, "@@:"),
            me("password", nscredential::shim_nsurlcredential_password as *const () as u64, "@@:"),
            me("persistence", nscredential::shim_nsurlcredential_persistence as *const () as u64, "Q@:"),
            me("hasPassword", nscredential::shim_nsurlcredential_has_password as *const () as u64, "B@:"),
            me("description", nscredential::shim_nsurlcredential_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("credentialWithUser:password:persistence:", nscredential::shim_nsurlcredential_with_user as *const () as u64, "@@:@@Q"),
        ],
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
    rt.register_metaclass_addr(metaclass_addr, "NSURLCredential");
}

/// Register `NSURLCredentialStorage` singleton stub.
pub fn register_nsurlcredentialstorage_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsurlcredentialstorage_class_addr();
    let metaclass_addr = nsobj::nsurlcredentialstorage_metaclass_addr();
    let info = ClassInfo {
        name: "NSURLCredentialStorage".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("allCredentials", nscredential::shim_nsurlcredentialstorage_all_credentials as *const () as u64, "@@:"),
            me("defaultCredentialForProtectionSpace:", nscredential::shim_nsurlcredentialstorage_default_credential as *const () as u64, "@@:@"),
            me("setDefaultCredential:forProtectionSpace:", nscredential::shim_nsurlcredentialstorage_set_default_credential as *const () as u64, "v@:@@"),
            me("removeCredential:forProtectionSpace:", nscredential::shim_nsurlcredentialstorage_remove_credential as *const () as u64, "v@:@@"),
            me("description", nscredential::shim_nsurlcredentialstorage_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("sharedCredentialStorage", nscredential::shim_nsurlcredentialstorage_shared as *const () as u64, "@@:"),
        ],
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
    rt.register_metaclass_addr(metaclass_addr, "NSURLCredentialStorage");
}

/// Register `NSHTTPCookie` with cookie property accessors.
pub fn register_nshttpcookie_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nshttpcookie_class_addr();
    let metaclass_addr = nsobj::nshttpcookie_metaclass_addr();
    let info = ClassInfo {
        name: "NSHTTPCookie".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("properties", nscredential::shim_nshttpcookie_properties as *const () as u64, "@@:"),
            me("name", nscredential::shim_nshttpcookie_name as *const () as u64, "@@:"),
            me("value", nscredential::shim_nshttpcookie_value as *const () as u64, "@@:"),
            me("domain", nscredential::shim_nshttpcookie_domain as *const () as u64, "@@:"),
            me("path", nscredential::shim_nshttpcookie_path as *const () as u64, "@@:"),
            me("isSecure", nscredential::shim_nshttpcookie_is_secure as *const () as u64, "B@:"),
            me("isHTTPOnly", nscredential::shim_nshttpcookie_is_http_only as *const () as u64, "B@:"),
            me("expiresDate", nscredential::shim_nshttpcookie_expires_date as *const () as u64, "@@:"),
            me("description", nscredential::shim_nshttpcookie_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("cookieWithProperties:", nscredential::shim_nshttpcookie_with_properties as *const () as u64, "@@:@"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSHTTPCookie");
}

/// Register `NSHTTPCookieStorage` singleton stub.
pub fn register_nshttpcookiestorage_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nshttpcookiestorage_class_addr();
    let metaclass_addr = nsobj::nshttpcookiestorage_metaclass_addr();
    let info = ClassInfo {
        name: "NSHTTPCookieStorage".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("cookies", nscredential::shim_nshttpcookiestorage_cookies as *const () as u64, "@@:"),
            me("cookiesForURL:", nscredential::shim_nshttpcookiestorage_cookies_for_url as *const () as u64, "@@:@"),
            me("setCookie:", nscredential::shim_nshttpcookiestorage_set_cookie as *const () as u64, "v@:@"),
            me("deleteCookie:", nscredential::shim_nshttpcookiestorage_delete_cookie as *const () as u64, "v@:@"),
            me("cookieAcceptPolicy", nscredential::shim_nshttpcookiestorage_accept_policy as *const () as u64, "Q@:"),
            me("setCookieAcceptPolicy:", nscredential::shim_nshttpcookiestorage_set_accept_policy as *const () as u64, "v@:Q"),
            me("description", nscredential::shim_nshttpcookiestorage_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("sharedHTTPCookieStorage", nscredential::shim_nshttpcookiestorage_shared as *const () as u64, "@@:"),
        ],
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
    rt.register_metaclass_addr(metaclass_addr, "NSHTTPCookieStorage");
}

/// Register `NSAutoreleasePool` — explicit autorelease pool class.
pub fn register_nsautoreleasepool_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsautoreleasepool_class_addr();
    let metaclass_addr = nsobj::nsautoreleasepool_metaclass_addr();
    let info = ClassInfo {
        name: "NSAutoreleasePool".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", crate::nsautoreleasepool::shim_nsautoreleasepool_init as *const () as u64, "@@:"),
            me("drain", crate::nsautoreleasepool::shim_nsautoreleasepool_drain as *const () as u64, "v@:"),
            me("release", crate::nsautoreleasepool::shim_nsautoreleasepool_release as *const () as u64, "v@:"),
            me("dealloc", crate::nsautoreleasepool::shim_nsautoreleasepool_dealloc as *const () as u64, "v@:"),
        ],
        class_methods: vec![
            me("alloc", crate::nsautoreleasepool::shim_nsautoreleasepool_alloc as *const () as u64, "@@:"),
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
    rt.register_metaclass_addr(metaclass_addr, "NSAutoreleasePool");
}

// ── XCStringsParser.framework classes ───────────────────────────

/// Register `XCStringsdataEntry` — a single localization entry with key/value/comment.
pub fn register_xcstringsdataentry_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::xcstringsdataentry_class_addr();
    let metaclass_addr = nsobj::xcstringsdataentry_metaclass_addr();
    let info = ClassInfo {
        name: "XCStringsdataEntry".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithKey:value:comment:", crate::xcstringsdata_entry::shim_xcstringsdataentry_init_with_key_value_comment as *const () as u64, "@@:@@@"),
            me("key", crate::xcstringsdata_entry::shim_xcstringsdataentry_key as *const () as u64, "@@:"),
            me("value", crate::xcstringsdata_entry::shim_xcstringsdataentry_value as *const () as u64, "@@:"),
            me("comment", crate::xcstringsdata_entry::shim_xcstringsdataentry_comment as *const () as u64, "@@:"),
            me("attributed", crate::xcstringsdata_entry::shim_xcstringsdataentry_attributed as *const () as u64, "B@:"),
            me("setAttributed:", crate::xcstringsdata_entry::shim_xcstringsdataentry_set_attributed as *const () as u64, "v@:B"),
            me("setKey:", crate::xcstringsdata_entry::shim_xcstringsdataentry_set_key as *const () as u64, "v@:@"),
            me("setValue:", crate::xcstringsdata_entry::shim_xcstringsdataentry_set_value as *const () as u64, "v@:@"),
            me("setCommentary:", crate::xcstringsdata_entry::shim_xcstringsdataentry_set_commentary as *const () as u64, "v@:@"),
            me("commentary", crate::xcstringsdata_entry::shim_xcstringsdataentry_commentary as *const () as u64, "@@:"),
            me("commentaryDescription", crate::xcstringsdata_entry::shim_xcstringsdataentry_commentary_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 56,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "XCStringsdataEntry");
}

/// Register `XCStringsdataTable` — a named table containing an array of entries.
pub fn register_xcstringsdatatable_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::xcstringsdatatable_class_addr();
    let metaclass_addr = nsobj::xcstringsdatatable_metaclass_addr();
    let info = ClassInfo {
        name: "XCStringsdataTable".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithName:entries:", crate::xcstringsdata_table::shim_xcstringsdatatable_init_with_name_entries as *const () as u64, "@@:@@"),
            me("name", crate::xcstringsdata_table::shim_xcstringsdatatable_name as *const () as u64, "@@:"),
            me("entries", crate::xcstringsdata_table::shim_xcstringsdatatable_entries as *const () as u64, "@@:"),
            me("findEntryInTable:key:value:comments:unicode:attributed:", crate::xcstringsdata_table::shim_xcstringsdatatable_find_entry as *const () as u64, "@@:@@@@BB"),
        ],
        class_methods: vec![],
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
    rt.register_metaclass_addr(metaclass_addr, "XCStringsdataTable");
}

/// Register `XCStringsdataFile` — a localization file with tables and serialization.
pub fn register_xcstringsdatafile_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::xcstringsdatafile_class_addr();
    let metaclass_addr = nsobj::xcstringsdatafile_metaclass_addr();
    let info = ClassInfo {
        name: "XCStringsdataFile".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithSource:version:tables:", crate::xcstringsdata_file::shim_xcstringsdatafile_init_with_source_version_tables as *const () as u64, "@@:@@@"),
            me("initWithPath:options:", crate::xcstringsdata_file::shim_xcstringsdatafile_init_with_path_options as *const () as u64, "@@:@Q"),
            me("source", crate::xcstringsdata_file::shim_xcstringsdatafile_source as *const () as u64, "@@:"),
            me("tables", crate::xcstringsdata_file::shim_xcstringsdatafile_tables as *const () as u64, "@@:"),
            me("writeTo:error:", crate::xcstringsdata_file::shim_xcstringsdatafile_write_to as *const () as u64, "B@:@^@"),
            me("encoding", crate::xcstringsdata_file::shim_xcstringsdatafile_encoding as *const () as u64, "Q@:"),
            me("options", crate::xcstringsdata_file::shim_xcstringsdatafile_options as *const () as u64, "Q@:"),
            me("path", crate::xcstringsdata_file::shim_xcstringsdatafile_path as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 56,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "XCStringsdataFile");
}
