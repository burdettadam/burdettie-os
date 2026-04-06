//! AppKit GUI class method registrations (Phase C8).

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{
    nscontrol, nsobj, nstextfield, nsviewcontroller, nswindowcontroller,
};

// ── Phase C8 AppKit GUI classes ────────────────────────────────

/// Register `NSViewController` — view controller for Cocoa GUI apps.
pub fn register_nsviewcontroller_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsviewcontroller_class_addr();
    let metaclass_addr = nsobj::nsviewcontroller_metaclass_addr();
    let info = ClassInfo {
        name: "NSViewController".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("view", nsviewcontroller::shim_nsviewcontroller_view as *const () as u64, "@@:"),
            me("setView:", nsviewcontroller::shim_nsviewcontroller_set_view as *const () as u64, "v@:@"),
            me("title", nsviewcontroller::shim_nsviewcontroller_title as *const () as u64, "@@:"),
            me("setTitle:", nsviewcontroller::shim_nsviewcontroller_set_title as *const () as u64, "v@:@"),
            me("representedObject", nsviewcontroller::shim_nsviewcontroller_represented_object as *const () as u64, "@@:"),
            me("setRepresentedObject:", nsviewcontroller::shim_nsviewcontroller_set_represented_object as *const () as u64, "v@:@"),
            me("loadView", nsviewcontroller::shim_nsviewcontroller_load_view as *const () as u64, "v@:"),
            me("viewDidLoad", nsviewcontroller::shim_nsviewcontroller_view_did_load as *const () as u64, "v@:"),
            me("viewWillAppear", nsviewcontroller::shim_nsviewcontroller_view_will_appear as *const () as u64, "v@:"),
            me("viewDidAppear", nsviewcontroller::shim_nsviewcontroller_view_did_appear as *const () as u64, "v@:"),
            me("viewWillDisappear", nsviewcontroller::shim_nsviewcontroller_view_will_disappear as *const () as u64, "v@:"),
            me("viewDidDisappear", nsviewcontroller::shim_nsviewcontroller_view_did_disappear as *const () as u64, "v@:"),
            me("isViewLoaded", nsviewcontroller::shim_nsviewcontroller_is_view_loaded as *const () as u64, "B@:"),
            me("nibName", nsviewcontroller::shim_nsviewcontroller_nib_name as *const () as u64, "@@:"),
            me("nibBundle", nsviewcontroller::shim_nsviewcontroller_nib_bundle as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 40, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSViewController");
}

/// Register `NSWindowController` — window management controller.
pub fn register_nswindowcontroller_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nswindowcontroller_class_addr();
    let metaclass_addr = nsobj::nswindowcontroller_metaclass_addr();
    let info = ClassInfo {
        name: "NSWindowController".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithWindow:", nswindowcontroller::shim_nswindowcontroller_init_with_window as *const () as u64, "@@:@"),
            me("window", nswindowcontroller::shim_nswindowcontroller_window as *const () as u64, "@@:"),
            me("setWindow:", nswindowcontroller::shim_nswindowcontroller_set_window as *const () as u64, "v@:@"),
            me("showWindow:", nswindowcontroller::shim_nswindowcontroller_show_window as *const () as u64, "v@:@"),
            me("close", nswindowcontroller::shim_nswindowcontroller_close as *const () as u64, "v@:"),
            me("windowNibName", nswindowcontroller::shim_nswindowcontroller_window_nib_name as *const () as u64, "@@:"),
            me("shouldCascadeWindows", nswindowcontroller::shim_nswindowcontroller_should_cascade_windows as *const () as u64, "B@:"),
            me("setShouldCascadeWindows:", nswindowcontroller::shim_nswindowcontroller_set_should_cascade_windows as *const () as u64, "v@:B"),
            me("windowDidLoad", nswindowcontroller::shim_nswindowcontroller_window_did_load as *const () as u64, "v@:"),
            me("document", nswindowcontroller::shim_nswindowcontroller_document as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 24, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSWindowController");
}

/// Register `NSControl` — base class for interactive controls.
pub fn register_nscontrol_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscontrol_class_addr();
    let metaclass_addr = nsobj::nscontrol_metaclass_addr();
    let info = ClassInfo {
        name: "NSControl".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("isEnabled", nscontrol::shim_nscontrol_is_enabled as *const () as u64, "B@:"),
            me("setEnabled:", nscontrol::shim_nscontrol_set_enabled as *const () as u64, "v@:B"),
            me("target", nscontrol::shim_nscontrol_target as *const () as u64, "@@:"),
            me("setTarget:", nscontrol::shim_nscontrol_set_target as *const () as u64, "v@:@"),
            me("action", nscontrol::shim_nscontrol_action as *const () as u64, ":@:"),
            me("setAction:", nscontrol::shim_nscontrol_set_action as *const () as u64, "v@::"),
            me("stringValue", nscontrol::shim_nscontrol_string_value as *const () as u64, "@@:"),
            me("setStringValue:", nscontrol::shim_nscontrol_set_string_value as *const () as u64, "v@:@"),
            me("integerValue", nscontrol::shim_nscontrol_integer_value as *const () as u64, "q@:"),
            me("setIntegerValue:", nscontrol::shim_nscontrol_set_integer_value as *const () as u64, "v@:q"),
            me("doubleValue", nscontrol::shim_nscontrol_double_value as *const () as u64, "d@:"),
            me("setDoubleValue:", nscontrol::shim_nscontrol_set_double_value as *const () as u64, "v@:d"),
            me("objectValue", nscontrol::shim_nscontrol_object_value as *const () as u64, "@@:"),
            me("setObjectValue:", nscontrol::shim_nscontrol_set_object_value as *const () as u64, "v@:@"),
            me("isContinuous", nscontrol::shim_nscontrol_is_continuous as *const () as u64, "B@:"),
            me("setContinuous:", nscontrol::shim_nscontrol_set_continuous as *const () as u64, "v@:B"),
            me("sendAction:to:", nscontrol::shim_nscontrol_send_action as *const () as u64, "B@::@"),
            me("sizeToFit", nscontrol::shim_nscontrol_size_to_fit as *const () as u64, "v@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 120, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSControl");
}

/// Register `NSTextField` — text label and input control.
pub fn register_nstextfield_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstextfield_class_addr();
    let metaclass_addr = nsobj::nstextfield_metaclass_addr();
    let info = ClassInfo {
        name: "NSTextField".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("stringValue", nstextfield::shim_nstextfield_string_value as *const () as u64, "@@:"),
            me("setStringValue:", nstextfield::shim_nstextfield_set_string_value as *const () as u64, "v@:@"),
            me("isEditable", nstextfield::shim_nstextfield_is_editable as *const () as u64, "B@:"),
            me("setEditable:", nstextfield::shim_nstextfield_set_editable as *const () as u64, "v@:B"),
            me("isBezeled", nstextfield::shim_nstextfield_is_bezeled as *const () as u64, "B@:"),
            me("setBezeled:", nstextfield::shim_nstextfield_set_bezeled as *const () as u64, "v@:B"),
            me("drawsBackground", nstextfield::shim_nstextfield_draws_background as *const () as u64, "B@:"),
            me("setDrawsBackground:", nstextfield::shim_nstextfield_set_draws_background as *const () as u64, "v@:B"),
            me("isSelectable", nstextfield::shim_nstextfield_is_selectable as *const () as u64, "B@:"),
            me("setSelectable:", nstextfield::shim_nstextfield_set_selectable as *const () as u64, "v@:B"),
            me("isEnabled", nstextfield::shim_nstextfield_is_enabled as *const () as u64, "B@:"),
            me("setEnabled:", nstextfield::shim_nstextfield_set_enabled as *const () as u64, "v@:B"),
            me("placeholderString", nstextfield::shim_nstextfield_placeholder_string as *const () as u64, "@@:"),
            me("setPlaceholderString:", nstextfield::shim_nstextfield_set_placeholder_string as *const () as u64, "v@:@"),
            me("target", nstextfield::shim_nstextfield_target as *const () as u64, "@@:"),
            me("setTarget:", nstextfield::shim_nstextfield_set_target as *const () as u64, "v@:@"),
            me("action", nstextfield::shim_nstextfield_action as *const () as u64, ":@:"),
            me("setAction:", nstextfield::shim_nstextfield_set_action as *const () as u64, "v@::"),
            me("drawRect:", nstextfield::shim_nstextfield_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
        ],
        class_methods: vec![
            me("labelWithString:", nstextfield::shim_nstextfield_label_with_string as *const () as u64, "@@:@"),
            me("wrappingLabelWithString:", nstextfield::shim_nstextfield_wrapping_label_with_string as *const () as u64, "@@:@"),
            me("textFieldWithString:", nstextfield::shim_nstextfield_text_field_with_string as *const () as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 120, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscontrol_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTextField");
}
