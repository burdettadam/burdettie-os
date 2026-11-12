//! CoreAnimation class registrations (CALayer, CAMetalLayer, CATransaction, CADisplayLink).

use super::me;
use crate::{calayer, nsobj};
use objc_runtime::ClassInfo;
use std::collections::HashMap;

/// Register `CALayer` — core layer-backed view support.
pub fn register_calayer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::calayer_class_addr();
    let metaclass_addr = nsobj::calayer_metaclass_addr();
    let info = ClassInfo {
        name: "CALayer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                calayer::shim_calayer_init as *const () as u64,
                "@@:",
            ),
            me(
                "bounds",
                calayer::shim_calayer_bounds as *const () as u64,
                "{CGRect=dddd}@:",
            ),
            me(
                "setBounds:",
                calayer::shim_calayer_set_bounds as *const () as u64,
                "v@:{CGRect=dddd}",
            ),
            me(
                "frame",
                calayer::shim_calayer_frame as *const () as u64,
                "{CGRect=dddd}@:",
            ),
            me(
                "setFrame:",
                calayer::shim_calayer_set_frame as *const () as u64,
                "v@:{CGRect=dddd}",
            ),
            me(
                "position",
                calayer::shim_calayer_position as *const () as u64,
                "{CGPoint=dd}@:",
            ),
            me(
                "setPosition:",
                calayer::shim_calayer_set_position as *const () as u64,
                "v@:{CGPoint=dd}",
            ),
            me(
                "backgroundColor",
                calayer::shim_calayer_background_color as *const () as u64,
                "^v@:",
            ),
            me(
                "setBackgroundColor:",
                calayer::shim_calayer_set_background_color as *const () as u64,
                "v@:^v",
            ),
            me(
                "setNeedsDisplay",
                calayer::shim_calayer_set_needs_display as *const () as u64,
                "v@:",
            ),
            me(
                "displayIfNeeded",
                calayer::shim_calayer_display_if_needed as *const () as u64,
                "v@:",
            ),
            me(
                "setNeedsLayout",
                calayer::shim_calayer_set_needs_layout as *const () as u64,
                "v@:",
            ),
            me(
                "layoutIfNeeded",
                calayer::shim_calayer_layout_if_needed as *const () as u64,
                "v@:",
            ),
            me(
                "addSublayer:",
                calayer::shim_calayer_add_sublayer as *const () as u64,
                "v@:@",
            ),
            me(
                "removeFromSuperlayer",
                calayer::shim_calayer_remove_from_superlayer as *const () as u64,
                "v@:",
            ),
            me(
                "sublayers",
                calayer::shim_calayer_sublayers as *const () as u64,
                "@@:",
            ),
            me(
                "isHidden",
                calayer::shim_calayer_is_hidden as *const () as u64,
                "B@:",
            ),
            me(
                "setHidden:",
                calayer::shim_calayer_set_hidden as *const () as u64,
                "v@:B",
            ),
            me(
                "opacity",
                calayer::shim_calayer_opacity as *const () as u64,
                "f@:",
            ),
            me(
                "setOpacity:",
                calayer::shim_calayer_set_opacity as *const () as u64,
                "v@:f",
            ),
            me(
                "cornerRadius",
                calayer::shim_calayer_corner_radius as *const () as u64,
                "d@:",
            ),
            me(
                "setCornerRadius:",
                calayer::shim_calayer_set_corner_radius as *const () as u64,
                "v@:d",
            ),
            me(
                "borderWidth",
                calayer::shim_calayer_border_width as *const () as u64,
                "d@:",
            ),
            me(
                "setBorderWidth:",
                calayer::shim_calayer_set_border_width as *const () as u64,
                "v@:d",
            ),
            me(
                "borderColor",
                calayer::shim_calayer_border_color as *const () as u64,
                "^v@:",
            ),
            me(
                "setBorderColor:",
                calayer::shim_calayer_set_border_color as *const () as u64,
                "v@:^v",
            ),
            me(
                "shadowOpacity",
                calayer::shim_calayer_shadow_opacity as *const () as u64,
                "f@:",
            ),
            me(
                "setShadowOpacity:",
                calayer::shim_calayer_set_shadow_opacity as *const () as u64,
                "v@:f",
            ),
            me(
                "contents",
                calayer::shim_calayer_contents as *const () as u64,
                "@@:",
            ),
            me(
                "setContents:",
                calayer::shim_calayer_set_contents as *const () as u64,
                "v@:@",
            ),
            me(
                "contentsScale",
                calayer::shim_calayer_contents_scale as *const () as u64,
                "d@:",
            ),
            me(
                "setContentsScale:",
                calayer::shim_calayer_set_contents_scale as *const () as u64,
                "v@:d",
            ),
            me(
                "masksToBounds",
                calayer::shim_calayer_masks_to_bounds as *const () as u64,
                "B@:",
            ),
            me(
                "setMasksToBounds:",
                calayer::shim_calayer_set_masks_to_bounds as *const () as u64,
                "v@:B",
            ),
            me(
                "superlayer",
                calayer::shim_calayer_superlayer as *const () as u64,
                "@@:",
            ),
            me(
                "transform",
                calayer::shim_calayer_transform as *const () as u64,
                "{CATransform3D=dddddddddddddddd}@:",
            ),
            me(
                "setTransform:",
                calayer::shim_calayer_set_transform as *const () as u64,
                "v@:{CATransform3D=dddddddddddddddd}",
            ),
            me(
                "sublayerTransform",
                calayer::shim_calayer_sublayer_transform as *const () as u64,
                "{CATransform3D=dddddddddddddddd}@:",
            ),
            me(
                "setSublayerTransform:",
                calayer::shim_calayer_set_sublayer_transform as *const () as u64,
                "v@:{CATransform3D=dddddddddddddddd}",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 48,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "CALayer");
}

/// Register `CAMetalLayer` — CALayer subclass used by GPU-backed Cocoa apps.
pub fn register_cametallayer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::cametallayer_class_addr();
    let metaclass_addr = nsobj::cametallayer_metaclass_addr();
    let info = ClassInfo {
        name: "CAMetalLayer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                calayer::shim_cametallayer_init as *const () as u64,
                "@@:",
            ),
            me(
                "device",
                calayer::shim_cametallayer_device as *const () as u64,
                "@@:",
            ),
            me(
                "setDevice:",
                calayer::shim_cametallayer_set_device as *const () as u64,
                "v@:@",
            ),
            me(
                "pixelFormat",
                calayer::shim_cametallayer_pixel_format as *const () as u64,
                "Q@:",
            ),
            me(
                "setPixelFormat:",
                calayer::shim_cametallayer_set_pixel_format as *const () as u64,
                "v@:Q",
            ),
            me(
                "framebufferOnly",
                calayer::shim_cametallayer_framebuffer_only as *const () as u64,
                "B@:",
            ),
            me(
                "setFramebufferOnly:",
                calayer::shim_cametallayer_set_framebuffer_only as *const () as u64,
                "v@:B",
            ),
            me(
                "setDrawableSize:",
                calayer::shim_cametallayer_set_drawable_size as *const () as u64,
                "v@:{CGSize=dd}",
            ),
            me(
                "opaque",
                calayer::shim_cametallayer_opaque as *const () as u64,
                "B@:",
            ),
            me(
                "isOpaque",
                calayer::shim_cametallayer_is_opaque as *const () as u64,
                "B@:",
            ),
            me(
                "setOpaque:",
                calayer::shim_cametallayer_set_opaque as *const () as u64,
                "v@:B",
            ),
            me(
                "nextDrawable",
                calayer::shim_cametallayer_next_drawable as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 48,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::calayer_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "CAMetalLayer");
}

/// Register `CATransaction` — animation transaction (class methods only).
pub fn register_catransaction_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::catransaction_class_addr();
    let metaclass_addr = nsobj::catransaction_metaclass_addr();
    let info = ClassInfo {
        name: "CATransaction".to_string(),
        mapped_addr: class_addr,
        methods: vec![],
        class_methods: vec![
            me(
                "begin",
                calayer::shim_catransaction_begin as *const () as u64,
                "v@:",
            ),
            me(
                "commit",
                calayer::shim_catransaction_commit as *const () as u64,
                "v@:",
            ),
            me(
                "flush",
                calayer::shim_catransaction_flush as *const () as u64,
                "v@:",
            ),
            me(
                "setDisableActions:",
                calayer::shim_catransaction_set_disable_actions as *const () as u64,
                "v@:B",
            ),
            me(
                "disableActions",
                calayer::shim_catransaction_disable_actions as *const () as u64,
                "B@:",
            ),
            me(
                "animationDuration",
                calayer::shim_catransaction_animation_duration as *const () as u64,
                "d@:",
            ),
            me(
                "setAnimationDuration:",
                calayer::shim_catransaction_set_animation_duration as *const () as u64,
                "v@:d",
            ),
            me(
                "setCompletionBlock:",
                calayer::shim_catransaction_set_completion_block as *const () as u64,
                "v@:@?",
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
    rt.register_metaclass_addr(metaclass_addr, "CATransaction");
}

/// Register `CADisplayLink` — display refresh timer.
pub fn register_cadisplaylink_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::cadisplaylink_class_addr();
    let metaclass_addr = nsobj::cadisplaylink_metaclass_addr();
    let info = ClassInfo {
        name: "CADisplayLink".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "addToRunLoop:forMode:",
                calayer::shim_cadisplaylink_add_to_run_loop as *const () as u64,
                "v@:@@",
            ),
            me(
                "invalidate",
                calayer::shim_cadisplaylink_invalidate as *const () as u64,
                "v@:",
            ),
            me(
                "isPaused",
                calayer::shim_cadisplaylink_is_paused as *const () as u64,
                "B@:",
            ),
            me(
                "setPaused:",
                calayer::shim_cadisplaylink_set_paused as *const () as u64,
                "v@:B",
            ),
            me(
                "timestamp",
                calayer::shim_cadisplaylink_timestamp as *const () as u64,
                "d@:",
            ),
            me(
                "duration",
                calayer::shim_cadisplaylink_duration as *const () as u64,
                "d@:",
            ),
        ],
        class_methods: vec![me(
            "displayLinkWithTarget:selector:",
            calayer::shim_cadisplaylink_create as *const () as u64,
            "@@:@:",
        )],
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
    rt.register_metaclass_addr(metaclass_addr, "CADisplayLink");
}

/// Register `CAMediaTimingFunction` — timing curve.
pub fn register_camediatimingfunction_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::camediatimingfunction_class_addr();
    let metaclass_addr = nsobj::camediatimingfunction_metaclass_addr();
    let info = ClassInfo {
        name: "CAMediaTimingFunction".to_string(),
        mapped_addr: class_addr,
        methods: vec![],
        class_methods: vec![me(
            "functionWithName:",
            calayer::shim_camediatimingfunction_with_name as *const () as u64,
            "@@:@",
        )],
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
    rt.register_metaclass_addr(metaclass_addr, "CAMediaTimingFunction");
}
