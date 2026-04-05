//! Core Foundation/CF-lite shim registrations (NSLog, CF functions, base classes).

use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

use crate::{cf_core, cfstring, nslog, nsobj, nsvalue};
use crate::{foundation_constants, nsapplication, nsrunloop, nspasteboard};
use super::cfconstant_string::cfconstant_string_class_addr;

/// Register core CF/Foundation shims and the first group of class data symbols.
pub(super) fn install_core_shims(table: &mut ShimTable) {
    // NSLog + CFConstantString
    register_shims!(table,
        ("_NSLog", nslog::nslog_trampoline_addr(), "NSLog → fprintf(stderr, ...)"),
        ("__NSIsNSString", cf_core::shim_ns_is_nsstring as *const () as u64, "__NSIsNSString → string/CFString predicate"),
        ("___CFConstantStringClassReference", cfconstant_string_class_addr(), "CFConstantString class reference (data symbol)"),
    );

    // NSObject class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSObject", nsobj::nsobject_class_addr(), "NSObject class data symbol"),
        ("_OBJC_METACLASS_$_NSObject", nsobj::nsobject_metaclass_addr(), "NSObject metaclass data symbol"),
    );

    // CFString functions
    register_shims!(table,
        ("_CFStringCreateWithCString", cfstring::shim_cf_string_create_with_cstring as *const () as u64, "CFStringCreateWithCString shim"),
        ("_CFStringGetLength", cfstring::shim_cf_string_get_length as *const () as u64, "CFStringGetLength shim"),
        ("_CFStringGetCStringPtr", cfstring::shim_cf_string_get_cstring_ptr as *const () as u64, "CFStringGetCStringPtr shim"),
        ("_kCFStringEncodingUTF8", cfstring::kcf_string_encoding_utf8_addr(), "kCFStringEncodingUTF8 data symbol"),
        ("_CFRelease", cfstring::shim_cf_release as *const () as u64, "CFRelease shim"),
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

    // Gap-analysis class/metaclass data symbols (Phase C8)
    register_shims!(table,
        ("_OBJC_CLASS_$_NSMutableData", nsobj::nsmutabledata_class_addr(), "NSMutableData class data symbol"),
        ("_OBJC_METACLASS_$_NSMutableData", nsobj::nsmutabledata_metaclass_addr(), "NSMutableData metaclass data symbol"),
        ("_OBJC_CLASS_$_NSFileHandle", nsobj::nsfilehandle_class_addr(), "NSFileHandle class data symbol"),
        ("_OBJC_METACLASS_$_NSFileHandle", nsobj::nsfilehandle_metaclass_addr(), "NSFileHandle metaclass data symbol"),
        ("_OBJC_CLASS_$_NSScanner", nsobj::nsscanner_class_addr(), "NSScanner class data symbol"),
        ("_OBJC_METACLASS_$_NSScanner", nsobj::nsscanner_metaclass_addr(), "NSScanner metaclass data symbol"),
        ("_OBJC_CLASS_$_NSOrderedSet", nsobj::nsorderedset_class_addr(), "NSOrderedSet class data symbol"),
        ("_OBJC_METACLASS_$_NSOrderedSet", nsobj::nsorderedset_metaclass_addr(), "NSOrderedSet metaclass data symbol"),
        ("_OBJC_CLASS_$_NSMutableOrderedSet", nsobj::nsmutableorderedset_class_addr(), "NSMutableOrderedSet class data symbol"),
        ("_OBJC_METACLASS_$_NSMutableOrderedSet", nsobj::nsmutableorderedset_metaclass_addr(), "NSMutableOrderedSet metaclass data symbol"),
        ("_OBJC_CLASS_$_NSCountedSet", nsobj::nscountedset_class_addr(), "NSCountedSet class data symbol"),
        ("_OBJC_METACLASS_$_NSCountedSet", nsobj::nscountedset_metaclass_addr(), "NSCountedSet metaclass data symbol"),
        ("_OBJC_CLASS_$_NSMutableCharacterSet", nsobj::nsmutablecharacterset_class_addr(), "NSMutableCharacterSet class data symbol"),
        ("_OBJC_METACLASS_$_NSMutableCharacterSet", nsobj::nsmutablecharacterset_metaclass_addr(), "NSMutableCharacterSet metaclass data symbol"),
        // Phase C9 gap-analysis classes
        ("_OBJC_CLASS_$_NSURLComponents", nsobj::nsurlcomponents_class_addr(), "NSURLComponents class data symbol"),
        ("_OBJC_METACLASS_$_NSURLComponents", nsobj::nsurlcomponents_metaclass_addr(), "NSURLComponents metaclass data symbol"),
        ("_OBJC_CLASS_$_NSURLQueryItem", nsobj::nsurlqueryitem_class_addr(), "NSURLQueryItem class data symbol"),
        ("_OBJC_METACLASS_$_NSURLQueryItem", nsobj::nsurlqueryitem_metaclass_addr(), "NSURLQueryItem metaclass data symbol"),
        ("_OBJC_CLASS_$_NSByteCountFormatter", nsobj::nsbytecountformatter_class_addr(), "NSByteCountFormatter class data symbol"),
        ("_OBJC_METACLASS_$_NSByteCountFormatter", nsobj::nsbytecountformatter_metaclass_addr(), "NSByteCountFormatter metaclass data symbol"),
        ("_OBJC_CLASS_$_NSISO8601DateFormatter", nsobj::nsiso8601dateformatter_class_addr(), "NSISO8601DateFormatter class data symbol"),
        ("_OBJC_METACLASS_$_NSISO8601DateFormatter", nsobj::nsiso8601dateformatter_metaclass_addr(), "NSISO8601DateFormatter metaclass data symbol"),
        ("_OBJC_CLASS_$_NSProgress", nsobj::nsprogress_class_addr(), "NSProgress class data symbol"),
        ("_OBJC_METACLASS_$_NSProgress", nsobj::nsprogress_metaclass_addr(), "NSProgress metaclass data symbol"),
        ("_OBJC_CLASS_$_NSHost", nsobj::nshost_class_addr(), "NSHost class data symbol"),
        ("_OBJC_METACLASS_$_NSHost", nsobj::nshost_metaclass_addr(), "NSHost metaclass data symbol"),
        ("_OBJC_CLASS_$_NSPipe", nsobj::nspipe_class_addr(), "NSPipe class data symbol"),
        ("_OBJC_METACLASS_$_NSPipe", nsobj::nspipe_metaclass_addr(), "NSPipe metaclass data symbol"),
        ("_OBJC_CLASS_$_NSURLRequest", nsobj::nsurlrequest_class_addr(), "NSURLRequest class data symbol"),
        ("_OBJC_METACLASS_$_NSURLRequest", nsobj::nsurlrequest_metaclass_addr(), "NSURLRequest metaclass data symbol"),
        ("_OBJC_CLASS_$_NSMutableURLRequest", nsobj::nsmutableurlrequest_class_addr(), "NSMutableURLRequest class data symbol"),
        ("_OBJC_METACLASS_$_NSMutableURLRequest", nsobj::nsmutableurlrequest_metaclass_addr(), "NSMutableURLRequest metaclass data symbol"),
        ("_OBJC_CLASS_$_NSURLResponse", nsobj::nsurlresponse_class_addr(), "NSURLResponse class data symbol"),
        ("_OBJC_METACLASS_$_NSURLResponse", nsobj::nsurlresponse_metaclass_addr(), "NSURLResponse metaclass data symbol"),
        ("_OBJC_CLASS_$_NSHTTPURLResponse", nsobj::nshttpurlresponse_class_addr(), "NSHTTPURLResponse class data symbol"),
        ("_OBJC_METACLASS_$_NSHTTPURLResponse", nsobj::nshttpurlresponse_metaclass_addr(), "NSHTTPURLResponse metaclass data symbol"),
        ("_OBJC_CLASS_$_NSTask", nsobj::nstask_class_addr(), "NSTask class data symbol"),
        ("_OBJC_METACLASS_$_NSTask", nsobj::nstask_metaclass_addr(), "NSTask metaclass data symbol"),
        ("_OBJC_CLASS_$_PKBundleComponentVersion", nsobj::pkbundlecomponentversion_class_addr(), "PKBundleComponentVersion class data symbol"),
        ("_OBJC_METACLASS_$_PKBundleComponentVersion", nsobj::pkbundlecomponentversion_metaclass_addr(), "PKBundleComponentVersion metaclass data symbol"),
        // Phase C10
        ("_OBJC_CLASS_$_NSStream", nsobj::nsstream_class_addr(), "NSStream class data symbol"),
        ("_OBJC_METACLASS_$_NSStream", nsobj::nsstream_metaclass_addr(), "NSStream metaclass data symbol"),
        ("_OBJC_CLASS_$_NSInputStream", nsobj::nsinputstream_class_addr(), "NSInputStream class data symbol"),
        ("_OBJC_METACLASS_$_NSInputStream", nsobj::nsinputstream_metaclass_addr(), "NSInputStream metaclass data symbol"),
        ("_OBJC_CLASS_$_NSOutputStream", nsobj::nsoutputstream_class_addr(), "NSOutputStream class data symbol"),
        ("_OBJC_METACLASS_$_NSOutputStream", nsobj::nsoutputstream_metaclass_addr(), "NSOutputStream metaclass data symbol"),
        ("_OBJC_CLASS_$_NSXMLParser", nsobj::nsxmlparser_class_addr(), "NSXMLParser class data symbol"),
        ("_OBJC_METACLASS_$_NSXMLParser", nsobj::nsxmlparser_metaclass_addr(), "NSXMLParser metaclass data symbol"),
        ("_OBJC_CLASS_$_NSURLSession", nsobj::nsurlsession_class_addr(), "NSURLSession class data symbol"),
        ("_OBJC_METACLASS_$_NSURLSession", nsobj::nsurlsession_metaclass_addr(), "NSURLSession metaclass data symbol"),
        ("_OBJC_CLASS_$_NSURLSessionConfiguration", nsobj::nsurlsessionconfiguration_class_addr(), "NSURLSessionConfiguration class data symbol"),
        ("_OBJC_METACLASS_$_NSURLSessionConfiguration", nsobj::nsurlsessionconfiguration_metaclass_addr(), "NSURLSessionConfiguration metaclass data symbol"),
        ("_OBJC_CLASS_$_NSURLSessionDataTask", nsobj::nsurlsessiondatatask_class_addr(), "NSURLSessionDataTask class data symbol"),
        ("_OBJC_METACLASS_$_NSURLSessionDataTask", nsobj::nsurlsessiondatatask_metaclass_addr(), "NSURLSessionDataTask metaclass data symbol"),
        ("_OBJC_CLASS_$_NSKeyedArchiver", nsobj::nskeyedarchiver_class_addr(), "NSKeyedArchiver class data symbol"),
        ("_OBJC_METACLASS_$_NSKeyedArchiver", nsobj::nskeyedarchiver_metaclass_addr(), "NSKeyedArchiver metaclass data symbol"),
        ("_OBJC_CLASS_$_NSKeyedUnarchiver", nsobj::nskeyedunarchiver_class_addr(), "NSKeyedUnarchiver class data symbol"),
        ("_OBJC_METACLASS_$_NSKeyedUnarchiver", nsobj::nskeyedunarchiver_metaclass_addr(), "NSKeyedUnarchiver metaclass data symbol"),
        ("_OBJC_CLASS_$_NSURLCredential", nsobj::nsurlcredential_class_addr(), "NSURLCredential class data symbol"),
        ("_OBJC_METACLASS_$_NSURLCredential", nsobj::nsurlcredential_metaclass_addr(), "NSURLCredential metaclass data symbol"),
        ("_OBJC_CLASS_$_NSURLCredentialStorage", nsobj::nsurlcredentialstorage_class_addr(), "NSURLCredentialStorage class data symbol"),
        ("_OBJC_METACLASS_$_NSURLCredentialStorage", nsobj::nsurlcredentialstorage_metaclass_addr(), "NSURLCredentialStorage metaclass data symbol"),
        ("_OBJC_CLASS_$_NSHTTPCookie", nsobj::nshttpcookie_class_addr(), "NSHTTPCookie class data symbol"),
        ("_OBJC_METACLASS_$_NSHTTPCookie", nsobj::nshttpcookie_metaclass_addr(), "NSHTTPCookie metaclass data symbol"),
        ("_OBJC_CLASS_$_NSHTTPCookieStorage", nsobj::nshttpcookiestorage_class_addr(), "NSHTTPCookieStorage class data symbol"),
        ("_OBJC_METACLASS_$_NSHTTPCookieStorage", nsobj::nshttpcookiestorage_metaclass_addr(), "NSHTTPCookieStorage metaclass data symbol"),
        ("_OBJC_CLASS_$_NSAutoreleasePool", nsobj::nsautoreleasepool_class_addr(), "NSAutoreleasePool class data symbol"),
        ("_OBJC_METACLASS_$_NSAutoreleasePool", nsobj::nsautoreleasepool_metaclass_addr(), "NSAutoreleasePool metaclass data symbol"),
        // NSPasteboard class data symbols
        ("_OBJC_CLASS_$_NSPasteboard", nsobj::nspasteboard_class_addr(), "NSPasteboard class data symbol"),
        ("_OBJC_METACLASS_$_NSPasteboard", nsobj::nspasteboard_metaclass_addr(), "NSPasteboard metaclass data symbol"),
        // NSUnit / NSMeasurement class data symbols
        ("_OBJC_CLASS_$_NSUnit", nsobj::nsunit_class_addr(), "NSUnit class data symbol"),
        ("_OBJC_METACLASS_$_NSUnit", nsobj::nsunit_metaclass_addr(), "NSUnit metaclass data symbol"),
        ("_OBJC_CLASS_$_NSDimension", nsobj::nsdimension_class_addr(), "NSDimension class data symbol"),
        ("_OBJC_METACLASS_$_NSDimension", nsobj::nsdimension_metaclass_addr(), "NSDimension metaclass data symbol"),
        ("_OBJC_CLASS_$_NSMeasurement", nsobj::nsmeasurement_class_addr(), "NSMeasurement class data symbol"),
        ("_OBJC_METACLASS_$_NSMeasurement", nsobj::nsmeasurement_metaclass_addr(), "NSMeasurement metaclass data symbol"),
        ("_OBJC_CLASS_$_NSUnitLength", nsobj::nsunitlength_class_addr(), "NSUnitLength class data symbol"),
        ("_OBJC_METACLASS_$_NSUnitLength", nsobj::nsunitlength_metaclass_addr(), "NSUnitLength metaclass data symbol"),
        ("_OBJC_CLASS_$_NSUnitMass", nsobj::nsunitmass_class_addr(), "NSUnitMass class data symbol"),
        ("_OBJC_METACLASS_$_NSUnitMass", nsobj::nsunitmass_metaclass_addr(), "NSUnitMass metaclass data symbol"),
        ("_OBJC_CLASS_$_NSUnitTemperature", nsobj::nsunittemperature_class_addr(), "NSUnitTemperature class data symbol"),
        ("_OBJC_METACLASS_$_NSUnitTemperature", nsobj::nsunittemperature_metaclass_addr(), "NSUnitTemperature metaclass data symbol"),
        ("_OBJC_CLASS_$_NSUnitVolume", nsobj::nsunitvolume_class_addr(), "NSUnitVolume class data symbol"),
        ("_OBJC_METACLASS_$_NSUnitVolume", nsobj::nsunitvolume_metaclass_addr(), "NSUnitVolume metaclass data symbol"),
        ("_OBJC_CLASS_$_NSUnitAngle", nsobj::nsunitangle_class_addr(), "NSUnitAngle class data symbol"),
        ("_OBJC_METACLASS_$_NSUnitAngle", nsobj::nsunitangle_metaclass_addr(), "NSUnitAngle metaclass data symbol"),
        ("_OBJC_CLASS_$_NSUnitSpeed", nsobj::nsunitspeed_class_addr(), "NSUnitSpeed class data symbol"),
        ("_OBJC_METACLASS_$_NSUnitSpeed", nsobj::nsunitspeed_metaclass_addr(), "NSUnitSpeed metaclass data symbol"),
        ("_OBJC_CLASS_$_NSUnitDuration", nsobj::nsunitduration_class_addr(), "NSUnitDuration class data symbol"),
        ("_OBJC_METACLASS_$_NSUnitDuration", nsobj::nsunitduration_metaclass_addr(), "NSUnitDuration metaclass data symbol"),
        ("_OBJC_CLASS_$_NSMeasurementFormatter", nsobj::nsmeasurementformatter_class_addr(), "NSMeasurementFormatter class data symbol"),
        ("_OBJC_METACLASS_$_NSMeasurementFormatter", nsobj::nsmeasurementformatter_metaclass_addr(), "NSMeasurementFormatter metaclass data symbol"),
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
        ("_NSStringFromPoint", nsvalue::shim_nsstringfrompoint as *const () as u64, "NSStringFromPoint → format CGPoint as NSString"),
        ("_NSStringFromSize", nsvalue::shim_nsstringfromsize as *const () as u64, "NSStringFromSize → format CGSize as NSString"),
        ("_NSStringFromRect", nsvalue::shim_nsstringfromrect as *const () as u64, "NSStringFromRect → format CGRect as NSString"),
        ("_NSStringFromRange", nsvalue::shim_nsstringfromrange as *const () as u64, "NSStringFromRange → format NSRange as NSString"),
    );

    // CoreFoundation core functions
    register_shims!(table,
        ("_CFRetain", cf_core::shim_cf_retain as *const () as u64, "CFRetain shim"),
        ("_CFGetTypeID", cf_core::shim_cf_get_type_id as *const () as u64, "CFGetTypeID shim"),
        ("_CFEqual", cf_core::shim_cf_equal as *const () as u64, "CFEqual shim"),
        ("_CFHash", cf_core::shim_cf_hash as *const () as u64, "CFHash shim"),
        ("_CFCopyDescription", cf_core::shim_cf_copy_description as *const () as u64, "CFCopyDescription shim"),
        ("_CFPreferencesCopyValue", cf_core::shim_cf_preferences_copy_value as *const () as u64, "CFPreferencesCopyValue shim"),
    );

    // CF type ID getters
    register_shims!(table,
        ("_CFStringGetTypeID", cf_core::shim_cf_string_get_type_id as *const () as u64, "CFStringGetTypeID shim"),
        ("_CFArrayGetTypeID", cf_core::shim_cf_array_get_type_id as *const () as u64, "CFArrayGetTypeID shim"),
        ("_CFDictionaryGetTypeID", cf_core::shim_cf_dictionary_get_type_id as *const () as u64, "CFDictionaryGetTypeID shim"),
        ("_CFDataGetTypeID", cf_core::shim_cf_data_get_type_id as *const () as u64, "CFDataGetTypeID shim"),
        ("_CFURLGetTypeID", cf_core::shim_cf_url_get_type_id as *const () as u64, "CFURLGetTypeID shim"),
        ("_CFBooleanGetTypeID", cf_core::shim_cf_boolean_get_type_id as *const () as u64, "CFBooleanGetTypeID shim"),
        ("_CFNumberGetTypeID", cf_core::shim_cf_number_get_type_id as *const () as u64, "CFNumberGetTypeID shim"),
        ("_CFNullGetTypeID", cf_core::shim_cf_null_get_type_id as *const () as u64, "CFNullGetTypeID shim"),
    );

    // CFArray C functions
    register_shims!(table,
        ("_CFArrayCreate", cf_core::shim_cf_array_create as *const () as u64, "CFArrayCreate shim"),
        ("_CFArrayGetCount", cf_core::shim_cf_array_get_count as *const () as u64, "CFArrayGetCount shim"),
        ("_CFArrayGetValueAtIndex", cf_core::shim_cf_array_get_value_at_index as *const () as u64, "CFArrayGetValueAtIndex shim"),
    );

    // CFDictionary C functions
    register_shims!(table,
        ("_CFDictionaryCreate", cf_core::shim_cf_dictionary_create as *const () as u64, "CFDictionaryCreate shim"),
        ("_CFDictionaryGetCount", cf_core::shim_cf_dictionary_get_count as *const () as u64, "CFDictionaryGetCount shim"),
        ("_CFDictionaryGetValue", cf_core::shim_cf_dictionary_get_value as *const () as u64, "CFDictionaryGetValue shim"),
        ("_CFDictionaryGetValueIfPresent", cf_core::shim_cf_dictionary_get_value_if_present as *const () as u64, "CFDictionaryGetValueIfPresent shim"),
    );

    // CFData C functions
    register_shims!(table,
        ("_CFDataCreate", cf_core::shim_cf_data_create as *const () as u64, "CFDataCreate shim"),
        ("_CFDataGetBytePtr", cf_core::shim_cf_data_get_byte_ptr as *const () as u64, "CFDataGetBytePtr shim"),
        ("_CFDataGetLength", cf_core::shim_cf_data_get_length as *const () as u64, "CFDataGetLength shim"),
        ("_CFDataCreateWithBytesNoCopy", cf_core::shim_cf_data_create_with_bytes_no_copy as *const () as u64, "CFDataCreateWithBytesNoCopy shim"),
    );

    // CFURL C functions
    register_shims!(table,
        ("_CFURLCreateWithString", cf_core::shim_cf_url_create_with_string as *const () as u64, "CFURLCreateWithString shim"),
        ("_CFURLGetString", cf_core::shim_cf_url_get_string as *const () as u64, "CFURLGetString shim"),
        ("_CFURLCreateWithFileSystemPath", cf_core::shim_cf_url_create_with_file_system_path as *const () as u64, "CFURLCreateWithFileSystemPath shim"),
    );

    // CFBundle C functions
    register_shims!(table,
        ("_CFBundleGetMainBundle", cf_core::shim_cf_bundle_get_main_bundle as *const () as u64, "CFBundleGetMainBundle shim"),
        ("_CFBundleCopyBundleURL", cf_core::shim_cf_bundle_copy_bundle_url as *const () as u64, "CFBundleCopyBundleURL shim"),
        ("_CFBundleCopyInfoDictionaryForURL", cf_core::shim_cf_bundle_copy_info_dictionary_for_url as *const () as u64, "CFBundleCopyInfoDictionaryForURL shim"),
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

    // CFString encoding conversion functions
    register_shims!(table,
        ("_CFStringConvertEncodingToNSStringEncoding", cf_core::shim_cf_string_convert_encoding_to_nsstring_encoding as *const () as u64, "CFStringConvertEncodingToNSStringEncoding shim"),
        ("_CFStringConvertIANACharSetNameToEncoding", cf_core::shim_cf_string_convert_iana_charset_name_to_encoding as *const () as u64, "CFStringConvertIANACharSetNameToEncoding shim"),
    );

    // ___NSArray0__struct — empty immutable array singleton
    register_shims!(table,
        ("____NSArray0__struct", cf_core::nsarray0_struct_addr(), "___NSArray0__struct empty array sentinel"),
    );

    // XCStringsParser.framework class symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_XCStringsdataEntry", nsobj::xcstringsdataentry_class_addr(), "XCStringsdataEntry class"),
        ("_OBJC_CLASS_$_XCStringsdataTable", nsobj::xcstringsdatatable_class_addr(), "XCStringsdataTable class"),
        ("_OBJC_CLASS_$_XCStringsdataFile", nsobj::xcstringsdatafile_class_addr(), "XCStringsdataFile class"),
    );

    // CoreData class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSManagedObjectContext",      nsobj::nsmanagedobjectcontext_class_addr(),      "NSManagedObjectContext class"),
        ("_OBJC_METACLASS_$_NSManagedObjectContext",  nsobj::nsmanagedobjectcontext_metaclass_addr(),  "NSManagedObjectContext metaclass"),
        ("_OBJC_CLASS_$_NSPersistentStoreCoordinator", nsobj::nspersistentstorecoordinator_class_addr(), "NSPersistentStoreCoordinator class"),
        ("_OBJC_METACLASS_$_NSPersistentStoreCoordinator", nsobj::nspersistentstorecoordinator_metaclass_addr(), "NSPersistentStoreCoordinator metaclass"),
        ("_OBJC_CLASS_$_NSFetchRequest",              nsobj::nsfetchrequest_class_addr(),              "NSFetchRequest class"),
        ("_OBJC_METACLASS_$_NSFetchRequest",          nsobj::nsfetchrequest_metaclass_addr(),          "NSFetchRequest metaclass"),
        ("_OBJC_CLASS_$_NSManagedObject",             nsobj::nsmanagedobject_class_addr(),             "NSManagedObject class"),
        ("_OBJC_METACLASS_$_NSManagedObject",         nsobj::nsmanagedobject_metaclass_addr(),         "NSManagedObject metaclass"),
        ("_OBJC_CLASS_$_NSEntityDescription",         nsobj::nsentitydescription_class_addr(),         "NSEntityDescription class"),
        ("_OBJC_METACLASS_$_NSEntityDescription",     nsobj::nsentitydescription_metaclass_addr(),     "NSEntityDescription metaclass"),
        ("_OBJC_CLASS_$_NSManagedObjectModel",        nsobj::nsmanagedobjectmodel_class_addr(),        "NSManagedObjectModel class"),
        ("_OBJC_METACLASS_$_NSManagedObjectModel",    nsobj::nsmanagedobjectmodel_metaclass_addr(),    "NSManagedObjectModel metaclass"),
    );

    // AVFoundation class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_AVAudioPlayer",     nsobj::avaudioplayer_class_addr(),     "AVAudioPlayer class"),
        ("_OBJC_METACLASS_$_AVAudioPlayer", nsobj::avaudioplayer_metaclass_addr(), "AVAudioPlayer metaclass"),
        ("_OBJC_CLASS_$_AVAudioRecorder",     nsobj::avaudiorecorder_class_addr(),     "AVAudioRecorder class"),
        ("_OBJC_METACLASS_$_AVAudioRecorder", nsobj::avaudiorecorder_metaclass_addr(), "AVAudioRecorder metaclass"),
        ("_OBJC_CLASS_$_AVAudioEngine",     nsobj::avaudioengine_class_addr(),     "AVAudioEngine class"),
        ("_OBJC_METACLASS_$_AVAudioEngine", nsobj::avaudioengine_metaclass_addr(), "AVAudioEngine metaclass"),
        ("_OBJC_CLASS_$_AVCaptureDevice",     nsobj::avcapturedevice_class_addr(),     "AVCaptureDevice class"),
        ("_OBJC_METACLASS_$_AVCaptureDevice", nsobj::avcapturedevice_metaclass_addr(), "AVCaptureDevice metaclass"),
        ("_OBJC_CLASS_$_AVCaptureSession",     nsobj::avcapturesession_class_addr(),     "AVCaptureSession class"),
        ("_OBJC_METACLASS_$_AVCaptureSession", nsobj::avcapturesession_metaclass_addr(), "AVCaptureSession metaclass"),
        ("_OBJC_CLASS_$_AVPlayer",     nsobj::avplayer_class_addr(),     "AVPlayer class"),
        ("_OBJC_METACLASS_$_AVPlayer", nsobj::avplayer_metaclass_addr(), "AVPlayer metaclass"),
    );

    // CoreLocation class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_CLLocationManager",     nsobj::cllocationmanager_class_addr(),     "CLLocationManager class"),
        ("_OBJC_METACLASS_$_CLLocationManager", nsobj::cllocationmanager_metaclass_addr(), "CLLocationManager metaclass"),
        ("_OBJC_CLASS_$_CLLocation",     nsobj::cllocation_class_addr(),     "CLLocation class"),
        ("_OBJC_METACLASS_$_CLLocation", nsobj::cllocation_metaclass_addr(), "CLLocation metaclass"),
        ("_OBJC_CLASS_$_CLGeocoder",     nsobj::clgeocoder_class_addr(),     "CLGeocoder class"),
        ("_OBJC_METACLASS_$_CLGeocoder", nsobj::clgeocoder_metaclass_addr(), "CLGeocoder metaclass"),
    );

    // CoreBluetooth class/metaclass data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_CBCentralManager",     nsobj::cbcentralmanager_class_addr(),     "CBCentralManager class"),
        ("_OBJC_METACLASS_$_CBCentralManager", nsobj::cbcentralmanager_metaclass_addr(), "CBCentralManager metaclass"),
        ("_OBJC_CLASS_$_CBPeripheralManager",     nsobj::cbperipheralmanager_class_addr(),     "CBPeripheralManager class"),
        ("_OBJC_METACLASS_$_CBPeripheralManager", nsobj::cbperipheralmanager_metaclass_addr(), "CBPeripheralManager metaclass"),
    );

    // Foundation string constant data symbols (pointer-stable)
    register_shims!(table,
        ("_NSStreamDataWrittenToMemoryStreamKey", foundation_constants::nsstream_data_written_to_memory_key_addr(), "NSStreamDataWrittenToMemoryStreamKey"),
        ("_NSCocoaErrorDomain", foundation_constants::nscocoa_error_domain_addr(), "NSCocoaErrorDomain constant string"),
        ("_NSPOSIXErrorDomain", foundation_constants::nsposix_error_domain_addr(), "NSPOSIXErrorDomain constant string"),
        ("_NSURLErrorDomain", foundation_constants::nsurl_error_domain_addr(), "NSURLErrorDomain constant string"),
        ("_NSMachErrorDomain", foundation_constants::nsmach_error_domain_addr(), "NSMachErrorDomain constant string"),
        ("_NSLocalizedDescriptionKey", foundation_constants::nslocalized_description_key_addr(), "NSLocalizedDescriptionKey constant string"),
        ("_NSLocalizedFailureReasonErrorKey", foundation_constants::nslocalized_failure_reason_key_addr(), "NSLocalizedFailureReasonErrorKey constant string"),
        ("_NSFilePathErrorKey", foundation_constants::nsfile_path_error_key_addr(), "NSFilePathErrorKey constant string"),
        ("_NSUserDefaultsDidChangeNotification", foundation_constants::nsuserdefaults_did_change_notification_addr(), "NSUserDefaultsDidChangeNotification constant string"),
        ("_kCFBundleShortVersionStringKey", foundation_constants::cfbundle_short_version_string_key_addr(), "kCFBundleShortVersionStringKey constant string"),
        ("__kCFBundleShortVersionStringKey", foundation_constants::cfbundle_short_version_string_key_addr(), "__kCFBundleShortVersionStringKey constant string"),
        ("_kCFBundleVersionKey", foundation_constants::cfbundle_version_key_addr(), "kCFBundleVersionKey constant string"),
    );

    // CFNumber C functions
    register_shims!(table,
        ("_CFNumberCreate", cf_core::shim_cf_number_create as *const () as u64, "CFNumberCreate → create number from type+ptr"),
        ("_CFNumberGetValue", cf_core::shim_cf_number_get_value as *const () as u64, "CFNumberGetValue → extract value by type"),
        ("_CFNumberGetType", cf_core::shim_cf_number_get_type as *const () as u64, "CFNumberGetType → CFNumberType enum"),
        ("_CFNumberCompare", cf_core::shim_cf_number_compare as *const () as u64, "CFNumberCompare → numeric ordering"),
        ("_CFNumberIsFloatType", cf_core::shim_cf_number_is_float_type as *const () as u64, "CFNumberIsFloatType → Boolean"),
    );

    // CFString expanded C functions
    register_shims!(table,
        ("_CFStringGetCString", cf_core::shim_cf_string_get_cstring as *const () as u64, "CFStringGetCString → copy to C buffer"),
        ("_CFStringCreateCopy", cf_core::shim_cf_string_create_copy as *const () as u64, "CFStringCreateCopy → immutable copy"),
        ("_CFStringCompare", cf_core::shim_cf_string_compare as *const () as u64, "CFStringCompare → lexical ordering"),
        ("_CFStringCreateWithCharacters", cf_core::shim_cf_string_create_with_characters as *const () as u64, "CFStringCreateWithCharacters → from UTF-16"),
        ("_CFStringGetCharacterAtIndex", cf_core::shim_cf_string_get_character_at_index as *const () as u64, "CFStringGetCharacterAtIndex → UniChar"),
        ("_CFStringFind", cf_core::shim_cf_string_find as *const () as u64, "CFStringFind → substring search"),
        ("_CFStringGetBytes", cf_core::shim_cf_string_get_bytes as *const () as u64, "CFStringGetBytes → convert to byte buffer"),
        ("_CFStringGetFileSystemRepresentation", cf_core::shim_cf_string_get_file_system_representation as *const () as u64, "CFStringGetFileSystemRepresentation → path bytes"),
    );

    // CFRunLoop C functions
    register_shims!(table,
        ("_CFRunLoopGetCurrent", cf_core::shim_cf_run_loop_get_current as *const () as u64, "CFRunLoopGetCurrent → current thread run loop"),
        ("_CFRunLoopGetMain", cf_core::shim_cf_run_loop_get_main as *const () as u64, "CFRunLoopGetMain → main thread run loop"),
        ("_CFRunLoopRun", cf_core::shim_cf_run_loop_run as *const () as u64, "CFRunLoopRun → run default mode"),
        ("_CFRunLoopRunInMode", cf_core::shim_cf_run_loop_run_in_mode as *const () as u64, "CFRunLoopRunInMode → timed mode run"),
        ("_CFRunLoopStop", cf_core::shim_cf_run_loop_stop as *const () as u64, "CFRunLoopStop → exit run loop"),
        ("_CFRunLoopAddTimer", cf_core::shim_cf_run_loop_add_timer as *const () as u64, "CFRunLoopAddTimer → schedule timer"),
        ("_CFRunLoopAddSource", cf_core::shim_cf_run_loop_add_source as *const () as u64, "CFRunLoopAddSource → add input source"),
        ("_CFRunLoopRemoveSource", cf_core::shim_cf_run_loop_remove_source as *const () as u64, "CFRunLoopRemoveSource → remove input source"),
        ("_CFRunLoopSourceCreate", cf_core::shim_cf_run_loop_source_create as *const () as u64, "CFRunLoopSourceCreate → allocate source"),
        ("_CFRunLoopSourceSignal", cf_core::shim_cf_run_loop_source_signal as *const () as u64, "CFRunLoopSourceSignal → signal source"),
        ("_CFRunLoopSourceInvalidate", cf_core::shim_cf_run_loop_source_invalidate as *const () as u64, "CFRunLoopSourceInvalidate → invalidate source"),
        ("_CFRunLoopSourceIsValid", cf_core::shim_cf_run_loop_source_is_valid as *const () as u64, "CFRunLoopSourceIsValid → check source validity"),
        ("_CFRunLoopObserverCreate", cf_core::shim_cf_run_loop_observer_create as *const () as u64, "CFRunLoopObserverCreate → allocate observer"),
        ("_CFRunLoopAddObserver", cf_core::shim_cf_run_loop_add_observer as *const () as u64, "CFRunLoopAddObserver → register observer"),
        ("_CFRunLoopRemoveObserver", cf_core::shim_cf_run_loop_remove_observer as *const () as u64, "CFRunLoopRemoveObserver → unregister observer"),
        ("_CFRunLoopObserverInvalidate", cf_core::shim_cf_run_loop_observer_invalidate as *const () as u64, "CFRunLoopObserverInvalidate → invalidate observer"),
        ("_kCFRunLoopDefaultMode", cf_core::kcf_run_loop_default_mode_addr(), "kCFRunLoopDefaultMode constant string"),
        ("_kCFRunLoopCommonModes", cf_core::kcf_run_loop_common_modes_addr(), "kCFRunLoopCommonModes constant string"),
    );

    // CFAllocator C functions
    register_shims!(table,
        ("_CFAllocatorGetDefault", cf_core::shim_cf_allocator_get_default as *const () as u64, "CFAllocatorGetDefault → default allocator"),
        ("_CFAllocatorAllocate", cf_core::shim_cf_allocator_allocate as *const () as u64, "CFAllocatorAllocate → calloc wrapper"),
        ("_CFAllocatorDeallocate", cf_core::shim_cf_allocator_deallocate as *const () as u64, "CFAllocatorDeallocate → free wrapper"),
    );

    // CFMutableArray C functions
    register_shims!(table,
        ("_CFArrayCreateMutable", cf_core::shim_cf_array_create_mutable as *const () as u64, "CFArrayCreateMutable → NSMutableArray"),
        ("_CFArrayAppendValue", cf_core::shim_cf_array_append_value as *const () as u64, "CFArrayAppendValue → addObject:"),
        ("_CFArrayInsertValueAtIndex", cf_core::shim_cf_array_insert_value_at_index as *const () as u64, "CFArrayInsertValueAtIndex → insertObject:atIndex:"),
        ("_CFArrayRemoveValueAtIndex", cf_core::shim_cf_array_remove_value_at_index as *const () as u64, "CFArrayRemoveValueAtIndex → removeObjectAtIndex:"),
        ("_CFArraySetValueAtIndex", cf_core::shim_cf_array_set_value_at_index as *const () as u64, "CFArraySetValueAtIndex → replaceObjectAtIndex:withObject:"),
        ("_CFArrayRemoveAllValues", cf_core::shim_cf_array_remove_all_values as *const () as u64, "CFArrayRemoveAllValues → removeAllObjects"),
        ("_CFArraySortValues", cf_core::shim_cf_array_sort_values as *const () as u64, "CFArraySortValues → stub no-op"),
    );

    // CFMutableDictionary C functions
    register_shims!(table,
        ("_CFDictionaryCreateMutable", cf_core::shim_cf_dictionary_create_mutable as *const () as u64, "CFDictionaryCreateMutable → NSMutableDictionary"),
        ("_CFDictionarySetValue", cf_core::shim_cf_dictionary_set_value as *const () as u64, "CFDictionarySetValue → setObject:forKey:"),
        ("_CFDictionaryAddValue", cf_core::shim_cf_dictionary_add_value as *const () as u64, "CFDictionaryAddValue → setObject:forKey:"),
        ("_CFDictionaryReplaceValue", cf_core::shim_cf_dictionary_replace_value as *const () as u64, "CFDictionaryReplaceValue → replace if exists"),
        ("_CFDictionaryRemoveValue", cf_core::shim_cf_dictionary_remove_value as *const () as u64, "CFDictionaryRemoveValue → removeObjectForKey:"),
        ("_CFDictionaryRemoveAllValues", cf_core::shim_cf_dictionary_remove_all_values as *const () as u64, "CFDictionaryRemoveAllValues → removeAllObjects"),
    );

    // CFSet / CFMutableSet C functions
    register_shims!(table,
        ("_CFSetCreate", cf_core::shim_cf_set_create as *const () as u64, "CFSetCreate → NSSet"),
        ("_CFSetGetTypeID", cf_core::shim_cf_set_get_type_id as *const () as u64, "CFSetGetTypeID"),
        ("_CFSetGetCount", cf_core::shim_cf_set_get_count as *const () as u64, "CFSetGetCount → count"),
        ("_CFSetContainsValue", cf_core::shim_cf_set_contains_value as *const () as u64, "CFSetContainsValue → containsObject:"),
        ("_CFSetGetValues", cf_core::shim_cf_set_get_values as *const () as u64, "CFSetGetValues → copy pointers"),
        ("_CFSetCreateMutable", cf_core::shim_cf_set_create_mutable as *const () as u64, "CFSetCreateMutable → NSMutableSet"),
        ("_CFSetAddValue", cf_core::shim_cf_set_add_value as *const () as u64, "CFSetAddValue → addObject:"),
        ("_CFSetRemoveValue", cf_core::shim_cf_set_remove_value as *const () as u64, "CFSetRemoveValue → removeObject:"),
        ("_CFSetRemoveAllValues", cf_core::shim_cf_set_remove_all_values as *const () as u64, "CFSetRemoveAllValues → removeAllObjects"),
    );

    // CFUUID C functions
    register_shims!(table,
        ("_CFUUIDCreate", cf_core::shim_cf_uuid_create as *const () as u64, "CFUUIDCreate → new random UUID"),
        ("_CFUUIDCreateString", cf_core::shim_cf_uuid_create_string as *const () as u64, "CFUUIDCreateString → UUID string"),
        ("_CFUUIDCreateFromString", cf_core::shim_cf_uuid_create_from_string as *const () as u64, "CFUUIDCreateFromString → parse string"),
        ("_CFUUIDGetUUIDBytes", cf_core::shim_cf_uuid_get_uuid_bytes as *const () as u64, "CFUUIDGetUUIDBytes → stub 0"),
        ("_CFUUIDCreateFromUUIDBytes", cf_core::shim_cf_uuid_create_from_uuid_bytes as *const () as u64, "CFUUIDCreateFromUUIDBytes → new UUID"),
    );

    // CFByteOrder C functions
    register_shims!(table,
        ("_CFByteOrderGetCurrent", cf_core::shim_cf_byte_order_get_current as *const () as u64, "CFByteOrderGetCurrent → 1 (little-endian)"),
        ("_CFSwapInt16BigToHost", cf_core::shim_cf_swap_int16_big_to_host as *const () as u64, "CFSwapInt16BigToHost"),
        ("_CFSwapInt32BigToHost", cf_core::shim_cf_swap_int32_big_to_host as *const () as u64, "CFSwapInt32BigToHost"),
        ("_CFSwapInt64BigToHost", cf_core::shim_cf_swap_int64_big_to_host as *const () as u64, "CFSwapInt64BigToHost"),
        ("_CFSwapInt16HostToBig", cf_core::shim_cf_swap_int16_host_to_big as *const () as u64, "CFSwapInt16HostToBig"),
        ("_CFSwapInt32HostToBig", cf_core::shim_cf_swap_int32_host_to_big as *const () as u64, "CFSwapInt32HostToBig"),
        ("_CFSwapInt64HostToBig", cf_core::shim_cf_swap_int64_host_to_big as *const () as u64, "CFSwapInt64HostToBig"),
        ("_CFSwapInt16LittleToHost", cf_core::shim_cf_swap_int16_little_to_host as *const () as u64, "CFSwapInt16LittleToHost"),
        ("_CFSwapInt32LittleToHost", cf_core::shim_cf_swap_int32_little_to_host as *const () as u64, "CFSwapInt32LittleToHost"),
        ("_CFSwapInt64LittleToHost", cf_core::shim_cf_swap_int64_little_to_host as *const () as u64, "CFSwapInt64LittleToHost"),
        ("_CFSwapInt16HostToLittle", cf_core::shim_cf_swap_int16_host_to_little as *const () as u64, "CFSwapInt16HostToLittle"),
        ("_CFSwapInt32HostToLittle", cf_core::shim_cf_swap_int32_host_to_little as *const () as u64, "CFSwapInt32HostToLittle"),
        ("_CFSwapInt64HostToLittle", cf_core::shim_cf_swap_int64_host_to_little as *const () as u64, "CFSwapInt64HostToLittle"),
        ("_CFConvertFloat64SwappedToHost", cf_core::shim_cf_convert_float64_swapped_to_host as *const () as u64, "CFConvertFloat64SwappedToHost"),
        ("_CFConvertFloat32HostToSwapped", cf_core::shim_cf_convert_float32_host_to_swapped as *const () as u64, "CFConvertFloat32HostToSwapped"),
    );

    // CFPropertyList C functions
    register_shims!(table,
        ("_CFPropertyListCreateWithData", cf_core::shim_cf_property_list_create_with_data as *const () as u64, "CFPropertyListCreateWithData → parse plist"),
        ("_CFPropertyListCreateData", cf_core::shim_cf_property_list_create_data as *const () as u64, "CFPropertyListCreateData → serialize plist"),
        ("_CFPropertyListIsValid", cf_core::shim_cf_property_list_is_valid as *const () as u64, "CFPropertyListIsValid → validate"),
    );

    // NSPasteboard C-level function stubs (for programs that call +[NSPasteboard generalPasteboard] directly)
    register_shims!(table,
        ("_NSPasteboardTypeString", nspasteboard::ns_pasteboard_type_string_addr(), "NSPasteboardTypeString constant"),
        ("_NSPasteboardTypeRTF", nspasteboard::ns_pasteboard_type_rtf_addr(), "NSPasteboardTypeRTF constant"),
        ("_NSPasteboardTypeHTML", nspasteboard::ns_pasteboard_type_html_addr(), "NSPasteboardTypeHTML constant"),
        ("_NSPasteboardTypeTIFF", nspasteboard::ns_pasteboard_type_tiff_addr(), "NSPasteboardTypeTIFF constant"),
        ("_NSPasteboardTypePNG", nspasteboard::ns_pasteboard_type_png_addr(), "NSPasteboardTypePNG constant"),
        ("_NSPasteboardTypePDF", nspasteboard::ns_pasteboard_type_pdf_addr(), "NSPasteboardTypePDF constant"),
        ("_NSPasteboardTypeFileURL", nspasteboard::ns_pasteboard_type_file_url_addr(), "NSPasteboardTypeFileURL constant"),
        ("_NSPasteboardTypeURL", nspasteboard::ns_pasteboard_type_url_addr(), "NSPasteboardTypeURL constant"),
        ("_NSStringPboardType", nspasteboard::ns_string_pasteboard_type_addr(), "NSStringPboardType (legacy)"),
        ("_NSRTFPboardType", nspasteboard::ns_rtf_pasteboard_type_addr(), "NSRTFPboardType (legacy)"),
        ("_NSFilenamesPboardType", nspasteboard::ns_file_names_pasteboard_type_addr(), "NSFilenamesPboardType (legacy)"),
    );

    // NSAttributedString attribute key constants (GUI text rendering)
    register_shims!(table,
        ("_NSFontAttributeName", foundation_constants::nsfont_attribute_name_addr(), "NSFontAttributeName constant"),
        ("_NSForegroundColorAttributeName", foundation_constants::nsforeground_color_attribute_name_addr(), "NSForegroundColorAttributeName constant"),
        ("_NSBackgroundColorAttributeName", foundation_constants::nsbackground_color_attribute_name_addr(), "NSBackgroundColorAttributeName constant"),
        ("_NSParagraphStyleAttributeName", foundation_constants::nsparagraph_style_attribute_name_addr(), "NSParagraphStyleAttributeName constant"),
        ("_NSKernAttributeName", foundation_constants::nskern_attribute_name_addr(), "NSKernAttributeName constant"),
        ("_NSStrikethroughStyleAttributeName", foundation_constants::nsstrikethrough_style_attribute_name_addr(), "NSStrikethroughStyleAttributeName constant"),
        ("_NSUnderlineStyleAttributeName", foundation_constants::nsunderline_style_attribute_name_addr(), "NSUnderlineStyleAttributeName constant"),
        ("_NSShadowAttributeName", foundation_constants::nsshadow_attribute_name_addr(), "NSShadowAttributeName constant"),
        ("_NSLinkAttributeName", foundation_constants::nslink_attribute_name_addr(), "NSLinkAttributeName constant"),
        ("_NSBaselineOffsetAttributeName", foundation_constants::nsbaseline_offset_attribute_name_addr(), "NSBaselineOffsetAttributeName constant"),
        ("_NSAttachmentAttributeName", foundation_constants::nsattachment_attribute_name_addr(), "NSAttachmentAttributeName constant"),
        ("_NSSuperscriptAttributeName", foundation_constants::nssuperscript_attribute_name_addr(), "NSSuperscriptAttributeName constant"),
        ("_NSStrikethroughColorAttributeName", foundation_constants::nsstrikethrough_color_attribute_name_addr(), "NSStrikethroughColorAttributeName constant"),
        ("_NSUnderlineColorAttributeName", foundation_constants::nsunderline_color_attribute_name_addr(), "NSUnderlineColorAttributeName constant"),
    );

    // NSAccessibility constants
    register_shims!(table,
        ("_NSAccessibilityRoleAttribute", foundation_constants::nsaccessibility_role_attribute_addr(), "NSAccessibilityRoleAttribute constant"),
        ("_NSAccessibilityRoleDescriptionAttribute", foundation_constants::nsaccessibility_role_description_attribute_addr(), "NSAccessibilityRoleDescriptionAttribute constant"),
        ("_NSAccessibilityTitleAttribute", foundation_constants::nsaccessibility_title_attribute_addr(), "NSAccessibilityTitleAttribute constant"),
        ("_NSAccessibilityValueAttribute", foundation_constants::nsaccessibility_value_attribute_addr(), "NSAccessibilityValueAttribute constant"),
    );

    // NSAppKitVersionNumber data symbol
    register_shims!(table,
        ("_NSAppKitVersionNumber", foundation_constants::nsappkit_version_number_addr(), "NSAppKitVersionNumber version double"),
    );

    // CFError / CFURL resource / SystemVersion shims (sw_vers support)
    register_shims!(table,
        ("_CFErrorCopyDescription", cf_core::shim_cf_error_copy_description as *const () as u64, "CFErrorCopyDescription → generic string"),
        ("_CFURLCreateDataAndPropertiesFromResource", cf_core::shim_cf_url_create_data_and_properties_from_resource as *const () as u64, "CFURLCreateDataAndPropertiesFromResource → read file URL"),
        ("__CFCopySupplementalVersionDictionary", cf_core::shim_cf_copy_supplemental_version_dictionary as *const () as u64, "_CFCopySupplementalVersionDictionary → synthetic macOS version"),
    );

    // SystemVersion key constants (sw_vers)
    register_shims!(table,
        ("__kCFSystemVersionProductNameKey", foundation_constants::kcf_system_version_product_name_key_addr(), "_kCFSystemVersionProductNameKey → ProductName"),
        ("__kCFSystemVersionProductVersionKey", foundation_constants::kcf_system_version_product_version_key_addr(), "_kCFSystemVersionProductVersionKey → ProductVersion"),
        ("__kCFSystemVersionBuildVersionKey", foundation_constants::kcf_system_version_build_version_key_addr(), "_kCFSystemVersionBuildVersionKey → ProductBuildVersion"),
        ("__kCFSystemVersionProductVersionExtraKey", foundation_constants::kcf_system_version_product_version_extra_key_addr(), "_kCFSystemVersionProductVersionExtraKey → ProductVersionExtra"),
    );

    // NSEvent tracking run loop mode
    register_shims!(table,
        ("_NSEventTrackingRunLoopMode", nsrunloop::nsevent_tracking_run_loop_mode_addr(), "NSEventTrackingRunLoopMode constant"),
        ("_NSModalPanelRunLoopMode", nsrunloop::nsmodal_panel_run_loop_mode_addr(), "NSModalPanelRunLoopMode constant"),
    );

    // CGContextGetCurrentContext — returns thread-local current CGContext handle.
    // Registered here (not in cg-shims) because the thread-local state
    // lives in cf-lite's NSGraphicsContext.
    register_shims!(table,
        ("_CGContextGetCurrentContext", shim_cg_context_get_current_context as *const () as u64, "CGContextGetCurrentContext (thread-local via NSGraphicsContext)"),
        ("_UIGraphicsGetCurrentContext", shim_cg_context_get_current_context as *const () as u64, "UIGraphicsGetCurrentContext (alias for CGContextGetCurrentContext)"),
    );

    // NSGraphicsContext / NSBezierPath class data symbols
    register_shims!(table,
        ("_OBJC_CLASS_$_NSGraphicsContext", nsobj::nsgraphicscontext_class_addr(), "NSGraphicsContext class data symbol"),
        ("_OBJC_METACLASS_$_NSGraphicsContext", nsobj::nsgraphicscontext_metaclass_addr(), "NSGraphicsContext metaclass data symbol"),
        ("_OBJC_CLASS_$_NSBezierPath", nsobj::nsbezierpath_class_addr(), "NSBezierPath class data symbol"),
        ("_OBJC_METACLASS_$_NSBezierPath", nsobj::nsbezierpath_metaclass_addr(), "NSBezierPath metaclass data symbol"),
    );
}

use crate::nsgraphicscontext;

extern "C" fn shim_cg_context_get_current_context() -> u64 {
    nsgraphicscontext::current_cg_context_handle()
}
