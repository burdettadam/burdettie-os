//! Foundation class/metaclass pair declarations (child classes).

use std::cell::UnsafeCell;

use super::{SyncArray, nsobject_class_addr, nsobject_metaclass_addr};

define_class_pair!(child NSSTRING_CLASS_DATA, NSSTRING_METACLASS_DATA, NSSTRING_INIT,
    ensure: ensure_nsstring_init,
    class_addr: nsstring_class_addr,
    meta_addr: nsstring_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSARRAY_CLASS_DATA, NSARRAY_METACLASS_DATA, NSARRAY_INIT,
    ensure: ensure_nsarray_init,
    class_addr: nsarray_class_addr,
    meta_addr: nsarray_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNUMBER_CLASS_DATA, NSNUMBER_METACLASS_DATA, NSNUMBER_INIT,
    ensure: ensure_nsnumber_init,
    class_addr: nsnumber_class_addr,
    meta_addr: nsnumber_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSDICTIONARY_CLASS_DATA, NSDICTIONARY_METACLASS_DATA, NSDICTIONARY_INIT,
    ensure: ensure_nsdictionary_init,
    class_addr: nsdictionary_class_addr,
    meta_addr: nsdictionary_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLEARRAY_CLASS_DATA, NSMUTABLEARRAY_METACLASS_DATA, NSMUTABLEARRAY_INIT,
    ensure: ensure_nsmutablearray_init,
    class_addr: nsmutablearray_class_addr,
    meta_addr: nsmutablearray_metaclass_addr,
    parent_class: nsarray_class_addr,
    parent_meta: nsarray_metaclass_addr);

define_class_pair!(child NSMUTABLEDICTIONARY_CLASS_DATA, NSMUTABLEDICTIONARY_METACLASS_DATA, NSMUTABLEDICTIONARY_INIT,
    ensure: ensure_nsmutabledictionary_init,
    class_addr: nsmutabledictionary_class_addr,
    meta_addr: nsmutabledictionary_metaclass_addr,
    parent_class: nsdictionary_class_addr,
    parent_meta: nsdictionary_metaclass_addr);

define_class_pair!(child NSMUTABLESTRING_CLASS_DATA, NSMUTABLESTRING_METACLASS_DATA, NSMUTABLESTRING_INIT,
    ensure: ensure_nsmutablestring_init,
    class_addr: nsmutablestring_class_addr,
    meta_addr: nsmutablestring_metaclass_addr,
    parent_class: nsstring_class_addr,
    parent_meta: nsstring_metaclass_addr);

define_class_pair!(child NSDATA_CLASS_DATA, NSDATA_METACLASS_DATA, NSDATA_INIT,
    ensure: ensure_nsdata_init,
    class_addr: nsdata_class_addr,
    meta_addr: nsdata_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPROCESSINFO_CLASS_DATA, NSPROCESSINFO_METACLASS_DATA, NSPROCESSINFO_INIT,
    ensure: ensure_nsprocessinfo_init,
    class_addr: nsprocessinfo_class_addr,
    meta_addr: nsprocessinfo_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSERROR_CLASS_DATA, NSERROR_METACLASS_DATA, NSERROR_INIT,
    ensure: ensure_nserror_init,
    class_addr: nserror_class_addr,
    meta_addr: nserror_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSBUNDLE_CLASS_DATA, NSBUNDLE_METACLASS_DATA, NSBUNDLE_INIT,
    ensure: ensure_nsbundle_init,
    class_addr: nsbundle_class_addr,
    meta_addr: nsbundle_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSFILEMANAGER_CLASS_DATA, NSFILEMANAGER_METACLASS_DATA, NSFILEMANAGER_INIT,
    ensure: ensure_nsfilemanager_init,
    class_addr: nsfilemanager_class_addr,
    meta_addr: nsfilemanager_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSEXCEPTION_CLASS_DATA, NSEXCEPTION_METACLASS_DATA, NSEXCEPTION_INIT,
    ensure: ensure_nsexception_init,
    class_addr: nsexception_class_addr,
    meta_addr: nsexception_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSENUMERATOR_CLASS_DATA, NSENUMERATOR_METACLASS_DATA, NSENUMERATOR_INIT,
    ensure: ensure_nsenumerator_init,
    class_addr: nsenumerator_class_addr,
    meta_addr: nsenumerator_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSDATE_CLASS_DATA, NSDATE_METACLASS_DATA, NSDATE_INIT,
    ensure: ensure_nsdate_init,
    class_addr: nsdate_class_addr,
    meta_addr: nsdate_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSUSERDEFAULTS_CLASS_DATA, NSUSERDEFAULTS_METACLASS_DATA, NSUSERDEFAULTS_INIT,
    ensure: ensure_nsuserdefaults_init,
    class_addr: nsuserdefaults_class_addr,
    meta_addr: nsuserdefaults_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSSET_CLASS_DATA, NSSET_METACLASS_DATA, NSSET_INIT,
    ensure: ensure_nsset_init,
    class_addr: nsset_class_addr,
    meta_addr: nsset_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLESET_CLASS_DATA, NSMUTABLESET_METACLASS_DATA, NSMUTABLESET_INIT,
    ensure: ensure_nsmutableset_init,
    class_addr: nsmutableset_class_addr,
    meta_addr: nsmutableset_metaclass_addr,
    parent_class: nsset_class_addr,
    parent_meta: nsset_metaclass_addr);

define_class_pair!(child NSURL_CLASS_DATA, NSURL_METACLASS_DATA, NSURL_INIT,
    ensure: ensure_nsurl_init,
    class_addr: nsurl_class_addr,
    meta_addr: nsurl_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNULL_CLASS_DATA, NSNULL_METACLASS_DATA, NSNULL_INIT,
    ensure: ensure_nsnull_init,
    class_addr: nsnull_class_addr,
    meta_addr: nsnull_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSDATEFORMATTER_CLASS_DATA, NSDATEFORMATTER_METACLASS_DATA, NSDATEFORMATTER_INIT,
    ensure: ensure_nsdateformatter_init,
    class_addr: nsdateformatter_class_addr,
    meta_addr: nsdateformatter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCHARACTERSET_CLASS_DATA, NSCHARACTERSET_METACLASS_DATA, NSCHARACTERSET_INIT,
    ensure: ensure_nscharacterset_init,
    class_addr: nscharacterset_class_addr,
    meta_addr: nscharacterset_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNOTIFICATIONCENTER_CLASS_DATA, NSNOTIFICATIONCENTER_METACLASS_DATA, NSNOTIFICATIONCENTER_INIT,
    ensure: ensure_nsnotificationcenter_init,
    class_addr: nsnotificationcenter_class_addr,
    meta_addr: nsnotificationcenter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNOTIFICATION_CLASS_DATA, NSNOTIFICATION_METACLASS_DATA, NSNOTIFICATION_INIT,
    ensure: ensure_nsnotification_init,
    class_addr: nsnotification_class_addr,
    meta_addr: nsnotification_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSVALUE_CLASS_DATA, NSVALUE_METACLASS_DATA, NSVALUE_INIT,
    ensure: ensure_nsvalue_init,
    class_addr: nsvalue_class_addr,
    meta_addr: nsvalue_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSUUID_CLASS_DATA, NSUUID_METACLASS_DATA, NSUUID_INIT,
    ensure: ensure_nsuuid_init,
    class_addr: nsuuid_class_addr,
    meta_addr: nsuuid_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSLOCALE_CLASS_DATA, NSLOCALE_METACLASS_DATA, NSLOCALE_INIT,
    ensure: ensure_nslocale_init,
    class_addr: nslocale_class_addr,
    meta_addr: nslocale_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSTIMEZONE_CLASS_DATA, NSTIMEZONE_METACLASS_DATA, NSTIMEZONE_INIT,
    ensure: ensure_nstimezone_init,
    class_addr: nstimezone_class_addr,
    meta_addr: nstimezone_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCALENDAR_CLASS_DATA, NSCALENDAR_METACLASS_DATA, NSCALENDAR_INIT,
    ensure: ensure_nscalendar_init,
    class_addr: nscalendar_class_addr,
    meta_addr: nscalendar_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNUMBERFORMATTER_CLASS_DATA, NSNUMBERFORMATTER_METACLASS_DATA, NSNUMBERFORMATTER_INIT,
    ensure: ensure_nsnumberformatter_init,
    class_addr: nsnumberformatter_class_addr,
    meta_addr: nsnumberformatter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSINDEXSET_CLASS_DATA, NSINDEXSET_METACLASS_DATA, NSINDEXSET_INIT,
    ensure: ensure_nsindexset_init,
    class_addr: nsindexset_class_addr,
    meta_addr: nsindexset_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLEINDEXSET_CLASS_DATA, NSMUTABLEINDEXSET_METACLASS_DATA, NSMUTABLEINDEXSET_INIT,
    ensure: ensure_nsmutableindexset_init,
    class_addr: nsmutableindexset_class_addr,
    meta_addr: nsmutableindexset_metaclass_addr,
    parent_class: nsindexset_class_addr,
    parent_meta: nsindexset_metaclass_addr);

define_class_pair!(child NSSORTDESCRIPTOR_CLASS_DATA, NSSORTDESCRIPTOR_METACLASS_DATA, NSSORTDESCRIPTOR_INIT,
    ensure: ensure_nssortdescriptor_init,
    class_addr: nssortdescriptor_class_addr,
    meta_addr: nssortdescriptor_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSTIMER_CLASS_DATA, NSTIMER_METACLASS_DATA, NSTIMER_INIT,
    ensure: ensure_nstimer_init,
    class_addr: nstimer_class_addr,
    meta_addr: nstimer_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSTHREAD_CLASS_DATA, NSTHREAD_METACLASS_DATA, NSTHREAD_INIT,
    ensure: ensure_nsthread_init,
    class_addr: nsthread_class_addr,
    meta_addr: nsthread_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSLOCK_CLASS_DATA, NSLOCK_METACLASS_DATA, NSLOCK_INIT,
    ensure: ensure_nslock_init,
    class_addr: nslock_class_addr,
    meta_addr: nslock_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSRECURSIVELOCK_CLASS_DATA, NSRECURSIVELOCK_METACLASS_DATA, NSRECURSIVELOCK_INIT,
    ensure: ensure_nsrecursivelock_init,
    class_addr: nsrecursivelock_class_addr,
    meta_addr: nsrecursivelock_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCONDITION_CLASS_DATA, NSCONDITION_METACLASS_DATA, NSCONDITION_INIT,
    ensure: ensure_nscondition_init,
    class_addr: nscondition_class_addr,
    meta_addr: nscondition_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCONDITIONLOCK_CLASS_DATA, NSCONDITIONLOCK_METACLASS_DATA, NSCONDITIONLOCK_INIT,
    ensure: ensure_nsconditionlock_init,
    class_addr: nsconditionlock_class_addr,
    meta_addr: nsconditionlock_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMETHODSIGNATURE_CLASS_DATA, NSMETHODSIGNATURE_METACLASS_DATA, NSMETHODSIGNATURE_INIT,
    ensure: ensure_nsmethodsignature_init,
    class_addr: nsmethodsignature_class_addr,
    meta_addr: nsmethodsignature_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSINVOCATION_CLASS_DATA, NSINVOCATION_METACLASS_DATA, NSINVOCATION_INIT,
    ensure: ensure_nsinvocation_init,
    class_addr: nsinvocation_class_addr,
    meta_addr: nsinvocation_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── Phase B6 Foundation classes ────────────────────────────────────

define_class_pair!(child NSREGULAREXPRESSION_CLASS_DATA, NSREGULAREXPRESSION_METACLASS_DATA, NSREGULAREXPRESSION_INIT,
    ensure: ensure_nsregularexpression_init,
    class_addr: nsregularexpression_class_addr,
    meta_addr: nsregularexpression_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPREDICATE_CLASS_DATA, NSPREDICATE_METACLASS_DATA, NSPREDICATE_INIT,
    ensure: ensure_nspredicate_init,
    class_addr: nspredicate_class_addr,
    meta_addr: nspredicate_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSATTRIBUTEDSTRING_CLASS_DATA, NSATTRIBUTEDSTRING_METACLASS_DATA, NSATTRIBUTEDSTRING_INIT,
    ensure: ensure_nsattributedstring_init,
    class_addr: nsattributedstring_class_addr,
    meta_addr: nsattributedstring_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLEATTRIBUTEDSTRING_CLASS_DATA, NSMUTABLEATTRIBUTEDSTRING_METACLASS_DATA, NSMUTABLEATTRIBUTEDSTRING_INIT,
    ensure: ensure_nsmutableattributedstring_init,
    class_addr: nsmutableattributedstring_class_addr,
    meta_addr: nsmutableattributedstring_metaclass_addr,
    parent_class: nsattributedstring_class_addr,
    parent_meta: nsattributedstring_metaclass_addr);

define_class_pair!(child NSCACHE_CLASS_DATA, NSCACHE_METACLASS_DATA, NSCACHE_INIT,
    ensure: ensure_nscache_init,
    class_addr: nscache_class_addr,
    meta_addr: nscache_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMAPTABLE_CLASS_DATA, NSMAPTABLE_METACLASS_DATA, NSMAPTABLE_INIT,
    ensure: ensure_nsmaptable_init,
    class_addr: nsmaptable_class_addr,
    meta_addr: nsmaptable_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSHASHTABLE_CLASS_DATA, NSHASHTABLE_METACLASS_DATA, NSHASHTABLE_INIT,
    ensure: ensure_nshashtable_init,
    class_addr: nshashtable_class_addr,
    meta_addr: nshashtable_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSOPERATIONQUEUE_CLASS_DATA, NSOPERATIONQUEUE_METACLASS_DATA, NSOPERATIONQUEUE_INIT,
    ensure: ensure_nsoperationqueue_init,
    class_addr: nsoperationqueue_class_addr,
    meta_addr: nsoperationqueue_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSOPERATION_CLASS_DATA, NSOPERATION_METACLASS_DATA, NSOPERATION_INIT,
    ensure: ensure_nsoperation_init,
    class_addr: nsoperation_class_addr,
    meta_addr: nsoperation_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSBLOCKOPERATION_CLASS_DATA, NSBLOCKOPERATION_METACLASS_DATA, NSBLOCKOPERATION_INIT,
    ensure: ensure_nsblockoperation_init,
    class_addr: nsblockoperation_class_addr,
    meta_addr: nsblockoperation_metaclass_addr,
    parent_class: nsoperation_class_addr,
    parent_meta: nsoperation_metaclass_addr);

define_class_pair!(child NSJSONSERIALIZATION_CLASS_DATA, NSJSONSERIALIZATION_METACLASS_DATA, NSJSONSERIALIZATION_INIT,
    ensure: ensure_nsjsonserialization_init,
    class_addr: nsjsonserialization_class_addr,
    meta_addr: nsjsonserialization_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPROPERTYLISTSERIALIZATION_CLASS_DATA, NSPROPERTYLISTSERIALIZATION_METACLASS_DATA, NSPROPERTYLISTSERIALIZATION_INIT,
    ensure: ensure_nspropertylistserialization_init,
    class_addr: nspropertylistserialization_class_addr,
    meta_addr: nspropertylistserialization_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── AppKit class pairs (Phase C5) ──────────────────────────────────

define_class_pair!(child NSRESPONDER_CLASS_DATA, NSRESPONDER_METACLASS_DATA, NSRESPONDER_INIT,
    ensure: ensure_nsresponder_init,
    class_addr: nsresponder_class_addr,
    meta_addr: nsresponder_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSAPPLICATION_CLASS_DATA, NSAPPLICATION_METACLASS_DATA, NSAPPLICATION_INIT,
    ensure: ensure_nsapplication_init,
    class_addr: nsapplication_class_addr,
    meta_addr: nsapplication_metaclass_addr,
    parent_class: nsresponder_class_addr,
    parent_meta: nsresponder_metaclass_addr);

define_class_pair!(child NSRUNLOOP_CLASS_DATA, NSRUNLOOP_METACLASS_DATA, NSRUNLOOP_INIT,
    ensure: ensure_nsrunloop_init,
    class_addr: nsrunloop_class_addr,
    meta_addr: nsrunloop_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSWINDOW_CLASS_DATA, NSWINDOW_METACLASS_DATA, NSWINDOW_INIT,
    ensure: ensure_nswindow_init,
    class_addr: nswindow_class_addr,
    meta_addr: nswindow_metaclass_addr,
    parent_class: nsresponder_class_addr,
    parent_meta: nsresponder_metaclass_addr);

define_class_pair!(child NSVIEW_CLASS_DATA, NSVIEW_METACLASS_DATA, NSVIEW_INIT,
    ensure: ensure_nsview_init,
    class_addr: nsview_class_addr,
    meta_addr: nsview_metaclass_addr,
    parent_class: nsresponder_class_addr,
    parent_meta: nsresponder_metaclass_addr);

define_class_pair!(child NSEVENT_CLASS_DATA, NSEVENT_METACLASS_DATA, NSEVENT_INIT,
    ensure: ensure_nsevent_init,
    class_addr: nsevent_class_addr,
    meta_addr: nsevent_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSWORKSPACE_CLASS_DATA, NSWORKSPACE_METACLASS_DATA, NSWORKSPACE_INIT,
    ensure: ensure_nsworkspace_init,
    class_addr: nsworkspace_class_addr,
    meta_addr: nsworkspace_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSSCREEN_CLASS_DATA, NSSCREEN_METACLASS_DATA, NSSCREEN_INIT,
    ensure: ensure_nsscreen_init,
    class_addr: nsscreen_class_addr,
    meta_addr: nsscreen_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCOLOR_CLASS_DATA, NSCOLOR_METACLASS_DATA, NSCOLOR_INIT,
    ensure: ensure_nscolor_init,
    class_addr: nscolor_class_addr,
    meta_addr: nscolor_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSFONT_CLASS_DATA, NSFONT_METACLASS_DATA, NSFONT_INIT,
    ensure: ensure_nsfont_init,
    class_addr: nsfont_class_addr,
    meta_addr: nsfont_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSIMAGE_CLASS_DATA, NSIMAGE_METACLASS_DATA, NSIMAGE_INIT,
    ensure: ensure_nsimage_init,
    class_addr: nsimage_class_addr,
    meta_addr: nsimage_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSConstant* classes (compiler-embedded ObjC literals) ──────

define_class_pair!(child NSCONSTANTDICTIONARY_CLASS_DATA, NSCONSTANTDICTIONARY_METACLASS_DATA, NSCONSTANTDICTIONARY_INIT,
    ensure: ensure_nsconstantdictionary_init,
    class_addr: nsconstantdictionary_class_addr,
    meta_addr: nsconstantdictionary_metaclass_addr,
    parent_class: nsdictionary_class_addr,
    parent_meta: nsdictionary_metaclass_addr);

define_class_pair!(child NSCONSTANTINTEGERNUMBER_CLASS_DATA, NSCONSTANTINTEGERNUMBER_METACLASS_DATA, NSCONSTANTINTEGERNUMBER_INIT,
    ensure: ensure_nsconstantintegernumber_init,
    class_addr: nsconstantintegernumber_class_addr,
    meta_addr: nsconstantintegernumber_metaclass_addr,
    parent_class: nsnumber_class_addr,
    parent_meta: nsnumber_metaclass_addr);

// ── NSTextCheckingResult (regex match results) ─────────────────

define_class_pair!(child NSTEXTCHECKINGRESULT_CLASS_DATA, NSTEXTCHECKINGRESULT_METACLASS_DATA, NSTEXTCHECKINGRESULT_INIT,
    ensure: ensure_nstextcheckingresult_init,
    class_addr: nstextcheckingresult_class_addr,
    meta_addr: nstextcheckingresult_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── AppKit Phase C7 classes ────────────────────────────────────

define_class_pair!(child NSAPPEARANCE_CLASS_DATA, NSAPPEARANCE_METACLASS_DATA, NSAPPEARANCE_INIT,
    ensure: ensure_nsappearance_init,
    class_addr: nsappearance_class_addr,
    meta_addr: nsappearance_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSUSERNOTIFICATIONCENTER_CLASS_DATA, NSUSERNOTIFICATIONCENTER_METACLASS_DATA, NSUSERNOTIFICATIONCENTER_INIT,
    ensure: ensure_nsusernotificationcenter_init,
    class_addr: nsusernotificationcenter_class_addr,
    meta_addr: nsusernotificationcenter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMENU_CLASS_DATA, NSMENU_METACLASS_DATA, NSMENU_INIT,
    ensure: ensure_nsmenu_init,
    class_addr: nsmenu_class_addr,
    meta_addr: nsmenu_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMENUITEM_CLASS_DATA, NSMENUITEM_METACLASS_DATA, NSMENUITEM_INIT,
    ensure: ensure_nsmenuitem_init,
    class_addr: nsmenuitem_class_addr,
    meta_addr: nsmenuitem_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNIB_CLASS_DATA, NSNIB_METACLASS_DATA, NSNIB_INIT,
    ensure: ensure_nsnib_init,
    class_addr: nsnib_class_addr,
    meta_addr: nsnib_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);
