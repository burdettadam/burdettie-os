use super::register;
use super::FoundationShims;

impl FoundationShims {
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

    pub fn register_nsdistributednotificationcenter_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsdistributednotificationcenter_class(rt);
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

    pub fn register_nsdecimalnumber_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsdecimalnumber_class(rt);
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
}
