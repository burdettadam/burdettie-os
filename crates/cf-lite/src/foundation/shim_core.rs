//! Core Foundation/CF-lite shim registrations (NSLog, CF functions, base classes).

use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

use crate::{cf_core, cfstring, nslog, nsobj, nsvalue};
use crate::{nsapplication, nsrunloop};
use super::cfconstant_string::cfconstant_string_class_addr;

/// Register core CF/Foundation shims and the first group of class data symbols.
pub(super) fn install_core_shims(table: &mut ShimTable) {
    // NSLog + CFConstantString
    register_shims!(table,
        ("_NSLog", nslog::nslog_trampoline_addr(), "NSLog → fprintf(stderr, ...)"),
        ("___CFConstantStringClassReference", cfconstant_string_class_addr(), "CFConstantString class reference (data symbol)"),
    );

    // NSObject class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSObject", nsobj::nsobject_class_addr(), "NSObject class data symbol"),
        ("_OBJC_METACLASS_$_NSObject", nsobj::nsobject_metaclass_addr(), "NSObject metaclass data symbol"),
    );

    // CFString functions
    register_shims!(table,
        ("_CFStringCreateWithCString", cfstring::shim_cf_string_create_with_cstring as u64, "CFStringCreateWithCString shim"),
        ("_CFStringGetLength", cfstring::shim_cf_string_get_length as u64, "CFStringGetLength shim"),
        ("_CFStringGetCStringPtr", cfstring::shim_cf_string_get_cstring_ptr as u64, "CFStringGetCStringPtr shim"),
        ("_kCFStringEncodingUTF8", cfstring::kcf_string_encoding_utf8_addr(), "kCFStringEncodingUTF8 data symbol"),
        ("_CFRelease", cfstring::shim_cf_release as u64, "CFRelease shim"),
        ("_kCFAllocatorDefault", cfstring::kcf_allocator_default_addr(), "kCFAllocatorDefault data symbol"),
    );

    // Foundation class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSString", nsobj::nsstring_class_addr(), "NSString class data symbol"),
        ("_OBJC_METACLASS_$_NSString", nsobj::nsstring_metaclass_addr(), "NSString metaclass data symbol"),
        ("_OBJC_CLASS_$_NSArray", nsobj::nsarray_class_addr(), "NSArray class data symbol"),
        ("_OBJC_METACLASS_$_NSArray", nsobj::nsarray_metaclass_addr(), "NSArray metaclass data symbol"),
        ("_OBJC_CLASS_$_NSNumber", nsobj::nsnumber_class_addr(), "NSNumber class data symbol"),
        ("_OBJC_METACLASS_$_NSNumber", nsobj::nsnumber_metaclass_addr(), "NSNumber metaclass data symbol"),
        ("_OBJC_CLASS_$_NSDictionary", nsobj::nsdictionary_class_addr(), "NSDictionary class data symbol"),
        ("_OBJC_METACLASS_$_NSDictionary", nsobj::nsdictionary_metaclass_addr(), "NSDictionary metaclass data symbol"),
        ("_OBJC_CLASS_$_NSMutableArray", nsobj::nsmutablearray_class_addr(), "NSMutableArray class data symbol"),
        ("_OBJC_METACLASS_$_NSMutableArray", nsobj::nsmutablearray_metaclass_addr(), "NSMutableArray metaclass data symbol"),
        ("_OBJC_CLASS_$_NSMutableDictionary", nsobj::nsmutabledictionary_class_addr(), "NSMutableDictionary class data symbol"),
        ("_OBJC_METACLASS_$_NSMutableDictionary", nsobj::nsmutabledictionary_metaclass_addr(), "NSMutableDictionary metaclass data symbol"),
        ("_OBJC_CLASS_$_NSMutableString", nsobj::nsmutablestring_class_addr(), "NSMutableString class data symbol"),
        ("_OBJC_METACLASS_$_NSMutableString", nsobj::nsmutablestring_metaclass_addr(), "NSMutableString metaclass data symbol"),
    );

    // NSData class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSData", nsobj::nsdata_class_addr(), "NSData class data symbol"),
        ("_OBJC_METACLASS_$_NSData", nsobj::nsdata_metaclass_addr(), "NSData metaclass data symbol"),
    );

    // NSProcessInfo class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSProcessInfo", nsobj::nsprocessinfo_class_addr(), "NSProcessInfo class data symbol"),
        ("_OBJC_METACLASS_$_NSProcessInfo", nsobj::nsprocessinfo_metaclass_addr(), "NSProcessInfo metaclass data symbol"),
    );

    // NSError class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSError", nsobj::nserror_class_addr(), "NSError class data symbol"),
        ("_OBJC_METACLASS_$_NSError", nsobj::nserror_metaclass_addr(), "NSError metaclass data symbol"),
    );

    // NSBundle class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSBundle", nsobj::nsbundle_class_addr(), "NSBundle class data symbol"),
        ("_OBJC_METACLASS_$_NSBundle", nsobj::nsbundle_metaclass_addr(), "NSBundle metaclass data symbol"),
    );

    // NSFileManager class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSFileManager", nsobj::nsfilemanager_class_addr(), "NSFileManager class data symbol"),
        ("_OBJC_METACLASS_$_NSFileManager", nsobj::nsfilemanager_metaclass_addr(), "NSFileManager metaclass data symbol"),
    );

    // NSException class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSException", nsobj::nsexception_class_addr(), "NSException class data symbol"),
        ("_OBJC_METACLASS_$_NSException", nsobj::nsexception_metaclass_addr(), "NSException metaclass data symbol"),
    );

    // NSEnumerator class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSEnumerator", nsobj::nsenumerator_class_addr(), "NSEnumerator class data symbol"),
        ("_OBJC_METACLASS_$_NSEnumerator", nsobj::nsenumerator_metaclass_addr(), "NSEnumerator metaclass data symbol"),
    );

    // NSDate class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSDate", nsobj::nsdate_class_addr(), "NSDate class data symbol"),
        ("_OBJC_METACLASS_$_NSDate", nsobj::nsdate_metaclass_addr(), "NSDate metaclass data symbol"),
    );

    // NSUserDefaults class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSUserDefaults", nsobj::nsuserdefaults_class_addr(), "NSUserDefaults class data symbol"),
        ("_OBJC_METACLASS_$_NSUserDefaults", nsobj::nsuserdefaults_metaclass_addr(), "NSUserDefaults metaclass data symbol"),
    );

    // NSSet class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSSet", nsobj::nsset_class_addr(), "NSSet class data symbol"),
        ("_OBJC_METACLASS_$_NSSet", nsobj::nsset_metaclass_addr(), "NSSet metaclass data symbol"),
    );

    // NSMutableSet class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSMutableSet", nsobj::nsmutableset_class_addr(), "NSMutableSet class data symbol"),
        ("_OBJC_METACLASS_$_NSMutableSet", nsobj::nsmutableset_metaclass_addr(), "NSMutableSet metaclass data symbol"),
    );

    // NSURL class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSURL", nsobj::nsurl_class_addr(), "NSURL class data symbol"),
        ("_OBJC_METACLASS_$_NSURL", nsobj::nsurl_metaclass_addr(), "NSURL metaclass data symbol"),
    );

    // NSNull class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSNull", nsobj::nsnull_class_addr(), "NSNull class data symbol"),
        ("_OBJC_METACLASS_$_NSNull", nsobj::nsnull_metaclass_addr(), "NSNull metaclass data symbol"),
    );

    // NSStringFrom* geometry formatting functions
    register_shims!(table,
        ("_NSStringFromPoint", nsvalue::shim_nsstringfrompoint as u64, "NSStringFromPoint → format CGPoint as NSString"),
        ("_NSStringFromSize", nsvalue::shim_nsstringfromsize as u64, "NSStringFromSize → format CGSize as NSString"),
        ("_NSStringFromRect", nsvalue::shim_nsstringfromrect as u64, "NSStringFromRect → format CGRect as NSString"),
        ("_NSStringFromRange", nsvalue::shim_nsstringfromrange as u64, "NSStringFromRange → format NSRange as NSString"),
    );

    // CoreFoundation core functions
    register_shims!(table,
        ("_CFRetain", cf_core::shim_cf_retain as u64, "CFRetain shim"),
        ("_CFGetTypeID", cf_core::shim_cf_get_type_id as u64, "CFGetTypeID shim"),
        ("_CFEqual", cf_core::shim_cf_equal as u64, "CFEqual shim"),
        ("_CFHash", cf_core::shim_cf_hash as u64, "CFHash shim"),
        ("_CFCopyDescription", cf_core::shim_cf_copy_description as u64, "CFCopyDescription shim"),
    );

    // CF type ID getters
    register_shims!(table,
        ("_CFStringGetTypeID", cf_core::shim_cf_string_get_type_id as u64, "CFStringGetTypeID shim"),
        ("_CFArrayGetTypeID", cf_core::shim_cf_array_get_type_id as u64, "CFArrayGetTypeID shim"),
        ("_CFDictionaryGetTypeID", cf_core::shim_cf_dictionary_get_type_id as u64, "CFDictionaryGetTypeID shim"),
        ("_CFDataGetTypeID", cf_core::shim_cf_data_get_type_id as u64, "CFDataGetTypeID shim"),
        ("_CFURLGetTypeID", cf_core::shim_cf_url_get_type_id as u64, "CFURLGetTypeID shim"),
        ("_CFBooleanGetTypeID", cf_core::shim_cf_boolean_get_type_id as u64, "CFBooleanGetTypeID shim"),
        ("_CFNumberGetTypeID", cf_core::shim_cf_number_get_type_id as u64, "CFNumberGetTypeID shim"),
        ("_CFNullGetTypeID", cf_core::shim_cf_null_get_type_id as u64, "CFNullGetTypeID shim"),
    );

    // CFArray C functions
    register_shims!(table,
        ("_CFArrayCreate", cf_core::shim_cf_array_create as u64, "CFArrayCreate shim"),
        ("_CFArrayGetCount", cf_core::shim_cf_array_get_count as u64, "CFArrayGetCount shim"),
        ("_CFArrayGetValueAtIndex", cf_core::shim_cf_array_get_value_at_index as u64, "CFArrayGetValueAtIndex shim"),
    );

    // CFDictionary C functions
    register_shims!(table,
        ("_CFDictionaryCreate", cf_core::shim_cf_dictionary_create as u64, "CFDictionaryCreate shim"),
        ("_CFDictionaryGetCount", cf_core::shim_cf_dictionary_get_count as u64, "CFDictionaryGetCount shim"),
        ("_CFDictionaryGetValue", cf_core::shim_cf_dictionary_get_value as u64, "CFDictionaryGetValue shim"),
    );

    // CFData C functions
    register_shims!(table,
        ("_CFDataCreate", cf_core::shim_cf_data_create as u64, "CFDataCreate shim"),
        ("_CFDataGetBytePtr", cf_core::shim_cf_data_get_byte_ptr as u64, "CFDataGetBytePtr shim"),
        ("_CFDataGetLength", cf_core::shim_cf_data_get_length as u64, "CFDataGetLength shim"),
    );

    // CFURL C functions
    register_shims!(table,
        ("_CFURLCreateWithString", cf_core::shim_cf_url_create_with_string as u64, "CFURLCreateWithString shim"),
        ("_CFURLGetString", cf_core::shim_cf_url_get_string as u64, "CFURLGetString shim"),
        ("_CFURLCreateWithFileSystemPath", cf_core::shim_cf_url_create_with_file_system_path as u64, "CFURLCreateWithFileSystemPath shim"),
    );

    // CFBundle C functions
    register_shims!(table,
        ("_CFBundleGetMainBundle", cf_core::shim_cf_bundle_get_main_bundle as u64, "CFBundleGetMainBundle shim"),
        ("_CFBundleCopyBundleURL", cf_core::shim_cf_bundle_copy_bundle_url as u64, "CFBundleCopyBundleURL shim"),
    );

    // CF boolean/null data symbols
    register_shims!(table,
        ("_kCFBooleanTrue", cf_core::kcf_boolean_true_addr(), "kCFBooleanTrue data symbol"),
        ("_kCFBooleanFalse", cf_core::kcf_boolean_false_addr(), "kCFBooleanFalse data symbol"),
        ("___kCFBooleanTrue", cf_core::kcf_boolean_true_addr(), "__kCFBooleanTrue for @YES literals"),
        ("___kCFBooleanFalse", cf_core::kcf_boolean_false_addr(), "__kCFBooleanFalse for @NO literals"),
    );

    // NSApp global variable + NSRunLoop mode constants (Phase C5)
    register_shims!(table,
        ("_NSApp", nsapplication::nsapp_global_addr(), "NSApp global variable (data symbol)"),
        ("_NSDefaultRunLoopMode", nsrunloop::nsdefault_run_loop_mode_addr(), "NSDefaultRunLoopMode constant string"),
        ("_NSRunLoopCommonModes", nsrunloop::nsrun_loop_common_modes_addr(), "NSRunLoopCommonModes constant string"),
    );
}
