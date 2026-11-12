//! AppKit class method registrations (Phase C5).

use super::me;
use crate::{
    nsaccessibility, nsapplication, nsbitmapimagerep, nscolor, nsevent, nsfont, nsfontmanager,
    nsimage, nsimagerep, nslayoutconstraint, nsobj, nsresponder, nsrunloop, nsscreen,
    nstrackingarea, nsview, nswindow, nsworkspace,
};
use objc_runtime::ClassInfo;
use std::collections::HashMap;

/// Register `NSResponder` — base class for responder chain.
pub fn register_nsresponder_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsresponder_class_addr();
    let metaclass_addr = nsobj::nsresponder_metaclass_addr();
    let info = ClassInfo {
        name: "NSResponder".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "nextResponder",
                nsresponder::shim_nsresponder_next_responder as *const () as u64,
                "@@:",
            ),
            me(
                "setNextResponder:",
                nsresponder::shim_nsresponder_set_next_responder as *const () as u64,
                "v@:@",
            ),
            me(
                "tryToPerform:with:",
                nsresponder::shim_nsresponder_try_to_perform_with as *const () as u64,
                "B@::@",
            ),
            me(
                "keyDown:",
                nsresponder::shim_nsresponder_key_down as *const () as u64,
                "v@:@",
            ),
            me(
                "keyUp:",
                nsresponder::shim_nsresponder_key_up as *const () as u64,
                "v@:@",
            ),
            me(
                "mouseDown:",
                nsresponder::shim_nsresponder_mouse_down as *const () as u64,
                "v@:@",
            ),
            me(
                "mouseUp:",
                nsresponder::shim_nsresponder_mouse_up as *const () as u64,
                "v@:@",
            ),
            me(
                "mouseMoved:",
                nsresponder::shim_nsresponder_mouse_moved as *const () as u64,
                "v@:@",
            ),
            me(
                "mouseDragged:",
                nsresponder::shim_nsresponder_mouse_dragged as *const () as u64,
                "v@:@",
            ),
            me(
                "rightMouseDown:",
                nsresponder::shim_nsresponder_right_mouse_down as *const () as u64,
                "v@:@",
            ),
            me(
                "rightMouseUp:",
                nsresponder::shim_nsresponder_right_mouse_up as *const () as u64,
                "v@:@",
            ),
            me(
                "rightMouseDragged:",
                nsresponder::shim_nsresponder_right_mouse_dragged as *const () as u64,
                "v@:@",
            ),
            me(
                "flagsChanged:",
                nsresponder::shim_nsresponder_flags_changed as *const () as u64,
                "v@:@",
            ),
            me(
                "scrollWheel:",
                nsresponder::shim_nsresponder_scroll_wheel as *const () as u64,
                "v@:@",
            ),
            me(
                "acceptsFirstResponder",
                nsresponder::shim_nsresponder_accepts_first_responder as *const () as u64,
                "B@:",
            ),
            me(
                "becomeFirstResponder",
                nsresponder::shim_nsresponder_become_first_responder as *const () as u64,
                "B@:",
            ),
            me(
                "resignFirstResponder",
                nsresponder::shim_nsresponder_resign_first_responder as *const () as u64,
                "B@:",
            ),
            me(
                "performKeyEquivalent:",
                nsresponder::shim_nsresponder_perform_key_equivalent as *const () as u64,
                "B@:@",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me(
                "delegate",
                nsapplication::shim_nsapplication_delegate as *const () as u64,
                "@@:",
            ),
            me(
                "setDelegate:",
                nsapplication::shim_nsapplication_set_delegate as *const () as u64,
                "v@:@",
            ),
            me(
                "mainMenu",
                nsapplication::shim_nsapplication_main_menu as *const () as u64,
                "@@:",
            ),
            me(
                "setMainMenu:",
                nsapplication::shim_nsapplication_set_main_menu as *const () as u64,
                "v@:@",
            ),
            me(
                "windowsMenu",
                nsapplication::shim_nsapplication_windows_menu as *const () as u64,
                "@@:",
            ),
            me(
                "setWindowsMenu:",
                nsapplication::shim_nsapplication_set_windows_menu as *const () as u64,
                "v@:@",
            ),
            me(
                "helpMenu",
                nsapplication::shim_nsapplication_help_menu as *const () as u64,
                "@@:",
            ),
            me(
                "setHelpMenu:",
                nsapplication::shim_nsapplication_set_help_menu as *const () as u64,
                "v@:@",
            ),
            me(
                "isRunning",
                nsapplication::shim_nsapplication_is_running as *const () as u64,
                "B@:",
            ),
            me(
                "run",
                nsapplication::shim_nsapplication_run as *const () as u64,
                "v@:",
            ),
            me(
                "terminate:",
                nsapplication::shim_nsapplication_terminate as *const () as u64,
                "v@:@",
            ),
            me(
                "activateIgnoringOtherApps:",
                nsapplication::shim_nsapplication_activate as *const () as u64,
                "v@:B",
            ),
            me(
                "finishLaunching",
                nsapplication::shim_nsapplication_finish_launching as *const () as u64,
                "v@:",
            ),
            me(
                "setActivationPolicy:",
                nsapplication::shim_nsapplication_set_activation_policy as *const () as u64,
                "B@:q",
            ),
            me(
                "windows",
                nsapplication::shim_nsapplication_windows as *const () as u64,
                "@@:",
            ),
            me(
                "orderedWindows",
                nsapplication::shim_nsapplication_ordered_windows as *const () as u64,
                "@@:",
            ),
            me(
                "sendEvent:",
                nsapplication::shim_nsapplication_send_event as *const () as u64,
                "v@:@",
            ),
            me(
                "sendAction:to:from:",
                nsapplication::shim_nsapplication_send_action_to_from as *const () as u64,
                "B@::@@",
            ),
            me(
                "targetForAction:",
                nsapplication::shim_nsapplication_target_for_action as *const () as u64,
                "@@::",
            ),
            me(
                "targetForAction:to:from:",
                nsapplication::shim_nsapplication_target_for_action_to_from as *const () as u64,
                "@@::@@",
            ),
            me(
                "keyWindow",
                nsapplication::shim_nsapplication_key_window as *const () as u64,
                "@@:",
            ),
            me(
                "mainWindow",
                nsapplication::shim_nsapplication_main_window as *const () as u64,
                "@@:",
            ),
            me(
                "currentEvent",
                nsapplication::shim_nsapplication_current_event as *const () as u64,
                "@@:",
            ),
            me(
                "performKeyEquivalent:",
                nsapplication::shim_nsapplication_perform_key_equivalent as *const () as u64,
                "B@:@",
            ),
            me(
                "nextEventMatchingMask:untilDate:inMode:dequeue:",
                nsapplication::shim_nsapplication_next_event as *const () as u64,
                "@@:Q@@B",
            ),
            me(
                "postEvent:atStart:",
                nsapplication::shim_nsapplication_post_event as *const () as u64,
                "v@:@B",
            ),
            me(
                "discardEventsMatchingMask:beforeEvent:",
                nsapplication::shim_nsapplication_discard_events as *const () as u64,
                "v@:Q@",
            ),
            me(
                "updateWindows",
                nsapplication::shim_nsapplication_update_windows as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![me(
            "sharedApplication",
            nsapplication::shim_nsapplication_shared_application as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 56,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me(
                "runUntilDate:",
                nsrunloop::shim_nsrunloop_run_until_date as *const () as u64,
                "v@:@",
            ),
            me(
                "runMode:beforeDate:",
                nsrunloop::shim_nsrunloop_run_mode_before_date as *const () as u64,
                "B@:@@",
            ),
            me(
                "run",
                nsrunloop::shim_nsrunloop_run as *const () as u64,
                "v@:",
            ),
            me(
                "addTimer:forMode:",
                nsrunloop::shim_nsrunloop_add_timer_for_mode as *const () as u64,
                "v@:@@",
            ),
            me(
                "currentMode",
                nsrunloop::shim_nsrunloop_current_mode as *const () as u64,
                "@@:",
            ),
            me(
                "getCFRunLoop",
                nsrunloop::shim_nsrunloop_get_cf_run_loop as *const () as u64,
                "^v@:",
            ),
            me(
                "performSelector:target:argument:order:modes:",
                nsrunloop::shim_nsrunloop_perform_selector as *const () as u64,
                "v@::@Q@",
            ),
            me(
                "acceptInputForMode:beforeDate:",
                nsrunloop::shim_nsrunloop_accept_input as *const () as u64,
                "v@:@@",
            ),
            me(
                "description",
                nsrunloop::shim_nsrunloop_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "currentRunLoop",
                nsrunloop::shim_nsrunloop_current_run_loop as *const () as u64,
                "@@:",
            ),
            me(
                "mainRunLoop",
                nsrunloop::shim_nsrunloop_main_run_loop as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me(
                "initWithCoder:",
                nswindow::shim_nswindow_init_with_coder as *const () as u64,
                "@@:@",
            ),
            me(
                "initWithContentRect:styleMask:backing:defer:",
                nswindow::shim_nswindow_init_with_content_rect as *const () as u64,
                "@@:{CGRect=dddd}QQB",
            ),
            me(
                "contentView",
                nswindow::shim_nswindow_content_view as *const () as u64,
                "@@:",
            ),
            me(
                "setContentView:",
                nswindow::shim_nswindow_set_content_view as *const () as u64,
                "v@:@",
            ),
            me(
                "firstResponder",
                nswindow::shim_nswindow_first_responder as *const () as u64,
                "@@:",
            ),
            me(
                "makeFirstResponder:",
                nswindow::shim_nswindow_make_first_responder as *const () as u64,
                "B@:@",
            ),
            me(
                "sendEvent:",
                nswindow::shim_nswindow_send_event as *const () as u64,
                "v@:@",
            ),
            me(
                "makeKeyAndOrderFront:",
                nswindow::shim_nswindow_make_key_and_order_front as *const () as u64,
                "v@:@",
            ),
            me(
                "makeKeyWindow",
                nswindow::shim_nswindow_make_key_window as *const () as u64,
                "v@:",
            ),
            me(
                "makeMainWindow",
                nswindow::shim_nswindow_make_main_window as *const () as u64,
                "v@:",
            ),
            me(
                "title",
                nswindow::shim_nswindow_title as *const () as u64,
                "@@:",
            ),
            me(
                "setTitle:",
                nswindow::shim_nswindow_set_title as *const () as u64,
                "v@:@",
            ),
            me(
                "styleMask",
                nswindow::shim_nswindow_style_mask as *const () as u64,
                "Q@:",
            ),
            me(
                "frame",
                nswindow::shim_nswindow_frame as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:",
            ),
            me(
                "contentRectForFrameRect:",
                nswindow::shim_nswindow_content_rect_for_frame_rect as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:{CGRect={CGPoint=dd}{CGSize=dd}}",
            ),
            me(
                "close",
                nswindow::shim_nswindow_close as *const () as u64,
                "v@:",
            ),
            me(
                "orderOut:",
                nswindow::shim_nswindow_order_out as *const () as u64,
                "v@:@",
            ),
            me(
                "orderFront:",
                nswindow::shim_nswindow_order_front as *const () as u64,
                "v@:@",
            ),
            me(
                "isVisible",
                nswindow::shim_nswindow_is_visible as *const () as u64,
                "B@:",
            ),
            me(
                "isKeyWindow",
                nswindow::shim_nswindow_is_key_window as *const () as u64,
                "B@:",
            ),
            me(
                "isMainWindow",
                nswindow::shim_nswindow_is_main_window as *const () as u64,
                "B@:",
            ),
            me(
                "delegate",
                nswindow::shim_nswindow_delegate as *const () as u64,
                "@@:",
            ),
            me(
                "setDelegate:",
                nswindow::shim_nswindow_set_delegate as *const () as u64,
                "v@:@",
            ),
            me(
                "performKeyEquivalent:",
                nswindow::shim_nswindow_perform_key_equivalent as *const () as u64,
                "B@:@",
            ),
            me(
                "performClose:",
                nswindow::shim_nswindow_perform_close as *const () as u64,
                "v@:@",
            ),
            me(
                "setFrame:display:",
                nswindow::shim_nswindow_set_frame_display as *const () as u64,
                "v@:{CGRect=dddd}B",
            ),
            me(
                "center",
                nswindow::shim_nswindow_center as *const () as u64,
                "v@:",
            ),
            me(
                "screen",
                nswindow::shim_nswindow_screen as *const () as u64,
                "@@:",
            ),
            me(
                "backgroundColor",
                nswindow::shim_nswindow_background_color as *const () as u64,
                "@@:",
            ),
            me(
                "setBackgroundColor:",
                nswindow::shim_nswindow_set_background_color as *const () as u64,
                "v@:@",
            ),
            me(
                "minSize",
                nswindow::shim_nswindow_min_size as *const () as u64,
                "{CGSize=dd}@:",
            ),
            me(
                "setMinSize:",
                nswindow::shim_nswindow_set_min_size as *const () as u64,
                "v@:{CGSize=dd}",
            ),
            me(
                "maxSize",
                nswindow::shim_nswindow_max_size as *const () as u64,
                "{CGSize=dd}@:",
            ),
            me(
                "setMaxSize:",
                nswindow::shim_nswindow_set_max_size as *const () as u64,
                "v@:{CGSize=dd}",
            ),
            me(
                "toggleFullScreen:",
                nswindow::shim_nswindow_toggle_full_screen as *const () as u64,
                "v@:@",
            ),
            me(
                "beginSheet:completionHandler:",
                nswindow::shim_nswindow_begin_sheet as *const () as u64,
                "v@:@@?",
            ),
            me(
                "endSheet:",
                nswindow::shim_nswindow_end_sheet as *const () as u64,
                "v@:@",
            ),
            me(
                "endSheet:returnCode:",
                nswindow::shim_nswindow_end_sheet_return_code as *const () as u64,
                "v@:@q",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 128,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me(
                "initWithCoder:",
                nsview::shim_nsview_init_with_coder as *const () as u64,
                "@@:@",
            ),
            me(
                "initWithFrame:",
                nsview::shim_nsview_init_with_frame as *const () as u64,
                "@@:{CGRect=dddd}",
            ),
            me(
                "addSubview:",
                nsview::shim_nsview_add_subview as *const () as u64,
                "v@:@",
            ),
            me(
                "subviews",
                nsview::shim_nsview_subviews as *const () as u64,
                "@@:",
            ),
            me(
                "removeFromSuperview",
                nsview::shim_nsview_remove_from_superview as *const () as u64,
                "v@:",
            ),
            me(
                "setFrame:",
                nsview::shim_nsview_set_frame as *const () as u64,
                "v@:{CGRect=dddd}",
            ),
            me(
                "setFrameSize:",
                nsview::shim_nsview_set_frame_size as *const () as u64,
                "v@:{CGSize=dd}",
            ),
            me(
                "setFrameOrigin:",
                nsview::shim_nsview_set_frame_origin as *const () as u64,
                "v@:{CGPoint=dd}",
            ),
            me(
                "setNeedsDisplay:",
                nsview::shim_nsview_set_needs_display as *const () as u64,
                "v@:B",
            ),
            me(
                "superview",
                nsview::shim_nsview_superview as *const () as u64,
                "@@:",
            ),
            me(
                "frame",
                nsview::shim_nsview_frame as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:",
            ),
            me(
                "bounds",
                nsview::shim_nsview_bounds as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:",
            ),
            me(
                "window",
                nsview::shim_nsview_window as *const () as u64,
                "@@:",
            ),
            me(
                "isFlipped",
                nsview::shim_nsview_is_flipped as *const () as u64,
                "B@:",
            ),
            me(
                "needsDisplay",
                nsview::shim_nsview_needs_display as *const () as u64,
                "B@:",
            ),
            me(
                "drawRect:",
                nsview::shim_nsview_draw_rect as *const () as u64,
                "v@:{CGRect=dddd}",
            ),
            me(
                "display",
                nsview::shim_nsview_display as *const () as u64,
                "v@:",
            ),
            me(
                "displayIfNeeded",
                nsview::shim_nsview_display_if_needed as *const () as u64,
                "v@:",
            ),
            me(
                "lockFocus",
                nsview::shim_nsview_lock_focus as *const () as u64,
                "v@:",
            ),
            me(
                "unlockFocus",
                nsview::shim_nsview_unlock_focus as *const () as u64,
                "v@:",
            ),
            me(
                "wantsLayer",
                nsview::shim_nsview_wants_layer as *const () as u64,
                "B@:",
            ),
            me(
                "setWantsLayer:",
                nsview::shim_nsview_set_wants_layer as *const () as u64,
                "v@:B",
            ),
            me(
                "layer",
                nsview::shim_nsview_layer as *const () as u64,
                "@@:",
            ),
            me(
                "translatesAutoresizingMaskIntoConstraints",
                nslayoutconstraint::shim_nsview_translates_autoresize as *const () as u64,
                "B@:",
            ),
            me(
                "setTranslatesAutoresizingMaskIntoConstraints:",
                nslayoutconstraint::shim_nsview_set_translates_autoresize as *const () as u64,
                "v@:B",
            ),
            me(
                "addConstraint:",
                nslayoutconstraint::shim_nsview_add_constraint as *const () as u64,
                "v@:@",
            ),
            me(
                "addConstraints:",
                nslayoutconstraint::shim_nsview_add_constraints as *const () as u64,
                "v@:@",
            ),
            me(
                "removeConstraint:",
                nslayoutconstraint::shim_nsview_remove_constraint as *const () as u64,
                "v@:@",
            ),
            me(
                "removeConstraints:",
                nslayoutconstraint::shim_nsview_remove_constraints as *const () as u64,
                "v@:@",
            ),
            me(
                "constraints",
                nslayoutconstraint::shim_nsview_constraints as *const () as u64,
                "@@:",
            ),
            me(
                "setNeedsLayout:",
                nsview::shim_nsview_set_needs_layout as *const () as u64,
                "v@:B",
            ),
            me(
                "needsLayout",
                nsview::shim_nsview_needs_layout as *const () as u64,
                "B@:",
            ),
            me(
                "layoutIfNeeded",
                nsview::shim_nsview_layout_if_needed as *const () as u64,
                "v@:",
            ),
            me(
                "layoutSubtreeIfNeeded",
                nsview::shim_nsview_layout_subtree_if_needed as *const () as u64,
                "v@:",
            ),
            me(
                "updateConstraints",
                nsview::shim_nsview_update_constraints as *const () as u64,
                "v@:",
            ),
            me(
                "setNeedsUpdateConstraints",
                nsview::shim_nsview_set_needs_update_constraints as *const () as u64,
                "v@:",
            ),
            me(
                "autoresizingMask",
                nsview::shim_nsview_autoresizing_mask as *const () as u64,
                "Q@:",
            ),
            me(
                "setAutoresizingMask:",
                nsview::shim_nsview_set_autoresizing_mask as *const () as u64,
                "v@:Q",
            ),
            me(
                "resizeSubviewsWithOldSize:",
                nsview::shim_nsview_resize_subviews_with_old_size as *const () as u64,
                "v@:{CGSize=dd}",
            ),
            me(
                "nextKeyView",
                nsview::shim_nsview_next_key_view as *const () as u64,
                "@@:",
            ),
            me(
                "setNextKeyView:",
                nsview::shim_nsview_set_next_key_view as *const () as u64,
                "v@:@",
            ),
            me(
                "previousKeyView",
                nsview::shim_nsview_previous_key_view as *const () as u64,
                "@@:",
            ),
            me(
                "nextValidKeyView",
                nsview::shim_nsview_next_valid_key_view as *const () as u64,
                "@@:",
            ),
            me(
                "previousValidKeyView",
                nsview::shim_nsview_previous_valid_key_view as *const () as u64,
                "@@:",
            ),
            me(
                "addTrackingArea:",
                nstrackingarea::shim_nsview_add_tracking_area as *const () as u64,
                "v@:@",
            ),
            me(
                "removeTrackingArea:",
                nstrackingarea::shim_nsview_remove_tracking_area as *const () as u64,
                "v@:@",
            ),
            me(
                "trackingAreas",
                nstrackingarea::shim_nsview_tracking_areas as *const () as u64,
                "@@:",
            ),
            me(
                "updateTrackingAreas",
                nstrackingarea::shim_nsview_update_tracking_areas as *const () as u64,
                "v@:",
            ),
            me(
                "hitTest:",
                nsview::shim_nsview_hit_test as *const () as u64,
                "@@:{CGPoint=dd}",
            ),
            me(
                "acceptsFirstResponder",
                nsview::shim_nsview_accepts_first_responder as *const () as u64,
                "B@:",
            ),
            me(
                "mouseDown:",
                nsview::shim_nsview_mouse_down as *const () as u64,
                "v@:@",
            ),
            me(
                "mouseUp:",
                nsview::shim_nsview_mouse_up as *const () as u64,
                "v@:@",
            ),
            me(
                "mouseDragged:",
                nsview::shim_nsview_mouse_dragged as *const () as u64,
                "v@:@",
            ),
            me(
                "rightMouseDown:",
                nsview::shim_nsview_right_mouse_down as *const () as u64,
                "v@:@",
            ),
            me(
                "rightMouseUp:",
                nsview::shim_nsview_right_mouse_up as *const () as u64,
                "v@:@",
            ),
            me(
                "rightMouseDragged:",
                nsview::shim_nsview_right_mouse_dragged as *const () as u64,
                "v@:@",
            ),
            me(
                "convertPoint:fromView:",
                nsview::shim_nsview_convert_point_from_view as *const () as u64,
                "{CGPoint=dd}@:{CGPoint=dd}@",
            ),
            me(
                "convertPoint:toView:",
                nsview::shim_nsview_convert_point_to_view as *const () as u64,
                "{CGPoint=dd}@:{CGPoint=dd}@",
            ),
            me(
                "convertRect:fromView:",
                nsview::shim_nsview_convert_rect_from_view as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:{CGRect={CGPoint=dd}{CGSize=dd}}@",
            ),
            me(
                "convertRect:toView:",
                nsview::shim_nsview_convert_rect_to_view as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:{CGRect={CGPoint=dd}{CGSize=dd}}@",
            ),
            // NSAccessibility protocol
            me(
                "accessibilityRole",
                nsaccessibility::shim_accessibility_role as *const () as u64,
                "@@:",
            ),
            me(
                "accessibilityRoleDescription",
                nsaccessibility::shim_accessibility_role_description as *const () as u64,
                "@@:",
            ),
            me(
                "accessibilityLabel",
                nsaccessibility::shim_accessibility_label as *const () as u64,
                "@@:",
            ),
            me(
                "accessibilityValue",
                nsaccessibility::shim_accessibility_value as *const () as u64,
                "@@:",
            ),
            me(
                "isAccessibilityElement",
                nsaccessibility::shim_is_accessibility_element as *const () as u64,
                "B@:",
            ),
            me(
                "accessibilityChildren",
                nsaccessibility::shim_accessibility_children as *const () as u64,
                "@@:",
            ),
            me(
                "accessibilityParent",
                nsaccessibility::shim_accessibility_parent as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 80,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me(
                "type",
                nsevent::shim_nsevent_type as *const () as u64,
                "Q@:",
            ),
            me(
                "locationInWindow",
                nsevent::shim_nsevent_location_in_window as *const () as u64,
                "d@:",
            ),
            me(
                "modifierFlags",
                nsevent::shim_nsevent_modifier_flags as *const () as u64,
                "Q@:",
            ),
            me(
                "keyCode",
                nsevent::shim_nsevent_key_code as *const () as u64,
                "S@:",
            ),
            me(
                "characters",
                nsevent::shim_nsevent_characters as *const () as u64,
                "@@:",
            ),
            me(
                "charactersIgnoringModifiers",
                nsevent::shim_nsevent_characters_ignoring_modifiers as *const () as u64,
                "@@:",
            ),
            me(
                "window",
                nsevent::shim_nsevent_window as *const () as u64,
                "@@:",
            ),
            me(
                "clickCount",
                nsevent::shim_nsevent_click_count as *const () as u64,
                "q@:",
            ),
            me(
                "pressure",
                nsevent::shim_nsevent_pressure as *const () as u64,
                "f@:",
            ),
            me(
                "timestamp",
                nsevent::shim_nsevent_timestamp as *const () as u64,
                "d@:",
            ),
            me(
                "windowNumber",
                nsevent::shim_nsevent_window_number as *const () as u64,
                "q@:",
            ),
            me(
                "isARepeat",
                nsevent::shim_nsevent_is_a_repeat as *const () as u64,
                "B@:",
            ),
            me(
                "subtype",
                nsevent::shim_nsevent_subtype as *const () as u64,
                "s@:",
            ),
            me(
                "buttonNumber",
                nsevent::shim_nsevent_button_number as *const () as u64,
                "q@:",
            ),
            me(
                "scrollingDeltaX",
                nsevent::shim_nsevent_scrolling_delta_x as *const () as u64,
                "d@:",
            ),
            me(
                "scrollingDeltaY",
                nsevent::shim_nsevent_scrolling_delta_y as *const () as u64,
                "d@:",
            ),
            me(
                "deltaX",
                nsevent::shim_nsevent_delta_x as *const () as u64,
                "d@:",
            ),
            me(
                "deltaY",
                nsevent::shim_nsevent_delta_y as *const () as u64,
                "d@:",
            ),
        ],
        class_methods: vec![
            me(
                "mouseEventWithType:location:modifierFlags:timestamp:windowNumber:context:eventNumber:clickCount:pressure:",
                nsevent::shim_nsevent_mouse_event_with_type as *const () as u64,
                "@@:Q{CGPoint=dd}QdqQqQf",
            ),
            me(
                "keyEventWithType:location:modifierFlags:timestamp:windowNumber:context:characters:charactersIgnoringModifiers:isARepeat:keyCode:",
                nsevent::shim_nsevent_key_event_with_type as *const () as u64,
                "@@:Q{CGPoint=dd}QdqQ@@BS",
            ),
            me(
                "otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:",
                nsevent::shim_nsevent_other_event_with_type as *const () as u64,
                "@@:Q{CGPoint=dd}QdqQsqq",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 80,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me("openFile:withApplication:andDeactivate:", nsworkspace::shim_nsworkspace_open_file_with_app_and_deactivate as *const () as u64, "c@:@@B"),
            me("openURLs:withApplicationAtURL:options:configuration:error:", nsworkspace::shim_nsworkspace_open_urls_with_application_at_url_options_configuration_error as *const () as u64, "B@:@@Q@^@"),
            me("launchApplication:", nsworkspace::shim_nsworkspace_launch_application as *const () as u64, "c@:@"),
            me("launchApplicationAtURL:options:configuration:error:", nsworkspace::shim_nsworkspace_launch_application_at_url_options_configuration_error as *const () as u64, "B@:@Q@^@"),
            me("fullPathForApplication:", nsworkspace::shim_nsworkspace_full_path_for_application as *const () as u64, "@@:@"),
            me("URLForApplicationWithBundleIdentifier:", nsworkspace::shim_nsworkspace_url_for_application_with_bundle_identifier as *const () as u64, "@@:@"),
            me("URLsForApplicationsWithBundleIdentifier:", nsworkspace::shim_nsworkspace_urls_for_applications_with_bundle_identifier as *const () as u64, "@@:@"),
            me("URLForApplicationToOpenURL:", nsworkspace::shim_nsworkspace_url_for_application_to_open_url as *const () as u64, "@@:@"),
            me("applicationsForOpeningURL:", nsworkspace::shim_nsworkspace_applications_for_opening_url as *const () as u64, "@@:@"),
            me("selectFile:inFileViewerRootedAtPath:", nsworkspace::shim_nsworkspace_select_file_in_file_viewer_rooted_at_path as *const () as u64, "B@:@@"),
            me("activateFileViewerSelectingURLs:", nsworkspace::shim_nsworkspace_activate_file_viewer_selecting_urls as *const () as u64, "v@:@"),
            me("noteFileSystemChanged:", nsworkspace::shim_nsworkspace_note_file_system_changed as *const () as u64, "v@:@"),
            me("mountedLocalVolumePaths", nsworkspace::shim_nsworkspace_mounted_local_volume_paths as *const () as u64, "@@:"),
            me("typeOfFile:error:", nsworkspace::shim_nsworkspace_type_of_file_error as *const () as u64, "@@:@^@"),
            me("localizedDescriptionForType:", nsworkspace::shim_nsworkspace_localized_description_for_type as *const () as u64, "@@:@"),
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
            me(
                "frame",
                nsscreen::shim_nsscreen_frame as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:",
            ),
            me(
                "visibleFrame",
                nsscreen::shim_nsscreen_visible_frame as *const () as u64,
                "{CGRect={CGPoint=dd}{CGSize=dd}}@:",
            ),
            me(
                "backingScaleFactor",
                nsscreen::shim_nsscreen_backing_scale_factor as *const () as u64,
                "d@:",
            ),
            me(
                "deviceDescription",
                nsscreen::shim_nsscreen_device_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "mainScreen",
                nsscreen::shim_nsscreen_main_screen as *const () as u64,
                "@@:",
            ),
            me(
                "screens",
                nsscreen::shim_nsscreen_screens as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me(
                "redComponent",
                nscolor::shim_nscolor_red_component as *const () as u64,
                "d@:",
            ),
            me(
                "greenComponent",
                nscolor::shim_nscolor_green_component as *const () as u64,
                "d@:",
            ),
            me(
                "blueComponent",
                nscolor::shim_nscolor_blue_component as *const () as u64,
                "d@:",
            ),
            me(
                "alphaComponent",
                nscolor::shim_nscolor_alpha_component as *const () as u64,
                "d@:",
            ),
            me(
                "hueComponent",
                nscolor::shim_nscolor_hue_component as *const () as u64,
                "d@:",
            ),
            me(
                "saturationComponent",
                nscolor::shim_nscolor_saturation_component as *const () as u64,
                "d@:",
            ),
            me(
                "brightnessComponent",
                nscolor::shim_nscolor_brightness_component as *const () as u64,
                "d@:",
            ),
            me(
                "colorUsingColorSpace:",
                nscolor::shim_nscolor_color_using_color_space as *const () as u64,
                "@@:@",
            ),
            me(
                "getRed:green:blue:alpha:",
                nscolor::shim_nscolor_get_rgba as *const () as u64,
                "v@:^d^d^d^d",
            ),
            me("set", nscolor::shim_nscolor_set as *const () as u64, "v@:"),
            me(
                "setFill",
                nscolor::shim_nscolor_set_fill as *const () as u64,
                "v@:",
            ),
            me(
                "setStroke",
                nscolor::shim_nscolor_set_stroke as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![
            me(
                "whiteColor",
                nscolor::shim_nscolor_white_color as *const () as u64,
                "@@:",
            ),
            me(
                "blackColor",
                nscolor::shim_nscolor_black_color as *const () as u64,
                "@@:",
            ),
            me(
                "redColor",
                nscolor::shim_nscolor_red_color as *const () as u64,
                "@@:",
            ),
            me(
                "greenColor",
                nscolor::shim_nscolor_green_color as *const () as u64,
                "@@:",
            ),
            me(
                "blueColor",
                nscolor::shim_nscolor_blue_color as *const () as u64,
                "@@:",
            ),
            me(
                "clearColor",
                nscolor::shim_nscolor_clear_color as *const () as u64,
                "@@:",
            ),
            me(
                "grayColor",
                nscolor::shim_nscolor_gray_color as *const () as u64,
                "@@:",
            ),
            me(
                "colorWithRed:green:blue:alpha:",
                nscolor::shim_nscolor_with_rgba as *const () as u64,
                "@@:dddd",
            ),
            me(
                "colorWithWhite:alpha:",
                nscolor::shim_nscolor_with_white_alpha as *const () as u64,
                "@@:dd",
            ),
            me(
                "colorWithHue:saturation:brightness:alpha:",
                nscolor::shim_nscolor_with_hsba as *const () as u64,
                "@@:dddd",
            ),
            me(
                "controlColor",
                nscolor::shim_nscolor_control_color as *const () as u64,
                "@@:",
            ),
            me(
                "controlBackgroundColor",
                nscolor::shim_nscolor_control_background_color as *const () as u64,
                "@@:",
            ),
            me(
                "controlTextColor",
                nscolor::shim_nscolor_control_text_color as *const () as u64,
                "@@:",
            ),
            me(
                "disabledControlTextColor",
                nscolor::shim_nscolor_disabled_control_text_color as *const () as u64,
                "@@:",
            ),
            me(
                "selectedControlColor",
                nscolor::shim_nscolor_selected_control_color as *const () as u64,
                "@@:",
            ),
            me(
                "selectedTextBackgroundColor",
                nscolor::shim_nscolor_selected_text_background_color as *const () as u64,
                "@@:",
            ),
            me(
                "selectedTextColor",
                nscolor::shim_nscolor_selected_text_color as *const () as u64,
                "@@:",
            ),
            me(
                "textColor",
                nscolor::shim_nscolor_text_color as *const () as u64,
                "@@:",
            ),
            me(
                "textBackgroundColor",
                nscolor::shim_nscolor_text_background_color as *const () as u64,
                "@@:",
            ),
            me(
                "windowBackgroundColor",
                nscolor::shim_nscolor_window_background_color as *const () as u64,
                "@@:",
            ),
            me(
                "windowFrameTextColor",
                nscolor::shim_nscolor_window_frame_text_color as *const () as u64,
                "@@:",
            ),
            me(
                "labelColor",
                nscolor::shim_nscolor_label_color as *const () as u64,
                "@@:",
            ),
            me(
                "secondaryLabelColor",
                nscolor::shim_nscolor_secondary_label_color as *const () as u64,
                "@@:",
            ),
            me(
                "tertiaryLabelColor",
                nscolor::shim_nscolor_tertiary_label_color as *const () as u64,
                "@@:",
            ),
            me(
                "quaternaryLabelColor",
                nscolor::shim_nscolor_quaternary_label_color as *const () as u64,
                "@@:",
            ),
            me(
                "separatorColor",
                nscolor::shim_nscolor_separator_color as *const () as u64,
                "@@:",
            ),
            me(
                "gridColor",
                nscolor::shim_nscolor_grid_color as *const () as u64,
                "@@:",
            ),
            me(
                "highlightColor",
                nscolor::shim_nscolor_highlight_color as *const () as u64,
                "@@:",
            ),
            me(
                "shadowColor",
                nscolor::shim_nscolor_shadow_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemRedColor",
                nscolor::shim_nscolor_system_red_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemGreenColor",
                nscolor::shim_nscolor_system_green_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemBlueColor",
                nscolor::shim_nscolor_system_blue_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemOrangeColor",
                nscolor::shim_nscolor_system_orange_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemYellowColor",
                nscolor::shim_nscolor_system_yellow_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemPinkColor",
                nscolor::shim_nscolor_system_pink_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemPurpleColor",
                nscolor::shim_nscolor_system_purple_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemTealColor",
                nscolor::shim_nscolor_system_teal_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemIndigoColor",
                nscolor::shim_nscolor_system_indigo_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemBrownColor",
                nscolor::shim_nscolor_system_brown_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemMintColor",
                nscolor::shim_nscolor_system_mint_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemCyanColor",
                nscolor::shim_nscolor_system_cyan_color as *const () as u64,
                "@@:",
            ),
            me(
                "systemGrayColor",
                nscolor::shim_nscolor_system_gray_color as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me(
                "fontName",
                nsfont::shim_nsfont_font_name as *const () as u64,
                "@@:",
            ),
            me(
                "pointSize",
                nsfont::shim_nsfont_point_size as *const () as u64,
                "d@:",
            ),
            me(
                "familyName",
                nsfont::shim_nsfont_family_name as *const () as u64,
                "@@:",
            ),
            me(
                "displayName",
                nsfont::shim_nsfont_display_name as *const () as u64,
                "@@:",
            ),
            me(
                "ascender",
                nsfont::shim_nsfont_ascender as *const () as u64,
                "d@:",
            ),
            me(
                "descender",
                nsfont::shim_nsfont_descender as *const () as u64,
                "d@:",
            ),
            me(
                "capHeight",
                nsfont::shim_nsfont_cap_height as *const () as u64,
                "d@:",
            ),
            me(
                "xHeight",
                nsfont::shim_nsfont_x_height as *const () as u64,
                "d@:",
            ),
            me(
                "leading",
                nsfont::shim_nsfont_leading as *const () as u64,
                "d@:",
            ),
            me(
                "underlinePosition",
                nsfont::shim_nsfont_underline_position as *const () as u64,
                "d@:",
            ),
            me(
                "underlineThickness",
                nsfont::shim_nsfont_underline_thickness as *const () as u64,
                "d@:",
            ),
            me(
                "isFixedPitch",
                nsfont::shim_nsfont_is_fixed_pitch as *const () as u64,
                "B@:",
            ),
        ],
        class_methods: vec![
            me(
                "systemFontOfSize:",
                nsfont::shim_nsfont_system_font_of_size as *const () as u64,
                "@@:d",
            ),
            me(
                "boldSystemFontOfSize:",
                nsfont::shim_nsfont_bold_system_font_of_size as *const () as u64,
                "@@:d",
            ),
            me(
                "labelFontOfSize:",
                nsfont::shim_nsfont_label_font_of_size as *const () as u64,
                "@@:d",
            ),
            me(
                "systemFontSize",
                nsfont::shim_nsfont_system_font_size as *const () as u64,
                "d@:",
            ),
            me(
                "fontWithName:size:",
                nsfont::shim_nsfont_font_with_name_size as *const () as u64,
                "@@:@d",
            ),
            me(
                "systemFontOfSize:weight:",
                nsfont::shim_nsfont_system_font_of_size_weight as *const () as u64,
                "@@:dd",
            ),
            me(
                "monospacedDigitSystemFontOfSize:weight:",
                nsfont::shim_nsfont_monospaced_digit_system_font as *const () as u64,
                "@@:dd",
            ),
            me(
                "monospacedSystemFontOfSize:weight:",
                nsfont::shim_nsfont_monospaced_system_font as *const () as u64,
                "@@:dd",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
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
            me(
                "initWithCoder:",
                nsimage::shim_nsimage_init_with_coder as *const () as u64,
                "@@:@",
            ),
            me(
                "name",
                nsimage::shim_nsimage_name as *const () as u64,
                "@@:",
            ),
            me(
                "size",
                nsimage::shim_nsimage_size as *const () as u64,
                "{CGSize=dd}@:",
            ),
            me(
                "setSize:",
                nsimage::shim_nsimage_set_size as *const () as u64,
                "v@:{CGSize=dd}",
            ),
            me(
                "isTemplate",
                nsimage::shim_nsimage_is_template as *const () as u64,
                "B@:",
            ),
            me(
                "setTemplate:",
                nsimage::shim_nsimage_set_template as *const () as u64,
                "v@:B",
            ),
            me(
                "initWithContentsOfFile:",
                nsimage::shim_nsimage_init_with_contents_of_file as *const () as u64,
                "@@:@",
            ),
            me(
                "initWithData:",
                nsimage::shim_nsimage_init_with_data as *const () as u64,
                "@@:@",
            ),
            me(
                "drawInRect:fromRect:operation:fraction:",
                nsimage::shim_nsimage_draw_in_rect as *const () as u64,
                "v@:{CGRect=dddd}{CGRect=dddd}Qd",
            ),
        ],
        class_methods: vec![
            me(
                "imageNamed:",
                nsimage::shim_nsimage_image_named as *const () as u64,
                "@@:@",
            ),
            me(
                "imageWithSystemSymbolName:accessibilityDescription:",
                nsimage::shim_nsimage_image_with_system_symbol_name as *const () as u64,
                "@@:@@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSImage");
}

/// Register `NSImageRep` — abstract image representation base class.
pub fn register_nsimagerep_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsimagerep_class_addr();
    let metaclass_addr = nsobj::nsimagerep_metaclass_addr();
    let info = ClassInfo {
        name: "NSImageRep".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nsimagerep::shim_nsimagerep_init as *const () as u64,
                "@@:",
            ),
            me(
                "pixelsWide",
                nsimagerep::shim_nsimagerep_pixels_wide as *const () as u64,
                "q@:",
            ),
            me(
                "setPixelsWide:",
                nsimagerep::shim_nsimagerep_set_pixels_wide as *const () as u64,
                "v@:q",
            ),
            me(
                "pixelsHigh",
                nsimagerep::shim_nsimagerep_pixels_high as *const () as u64,
                "q@:",
            ),
            me(
                "setPixelsHigh:",
                nsimagerep::shim_nsimagerep_set_pixels_high as *const () as u64,
                "v@:q",
            ),
            me(
                "size",
                nsimagerep::shim_nsimagerep_size as *const () as u64,
                "{CGSize=dd}@:",
            ),
            me(
                "setSize:",
                nsimagerep::shim_nsimagerep_set_size as *const () as u64,
                "v@:{CGSize=dd}",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nsimagerep::shim_nsimagerep_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsimagerep::IMAGE_REP_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSImageRep");
}

/// Register `NSBitmapImageRep` — concrete bitmap image representation.
pub fn register_nsbitmapimagerep_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbitmapimagerep_class_addr();
    let metaclass_addr = nsobj::nsbitmapimagerep_metaclass_addr();
    let info = ClassInfo {
        name: "NSBitmapImageRep".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithBitmapDataPlanes:pixelsWide:pixelsHigh:bitsPerSample:samplesPerPixel:hasAlpha:isPlanar:colorSpaceName:bytesPerRow:bitsPerPixel:",
                nsbitmapimagerep::shim_nsbitmapimagerep_init_planes as *const () as u64,
                "@@:^*qqqqBB@qq",
            ),
            me(
                "initWithCGImage:",
                nsbitmapimagerep::shim_nsbitmapimagerep_init_with_cgimage as *const () as u64,
                "@@:@",
            ),
            me(
                "initWithData:",
                nsbitmapimagerep::shim_nsbitmapimagerep_init_with_data as *const () as u64,
                "@@:@",
            ),
            me(
                "bitmapData",
                nsbitmapimagerep::shim_nsbitmapimagerep_bitmap_data as *const () as u64,
                "^v@:",
            ),
            me(
                "CGImage",
                nsbitmapimagerep::shim_nsbitmapimagerep_cgimage as *const () as u64,
                "@@:",
            ),
            me(
                "bitsPerSample",
                nsbitmapimagerep::shim_nsbitmapimagerep_bits_per_sample as *const () as u64,
                "q@:",
            ),
            me(
                "samplesPerPixel",
                nsbitmapimagerep::shim_nsbitmapimagerep_samples_per_pixel as *const () as u64,
                "q@:",
            ),
            me(
                "hasAlpha",
                nsbitmapimagerep::shim_nsbitmapimagerep_has_alpha as *const () as u64,
                "B@:",
            ),
            me(
                "isPlanar",
                nsbitmapimagerep::shim_nsbitmapimagerep_is_planar as *const () as u64,
                "B@:",
            ),
            me(
                "bytesPerRow",
                nsbitmapimagerep::shim_nsbitmapimagerep_bytes_per_row as *const () as u64,
                "q@:",
            ),
            me(
                "bitsPerPixel",
                nsbitmapimagerep::shim_nsbitmapimagerep_bits_per_pixel as *const () as u64,
                "q@:",
            ),
            me(
                "colorSpaceName",
                nsbitmapimagerep::shim_nsbitmapimagerep_color_space_name as *const () as u64,
                "@@:",
            ),
            me(
                "representationUsingType:properties:",
                nsbitmapimagerep::shim_nsbitmapimagerep_representation as *const () as u64,
                "@@:Q@",
            ),
            me(
                "TIFFRepresentation",
                nsbitmapimagerep::shim_nsbitmapimagerep_tiff as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nsbitmapimagerep::shim_nsbitmapimagerep_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsbitmapimagerep::BITMAP_REP_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsimagerep_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSBitmapImageRep");
}

/// Register `NSFontManager` — font discovery and enumeration.
pub fn register_nsfontmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsfontmanager_class_addr();
    let metaclass_addr = nsobj::nsfontmanager_metaclass_addr();
    let info = ClassInfo {
        name: "NSFontManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "availableFontFamilies",
                nsfontmanager::shim_nsfontmanager_available_font_families as *const () as u64,
                "@@:",
            ),
            me(
                "availableFonts",
                nsfontmanager::shim_nsfontmanager_available_fonts as *const () as u64,
                "@@:",
            ),
            me(
                "availableMembersOfFontFamily:",
                nsfontmanager::shim_nsfontmanager_available_members as *const () as u64,
                "@@:@",
            ),
            me(
                "fontWithFamily:traits:weight:size:",
                nsfontmanager::shim_nsfontmanager_font_with_family as *const () as u64,
                "@@:@QQd",
            ),
            me(
                "convertFont:",
                nsfontmanager::shim_nsfontmanager_convert_font as *const () as u64,
                "@@:@",
            ),
            me(
                "convertFont:toHaveTrait:",
                nsfontmanager::shim_nsfontmanager_convert_font_to_trait as *const () as u64,
                "@@:@Q",
            ),
            me(
                "convertFont:toSize:",
                nsfontmanager::shim_nsfontmanager_convert_font_to_size as *const () as u64,
                "@@:@d",
            ),
            me(
                "setSelectedFont:isMultiple:",
                nsfontmanager::shim_nsfontmanager_set_selected_font as *const () as u64,
                "v@:@c",
            ),
        ],
        class_methods: vec![
            me(
                "sharedFontManager",
                nsfontmanager::shim_nsfontmanager_shared as *const () as u64,
                "@@:",
            ),
            me(
                "setFontManagerFactory:",
                nsfontmanager::shim_nsfontmanager_set_factory as *const () as u64,
                "v@:#",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSFontManager");
}
