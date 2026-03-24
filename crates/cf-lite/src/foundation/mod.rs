mod register;

use std::cell::UnsafeCell;

use libsystem_shims::table::ShimTable;

use crate::cfstring;
use crate::nslog;
use crate::nsobj;

/// A wrapper around a byte array that can be safely shared across threads.
#[repr(transparent)]
struct SyncArray(UnsafeCell<[u8; 40]>);

// SAFETY: Mutation is guarded by `std::sync::Once`; after init the data is read-only.
unsafe impl Sync for SyncArray {}

/// A fake class struct for `__CFConstantStringClassReference`.
///
/// When the compiler sees `@"hello"`, it emits a `__CFConstantString`
/// struct whose ISA field references `___CFConstantStringClassReference`.
/// This must be a valid address so ISA reads don't segfault.
///
/// Layout: 40 bytes of zeros (same shape as objc_class). The ISA
/// (offset 0) points to itself — constant strings don't need method
/// dispatch through our runtime, only readable data access.
static CFCONSTANT_STRING_CLASS: SyncArray = SyncArray(UnsafeCell::new([0u8; 40]));

static CFSTRING_INIT: std::sync::Once = std::sync::Once::new();

pub(crate) fn cfconstant_string_class_addr() -> u64 {
    CFSTRING_INIT.call_once(|| {
        // SAFETY: Mutation is guarded by Once; the data is write-once.
        unsafe {
            let ptr = CFCONSTANT_STRING_CLASS.0.get() as *mut u8;
            let self_addr = ptr as u64;
            std::ptr::copy_nonoverlapping(self_addr.to_le_bytes().as_ptr(), ptr, 8);
        }
    });
    CFCONSTANT_STRING_CLASS.0.get() as u64
}

/// Install Foundation/CF-lite shims into a `ShimTable`.
pub struct FoundationShims;

impl FoundationShims {
    pub fn install(table: &mut ShimTable) {
        // NSLog
        table.register(
            "_NSLog",
            nslog::nslog_trampoline_addr(),
            "NSLog → fprintf(stderr, ...)",
        );

        // ___CFConstantStringClassReference
        table.register(
            "___CFConstantStringClassReference",
            cfconstant_string_class_addr(),
            "CFConstantString class reference (data symbol)",
        );

        // NSObject class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSObject",
            nsobj::nsobject_class_addr(),
            "NSObject class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSObject",
            nsobj::nsobject_metaclass_addr(),
            "NSObject metaclass data symbol",
        );

        // CFString functions
        table.register(
            "_CFStringCreateWithCString",
            cfstring::shim_cf_string_create_with_cstring as u64,
            "CFStringCreateWithCString shim",
        );
        table.register(
            "_CFStringGetLength",
            cfstring::shim_cf_string_get_length as u64,
            "CFStringGetLength shim",
        );
        table.register(
            "_CFStringGetCStringPtr",
            cfstring::shim_cf_string_get_cstring_ptr as u64,
            "CFStringGetCStringPtr shim",
        );
        table.register(
            "_kCFStringEncodingUTF8",
            cfstring::kcf_string_encoding_utf8_addr(),
            "kCFStringEncodingUTF8 data symbol",
        );
        table.register(
            "_CFRelease",
            cfstring::shim_cf_release as u64,
            "CFRelease shim",
        );
        table.register(
            "_kCFAllocatorDefault",
            cfstring::kcf_allocator_default_addr(),
            "kCFAllocatorDefault data symbol",
        );

        // Foundation class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSString",
            nsobj::nsstring_class_addr(),
            "NSString class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSString",
            nsobj::nsstring_metaclass_addr(),
            "NSString metaclass data symbol",
        );
        table.register(
            "_OBJC_CLASS_$_NSArray",
            nsobj::nsarray_class_addr(),
            "NSArray class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSArray",
            nsobj::nsarray_metaclass_addr(),
            "NSArray metaclass data symbol",
        );
        table.register(
            "_OBJC_CLASS_$_NSNumber",
            nsobj::nsnumber_class_addr(),
            "NSNumber class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSNumber",
            nsobj::nsnumber_metaclass_addr(),
            "NSNumber metaclass data symbol",
        );
        table.register(
            "_OBJC_CLASS_$_NSDictionary",
            nsobj::nsdictionary_class_addr(),
            "NSDictionary class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSDictionary",
            nsobj::nsdictionary_metaclass_addr(),
            "NSDictionary metaclass data symbol",
        );
        table.register(
            "_OBJC_CLASS_$_NSMutableArray",
            nsobj::nsmutablearray_class_addr(),
            "NSMutableArray class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSMutableArray",
            nsobj::nsmutablearray_metaclass_addr(),
            "NSMutableArray metaclass data symbol",
        );
        table.register(
            "_OBJC_CLASS_$_NSMutableDictionary",
            nsobj::nsmutabledictionary_class_addr(),
            "NSMutableDictionary class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSMutableDictionary",
            nsobj::nsmutabledictionary_metaclass_addr(),
            "NSMutableDictionary metaclass data symbol",
        );
        table.register(
            "_OBJC_CLASS_$_NSMutableString",
            nsobj::nsmutablestring_class_addr(),
            "NSMutableString class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSMutableString",
            nsobj::nsmutablestring_metaclass_addr(),
            "NSMutableString metaclass data symbol",
        );

        // NSData class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSData",
            nsobj::nsdata_class_addr(),
            "NSData class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSData",
            nsobj::nsdata_metaclass_addr(),
            "NSData metaclass data symbol",
        );

        // NSProcessInfo class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSProcessInfo",
            nsobj::nsprocessinfo_class_addr(),
            "NSProcessInfo class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSProcessInfo",
            nsobj::nsprocessinfo_metaclass_addr(),
            "NSProcessInfo metaclass data symbol",
        );

        // NSError class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSError",
            nsobj::nserror_class_addr(),
            "NSError class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSError",
            nsobj::nserror_metaclass_addr(),
            "NSError metaclass data symbol",
        );

        // NSBundle class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSBundle",
            nsobj::nsbundle_class_addr(),
            "NSBundle class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSBundle",
            nsobj::nsbundle_metaclass_addr(),
            "NSBundle metaclass data symbol",
        );

        // NSFileManager class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSFileManager",
            nsobj::nsfilemanager_class_addr(),
            "NSFileManager class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSFileManager",
            nsobj::nsfilemanager_metaclass_addr(),
            "NSFileManager metaclass data symbol",
        );

        // NSException class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSException",
            nsobj::nsexception_class_addr(),
            "NSException class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSException",
            nsobj::nsexception_metaclass_addr(),
            "NSException metaclass data symbol",
        );

        // NSEnumerator class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSEnumerator",
            nsobj::nsenumerator_class_addr(),
            "NSEnumerator class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSEnumerator",
            nsobj::nsenumerator_metaclass_addr(),
            "NSEnumerator metaclass data symbol",
        );

        // NSDate class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSDate",
            nsobj::nsdate_class_addr(),
            "NSDate class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSDate",
            nsobj::nsdate_metaclass_addr(),
            "NSDate metaclass data symbol",
        );

        // NSUserDefaults class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSUserDefaults",
            nsobj::nsuserdefaults_class_addr(),
            "NSUserDefaults class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSUserDefaults",
            nsobj::nsuserdefaults_metaclass_addr(),
            "NSUserDefaults metaclass data symbol",
        );

        // NSSet class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSSet",
            nsobj::nsset_class_addr(),
            "NSSet class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSSet",
            nsobj::nsset_metaclass_addr(),
            "NSSet metaclass data symbol",
        );

        // NSMutableSet class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSMutableSet",
            nsobj::nsmutableset_class_addr(),
            "NSMutableSet class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSMutableSet",
            nsobj::nsmutableset_metaclass_addr(),
            "NSMutableSet metaclass data symbol",
        );

        // NSURL class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSURL",
            nsobj::nsurl_class_addr(),
            "NSURL class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSURL",
            nsobj::nsurl_metaclass_addr(),
            "NSURL metaclass data symbol",
        );

        // NSNull class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSNull",
            nsobj::nsnull_class_addr(),
            "NSNull class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSNull",
            nsobj::nsnull_metaclass_addr(),
            "NSNull metaclass data symbol",
        );

        // NSDateFormatter class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSDateFormatter",
            nsobj::nsdateformatter_class_addr(),
            "NSDateFormatter class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSDateFormatter",
            nsobj::nsdateformatter_metaclass_addr(),
            "NSDateFormatter metaclass data symbol",
        );

        // NSCharacterSet class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSCharacterSet",
            nsobj::nscharacterset_class_addr(),
            "NSCharacterSet class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSCharacterSet",
            nsobj::nscharacterset_metaclass_addr(),
            "NSCharacterSet metaclass data symbol",
        );

        // NSNotificationCenter class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSNotificationCenter",
            nsobj::nsnotificationcenter_class_addr(),
            "NSNotificationCenter class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSNotificationCenter",
            nsobj::nsnotificationcenter_metaclass_addr(),
            "NSNotificationCenter metaclass data symbol",
        );

        // NSValue class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSValue",
            nsobj::nsvalue_class_addr(),
            "NSValue class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSValue",
            nsobj::nsvalue_metaclass_addr(),
            "NSValue metaclass data symbol",
        );

        // NSUUID class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSUUID",
            nsobj::nsuuid_class_addr(),
            "NSUUID class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSUUID",
            nsobj::nsuuid_metaclass_addr(),
            "NSUUID metaclass data symbol",
        );

        // NSLocale class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSLocale",
            nsobj::nslocale_class_addr(),
            "NSLocale class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSLocale",
            nsobj::nslocale_metaclass_addr(),
            "NSLocale metaclass data symbol",
        );

        // NSTimeZone class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSTimeZone",
            nsobj::nstimezone_class_addr(),
            "NSTimeZone class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSTimeZone",
            nsobj::nstimezone_metaclass_addr(),
            "NSTimeZone metaclass data symbol",
        );

        // NSCalendar class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSCalendar",
            nsobj::nscalendar_class_addr(),
            "NSCalendar class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSCalendar",
            nsobj::nscalendar_metaclass_addr(),
            "NSCalendar metaclass data symbol",
        );

        // NSNumberFormatter class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSNumberFormatter",
            nsobj::nsnumberformatter_class_addr(),
            "NSNumberFormatter class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSNumberFormatter",
            nsobj::nsnumberformatter_metaclass_addr(),
            "NSNumberFormatter metaclass data symbol",
        );

        // NSIndexSet class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSIndexSet",
            nsobj::nsindexset_class_addr(),
            "NSIndexSet class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSIndexSet",
            nsobj::nsindexset_metaclass_addr(),
            "NSIndexSet metaclass data symbol",
        );

        // NSMutableIndexSet class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSMutableIndexSet",
            nsobj::nsmutableindexset_class_addr(),
            "NSMutableIndexSet class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSMutableIndexSet",
            nsobj::nsmutableindexset_metaclass_addr(),
            "NSMutableIndexSet metaclass data symbol",
        );

        // NSSortDescriptor class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSSortDescriptor",
            nsobj::nssortdescriptor_class_addr(),
            "NSSortDescriptor class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSSortDescriptor",
            nsobj::nssortdescriptor_metaclass_addr(),
            "NSSortDescriptor metaclass data symbol",
        );

        // NSTimer class/metaclass data symbols
        table.register(
            "_OBJC_CLASS_$_NSTimer",
            nsobj::nstimer_class_addr(),
            "NSTimer class data symbol",
        );
        table.register(
            "_OBJC_METACLASS_$_NSTimer",
            nsobj::nstimer_metaclass_addr(),
            "NSTimer metaclass data symbol",
        );

        // NSException EHTYPE — used by __objc_personality_v0 for @catch type matching.
        // Must resolve to the host Foundation's EHTYPE so that exceptions thrown
        // via host libobjc's objc_exception_throw can be caught by @catch(NSException *).
        #[cfg(target_os = "macos")]
        {
            // Ensure Foundation is loaded so the EHTYPE symbol is available.
            let fnd = unsafe {
                libc::dlopen(
                    c"/System/Library/Frameworks/Foundation.framework/Foundation".as_ptr(),
                    libc::RTLD_LAZY | libc::RTLD_NOLOAD,
                )
            };
            let handle = if fnd.is_null() {
                unsafe {
                    libc::dlopen(
                        c"/System/Library/Frameworks/Foundation.framework/Foundation".as_ptr(),
                        libc::RTLD_LAZY,
                    )
                }
            } else {
                fnd
            };
            if !handle.is_null() {
                let sym = unsafe { libc::dlsym(handle, c"OBJC_EHTYPE_$_NSException".as_ptr()) };
                if !sym.is_null() {
                    table.register(
                        "_OBJC_EHTYPE_$_NSException",
                        sym as u64,
                        "NSException EHTYPE (host Foundation, for @catch matching)",
                    );
                }
            }
        }

        log::debug!("FoundationShims: registered Foundation/CF shims");
    }

    pub fn register_nsstring_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsstring_class(rt);
    }

    pub fn register_nsstring_proper_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsstring_proper_class(rt);
    }

    pub fn register_nsobject_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsobject_class(rt);
    }

    pub fn register_nsarray_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsarray_class(rt);
    }

    pub fn register_nsnumber_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsnumber_class(rt);
    }

    pub fn register_nsdictionary_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsdictionary_class(rt);
    }

    pub fn register_nsmutablearray_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutablearray_class(rt);
    }

    pub fn register_nsmutabledictionary_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutabledictionary_class(rt);
    }

    pub fn register_nsmutablestring_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutablestring_class(rt);
    }

    pub fn register_nsdata_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsdata_class(rt);
    }

    pub fn register_nsprocessinfo_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsprocessinfo_class(rt);
    }

    pub fn register_nserror_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nserror_class(rt);
    }

    pub fn register_nsbundle_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsbundle_class(rt);
    }

    pub fn register_nsfilemanager_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsfilemanager_class(rt);
    }

    pub fn register_nsexception_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsexception_class(rt);
    }

    pub fn register_nsenumerator_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsenumerator_class(rt);
    }

    pub fn register_nsdate_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsdate_class(rt);
    }

    pub fn register_nsuserdefaults_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsuserdefaults_class(rt);
    }

    pub fn register_nsset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsset_class(rt);
    }

    pub fn register_nsmutableset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutableset_class(rt);
    }

    pub fn register_nsurl_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurl_class(rt);
    }

    pub fn register_nsnull_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsnull_class(rt);
    }

    pub fn register_nsdateformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsdateformatter_class(rt);
    }

    pub fn register_nscharacterset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscharacterset_class(rt);
    }

    pub fn register_nsnotificationcenter_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsnotificationcenter_class(rt);
    }

    pub fn register_nsvalue_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsvalue_class(rt);
    }

    pub fn register_nsuuid_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsuuid_class(rt);
    }

    pub fn register_nslocale_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nslocale_class(rt);
    }

    pub fn register_nstimezone_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstimezone_class(rt);
    }

    pub fn register_nscalendar_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscalendar_class(rt);
    }

    pub fn register_nsnumberformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsnumberformatter_class(rt);
    }

    pub fn register_nsindexset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsindexset_class(rt);
    }

    pub fn register_nsmutableindexset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutableindexset_class(rt);
    }

    pub fn register_nssortdescriptor_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nssortdescriptor_class(rt);
    }

    pub fn register_nstimer_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstimer_class(rt);
    }

    pub fn register_nsthread_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsthread_class(rt);
    }

    pub fn register_nslock_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nslock_class(rt);
    }

    pub fn register_nsrecursivelock_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsrecursivelock_class(rt);
    }

    pub fn register_nscondition_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscondition_class(rt);
    }

    pub fn register_nsconditionlock_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsconditionlock_class(rt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundation_shims_registered() {
        let mut table = ShimTable::new();
        FoundationShims::install(&mut table);
        assert!(table.resolve("_NSLog").is_some());
        assert!(table.resolve("___CFConstantStringClassReference").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSObject").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSObject").is_some());
        assert!(table.resolve("_CFStringCreateWithCString").is_some());
        assert!(table.resolve("_CFStringGetLength").is_some());
        assert!(table.resolve("_CFStringGetCStringPtr").is_some());
        assert!(table.resolve("_kCFStringEncodingUTF8").is_some());
        assert!(table.resolve("_CFRelease").is_some());
        assert!(table.resolve("_kCFAllocatorDefault").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSString").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSString").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSArray").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSArray").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSNumber").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSNumber").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSDictionary").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSDictionary").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableArray").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSMutableArray").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableDictionary").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSMutableDictionary")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableString").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSMutableString").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSData").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSData").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSProcessInfo").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSProcessInfo").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSError").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSError").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSBundle").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSBundle").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSFileManager").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSFileManager").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSException").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSException").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSEnumerator").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSEnumerator").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSDate").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSDate").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSUserDefaults").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSUserDefaults").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSSet").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSSet").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableSet").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSMutableSet").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSURL").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSURL").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSNull").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSNull").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSDateFormatter").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSDateFormatter").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSCharacterSet").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSCharacterSet").is_some());
        assert!(table
            .resolve("_OBJC_CLASS_$_NSNotificationCenter")
            .is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSNotificationCenter")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSValue").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSValue").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSUUID").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSUUID").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSLocale").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSLocale").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSTimeZone").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSTimeZone").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSCalendar").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSCalendar").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSNumberFormatter").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSNumberFormatter")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSIndexSet").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSIndexSet").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableIndexSet").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSMutableIndexSet")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSSortDescriptor").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSSortDescriptor")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSTimer").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSTimer").is_some());
        // 76 base shims + 1 EHTYPE on macOS
        #[cfg(target_os = "macos")]
        assert_eq!(table.len(), 77);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(table.len(), 76);
    }

    #[test]
    fn cfconstant_string_class_nonzero() {
        let addr = cfconstant_string_class_addr();
        assert_ne!(addr, 0);
    }

    #[test]
    fn cfconstant_string_isa_points_to_self() {
        let addr = cfconstant_string_class_addr();
        let isa = unsafe { *(addr as *const u64) };
        assert_eq!(isa, addr);
    }
}
