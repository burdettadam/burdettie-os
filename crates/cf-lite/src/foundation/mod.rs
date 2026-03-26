mod cfconstant_string;
mod register;
mod shim_classes;
mod shim_core;

pub(crate) use cfconstant_string::cfconstant_string_class_addr;

use libsystem_shims::table::ShimTable;

/// Install Foundation/CF-lite shims into a `ShimTable`.
pub struct FoundationShims;

impl FoundationShims {
    pub fn install(table: &mut ShimTable) {
        shim_core::install_core_shims(table);
        shim_classes::install_class_symbols(table);
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

    pub fn register_nsconstantintegernumber_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsconstantintegernumber_class(rt);
    }

    pub fn register_nsdictionary_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsdictionary_class(rt);
    }

    pub fn register_nsconstantdictionary_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsconstantdictionary_class(rt);
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

    pub fn register_nsnotification_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsnotification_class(rt);
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

    pub fn register_nsmethodsignature_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmethodsignature_class(rt);
    }

    pub fn register_nsinvocation_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsinvocation_class(rt);
    }

    pub fn register_nsregularexpression_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsregularexpression_class(rt);
    }

    pub fn register_nstextcheckingresult_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstextcheckingresult_class(rt);
    }

    pub fn register_nspredicate_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nspredicate_class(rt);
    }

    pub fn register_nsattributedstring_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsattributedstring_class(rt);
    }

    pub fn register_nsmutableattributedstring_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutableattributedstring_class(rt);
    }

    pub fn register_nscache_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscache_class(rt);
    }

    pub fn register_nsmaptable_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmaptable_class(rt);
    }

    pub fn register_nshashtable_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nshashtable_class(rt);
    }

    pub fn register_nsoperation_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsoperation_class(rt);
    }

    pub fn register_nsblockoperation_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsblockoperation_class(rt);
    }

    pub fn register_nsoperationqueue_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsoperationqueue_class(rt);
    }

    pub fn register_nsjsonserialization_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsjsonserialization_class(rt);
    }

    pub fn register_nspropertylistserialization_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nspropertylistserialization_class(rt);
    }

    // ── AppKit classes (Phase C5) ─────────────────────────────────

    pub fn register_nsresponder_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsresponder_class(rt);
    }

    pub fn register_nsapplication_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsapplication_class(rt);
    }

    pub fn register_nsrunloop_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsrunloop_class(rt);
    }

    pub fn register_nswindow_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nswindow_class(rt);
    }

    pub fn register_nsview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsview_class(rt);
    }

    pub fn register_nsevent_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsevent_class(rt);
    }

    pub fn register_nsworkspace_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsworkspace_class(rt);
    }

    pub fn register_nsscreen_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsscreen_class(rt);
    }

    pub fn register_nscolor_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscolor_class(rt);
    }

    pub fn register_nsfont_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsfont_class(rt);
    }

    pub fn register_nsimage_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsimage_class(rt);
    }

    /// Register tagged pointer class table entries.
    ///
    /// On arm64, tagged pointers encode a 3-bit class index in bits 60-62.
    /// Apple's canonical mapping: 2 = NSNumber, 3 = NSString, 6 = NSDate.
    pub fn register_tagged_classes(rt: &mut objc_runtime::ObjcRuntime) {
        use crate::nsobj;
        rt.register_tagged_class(2, nsobj::nsnumber_class_addr());
        rt.register_tagged_class(3, nsobj::nsstring_class_addr());
        rt.register_tagged_class(6, nsobj::nsdate_class_addr());
        log::debug!(
            "FoundationShims: registered tagged pointer classes (NSNumber=2, NSString=3, NSDate=6)"
        );
    }

    /// Register host Foundation ISA addresses as aliases for our shim classes.
    ///
    /// The compiler embeds constant objects (`@[]`, `@{}`, `@(int)`) with ISA
    /// pointers to the host Foundation classes (NSConstantArray, etc.). This
    /// maps those host addresses to our synthetic classes so dispatch works
    /// without falling back to the host `objc_msgSend`.
    #[cfg(target_os = "macos")]
    pub fn register_host_isa_aliases(rt: &mut objc_runtime::ObjcRuntime) {
        let symbols = crate::pal_symbols();

        for (sym_name, target_class) in [
            ("OBJC_CLASS_$_NSConstantArray", "NSArray"),
            ("OBJC_CLASS_$_NSConstantDictionary", "NSConstantDictionary"),
            ("OBJC_CLASS_$_NSConstantIntegerNumber", "NSConstantIntegerNumber"),
            ("OBJC_CLASS_$_NSException", "NSException"),
        ] {
            if let Some(addr) = symbols.resolve(sym_name) {
                rt.register_isa_alias(addr as u64, target_class);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn register_host_isa_aliases(_rt: &mut objc_runtime::ObjcRuntime) {}
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
        assert!(table.resolve("_OBJC_CLASS_$_NSMethodSignature").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSMethodSignature")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSInvocation").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSInvocation").is_some());
        // 141 base + 22 AppKit class symbols + 3 AppKit data symbols = 166 on macOS
        #[cfg(target_os = "macos")]
        assert_eq!(table.len(), 172);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(table.len(), 162);
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
