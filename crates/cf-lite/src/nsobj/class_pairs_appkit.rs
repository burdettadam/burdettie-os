//! AppKit and NSConstant class/metaclass pair declarations.
//!
//! Phase C5 (NSResponder hierarchy), NSConstant compiler classes, and Phase C7 AppKit additions.

use std::cell::UnsafeCell;

use super::{
    SyncArray, nsobject_class_addr, nsobject_metaclass_addr,
    nsdictionary_class_addr, nsdictionary_metaclass_addr,
    nsnumber_class_addr, nsnumber_metaclass_addr,
};

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

// ── AppKit Phase C8 GUI classes ────────────────────────────────

define_class_pair!(child NSVIEWCONTROLLER_CLASS_DATA, NSVIEWCONTROLLER_METACLASS_DATA, NSVIEWCONTROLLER_INIT,
    ensure: ensure_nsviewcontroller_init,
    class_addr: nsviewcontroller_class_addr,
    meta_addr: nsviewcontroller_metaclass_addr,
    parent_class: nsresponder_class_addr,
    parent_meta: nsresponder_metaclass_addr);

define_class_pair!(child NSWINDOWCONTROLLER_CLASS_DATA, NSWINDOWCONTROLLER_METACLASS_DATA, NSWINDOWCONTROLLER_INIT,
    ensure: ensure_nswindowcontroller_init,
    class_addr: nswindowcontroller_class_addr,
    meta_addr: nswindowcontroller_metaclass_addr,
    parent_class: nsresponder_class_addr,
    parent_meta: nsresponder_metaclass_addr);

define_class_pair!(child NSCONTROL_CLASS_DATA, NSCONTROL_METACLASS_DATA, NSCONTROL_INIT,
    ensure: ensure_nscontrol_init,
    class_addr: nscontrol_class_addr,
    meta_addr: nscontrol_metaclass_addr,
    parent_class: nsview_class_addr,
    parent_meta: nsview_metaclass_addr);

define_class_pair!(child NSBUTTON_CLASS_DATA, NSBUTTON_METACLASS_DATA, NSBUTTON_INIT,
    ensure: ensure_nsbutton_init,
    class_addr: nsbutton_class_addr,
    meta_addr: nsbutton_metaclass_addr,
    parent_class: nscontrol_class_addr,
    parent_meta: nscontrol_metaclass_addr);

define_class_pair!(child NSPOPUPBUTTON_CLASS_DATA, NSPOPUPBUTTON_METACLASS_DATA, NSPOPUPBUTTON_INIT,
    ensure: ensure_nspopupbutton_init,
    class_addr: nspopupbutton_class_addr,
    meta_addr: nspopupbutton_metaclass_addr,
    parent_class: nsbutton_class_addr,
    parent_meta: nsbutton_metaclass_addr);

define_class_pair!(child NSTEXTFIELD_CLASS_DATA, NSTEXTFIELD_METACLASS_DATA, NSTEXTFIELD_INIT,
    ensure: ensure_nstextfield_init,
    class_addr: nstextfield_class_addr,
    meta_addr: nstextfield_metaclass_addr,
    parent_class: nscontrol_class_addr,
    parent_meta: nscontrol_metaclass_addr);

define_class_pair!(child NSALERT_CLASS_DATA, NSALERT_METACLASS_DATA, NSALERT_INIT,
    ensure: ensure_nsalert_init,
    class_addr: nsalert_class_addr,
    meta_addr: nsalert_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPANEL_CLASS_DATA, NSPANEL_METACLASS_DATA, NSPANEL_INIT,
    ensure: ensure_nspanel_init,
    class_addr: nspanel_class_addr,
    meta_addr: nspanel_metaclass_addr,
    parent_class: nswindow_class_addr,
    parent_meta: nswindow_metaclass_addr);

// ── AppKit Phase C9 AppKit classes ────────────────────────────

define_class_pair!(child NSSCROLLVIEW_CLASS_DATA, NSSCROLLVIEW_METACLASS_DATA, NSSCROLLVIEW_INIT,
    ensure: ensure_nsscrollview_init,
    class_addr: nsscrollview_class_addr,
    meta_addr: nsscrollview_metaclass_addr,
    parent_class: nsview_class_addr,
    parent_meta: nsview_metaclass_addr);

define_class_pair!(child NSBOX_CLASS_DATA, NSBOX_METACLASS_DATA, NSBOX_INIT,
    ensure: ensure_nsbox_init,
    class_addr: nsbox_class_addr,
    meta_addr: nsbox_metaclass_addr,
    parent_class: nsview_class_addr,
    parent_meta: nsview_metaclass_addr);

define_class_pair!(child NSSPLITVIEW_CLASS_DATA, NSSPLITVIEW_METACLASS_DATA, NSSPLITVIEW_INIT,
    ensure: ensure_nssplitview_init,
    class_addr: nssplitview_class_addr,
    meta_addr: nssplitview_metaclass_addr,
    parent_class: nsview_class_addr,
    parent_meta: nsview_metaclass_addr);

define_class_pair!(child NSSTACKVIEW_CLASS_DATA, NSSTACKVIEW_METACLASS_DATA, NSSTACKVIEW_INIT,
    ensure: ensure_nsstackview_init,
    class_addr: nsstackview_class_addr,
    meta_addr: nsstackview_metaclass_addr,
    parent_class: nsview_class_addr,
    parent_meta: nsview_metaclass_addr);

define_class_pair!(child NSTABVIEW_CLASS_DATA, NSTABVIEW_METACLASS_DATA, NSTABVIEW_INIT,
    ensure: ensure_nstabview_init,
    class_addr: nstabview_class_addr,
    meta_addr: nstabview_metaclass_addr,
    parent_class: nsview_class_addr,
    parent_meta: nsview_metaclass_addr);

define_class_pair!(child NSTABVIEWITEM_CLASS_DATA, NSTABVIEWITEM_METACLASS_DATA, NSTABVIEWITEM_INIT,
    ensure: ensure_nstabviewitem_init,
    class_addr: nstabviewitem_class_addr,
    meta_addr: nstabviewitem_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSTOOLBAR_CLASS_DATA, NSTOOLBAR_METACLASS_DATA, NSTOOLBAR_INIT,
    ensure: ensure_nstoolbar_init,
    class_addr: nstoolbar_class_addr,
    meta_addr: nstoolbar_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSTOOLBARITEM_CLASS_DATA, NSTOOLBARITEM_METACLASS_DATA, NSTOOLBARITEM_INIT,
    ensure: ensure_nstoolbaritem_init,
    class_addr: nstoolbaritem_class_addr,
    meta_addr: nstoolbaritem_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── AppKit Phase C10 classes ───────────────────────────────────

define_class_pair!(child NSSLIDER_CLASS_DATA, NSSLIDER_METACLASS_DATA, NSSLIDER_INIT,
    ensure: ensure_nsslider_init,
    class_addr: nsslider_class_addr,
    meta_addr: nsslider_metaclass_addr,
    parent_class: nscontrol_class_addr,
    parent_meta: nscontrol_metaclass_addr);

define_class_pair!(child NSPROGRESSINDICATOR_CLASS_DATA, NSPROGRESSINDICATOR_METACLASS_DATA, NSPROGRESSINDICATOR_INIT,
    ensure: ensure_nsprogressindicator_init,
    class_addr: nsprogressindicator_class_addr,
    meta_addr: nsprogressindicator_metaclass_addr,
    parent_class: nsview_class_addr,
    parent_meta: nsview_metaclass_addr);

define_class_pair!(child NSIMAGEVIEW_CLASS_DATA, NSIMAGEVIEW_METACLASS_DATA, NSIMAGEVIEW_INIT,
    ensure: ensure_nsimageview_init,
    class_addr: nsimageview_class_addr,
    meta_addr: nsimageview_metaclass_addr,
    parent_class: nscontrol_class_addr,
    parent_meta: nscontrol_metaclass_addr);

define_class_pair!(child NSSEGMENTEDCONTROL_CLASS_DATA, NSSEGMENTEDCONTROL_METACLASS_DATA, NSSEGMENTEDCONTROL_INIT,
    ensure: ensure_nssegmentedcontrol_init,
    class_addr: nssegmentedcontrol_class_addr,
    meta_addr: nssegmentedcontrol_metaclass_addr,
    parent_class: nscontrol_class_addr,
    parent_meta: nscontrol_metaclass_addr);

define_class_pair!(child NSCOMBOBOX_CLASS_DATA, NSCOMBOBOX_METACLASS_DATA, NSCOMBOBOX_INIT,
    ensure: ensure_nscombobox_init,
    class_addr: nscombobox_class_addr,
    meta_addr: nscombobox_metaclass_addr,
    parent_class: nstextfield_class_addr,
    parent_meta: nstextfield_metaclass_addr);

define_class_pair!(child NSCOLORWELL_CLASS_DATA, NSCOLORWELL_METACLASS_DATA, NSCOLORWELL_INIT,
    ensure: ensure_nscolorwell_init,
    class_addr: nscolorwell_class_addr,
    meta_addr: nscolorwell_metaclass_addr,
    parent_class: nscontrol_class_addr,
    parent_meta: nscontrol_metaclass_addr);

define_class_pair!(child NSPASTEBOARD_CLASS_DATA, NSPASTEBOARD_METACLASS_DATA, NSPASTEBOARD_INIT,
    ensure: ensure_nspasteboard_init,
    class_addr: nspasteboard_class_addr,
    meta_addr: nspasteboard_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSLayoutConstraint (Auto Layout) ───────────────────────────

define_class_pair!(child NSLAYOUTCONSTRAINT_CLASS_DATA, NSLAYOUTCONSTRAINT_METACLASS_DATA, NSLAYOUTCONSTRAINT_INIT,
    ensure: ensure_nslayoutconstraint_init,
    class_addr: nslayoutconstraint_class_addr,
    meta_addr: nslayoutconstraint_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSLayoutAnchor (Auto Layout) ──────────────────────────────

define_class_pair!(child NSLAYOUTANCHOR_CLASS_DATA, NSLAYOUTANCHOR_METACLASS_DATA, NSLAYOUTANCHOR_INIT,
    ensure: ensure_nslayoutanchor_init,
    class_addr: nslayoutanchor_class_addr,
    meta_addr: nslayoutanchor_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSLayoutGuide (Auto Layout) ──────────────────────────────

define_class_pair!(child NSLAYOUTGUIDE_CLASS_DATA, NSLAYOUTGUIDE_METACLASS_DATA, NSLAYOUTGUIDE_INIT,
    ensure: ensure_nslayoutguide_init,
    class_addr: nslayoutguide_class_addr,
    meta_addr: nslayoutguide_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSTraitCollection (size classes / trait environment) ───────

define_class_pair!(child NSTRAITCOLLECTION_CLASS_DATA, NSTRAITCOLLECTION_METACLASS_DATA, NSTRAITCOLLECTION_INIT,
    ensure: ensure_nstraitcollection_init,
    class_addr: nstraitcollection_class_addr,
    meta_addr: nstraitcollection_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSAnimationContext (implicit animations) ──────────────────

define_class_pair!(child NSANIMATIONCONTEXT_CLASS_DATA, NSANIMATIONCONTEXT_METACLASS_DATA, NSANIMATIONCONTEXT_INIT,
    ensure: ensure_nsanimationcontext_init,
    class_addr: nsanimationcontext_class_addr,
    meta_addr: nsanimationcontext_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSTableView (data source driven table) ────────────────────

define_class_pair!(child NSTABLEVIEW_CLASS_DATA, NSTABLEVIEW_METACLASS_DATA, NSTABLEVIEW_INIT,
    ensure: ensure_nstableview_init,
    class_addr: nstableview_class_addr,
    meta_addr: nstableview_metaclass_addr,
    parent_class: nsview_class_addr,
    parent_meta: nsview_metaclass_addr);

// ── CoreAnimation classes (CALayer, CATransaction, CADisplayLink) ──

define_class_pair!(child CALAYER_CLASS_DATA, CALAYER_METACLASS_DATA, CALAYER_INIT,
    ensure: ensure_calayer_init,
    class_addr: calayer_class_addr,
    meta_addr: calayer_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child CATRANSACTION_CLASS_DATA, CATRANSACTION_METACLASS_DATA, CATRANSACTION_INIT,
    ensure: ensure_catransaction_init,
    class_addr: catransaction_class_addr,
    meta_addr: catransaction_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child CADISPLAYLINK_CLASS_DATA, CADISPLAYLINK_METACLASS_DATA, CADISPLAYLINK_INIT,
    ensure: ensure_cadisplaylink_init,
    class_addr: cadisplaylink_class_addr,
    meta_addr: cadisplaylink_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child CAMEDIATIMINGFUNCTION_CLASS_DATA, CAMEDIATIMINGFUNCTION_METACLASS_DATA, CAMEDIATIMINGFUNCTION_INIT,
    ensure: ensure_camediatimingfunction_init,
    class_addr: camediatimingfunction_class_addr,
    meta_addr: camediatimingfunction_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── Phase C14: NSGraphicsContext, NSBezierPath ─────────────────────

define_class_pair!(child NSGRAPHICSCONTEXT_CLASS_DATA, NSGRAPHICSCONTEXT_METACLASS_DATA, NSGRAPHICSCONTEXT_INIT,
    ensure: ensure_nsgraphicscontext_init,
    class_addr: nsgraphicscontext_class_addr,
    meta_addr: nsgraphicscontext_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSBEZIERPATH_CLASS_DATA, NSBEZIERPATH_METACLASS_DATA, NSBEZIERPATH_INIT,
    ensure: ensure_nsbezierpath_init,
    class_addr: nsbezierpath_class_addr,
    meta_addr: nsbezierpath_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);
