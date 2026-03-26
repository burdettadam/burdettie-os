use crate::config::RuntimeConfig;
use crate::tracer::{EventKind, RuntimeTracer};

/// Register all synthetic Foundation classes in the ObjC runtime.
///
/// This registers NSObject, NSString, NSArray, NSDictionary, and all
/// other Foundation types that cf-lite provides shim implementations for.
pub(crate) fn register_foundation_classes(
    objc_rt: &mut objc_runtime::ObjcRuntime,
    config: &RuntimeConfig,
    tracer: &RuntimeTracer,
) {
    // Initialize cf-lite's platform provider for Foundation shims
    cf_lite::set_platform(config.platform.clone());

    cf_lite::FoundationShims::register_nsobject_class(objc_rt);
    cf_lite::FoundationShims::register_nsstring_proper_class(objc_rt);
    cf_lite::FoundationShims::register_nsstring_class(objc_rt);
    cf_lite::FoundationShims::register_nsarray_class(objc_rt);
    cf_lite::FoundationShims::register_nsnumber_class(objc_rt);
    cf_lite::FoundationShims::register_nsconstantintegernumber_class(objc_rt);
    cf_lite::FoundationShims::register_nsdictionary_class(objc_rt);
    cf_lite::FoundationShims::register_nsconstantdictionary_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutablearray_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutabledictionary_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutablestring_class(objc_rt);
    cf_lite::FoundationShims::register_nsdata_class(objc_rt);
    cf_lite::FoundationShims::register_nsprocessinfo_class(objc_rt);
    cf_lite::FoundationShims::register_nserror_class(objc_rt);
    cf_lite::FoundationShims::register_nsbundle_class(objc_rt);
    cf_lite::FoundationShims::register_nsfilemanager_class(objc_rt);
    cf_lite::FoundationShims::register_nsexception_class(objc_rt);
    cf_lite::FoundationShims::register_nsenumerator_class(objc_rt);
    cf_lite::FoundationShims::register_nsdate_class(objc_rt);
    cf_lite::FoundationShims::register_nsuserdefaults_class(objc_rt);
    cf_lite::FoundationShims::register_nsset_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutableset_class(objc_rt);
    cf_lite::FoundationShims::register_nsurl_class(objc_rt);
    cf_lite::FoundationShims::register_nsnull_class(objc_rt);
    cf_lite::FoundationShims::register_nsdateformatter_class(objc_rt);
    cf_lite::FoundationShims::register_nscharacterset_class(objc_rt);
    cf_lite::FoundationShims::register_nsnotificationcenter_class(objc_rt);
    cf_lite::FoundationShims::register_nsnotification_class(objc_rt);
    cf_lite::FoundationShims::register_nsvalue_class(objc_rt);
    cf_lite::FoundationShims::register_nsuuid_class(objc_rt);
    cf_lite::FoundationShims::register_nslocale_class(objc_rt);
    cf_lite::FoundationShims::register_nstimezone_class(objc_rt);
    cf_lite::FoundationShims::register_nscalendar_class(objc_rt);
    cf_lite::FoundationShims::register_nsnumberformatter_class(objc_rt);
    cf_lite::FoundationShims::register_nsindexset_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutableindexset_class(objc_rt);
    cf_lite::FoundationShims::register_nssortdescriptor_class(objc_rt);
    cf_lite::FoundationShims::register_nstimer_class(objc_rt);
    cf_lite::FoundationShims::register_nsthread_class(objc_rt);
    cf_lite::FoundationShims::register_nslock_class(objc_rt);
    cf_lite::FoundationShims::register_nsrecursivelock_class(objc_rt);
    cf_lite::FoundationShims::register_nscondition_class(objc_rt);
    cf_lite::FoundationShims::register_nsconditionlock_class(objc_rt);
    cf_lite::FoundationShims::register_nsmethodsignature_class(objc_rt);
    cf_lite::FoundationShims::register_nsinvocation_class(objc_rt);
    cf_lite::FoundationShims::register_nsregularexpression_class(objc_rt);
    cf_lite::FoundationShims::register_nstextcheckingresult_class(objc_rt);
    cf_lite::FoundationShims::register_nspredicate_class(objc_rt);
    cf_lite::FoundationShims::register_nsattributedstring_class(objc_rt);
    cf_lite::FoundationShims::register_nsmutableattributedstring_class(objc_rt);
    cf_lite::FoundationShims::register_nscache_class(objc_rt);
    cf_lite::FoundationShims::register_nsmaptable_class(objc_rt);
    cf_lite::FoundationShims::register_nshashtable_class(objc_rt);
    cf_lite::FoundationShims::register_nsoperation_class(objc_rt);
    cf_lite::FoundationShims::register_nsblockoperation_class(objc_rt);
    cf_lite::FoundationShims::register_nsoperationqueue_class(objc_rt);
    cf_lite::FoundationShims::register_nsjsonserialization_class(objc_rt);
    cf_lite::FoundationShims::register_nspropertylistserialization_class(objc_rt);

    // AppKit classes (Phase C5)
    cf_lite::FoundationShims::register_nsresponder_class(objc_rt);
    cf_lite::FoundationShims::register_nsapplication_class(objc_rt);
    cf_lite::FoundationShims::register_nsrunloop_class(objc_rt);
    cf_lite::FoundationShims::register_nswindow_class(objc_rt);
    cf_lite::FoundationShims::register_nsview_class(objc_rt);
    cf_lite::FoundationShims::register_nsevent_class(objc_rt);
    cf_lite::FoundationShims::register_nsworkspace_class(objc_rt);
    cf_lite::FoundationShims::register_nsscreen_class(objc_rt);
    cf_lite::FoundationShims::register_nscolor_class(objc_rt);
    cf_lite::FoundationShims::register_nsfont_class(objc_rt);
    cf_lite::FoundationShims::register_nsimage_class(objc_rt);
    tracer.record(
        EventKind::ClassRegistration,
        "65 synthetic Foundation/AppKit classes registered",
    );

    // Register host ISA aliases so constant objects
    // (@[], @{}, @(int)) dispatch through our shims.
    cf_lite::FoundationShims::register_host_isa_aliases(objc_rt);

    // Register tagged pointer class table (NSNumber=2, NSString=3, NSDate=6).
    cf_lite::FoundationShims::register_tagged_classes(objc_rt);
}
