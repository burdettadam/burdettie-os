//! AppKit class method registrations (Phase C5).

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{
    nsapplication, nscolor, nsevent, nsfont, nsimage, nsobj,
    nsresponder, nsrunloop, nsscreen, nsview, nswindow, nsworkspace,
};

/// Register `NSResponder` — base class for responder chain.
pub fn register_nsresponder_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsresponder_class_addr();
    let metaclass_addr = nsobj::nsresponder_metaclass_addr();
    let info = ClassInfo {
        name: "NSResponder".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("nextResponder", nsresponder::shim_nsresponder_next_responder as u64, "@@:"),
            me("setNextResponder:", nsresponder::shim_nsresponder_set_next_responder as u64, "v@:@"),
            me("keyDown:", nsresponder::shim_nsresponder_key_down as u64, "v@:@"),
            me("keyUp:", nsresponder::shim_nsresponder_key_up as u64, "v@:@"),
            me("mouseDown:", nsresponder::shim_nsresponder_mouse_down as u64, "v@:@"),
            me("mouseUp:", nsresponder::shim_nsresponder_mouse_up as u64, "v@:@"),
            me("acceptsFirstResponder", nsresponder::shim_nsresponder_accepts_first_responder as u64, "B@:"),
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
            me("delegate", nsapplication::shim_nsapplication_delegate as u64, "@@:"),
            me("setDelegate:", nsapplication::shim_nsapplication_set_delegate as u64, "v@:@"),
            me("mainMenu", nsapplication::shim_nsapplication_main_menu as u64, "@@:"),
            me("setMainMenu:", nsapplication::shim_nsapplication_set_main_menu as u64, "v@:@"),
            me("isRunning", nsapplication::shim_nsapplication_is_running as u64, "B@:"),
            me("run", nsapplication::shim_nsapplication_run as u64, "v@:"),
            me("terminate:", nsapplication::shim_nsapplication_terminate as u64, "v@:@"),
            me("activateIgnoringOtherApps:", nsapplication::shim_nsapplication_activate as u64, "v@:B"),
            me("finishLaunching", nsapplication::shim_nsapplication_finish_launching as u64, "v@:"),
            me("setActivationPolicy:", nsapplication::shim_nsapplication_set_activation_policy as u64, "B@:q"),
            me("windows", nsapplication::shim_nsapplication_windows as u64, "@@:"),
        ],
        class_methods: vec![
            me("sharedApplication", nsapplication::shim_nsapplication_shared_application as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 32, is_root: false, is_meta: false,
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
            me("runUntilDate:", nsrunloop::shim_nsrunloop_run_until_date as u64, "v@:@"),
            me("runMode:beforeDate:", nsrunloop::shim_nsrunloop_run_mode_before_date as u64, "B@:@@"),
            me("run", nsrunloop::shim_nsrunloop_run as u64, "v@:"),
            me("addTimer:forMode:", nsrunloop::shim_nsrunloop_add_timer_for_mode as u64, "v@:@@"),
            me("currentMode", nsrunloop::shim_nsrunloop_current_mode as u64, "@@:"),
        ],
        class_methods: vec![
            me("currentRunLoop", nsrunloop::shim_nsrunloop_current_run_loop as u64, "@@:"),
            me("mainRunLoop", nsrunloop::shim_nsrunloop_main_run_loop as u64, "@@:"),
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
            me("initWithContentRect:styleMask:backing:defer:", nswindow::shim_nswindow_init_with_content_rect as u64, "@@:{CGRect=dddd}QQB"),
            me("contentView", nswindow::shim_nswindow_content_view as u64, "@@:"),
            me("setContentView:", nswindow::shim_nswindow_set_content_view as u64, "v@:@"),
            me("makeKeyAndOrderFront:", nswindow::shim_nswindow_make_key_and_order_front as u64, "v@:@"),
            me("title", nswindow::shim_nswindow_title as u64, "@@:"),
            me("setTitle:", nswindow::shim_nswindow_set_title as u64, "v@:@"),
            me("styleMask", nswindow::shim_nswindow_style_mask as u64, "Q@:"),
            me("close", nswindow::shim_nswindow_close as u64, "v@:"),
            me("orderFront:", nswindow::shim_nswindow_order_front as u64, "v@:@"),
            me("isVisible", nswindow::shim_nswindow_is_visible as u64, "B@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 64, is_root: false, is_meta: false,
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
            me("initWithFrame:", nsview::shim_nsview_init_with_frame as u64, "@@:{CGRect=dddd}"),
            me("addSubview:", nsview::shim_nsview_add_subview as u64, "v@:@"),
            me("setNeedsDisplay:", nsview::shim_nsview_set_needs_display as u64, "v@:B"),
            me("superview", nsview::shim_nsview_superview as u64, "@@:"),
            me("window", nsview::shim_nsview_window as u64, "@@:"),
            me("isFlipped", nsview::shim_nsview_is_flipped as u64, "B@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 48, is_root: false, is_meta: false,
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
            me("type", nsevent::shim_nsevent_type as u64, "Q@:"),
            me("locationInWindow", nsevent::shim_nsevent_location_in_window as u64, "d@:"),
            me("modifierFlags", nsevent::shim_nsevent_modifier_flags as u64, "Q@:"),
            me("keyCode", nsevent::shim_nsevent_key_code as u64, "S@:"),
            me("characters", nsevent::shim_nsevent_characters as u64, "@@:"),
            me("charactersIgnoringModifiers", nsevent::shim_nsevent_characters_ignoring_modifiers as u64, "@@:"),
            me("window", nsevent::shim_nsevent_window as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 56, is_root: false, is_meta: false,
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
        methods: vec![],
        class_methods: vec![
            me("sharedWorkspace", nsworkspace::shim_nsworkspace_shared_workspace as u64, "@@:"),
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
        methods: vec![],
        class_methods: vec![
            me("mainScreen", nsscreen::shim_nsscreen_main_screen as u64, "@@:"),
            me("screens", nsscreen::shim_nsscreen_screens as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 8, is_root: false, is_meta: false,
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
            me("redComponent", nscolor::shim_nscolor_red_component as u64, "d@:"),
            me("greenComponent", nscolor::shim_nscolor_green_component as u64, "d@:"),
            me("blueComponent", nscolor::shim_nscolor_blue_component as u64, "d@:"),
            me("alphaComponent", nscolor::shim_nscolor_alpha_component as u64, "d@:"),
        ],
        class_methods: vec![
            me("whiteColor", nscolor::shim_nscolor_white_color as u64, "@@:"),
            me("blackColor", nscolor::shim_nscolor_black_color as u64, "@@:"),
            me("redColor", nscolor::shim_nscolor_red_color as u64, "@@:"),
            me("greenColor", nscolor::shim_nscolor_green_color as u64, "@@:"),
            me("blueColor", nscolor::shim_nscolor_blue_color as u64, "@@:"),
            me("clearColor", nscolor::shim_nscolor_clear_color as u64, "@@:"),
            me("grayColor", nscolor::shim_nscolor_gray_color as u64, "@@:"),
            me("colorWithRed:green:blue:alpha:", nscolor::shim_nscolor_with_rgba as u64, "@@:dddd"),
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
            me("fontName", nsfont::shim_nsfont_font_name as u64, "@@:"),
            me("pointSize", nsfont::shim_nsfont_point_size as u64, "d@:"),
            me("familyName", nsfont::shim_nsfont_family_name as u64, "@@:"),
        ],
        class_methods: vec![
            me("systemFontOfSize:", nsfont::shim_nsfont_system_font_of_size as u64, "@@:d"),
            me("boldSystemFontOfSize:", nsfont::shim_nsfont_bold_system_font_of_size as u64, "@@:d"),
            me("labelFontOfSize:", nsfont::shim_nsfont_label_font_of_size as u64, "@@:d"),
            me("systemFontSize", nsfont::shim_nsfont_system_font_size as u64, "d@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 24, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSFont");
}

/// Register `NSImage` — image factory stubs.
pub fn register_nsimage_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsimage_class_addr();
    let metaclass_addr = nsobj::nsimage_metaclass_addr();
    let info = ClassInfo {
        name: "NSImage".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nsimage::shim_nsimage_name as u64, "@@:"),
        ],
        class_methods: vec![
            me("imageNamed:", nsimage::shim_nsimage_image_named as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 8, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSImage");
}
