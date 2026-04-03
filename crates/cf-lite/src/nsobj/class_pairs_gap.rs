//! Gap-analysis class/metaclass pair declarations.
//!
//! Phase C8, C9, and C10: NSMutableData, NSTask, NSStream, NSURLSession, etc.

use std::cell::UnsafeCell;

use super::{
    SyncArray, nsobject_class_addr, nsobject_metaclass_addr,
    nsdata_class_addr, nsdata_metaclass_addr,
    nsmutableset_class_addr, nsmutableset_metaclass_addr,
    nscharacterset_class_addr, nscharacterset_metaclass_addr,
};

// ── Gap-analysis classes (Phase C8) ─────────────────────────────

define_class_pair!(child NSMUTABLEDATA_CLASS_DATA, NSMUTABLEDATA_METACLASS_DATA, NSMUTABLEDATA_INIT,
    ensure: ensure_nsmutabledata_init,
    class_addr: nsmutabledata_class_addr,
    meta_addr: nsmutabledata_metaclass_addr,
    parent_class: nsdata_class_addr,
    parent_meta: nsdata_metaclass_addr);

define_class_pair!(child NSFILEHANDLE_CLASS_DATA, NSFILEHANDLE_METACLASS_DATA, NSFILEHANDLE_INIT,
    ensure: ensure_nsfilehandle_init,
    class_addr: nsfilehandle_class_addr,
    meta_addr: nsfilehandle_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSSCANNER_CLASS_DATA, NSSCANNER_METACLASS_DATA, NSSCANNER_INIT,
    ensure: ensure_nsscanner_init,
    class_addr: nsscanner_class_addr,
    meta_addr: nsscanner_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSORDEREDSET_CLASS_DATA, NSORDEREDSET_METACLASS_DATA, NSORDEREDSET_INIT,
    ensure: ensure_nsorderedset_init,
    class_addr: nsorderedset_class_addr,
    meta_addr: nsorderedset_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLEORDEREDSET_CLASS_DATA, NSMUTABLEORDEREDSET_METACLASS_DATA, NSMUTABLEORDEREDSET_INIT,
    ensure: ensure_nsmutableorderedset_init,
    class_addr: nsmutableorderedset_class_addr,
    meta_addr: nsmutableorderedset_metaclass_addr,
    parent_class: nsorderedset_class_addr,
    parent_meta: nsorderedset_metaclass_addr);

define_class_pair!(child NSCOUNTEDSET_CLASS_DATA, NSCOUNTEDSET_METACLASS_DATA, NSCOUNTEDSET_INIT,
    ensure: ensure_nscountedset_init,
    class_addr: nscountedset_class_addr,
    meta_addr: nscountedset_metaclass_addr,
    parent_class: nsmutableset_class_addr,
    parent_meta: nsmutableset_metaclass_addr);

define_class_pair!(child NSMUTABLECHARACTERSET_CLASS_DATA, NSMUTABLECHARACTERSET_METACLASS_DATA, NSMUTABLECHARACTERSET_INIT,
    ensure: ensure_nsmutablecharacterset_init,
    class_addr: nsmutablecharacterset_class_addr,
    meta_addr: nsmutablecharacterset_metaclass_addr,
    parent_class: nscharacterset_class_addr,
    parent_meta: nscharacterset_metaclass_addr);

// ── Gap-analysis Phase C9 ───────────────────────────────────────

define_class_pair!(child NSURLCOMPONENTS_CLASS_DATA, NSURLCOMPONENTS_METACLASS_DATA, NSURLCOMPONENTS_INIT,
    ensure: ensure_nsurlcomponents_init,
    class_addr: nsurlcomponents_class_addr,
    meta_addr: nsurlcomponents_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSURLQUERYITEM_CLASS_DATA, NSURLQUERYITEM_METACLASS_DATA, NSURLQUERYITEM_INIT,
    ensure: ensure_nsurlqueryitem_init,
    class_addr: nsurlqueryitem_class_addr,
    meta_addr: nsurlqueryitem_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSBYTECOUNTFORMATTER_CLASS_DATA, NSBYTECOUNTFORMATTER_METACLASS_DATA, NSBYTECOUNTFORMATTER_INIT,
    ensure: ensure_nsbytecountformatter_init,
    class_addr: nsbytecountformatter_class_addr,
    meta_addr: nsbytecountformatter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSISO8601DATEFORMATTER_CLASS_DATA, NSISO8601DATEFORMATTER_METACLASS_DATA, NSISO8601DATEFORMATTER_INIT,
    ensure: ensure_nsiso8601dateformatter_init,
    class_addr: nsiso8601dateformatter_class_addr,
    meta_addr: nsiso8601dateformatter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPROGRESS_CLASS_DATA, NSPROGRESS_METACLASS_DATA, NSPROGRESS_INIT,
    ensure: ensure_nsprogress_init,
    class_addr: nsprogress_class_addr,
    meta_addr: nsprogress_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSHOST_CLASS_DATA, NSHOST_METACLASS_DATA, NSHOST_INIT,
    ensure: ensure_nshost_init,
    class_addr: nshost_class_addr,
    meta_addr: nshost_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPIPE_CLASS_DATA, NSPIPE_METACLASS_DATA, NSPIPE_INIT,
    ensure: ensure_nspipe_init,
    class_addr: nspipe_class_addr,
    meta_addr: nspipe_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSURLREQUEST_CLASS_DATA, NSURLREQUEST_METACLASS_DATA, NSURLREQUEST_INIT,
    ensure: ensure_nsurlrequest_init,
    class_addr: nsurlrequest_class_addr,
    meta_addr: nsurlrequest_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLEURLREQUEST_CLASS_DATA, NSMUTABLEURLREQUEST_METACLASS_DATA, NSMUTABLEURLREQUEST_INIT,
    ensure: ensure_nsmutableurlrequest_init,
    class_addr: nsmutableurlrequest_class_addr,
    meta_addr: nsmutableurlrequest_metaclass_addr,
    parent_class: nsurlrequest_class_addr,
    parent_meta: nsurlrequest_metaclass_addr);

define_class_pair!(child NSURLRESPONSE_CLASS_DATA, NSURLRESPONSE_METACLASS_DATA, NSURLRESPONSE_INIT,
    ensure: ensure_nsurlresponse_init,
    class_addr: nsurlresponse_class_addr,
    meta_addr: nsurlresponse_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSHTTPURLRESPONSE_CLASS_DATA, NSHTTPURLRESPONSE_METACLASS_DATA, NSHTTPURLRESPONSE_INIT,
    ensure: ensure_nshttpurlresponse_init,
    class_addr: nshttpurlresponse_class_addr,
    meta_addr: nshttpurlresponse_metaclass_addr,
    parent_class: nsurlresponse_class_addr,
    parent_meta: nsurlresponse_metaclass_addr);

define_class_pair!(child NSTASK_CLASS_DATA, NSTASK_METACLASS_DATA, NSTASK_INIT,
    ensure: ensure_nstask_init,
    class_addr: nstask_class_addr,
    meta_addr: nstask_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child PKBUNDLECOMPONENTVERSION_CLASS_DATA, PKBUNDLECOMPONENTVERSION_METACLASS_DATA, PKBUNDLECOMPONENTVERSION_INIT,
    ensure: ensure_pkbundlecomponentversion_init,
    class_addr: pkbundlecomponentversion_class_addr,
    meta_addr: pkbundlecomponentversion_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── Gap-analysis Phase C10 ──────────────────────────────────────

define_class_pair!(child NSSTREAM_CLASS_DATA, NSSTREAM_METACLASS_DATA, NSSTREAM_INIT,
    ensure: ensure_nsstream_init,
    class_addr: nsstream_class_addr,
    meta_addr: nsstream_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSINPUTSTREAM_CLASS_DATA, NSINPUTSTREAM_METACLASS_DATA, NSINPUTSTREAM_INIT,
    ensure: ensure_nsinputstream_init,
    class_addr: nsinputstream_class_addr,
    meta_addr: nsinputstream_metaclass_addr,
    parent_class: nsstream_class_addr,
    parent_meta: nsstream_metaclass_addr);

define_class_pair!(child NSOUTPUTSTREAM_CLASS_DATA, NSOUTPUTSTREAM_METACLASS_DATA, NSOUTPUTSTREAM_INIT,
    ensure: ensure_nsoutputstream_init,
    class_addr: nsoutputstream_class_addr,
    meta_addr: nsoutputstream_metaclass_addr,
    parent_class: nsstream_class_addr,
    parent_meta: nsstream_metaclass_addr);

define_class_pair!(child NSXMLPARSER_CLASS_DATA, NSXMLPARSER_METACLASS_DATA, NSXMLPARSER_INIT,
    ensure: ensure_nsxmlparser_init,
    class_addr: nsxmlparser_class_addr,
    meta_addr: nsxmlparser_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSURLSESSION_CLASS_DATA, NSURLSESSION_METACLASS_DATA, NSURLSESSION_INIT,
    ensure: ensure_nsurlsession_init,
    class_addr: nsurlsession_class_addr,
    meta_addr: nsurlsession_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSURLSESSIONCONFIGURATION_CLASS_DATA, NSURLSESSIONCONFIGURATION_METACLASS_DATA, NSURLSESSIONCONFIGURATION_INIT,
    ensure: ensure_nsurlsessionconfiguration_init,
    class_addr: nsurlsessionconfiguration_class_addr,
    meta_addr: nsurlsessionconfiguration_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSURLSESSIONDATATASK_CLASS_DATA, NSURLSESSIONDATATASK_METACLASS_DATA, NSURLSESSIONDATATASK_INIT,
    ensure: ensure_nsurlsessiondatatask_init,
    class_addr: nsurlsessiondatatask_class_addr,
    meta_addr: nsurlsessiondatatask_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── Gap-analysis Phase C11 ─────────────────────────────────────

define_class_pair!(child NSCODER_CLASS_DATA, NSCODER_METACLASS_DATA, NSCODER_INIT,
    ensure: ensure_nscoder_init,
    class_addr: nscoder_class_addr,
    meta_addr: nscoder_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSKEYEDARCHIVER_CLASS_DATA, NSKEYEDARCHIVER_METACLASS_DATA, NSKEYEDARCHIVER_INIT,
    ensure: ensure_nskeyedarchiver_init,
    class_addr: nskeyedarchiver_class_addr,
    meta_addr: nskeyedarchiver_metaclass_addr,
    parent_class: nscoder_class_addr,
    parent_meta: nscoder_metaclass_addr);

define_class_pair!(child NSKEYEDUNARCHIVER_CLASS_DATA, NSKEYEDUNARCHIVER_METACLASS_DATA, NSKEYEDUNARCHIVER_INIT,
    ensure: ensure_nskeyedunarchiver_init,
    class_addr: nskeyedunarchiver_class_addr,
    meta_addr: nskeyedunarchiver_metaclass_addr,
    parent_class: nscoder_class_addr,
    parent_meta: nscoder_metaclass_addr);

// ── Gap-analysis Phase C12: credential and cookie storage ──────

define_class_pair!(child NSURLCREDENTIAL_CLASS_DATA, NSURLCREDENTIAL_METACLASS_DATA, NSURLCREDENTIAL_INIT,
    ensure: ensure_nsurlcredential_init,
    class_addr: nsurlcredential_class_addr,
    meta_addr: nsurlcredential_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSURLCREDENTIALSTORAGE_CLASS_DATA, NSURLCREDENTIALSTORAGE_METACLASS_DATA, NSURLCREDENTIALSTORAGE_INIT,
    ensure: ensure_nsurlcredentialstorage_init,
    class_addr: nsurlcredentialstorage_class_addr,
    meta_addr: nsurlcredentialstorage_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSHTTPCOOKIE_CLASS_DATA, NSHTTPCOOKIE_METACLASS_DATA, NSHTTPCOOKIE_INIT,
    ensure: ensure_nshttpcookie_init,
    class_addr: nshttpcookie_class_addr,
    meta_addr: nshttpcookie_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSHTTPCOOKIESTORAGE_CLASS_DATA, NSHTTPCOOKIESTORAGE_METACLASS_DATA, NSHTTPCOOKIESTORAGE_INIT,
    ensure: ensure_nshttpcookiestorage_init,
    class_addr: nshttpcookiestorage_class_addr,
    meta_addr: nshttpcookiestorage_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSAUTORELEASEPOOL_CLASS_DATA, NSAUTORELEASEPOOL_METACLASS_DATA, NSAUTORELEASEPOOL_INIT,
    ensure: ensure_nsautoreleasepool_init,
    class_addr: nsautoreleasepool_class_addr,
    meta_addr: nsautoreleasepool_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── XCStringsParser.framework classes ───────────────────────────

define_class_pair!(child XCSTRINGSDATAENTRY_CLASS_DATA, XCSTRINGSDATAENTRY_METACLASS_DATA, XCSTRINGSDATAENTRY_INIT,
    ensure: ensure_xcstringsdataentry_init,
    class_addr: xcstringsdataentry_class_addr,
    meta_addr: xcstringsdataentry_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child XCSTRINGSDATATABLE_CLASS_DATA, XCSTRINGSDATATABLE_METACLASS_DATA, XCSTRINGSDATATABLE_INIT,
    ensure: ensure_xcstringsdatatable_init,
    class_addr: xcstringsdatatable_class_addr,
    meta_addr: xcstringsdatatable_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child XCSTRINGSDATAFILE_CLASS_DATA, XCSTRINGSDATAFILE_METACLASS_DATA, XCSTRINGSDATAFILE_INIT,
    ensure: ensure_xcstringsdatafile_init,
    class_addr: xcstringsdatafile_class_addr,
    meta_addr: xcstringsdatafile_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSUnit / NSMeasurement ───────────────────────────────────────

define_class_pair!(child NSUNIT_CLASS_DATA, NSUNIT_METACLASS_DATA, NSUNIT_INIT,
    ensure: ensure_nsunit_init,
    class_addr: nsunit_class_addr,
    meta_addr: nsunit_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSDIMENSION_CLASS_DATA, NSDIMENSION_METACLASS_DATA, NSDIMENSION_INIT,
    ensure: ensure_nsdimension_init,
    class_addr: nsdimension_class_addr,
    meta_addr: nsdimension_metaclass_addr,
    parent_class: nsunit_class_addr,
    parent_meta: nsunit_metaclass_addr);

define_class_pair!(child NSMEASUREMENT_CLASS_DATA, NSMEASUREMENT_METACLASS_DATA, NSMEASUREMENT_INIT,
    ensure: ensure_nsmeasurement_init,
    class_addr: nsmeasurement_class_addr,
    meta_addr: nsmeasurement_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSUNITLENGTH_CLASS_DATA, NSUNITLENGTH_METACLASS_DATA, NSUNITLENGTH_INIT,
    ensure: ensure_nsunitlength_init,
    class_addr: nsunitlength_class_addr,
    meta_addr: nsunitlength_metaclass_addr,
    parent_class: nsdimension_class_addr,
    parent_meta: nsdimension_metaclass_addr);

define_class_pair!(child NSUNITMASS_CLASS_DATA, NSUNITMASS_METACLASS_DATA, NSUNITMASS_INIT,
    ensure: ensure_nsunitmass_init,
    class_addr: nsunitmass_class_addr,
    meta_addr: nsunitmass_metaclass_addr,
    parent_class: nsdimension_class_addr,
    parent_meta: nsdimension_metaclass_addr);

define_class_pair!(child NSUNITTEMPERATURE_CLASS_DATA, NSUNITTEMPERATURE_METACLASS_DATA, NSUNITTEMPERATURE_INIT,
    ensure: ensure_nsunittemperature_init,
    class_addr: nsunittemperature_class_addr,
    meta_addr: nsunittemperature_metaclass_addr,
    parent_class: nsdimension_class_addr,
    parent_meta: nsdimension_metaclass_addr);

define_class_pair!(child NSUNITVOLUME_CLASS_DATA, NSUNITVOLUME_METACLASS_DATA, NSUNITVOLUME_INIT,
    ensure: ensure_nsunitvolume_init,
    class_addr: nsunitvolume_class_addr,
    meta_addr: nsunitvolume_metaclass_addr,
    parent_class: nsdimension_class_addr,
    parent_meta: nsdimension_metaclass_addr);

define_class_pair!(child NSUNITANGLE_CLASS_DATA, NSUNITANGLE_METACLASS_DATA, NSUNITANGLE_INIT,
    ensure: ensure_nsunitangle_init,
    class_addr: nsunitangle_class_addr,
    meta_addr: nsunitangle_metaclass_addr,
    parent_class: nsdimension_class_addr,
    parent_meta: nsdimension_metaclass_addr);

define_class_pair!(child NSUNITSPEED_CLASS_DATA, NSUNITSPEED_METACLASS_DATA, NSUNITSPEED_INIT,
    ensure: ensure_nsunitspeed_init,
    class_addr: nsunitspeed_class_addr,
    meta_addr: nsunitspeed_metaclass_addr,
    parent_class: nsdimension_class_addr,
    parent_meta: nsdimension_metaclass_addr);

define_class_pair!(child NSUNITDURATION_CLASS_DATA, NSUNITDURATION_METACLASS_DATA, NSUNITDURATION_INIT,
    ensure: ensure_nsunitduration_init,
    class_addr: nsunitduration_class_addr,
    meta_addr: nsunitduration_metaclass_addr,
    parent_class: nsdimension_class_addr,
    parent_meta: nsdimension_metaclass_addr);

define_class_pair!(child NSMEASUREMENTFORMATTER_CLASS_DATA, NSMEASUREMENTFORMATTER_METACLASS_DATA, NSMEASUREMENTFORMATTER_INIT,
    ensure: ensure_nsmeasurementformatter_init,
    class_addr: nsmeasurementformatter_class_addr,
    meta_addr: nsmeasurementformatter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── CoreData ──────────────────────────────────────────────────────────────────

define_class_pair!(child NSMANAGEDOBJECTCONTEXT_CLASS_DATA, NSMANAGEDOBJECTCONTEXT_METACLASS_DATA, NSMANAGEDOBJECTCONTEXT_INIT,
    ensure: ensure_nsmanagedobjectcontext_init,
    class_addr: nsmanagedobjectcontext_class_addr,
    meta_addr: nsmanagedobjectcontext_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPERSISTENTSTORECOORDINATOR_CLASS_DATA, NSPERSISTENTSTORECOORDINATOR_METACLASS_DATA, NSPERSISTENTSTORECOORDINATOR_INIT,
    ensure: ensure_nspersistentstorecoordinator_init,
    class_addr: nspersistentstorecoordinator_class_addr,
    meta_addr: nspersistentstorecoordinator_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSFETCHREQUEST_CLASS_DATA, NSFETCHREQUEST_METACLASS_DATA, NSFETCHREQUEST_INIT,
    ensure: ensure_nsfetchrequest_init,
    class_addr: nsfetchrequest_class_addr,
    meta_addr: nsfetchrequest_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMANAGEDOBJECT_CLASS_DATA, NSMANAGEDOBJECT_METACLASS_DATA, NSMANAGEDOBJECT_INIT,
    ensure: ensure_nsmanagedobject_init,
    class_addr: nsmanagedobject_class_addr,
    meta_addr: nsmanagedobject_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSENTITYDESCRIPTION_CLASS_DATA, NSENTITYDESCRIPTION_METACLASS_DATA, NSENTITYDESCRIPTION_INIT,
    ensure: ensure_nsentitydescription_init,
    class_addr: nsentitydescription_class_addr,
    meta_addr: nsentitydescription_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMANAGEDOBJECTMODEL_CLASS_DATA, NSMANAGEDOBJECTMODEL_METACLASS_DATA, NSMANAGEDOBJECTMODEL_INIT,
    ensure: ensure_nsmanagedobjectmodel_init,
    class_addr: nsmanagedobjectmodel_class_addr,
    meta_addr: nsmanagedobjectmodel_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── AVFoundation ──────────────────────────────────────────────────────────────

define_class_pair!(child AVAUDIOPLAYER_CLASS_DATA, AVAUDIOPLAYER_METACLASS_DATA, AVAUDIOPLAYER_INIT,
    ensure: ensure_avaudioplayer_init,
    class_addr: avaudioplayer_class_addr,
    meta_addr: avaudioplayer_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child AVAUDIORECORDER_CLASS_DATA, AVAUDIORECORDER_METACLASS_DATA, AVAUDIORECORDER_INIT,
    ensure: ensure_avaudiorecorder_init,
    class_addr: avaudiorecorder_class_addr,
    meta_addr: avaudiorecorder_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child AVAUDIOENGINE_CLASS_DATA, AVAUDIOENGINE_METACLASS_DATA, AVAUDIOENGINE_INIT,
    ensure: ensure_avaudioengine_init,
    class_addr: avaudioengine_class_addr,
    meta_addr: avaudioengine_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child AVCAPTUREDEVICE_CLASS_DATA, AVCAPTUREDEVICE_METACLASS_DATA, AVCAPTUREDEVICE_INIT,
    ensure: ensure_avcapturedevice_init,
    class_addr: avcapturedevice_class_addr,
    meta_addr: avcapturedevice_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child AVCAPTURESESSION_CLASS_DATA, AVCAPTURESESSION_METACLASS_DATA, AVCAPTURESESSION_INIT,
    ensure: ensure_avcapturesession_init,
    class_addr: avcapturesession_class_addr,
    meta_addr: avcapturesession_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child AVPLAYER_CLASS_DATA, AVPLAYER_METACLASS_DATA, AVPLAYER_INIT,
    ensure: ensure_avplayer_init,
    class_addr: avplayer_class_addr,
    meta_addr: avplayer_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── CoreLocation ──────────────────────────────────────────────────────────────

define_class_pair!(child CLLOCATIONMANAGER_CLASS_DATA, CLLOCATIONMANAGER_METACLASS_DATA, CLLOCATIONMANAGER_INIT,
    ensure: ensure_cllocationmanager_init,
    class_addr: cllocationmanager_class_addr,
    meta_addr: cllocationmanager_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child CLLOCATION_CLASS_DATA, CLLOCATION_METACLASS_DATA, CLLOCATION_INIT,
    ensure: ensure_cllocation_init,
    class_addr: cllocation_class_addr,
    meta_addr: cllocation_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child CLGEOCODER_CLASS_DATA, CLGEOCODER_METACLASS_DATA, CLGEOCODER_INIT,
    ensure: ensure_clgeocoder_init,
    class_addr: clgeocoder_class_addr,
    meta_addr: clgeocoder_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── CoreBluetooth ─────────────────────────────────────────────────────────────

define_class_pair!(child CBCENTRALMANAGER_CLASS_DATA, CBCENTRALMANAGER_METACLASS_DATA, CBCENTRALMANAGER_INIT,
    ensure: ensure_cbcentralmanager_init,
    class_addr: cbcentralmanager_class_addr,
    meta_addr: cbcentralmanager_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child CBPERIPHERALMANAGER_CLASS_DATA, CBPERIPHERALMANAGER_METACLASS_DATA, CBPERIPHERALMANAGER_INIT,
    ensure: ensure_cbperipheralmanager_init,
    class_addr: cbperipheralmanager_class_addr,
    meta_addr: cbperipheralmanager_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);
