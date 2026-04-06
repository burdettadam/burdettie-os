use super::register;
use super::FoundationShims;

impl FoundationShims {
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

    // ── Phase C7 AppKit class registrations ─────────────────────

    pub fn register_nsappearance_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsappearance_class(rt);
    }

    pub fn register_nsusernotificationcenter_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsusernotificationcenter_class(rt);
    }

    pub fn register_nsmenu_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmenu_class(rt);
    }

    pub fn register_nsmenuitem_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmenuitem_class(rt);
    }

    pub fn register_nsnib_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsnib_class(rt);
    }

    // ── Phase C8 AppKit GUI classes ─────────────────────────────

    pub fn register_nsviewcontroller_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsviewcontroller_class(rt);
    }

    pub fn register_nswindowcontroller_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nswindowcontroller_class(rt);
    }

    pub fn register_nscontrol_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscontrol_class(rt);
    }

    pub fn register_nstextfield_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstextfield_class(rt);
    }

    // ── Phase C9 AppKit GUI classes ─────────────────────────────

    pub fn register_nsbutton_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsbutton_class(rt);
    }

    pub fn register_nspopupbutton_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nspopupbutton_class(rt);
    }

    pub fn register_nsalert_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsalert_class(rt);
    }

    pub fn register_nspanel_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nspanel_class(rt);
    }

    pub fn register_nsscrollview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsscrollview_class(rt);
    }

    pub fn register_nsbox_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsbox_class(rt);
    }

    pub fn register_nssplitview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nssplitview_class(rt);
    }

    pub fn register_nsstackview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsstackview_class(rt);
    }

    pub fn register_nstabview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstabview_class(rt);
    }

    pub fn register_nstabviewitem_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstabviewitem_class(rt);
    }

    pub fn register_nstoolbar_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstoolbar_class(rt);
    }

    pub fn register_nstoolbaritem_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstoolbaritem_class(rt);
    }

    #[cfg(target_os = "macos")]
    pub fn register_nssavepanel_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nssavepanel_class(rt);
    }

    #[cfg(target_os = "macos")]
    pub fn register_nsopenpanel_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsopenpanel_class(rt);
    }

    pub fn register_nsclipview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsclipview_class(rt);
    }

    pub fn register_nstextview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstextview_class(rt);
    }

    pub fn register_nstrackingarea_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstrackingarea_class(rt);
    }

    pub fn register_nscursor_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscursor_class(rt);
    }

    pub fn register_nscolorpanel_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscolorpanel_class(rt);
    }

    pub fn register_nsfontpanel_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsfontpanel_class(rt);
    }

    pub fn register_nsstepper_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsstepper_class(rt);
    }

    // ── Phase C10 AppKit widget classes ──────────────────────────

    pub fn register_nsslider_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsslider_class(rt);
    }

    pub fn register_nsprogressindicator_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsprogressindicator_class(rt);
    }

    pub fn register_nsimageview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsimageview_class(rt);
    }

    pub fn register_nssegmentedcontrol_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nssegmentedcontrol_class(rt);
    }

    pub fn register_nscombobox_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscombobox_class(rt);
    }

    pub fn register_nscolorwell_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscolorwell_class(rt);
    }

    pub fn register_nspasteboard_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nspasteboard_class(rt);
    }

    pub fn register_nslayoutconstraint_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nslayoutconstraint_class(rt);
    }

    // ── Gap-analysis classes (Phase C8) ──────────────────────────

    pub fn register_nsmutabledata_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutabledata_class(rt);
    }

    pub fn register_nsfilehandle_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsfilehandle_class(rt);
    }

    pub fn register_nsscanner_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsscanner_class(rt);
    }

    pub fn register_nsorderedset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsorderedset_class(rt);
    }

    pub fn register_nsmutableorderedset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutableorderedset_class(rt);
    }

    pub fn register_nscountedset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscountedset_class(rt);
    }

    pub fn register_nsmutablecharacterset_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutablecharacterset_class(rt);
    }

    // ── Gap-analysis Phase C9 ────────────────────────────────────

    pub fn register_nsurlcomponents_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlcomponents_class(rt);
    }

    pub fn register_nsurlqueryitem_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlqueryitem_class(rt);
    }

    pub fn register_nsbytecountformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsbytecountformatter_class(rt);
    }

    pub fn register_nsiso8601dateformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsiso8601dateformatter_class(rt);
    }

    pub fn register_nsprogress_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsprogress_class(rt);
    }

    pub fn register_nshost_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nshost_class(rt);
    }

    pub fn register_nspipe_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nspipe_class(rt);
    }

    pub fn register_nsurlrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlrequest_class(rt);
    }

    pub fn register_nsmutableurlrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmutableurlrequest_class(rt);
    }

    pub fn register_nsurlresponse_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlresponse_class(rt);
    }

    pub fn register_nshttpurlresponse_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nshttpurlresponse_class(rt);
    }

    pub fn register_nstask_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstask_class(rt);
    }

    pub fn register_pkbundlecomponentversion_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_pkbundlecomponentversion_class(rt);
    }

    // ─── Phase C10 ───
    pub fn register_nsstream_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsstream_class(rt);
    }
    pub fn register_nsinputstream_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsinputstream_class(rt);
    }
    pub fn register_nsoutputstream_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsoutputstream_class(rt);
    }
    pub fn register_nsxmlparser_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsxmlparser_class(rt);
    }
    pub fn register_nsurlsessionconfiguration_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlsessionconfiguration_class(rt);
    }
    pub fn register_nsurlsession_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlsession_class(rt);
    }
    pub fn register_nsurlsessiondatatask_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlsessiondatatask_class(rt);
    }
    pub fn register_nskeyedarchiver_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nskeyedarchiver_class(rt);
    }
    pub fn register_nscoder_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nscoder_class(rt);
    }
    pub fn register_nskeyedunarchiver_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nskeyedunarchiver_class(rt);
    }
    pub fn register_nsurlcredential_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlcredential_class(rt);
    }
    pub fn register_nsurlcredentialstorage_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsurlcredentialstorage_class(rt);
    }
    pub fn register_nshttpcookie_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nshttpcookie_class(rt);
    }
    pub fn register_nshttpcookiestorage_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nshttpcookiestorage_class(rt);
    }

    pub fn register_nsautoreleasepool_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsautoreleasepool_class(rt);
    }

    // NSUnit / NSMeasurement and concrete units
    pub fn register_nsunit_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsunit_class(rt);
    }
    pub fn register_nsdimension_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsdimension_class(rt);
    }
    pub fn register_nsmeasurement_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmeasurement_class(rt);
    }
    pub fn register_nsunitlength_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsunitlength_class(rt);
    }
    pub fn register_nsunitmass_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsunitmass_class(rt);
    }
    pub fn register_nsunittemperature_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsunittemperature_class(rt);
    }
    pub fn register_nsunitvolume_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsunitvolume_class(rt);
    }
    pub fn register_nsunitangle_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsunitangle_class(rt);
    }
    pub fn register_nsunitspeed_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsunitspeed_class(rt);
    }
    pub fn register_nsunitduration_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsunitduration_class(rt);
    }
    pub fn register_nsmeasurementformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmeasurementformatter_class(rt);
    }

    // XCStringsParser.framework classes
    pub fn register_xcstringsdataentry_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_xcstringsdataentry_class(rt);
    }
    pub fn register_xcstringsdatatable_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_xcstringsdatatable_class(rt);
    }
    pub fn register_xcstringsdatafile_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_xcstringsdatafile_class(rt);
    }

    // CoreData stub classes
    pub fn register_nsmanagedobjectcontext_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmanagedobjectcontext_class(rt);
    }
    pub fn register_nspersistentstorecoordinator_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nspersistentstorecoordinator_class(rt);
    }
    pub fn register_nsfetchrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsfetchrequest_class(rt);
    }
    pub fn register_nsmanagedobject_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmanagedobject_class(rt);
    }
    pub fn register_nsentitydescription_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsentitydescription_class(rt);
    }
    pub fn register_nsmanagedobjectmodel_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsmanagedobjectmodel_class(rt);
    }

    // AVFoundation stub classes
    pub fn register_avaudioplayer_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_avaudioplayer_class(rt);
    }
    pub fn register_avaudiorecorder_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_avaudiorecorder_class(rt);
    }
    pub fn register_avaudioengine_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_avaudioengine_class(rt);
    }
    pub fn register_avcapturedevice_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_avcapturedevice_class(rt);
    }
    pub fn register_avcapturesession_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_avcapturesession_class(rt);
    }
    pub fn register_avplayer_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_avplayer_class(rt);
    }

    // CoreLocation + CoreBluetooth stub classes
    pub fn register_cllocationmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_cllocationmanager_class(rt);
    }
    pub fn register_cllocation_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_cllocation_class(rt);
    }
    pub fn register_clgeocoder_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_clgeocoder_class(rt);
    }
    pub fn register_cbcentralmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_cbcentralmanager_class(rt);
    }
    pub fn register_cbperipheralmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_cbperipheralmanager_class(rt);
    }

    // ── Phase C11: Layout Anchors, Layout Guide, VFL, intrinsic content size ───

    pub fn register_nslayoutanchor_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nslayoutanchor_class(rt);
    }

    pub fn register_nslayoutguide_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nslayoutguide_class(rt);
    }

    pub fn register_nsview_anchor_methods(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsview_anchor_methods(rt);
    }

    pub fn register_nswindow_layout_guide_method(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nswindow_layout_guide_method(rt);
    }

    pub fn register_nslayoutconstraint_vfl_method(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nslayoutconstraint_vfl_method(rt);
    }

    // ── Phase C12: Safe area, NSTraitCollection, NSAnimationContext, NSTableView, NSUndoManager ──

    pub fn register_nstraitcollection_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstraitcollection_class(rt);
    }

    pub fn register_nsanimationcontext_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsanimationcontext_class(rt);
    }

    pub fn register_nstableview_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nstableview_class(rt);
    }

    pub fn register_nsundomanager_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsundomanager_class(rt);
    }

    pub fn register_nsview_safe_area_methods(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsview_safe_area_methods(rt);
    }

    // ── Phase C13: CoreAnimation (CALayer, CATransaction, CADisplayLink) ──

    pub fn register_calayer_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_calayer_class(rt);
    }

    pub fn register_catransaction_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_catransaction_class(rt);
    }

    pub fn register_cadisplaylink_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_cadisplaylink_class(rt);
    }

    pub fn register_camediatimingfunction_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_camediatimingfunction_class(rt);
    }

    // ── Phase C14: Drawing (NSGraphicsContext, NSBezierPath) ─────

    pub fn register_nsgraphicscontext_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsgraphicscontext_class(rt);
    }

    pub fn register_nsbezierpath_class(rt: &mut objc_runtime::ObjcRuntime) {
        register::register_nsbezierpath_class(rt);
    }
}
