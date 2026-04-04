//! AppKit class method registrations (Phase C5).

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{
    nsapplication, nscolor, nsevent, nsfont, nsimage,
    nslayoutconstraint, nsobj, nsresponder, nsrunloop, nsscreen, nsview,
    nswindow, nsworkspace,
};

/// Register `NSResponder` — base class for responder chain.
pub fn register_nsresponder_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsresponder_class_addr();
    let metaclass_addr = nsobj::nsresponder_metaclass_addr();
    let info = ClassInfo {
        name: "NSResponder".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("nextResponder", nsresponder::shim_nsresponder_next_responder as *const () as u64, "@@:"),
            me("setNextResponder:", nsresponder::shim_nsresponder_set_next_responder as *const () as u64, "v@:@"),
            me("tryToPerform:with:", nsresponder::shim_nsresponder_try_to_perform_with as *const () as u64, "B@::@"),
            me("keyDown:", nsresponder::shim_nsresponder_key_down as *const () as u64, "v@:@"),
            me("keyUp:", nsresponder::shim_nsresponder_key_up as *const () as u64, "v@:@"),
            me("mouseDown:", nsresponder::shim_nsresponder_mouse_down as *const () as u64, "v@:@"),
            me("mouseUp:", nsresponder::shim_nsresponder_mouse_up as *const () as u64, "v@:@"),
            me("mouseMoved:", nsresponder::shim_nsresponder_mouse_moved as *const () as u64, "v@:@"),
            me("flagsChanged:", nsresponder::shim_nsresponder_flags_changed as *const () as u64, "v@:@"),
            me("scrollWheel:", nsresponder::shim_nsresponder_scroll_wheel as *const () as u64, "v@:@"),
            me("acceptsFirstResponder", nsresponder::shim_nsresponder_accepts_first_responder as *const () as u64, "B@:"),
            me("becomeFirstResponder", nsresponder::shim_nsresponder_become_first_responder as *const () as u64, "B@:"),
            me("resignFirstResponder", nsresponder::shim_nsresponder_resign_first_responder as *const () as u64, "B@:"),
            me("performKeyEquivalent:", nsresponder::shim_nsresponder_perform_key_equivalent as *const () as u64, "B@:@"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 16, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSResponder");
}

/// Register `NSApplication` — application lifecycle singleton.
pub fn register_nsapplication_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsapplication_class_addr();
    let metaclass_addr = nsobj::nsapplication_metaclass_addr();
    let info = ClassInfo {
        name: "NSApplication".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("delegate", nsapplication::shim_nsapplication_delegate as *const () as u64, "@@:"),
            me("setDelegate:", nsapplication::shim_nsapplication_set_delegate as *const () as u64, "v@:@"),
            me("mainMenu", nsapplication::shim_nsapplication_main_menu as *const () as u64, "@@:"),
            me("setMainMenu:", nsapplication::shim_nsapplication_set_main_menu as *const () as u64, "v@:@"),
            me("isRunning", nsapplication::shim_nsapplication_is_running as *const () as u64, "B@:"),
            me("run", nsapplication::shim_nsapplication_run as *const () as u64, "v@:"),
            me("terminate:", nsapplication::shim_nsapplication_terminate as *const () as u64, "v@:@"),
            me("activateIgnoringOtherApps:", nsapplication::shim_nsapplication_activate as *const () as u64, "v@:B"),
            me("finishLaunching", nsapplication::shim_nsapplication_finish_launching as *const () as u64, "v@:"),
            me("setActivationPolicy:", nsapplication::shim_nsapplication_set_activation_policy as *const () as u64, "B@:q"),
            me("windows", nsapplication::shim_nsapplication_windows as *const () as u64, "@@:"),
            me("orderedWindows", nsapplication::shim_nsapplication_ordered_windows as *const () as u64, "@@:"),
            me("sendEvent:", nsapplication::shim_nsapplication_send_event as *const () as u64, "v@:@"),
            me("sendAction:to:from:", nsapplication::shim_nsapplication_send_action_to_from as *const () as u64, "B@::@@"),
            me("targetForAction:", nsapplication::shim_nsapplication_target_for_action as *const () as u64, "@@::"),
            me("targetForAction:to:from:", nsapplication::shim_nsapplication_target_for_action_to_from as *const () as u64, "@@::@@"),
            me("keyWindow", nsapplication::shim_nsapplication_key_window as *const () as u64, "@@:"),
            me("mainWindow", nsapplication::shim_nsapplication_main_window as *const () as u64, "@@:"),
            me("currentEvent", nsapplication::shim_nsapplication_current_event as *const () as u64, "@@:"),
            me("performKeyEquivalent:", nsapplication::shim_nsapplication_perform_key_equivalent as *const () as u64, "B@:@"),
            me("nextEventMatchingMask:untilDate:inMode:dequeue:", nsapplication::shim_nsapplication_next_event as *const () as u64, "@@:Q@@B"),
        ],
        class_methods: vec![
            me("sharedApplication", nsapplication::shim_nsapplication_shared_application as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 56, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSApplication");
}

/// Register `NSRunLoop` — run loop singleton.
pub fn register_nsrunloop_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsrunloop_class_addr();
    let metaclass_addr = nsobj::nsrunloop_metaclass_addr();
    let info = ClassInfo {
        name: "NSRunLoop".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("runUntilDate:", nsrunloop::shim_nsrunloop_run_until_date as *const () as u64, "v@:@"),
            me("runMode:beforeDate:", nsrunloop::shim_nsrunloop_run_mode_before_date as *const () as u64, "B@:@@"),
            me("run", nsrunloop::shim_nsrunloop_run as *const () as u64, "v@:"),
            me("addTimer:forMode:", nsrunloop::shim_nsrunloop_add_timer_for_mode as *const () as u64, "v@:@@"),
            me("currentMode", nsrunloop::shim_nsrunloop_current_mode as *const () as u64, "@@:"),
            me("getCFRunLoop", nsrunloop::shim_nsrunloop_get_cf_run_loop as *const () as u64, "^v@:"),
            me("performSelector:target:argument:order:modes:", nsrunloop::shim_nsrunloop_perform_selector as *const () as u64, "v@::@Q@"),
            me("acceptInputForMode:beforeDate:", nsrunloop::shim_nsrunloop_accept_input as *const () as u64, "v@:@@"),
            me("description", nsrunloop::shim_nsrunloop_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("currentRunLoop", nsrunloop::shim_nsrunloop_current_run_loop as *const () as u64, "@@:"),
            me("mainRunLoop", nsrunloop::shim_nsrunloop_main_run_loop as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 16, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSRunLoop");
}

/// Register `NSWindow` — window stub.
pub fn register_nswindow_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nswindow_class_addr();
    let metaclass_addr = nsobj::nswindow_metaclass_addr();
    let info = ClassInfo {
        name: "NSWindow".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithContentRect:styleMask:backing:defer:", nswindow::shim_nswindow_init_with_content_rect as *const () as u64, "@@:{CGRect=dddd}QQB"),
            me("contentView", nswindow::shim_nswindow_content_view as *const () as u64, "@@:"),
            me("setContentView:", nswindow::shim_nswindow_set_content_view as *const () as u64, "v@:@"),
            me("firstResponder", nswindow::shim_nswindow_first_responder as *const () as u64, "@@:"),
            me("makeFirstResponder:", nswindow::shim_nswindow_make_first_responder as *const () as u64, "B@:@"),
            me("sendEvent:", nswindow::shim_nswindow_send_event as *const () as u64, "v@:@"),
            me("makeKeyAndOrderFront:", nswindow::shim_nswindow_make_key_and_order_front as *const () as u64, "v@:@"),
            me("makeKeyWindow", nswindow::shim_nswindow_make_key_window as *const () as u64, "v@:"),
            me("makeMainWindow", nswindow::shim_nswindow_make_main_window as *const () as u64, "v@:"),
            me("title", nswindow::shim_nswindow_title as *const () as u64, "@@:"),
            me("setTitle:", nswindow::shim_nswindow_set_title as *const () as u64, "v@:@"),
            me("styleMask", nswindow::shim_nswindow_style_mask as *const () as u64, "Q@:"),
            me("frame", nswindow::shim_nswindow_frame as *const () as u64, "d@:"),
            me("close", nswindow::shim_nswindow_close as *const () as u64, "v@:"),
            me("orderOut:", nswindow::shim_nswindow_order_out as *const () as u64, "v@:@"),
            me("orderFront:", nswindow::shim_nswindow_order_front as *const () as u64, "v@:@"),
            me("isVisible", nswindow::shim_nswindow_is_visible as *const () as u64, "B@:"),
            me("isKeyWindow", nswindow::shim_nswindow_is_key_window as *const () as u64, "B@:"),
            me("isMainWindow", nswindow::shim_nswindow_is_main_window as *const () as u64, "B@:"),
            me("delegate", nswindow::shim_nswindow_delegate as *const () as u64, "@@:"),
            me("setDelegate:", nswindow::shim_nswindow_set_delegate as *const () as u64, "v@:@"),
            me("performKeyEquivalent:", nswindow::shim_nswindow_perform_key_equivalent as *const () as u64, "B@:@"),
            me("performClose:", nswindow::shim_nswindow_perform_close as *const () as u64, "v@:@"),
            me("setFrame:display:", nswindow::shim_nswindow_set_frame_display as *const () as u64, "v@:{CGRect=dddd}B"),
            me("center", nswindow::shim_nswindow_center as *const () as u64, "v@:"),
            me("screen", nswindow::shim_nswindow_screen as *const () as u64, "@@:"),
            me("backgroundColor", nswindow::shim_nswindow_background_color as *const () as u64, "@@:"),
            me("setBackgroundColor:", nswindow::shim_nswindow_set_background_color as *const () as u64, "v@:@"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 96, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSWindow");
}

/// Register `NSView` — view stub.
pub fn register_nsview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsview_class_addr();
    let metaclass_addr = nsobj::nsview_metaclass_addr();
    let info = ClassInfo {
        name: "NSView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithFrame:", nsview::shim_nsview_init_with_frame as *const () as u64, "@@:{CGRect=dddd}"),
            me("addSubview:", nsview::shim_nsview_add_subview as *const () as u64, "v@:@"),
            me("subviews", nsview::shim_nsview_subviews as *const () as u64, "@@:"),
            me("removeFromSuperview", nsview::shim_nsview_remove_from_superview as *const () as u64, "v@:"),
            me("setFrame:", nsview::shim_nsview_set_frame as *const () as u64, "v@:{CGRect=dddd}"),
            me("setFrameSize:", nsview::shim_nsview_set_frame_size as *const () as u64, "v@:{CGSize=dd}"),
            me("setFrameOrigin:", nsview::shim_nsview_set_frame_origin as *const () as u64, "v@:{CGPoint=dd}"),
            me("setNeedsDisplay:", nsview::shim_nsview_set_needs_display as *const () as u64, "v@:B"),
            me("superview", nsview::shim_nsview_superview as *const () as u64, "@@:"),
            me("frame", nsview::shim_nsview_frame as *const () as u64, "d@:"),
            me("bounds", nsview::shim_nsview_bounds as *const () as u64, "d@:"),
            me("window", nsview::shim_nsview_window as *const () as u64, "@@:"),
            me("isFlipped", nsview::shim_nsview_is_flipped as *const () as u64, "B@:"),
            me("needsDisplay", nsview::shim_nsview_needs_display as *const () as u64, "B@:"),
            me("drawRect:", nsview::shim_nsview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("wantsLayer", nsview::shim_nsview_wants_layer as *const () as u64, "B@:"),
            me("setWantsLayer:", nsview::shim_nsview_set_wants_layer as *const () as u64, "v@:B"),
            me("layer", nsview::shim_nsview_layer as *const () as u64, "@@:"),
            me("translatesAutoresizingMaskIntoConstraints", nslayoutconstraint::shim_nsview_translates_autoresize as *const () as u64, "B@:"),
            me("setTranslatesAutoresizingMaskIntoConstraints:", nslayoutconstraint::shim_nsview_set_translates_autoresize as *const () as u64, "v@:B"),
            me("addConstraint:", nslayoutconstraint::shim_nsview_add_constraint as *const () as u64, "v@:@"),
            me("addConstraints:", nslayoutconstraint::shim_nsview_add_constraints as *const () as u64, "v@:@"),
            me("removeConstraint:", nslayoutconstraint::shim_nsview_remove_constraint as *const () as u64, "v@:@"),
            me("removeConstraints:", nslayoutconstraint::shim_nsview_remove_constraints as *const () as u64, "v@:@"),
            me("constraints", nslayoutconstraint::shim_nsview_constraints as *const () as u64, "@@:"),
            me("setNeedsLayout:", nsview::shim_nsview_set_needs_layout as *const () as u64, "v@:B"),
            me("needsLayout", nsview::shim_nsview_needs_layout as *const () as u64, "B@:"),
            me("layoutIfNeeded", nsview::shim_nsview_layout_if_needed as *const () as u64, "v@:"),
            me("layoutSubtreeIfNeeded", nsview::shim_nsview_layout_subtree_if_needed as *const () as u64, "v@:"),
            me("updateConstraints", nsview::shim_nsview_update_constraints as *const () as u64, "v@:"),
            me("setNeedsUpdateConstraints", nsview::shim_nsview_set_needs_update_constraints as *const () as u64, "v@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 80, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSView");
}

/// Register `NSEvent` — event stub.
pub fn register_nsevent_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsevent_class_addr();
    let metaclass_addr = nsobj::nsevent_metaclass_addr();
    let info = ClassInfo {
        name: "NSEvent".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("type", nsevent::shim_nsevent_type as *const () as u64, "Q@:"),
            me("locationInWindow", nsevent::shim_nsevent_location_in_window as *const () as u64, "d@:"),
            me("modifierFlags", nsevent::shim_nsevent_modifier_flags as *const () as u64, "Q@:"),
            me("keyCode", nsevent::shim_nsevent_key_code as *const () as u64, "S@:"),
            me("characters", nsevent::shim_nsevent_characters as *const () as u64, "@@:"),
            me("charactersIgnoringModifiers", nsevent::shim_nsevent_characters_ignoring_modifiers as *const () as u64, "@@:"),
            me("window", nsevent::shim_nsevent_window as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 64, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSEvent");
}

/// Register `NSWorkspace` — workspace singleton stub.
pub fn register_nsworkspace_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsworkspace_class_addr();
    let metaclass_addr = nsobj::nsworkspace_metaclass_addr();
    let info = ClassInfo {
        name: "NSWorkspace".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("openURL:", nsworkspace::shim_nsworkspace_open_url as *const () as u64, "c@:@"),
            me("openFile:", nsworkspace::shim_nsworkspace_open_file as *const () as u64, "c@:@"),
            me("openFile:withApplication:", nsworkspace::shim_nsworkspace_open_file_with_app as *const () as u64, "c@:@@"),
            me("runningApplications", nsworkspace::shim_nsworkspace_running_applications as *const () as u64, "@@:"),
            me("frontmostApplication", nsworkspace::shim_nsworkspace_frontmost_application as *const () as u64, "@@:"),
            me("iconForFile:", nsworkspace::shim_nsworkspace_icon_for_file as *const () as u64, "@@:@"),
        ],
        class_methods: vec![
            me("sharedWorkspace", nsworkspace::shim_nsworkspace_shared_workspace as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 8, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSWorkspace");
}

/// Register `NSScreen` — screen geometry stub.
pub fn register_nsscreen_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsscreen_class_addr();
    let metaclass_addr = nsobj::nsscreen_metaclass_addr();
    let info = ClassInfo {
        name: "NSScreen".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("frame", nsscreen::shim_nsscreen_frame as *const () as u64, "d@:"),
            me("visibleFrame", nsscreen::shim_nsscreen_visible_frame as *const () as u64, "d@:"),
            me("backingScaleFactor", nsscreen::shim_nsscreen_backing_scale_factor as *const () as u64, "d@:"),
            me("deviceDescription", nsscreen::shim_nsscreen_device_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("mainScreen", nsscreen::shim_nsscreen_main_screen as *const () as u64, "@@:"),
            me("screens", nsscreen::shim_nsscreen_screens as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 24, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSScreen");
}

/// Register `NSColor` — color factory stubs.
pub fn register_nscolor_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscolor_class_addr();
    let metaclass_addr = nsobj::nscolor_metaclass_addr();
    let info = ClassInfo {
        name: "NSColor".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("redComponent", nscolor::shim_nscolor_red_component as *const () as u64, "d@:"),
            me("greenComponent", nscolor::shim_nscolor_green_component as *const () as u64, "d@:"),
            me("blueComponent", nscolor::shim_nscolor_blue_component as *const () as u64, "d@:"),
            me("alphaComponent", nscolor::shim_nscolor_alpha_component as *const () as u64, "d@:"),
            me("hueComponent", nscolor::shim_nscolor_hue_component as *const () as u64, "d@:"),
            me("saturationComponent", nscolor::shim_nscolor_saturation_component as *const () as u64, "d@:"),
            me("brightnessComponent", nscolor::shim_nscolor_brightness_component as *const () as u64, "d@:"),
            me("colorUsingColorSpace:", nscolor::shim_nscolor_color_using_color_space as *const () as u64, "@@:@"),
            me("getRed:green:blue:alpha:", nscolor::shim_nscolor_get_rgba as *const () as u64, "v@:^d^d^d^d"),
        ],
        class_methods: vec![
            me("whiteColor", nscolor::shim_nscolor_white_color as *const () as u64, "@@:"),
            me("blackColor", nscolor::shim_nscolor_black_color as *const () as u64, "@@:"),
            me("redColor", nscolor::shim_nscolor_red_color as *const () as u64, "@@:"),
            me("greenColor", nscolor::shim_nscolor_green_color as *const () as u64, "@@:"),
            me("blueColor", nscolor::shim_nscolor_blue_color as *const () as u64, "@@:"),
            me("clearColor", nscolor::shim_nscolor_clear_color as *const () as u64, "@@:"),
            me("grayColor", nscolor::shim_nscolor_gray_color as *const () as u64, "@@:"),
            me("colorWithRed:green:blue:alpha:", nscolor::shim_nscolor_with_rgba as *const () as u64, "@@:dddd"),
            me("colorWithWhite:alpha:", nscolor::shim_nscolor_with_white_alpha as *const () as u64, "@@:dd"),
            me("colorWithHue:saturation:brightness:alpha:", nscolor::shim_nscolor_with_hsba as *const () as u64, "@@:dddd"),
            me("controlColor", nscolor::shim_nscolor_control_color as *const () as u64, "@@:"),
            me("controlBackgroundColor", nscolor::shim_nscolor_control_background_color as *const () as u64, "@@:"),
            me("controlTextColor", nscolor::shim_nscolor_control_text_color as *const () as u64, "@@:"),
            me("disabledControlTextColor", nscolor::shim_nscolor_disabled_control_text_color as *const () as u64, "@@:"),
            me("selectedControlColor", nscolor::shim_nscolor_selected_control_color as *const () as u64, "@@:"),
            me("selectedTextBackgroundColor", nscolor::shim_nscolor_selected_text_background_color as *const () as u64, "@@:"),
            me("selectedTextColor", nscolor::shim_nscolor_selected_text_color as *const () as u64, "@@:"),
            me("textColor", nscolor::shim_nscolor_text_color as *const () as u64, "@@:"),
            me("textBackgroundColor", nscolor::shim_nscolor_text_background_color as *const () as u64, "@@:"),
            me("windowBackgroundColor", nscolor::shim_nscolor_window_background_color as *const () as u64, "@@:"),
            me("windowFrameTextColor", nscolor::shim_nscolor_window_frame_text_color as *const () as u64, "@@:"),
            me("labelColor", nscolor::shim_nscolor_label_color as *const () as u64, "@@:"),
            me("secondaryLabelColor", nscolor::shim_nscolor_secondary_label_color as *const () as u64, "@@:"),
            me("tertiaryLabelColor", nscolor::shim_nscolor_tertiary_label_color as *const () as u64, "@@:"),
            me("quaternaryLabelColor", nscolor::shim_nscolor_quaternary_label_color as *const () as u64, "@@:"),
            me("separatorColor", nscolor::shim_nscolor_separator_color as *const () as u64, "@@:"),
            me("gridColor", nscolor::shim_nscolor_grid_color as *const () as u64, "@@:"),
            me("highlightColor", nscolor::shim_nscolor_highlight_color as *const () as u64, "@@:"),
            me("shadowColor", nscolor::shim_nscolor_shadow_color as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 40, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSColor");
}

/// Register `NSFont` — font factory stubs.
pub fn register_nsfont_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsfont_class_addr();
    let metaclass_addr = nsobj::nsfont_metaclass_addr();
    let info = ClassInfo {
        name: "NSFont".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("fontName", nsfont::shim_nsfont_font_name as *const () as u64, "@@:"),
            me("pointSize", nsfont::shim_nsfont_point_size as *const () as u64, "d@:"),
            me("familyName", nsfont::shim_nsfont_family_name as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("systemFontOfSize:", nsfont::shim_nsfont_system_font_of_size as *const () as u64, "@@:d"),
            me("boldSystemFontOfSize:", nsfont::shim_nsfont_bold_system_font_of_size as *const () as u64, "@@:d"),
            me("labelFontOfSize:", nsfont::shim_nsfont_label_font_of_size as *const () as u64, "@@:d"),
            me("systemFontSize", nsfont::shim_nsfont_system_font_size as *const () as u64, "d@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 24, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSFont");
}

/// Register `NSImage` — image factory and instance methods.
pub fn register_nsimage_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsimage_class_addr();
    let metaclass_addr = nsobj::nsimage_metaclass_addr();
    let info = ClassInfo {
        name: "NSImage".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nsimage::shim_nsimage_name as *const () as u64, "@@:"),
            me("size", nsimage::shim_nsimage_size as *const () as u64, "{CGSize=dd}@:"),
            me("setSize:", nsimage::shim_nsimage_set_size as *const () as u64, "v@:{CGSize=dd}"),
            me("initWithContentsOfFile:", nsimage::shim_nsimage_init_with_contents_of_file as *const () as u64, "@@:@"),
            me("initWithData:", nsimage::shim_nsimage_init_with_data as *const () as u64, "@@:@"),
            me("drawInRect:fromRect:operation:fraction:", nsimage::shim_nsimage_draw_in_rect as *const () as u64, "v@:{CGRect=dddd}{CGRect=dddd}Qd"),
        ],
        class_methods: vec![
            me("imageNamed:", nsimage::shim_nsimage_image_named as *const () as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 8, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSImage");
}

