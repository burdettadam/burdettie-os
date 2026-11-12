//! Phase C14: Drawing infrastructure classes (NSOpenGLPixelFormat,
//! NSOpenGLContext, NSGraphicsContext, NSBezierPath).
//!
//! Also registers new methods on existing classes (NSFont metrics,
//! NSColor drawing context integration).

use super::me;
use crate::{nsbezierpath, nsgraphicscontext, nsobj, nsopengl};
use objc_runtime::ClassInfo;
use std::collections::HashMap;

/// Register `NSOpenGLPixelFormat` — pixel format description for legacy AppKit GL.
pub fn register_nsopenglpixelformat_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsopenglpixelformat_class_addr();
    let metaclass_addr = nsobj::nsopenglpixelformat_metaclass_addr();
    let info = ClassInfo {
        name: "NSOpenGLPixelFormat".to_string(),
        mapped_addr: class_addr,
        methods: vec![me(
            "initWithAttributes:",
            nsopengl::shim_nsopenglpixelformat_init_with_attributes as *const () as u64,
            "@@:^i",
        )],
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
    rt.register_metaclass_addr(metaclass_addr, "NSOpenGLPixelFormat");
}

/// Register `NSOpenGLContext` — minimal legacy OpenGL context state.
pub fn register_nsopenglcontext_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsopenglcontext_class_addr();
    let metaclass_addr = nsobj::nsopenglcontext_metaclass_addr();
    let info = ClassInfo {
        name: "NSOpenGLContext".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithFormat:shareContext:",
                nsopengl::shim_nsopenglcontext_init_with_format_share_context as *const () as u64,
                "@@:@@",
            ),
            me(
                "pixelFormat",
                nsopengl::shim_nsopenglcontext_pixel_format as *const () as u64,
                "@@:",
            ),
            me(
                "setPixelFormat:",
                nsopengl::shim_nsopenglcontext_set_pixel_format as *const () as u64,
                "v@:@",
            ),
            me(
                "view",
                nsopengl::shim_nsopenglcontext_view as *const () as u64,
                "@@:",
            ),
            me(
                "setView:",
                nsopengl::shim_nsopenglcontext_set_view as *const () as u64,
                "v@:@",
            ),
            me(
                "makeCurrentContext",
                nsopengl::shim_nsopenglcontext_make_current_context as *const () as u64,
                "v@:",
            ),
            me(
                "update",
                nsopengl::shim_nsopenglcontext_update as *const () as u64,
                "v@:",
            ),
            me(
                "flushBuffer",
                nsopengl::shim_nsopenglcontext_flush_buffer as *const () as u64,
                "v@:",
            ),
            me(
                "setValues:forParameter:",
                nsopengl::shim_nsopenglcontext_set_values_for_parameter as *const () as u64,
                "v@:^ii",
            ),
        ],
        class_methods: vec![
            me(
                "currentContext",
                nsopengl::shim_nsopenglcontext_current_context as *const () as u64,
                "@@:",
            ),
            me(
                "clearCurrentContext",
                nsopengl::shim_nsopenglcontext_clear_current_context as *const () as u64,
                "v@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSOpenGLContext");
}

/// Register `NSGraphicsContext` — thread-local drawing context wrapper.
pub fn register_nsgraphicscontext_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsgraphicscontext_class_addr();
    let metaclass_addr = nsobj::nsgraphicscontext_metaclass_addr();
    let info = ClassInfo {
        name: "NSGraphicsContext".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "CGContext",
                nsgraphicscontext::shim_nsgraphicscontext_cg_context as *const () as u64,
                "^v@:",
            ),
            me(
                "graphicsPort",
                nsgraphicscontext::shim_nsgraphicscontext_graphics_port as *const () as u64,
                "^v@:",
            ),
            me(
                "isFlipped",
                nsgraphicscontext::shim_nsgraphicscontext_is_flipped as *const () as u64,
                "B@:",
            ),
            me(
                "saveGraphicsState",
                nsgraphicscontext::shim_nsgraphicscontext_instance_save as *const () as u64,
                "v@:",
            ),
            me(
                "restoreGraphicsState",
                nsgraphicscontext::shim_nsgraphicscontext_instance_restore as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![
            me(
                "currentContext",
                nsgraphicscontext::shim_nsgraphicscontext_current_context as *const () as u64,
                "@@:",
            ),
            me(
                "setCurrentContext:",
                nsgraphicscontext::shim_nsgraphicscontext_set_current_context as *const () as u64,
                "v@:@",
            ),
            me(
                "graphicsContextWithCGContext:flipped:",
                nsgraphicscontext::shim_nsgraphicscontext_with_cg_context as *const () as u64,
                "@@:^vB",
            ),
            me(
                "saveGraphicsState",
                nsgraphicscontext::shim_nsgraphicscontext_save_state as *const () as u64,
                "v@:",
            ),
            me(
                "restoreGraphicsState",
                nsgraphicscontext::shim_nsgraphicscontext_restore_state as *const () as u64,
                "v@:",
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
    rt.register_metaclass_addr(metaclass_addr, "NSGraphicsContext");
}

/// Register `NSBezierPath` — Cocoa vector path drawing.
pub fn register_nsbezierpath_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbezierpath_class_addr();
    let metaclass_addr = nsobj::nsbezierpath_metaclass_addr();
    let info = ClassInfo {
        name: "NSBezierPath".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "moveToPoint:",
                nsbezierpath::shim_nsbezierpath_move_to as *const () as u64,
                "v@:{CGPoint=dd}",
            ),
            me(
                "lineToPoint:",
                nsbezierpath::shim_nsbezierpath_line_to as *const () as u64,
                "v@:{CGPoint=dd}",
            ),
            me(
                "curveToPoint:controlPoint1:controlPoint2:",
                nsbezierpath::shim_nsbezierpath_curve_to as *const () as u64,
                "v@:{CGPoint=dd}{CGPoint=dd}{CGPoint=dd}",
            ),
            me(
                "closePath",
                nsbezierpath::shim_nsbezierpath_close_path as *const () as u64,
                "v@:",
            ),
            me(
                "fill",
                nsbezierpath::shim_nsbezierpath_fill as *const () as u64,
                "v@:",
            ),
            me(
                "stroke",
                nsbezierpath::shim_nsbezierpath_stroke as *const () as u64,
                "v@:",
            ),
            me(
                "lineWidth",
                nsbezierpath::shim_nsbezierpath_line_width as *const () as u64,
                "d@:",
            ),
            me(
                "setLineWidth:",
                nsbezierpath::shim_nsbezierpath_set_line_width as *const () as u64,
                "v@:d",
            ),
            me(
                "addClip",
                nsbezierpath::shim_nsbezierpath_add_clip as *const () as u64,
                "v@:",
            ),
            me(
                "isEmpty",
                nsbezierpath::shim_nsbezierpath_is_empty as *const () as u64,
                "B@:",
            ),
            me(
                "appendBezierPath:",
                nsbezierpath::shim_nsbezierpath_append_bezier_path as *const () as u64,
                "v@:@",
            ),
            me(
                "appendBezierPathWithRect:",
                nsbezierpath::shim_nsbezierpath_append_rect as *const () as u64,
                "v@:{CGRect=dddd}",
            ),
            me(
                "appendBezierPathWithOvalInRect:",
                nsbezierpath::shim_nsbezierpath_append_oval as *const () as u64,
                "v@:{CGRect=dddd}",
            ),
        ],
        class_methods: vec![
            me(
                "bezierPath",
                nsbezierpath::shim_nsbezierpath_bezier_path as *const () as u64,
                "@@:",
            ),
            me(
                "bezierPathWithRect:",
                nsbezierpath::shim_nsbezierpath_with_rect as *const () as u64,
                "@@:{CGRect=dddd}",
            ),
            me(
                "bezierPathWithOvalInRect:",
                nsbezierpath::shim_nsbezierpath_with_oval as *const () as u64,
                "@@:{CGRect=dddd}",
            ),
            me(
                "bezierPathWithRoundedRect:xRadius:yRadius:",
                nsbezierpath::shim_nsbezierpath_with_rounded_rect as *const () as u64,
                "@@:{CGRect=dddd}dd",
            ),
            me(
                "fillRect:",
                nsbezierpath::shim_nsbezierpath_fill_rect as *const () as u64,
                "v@:{CGRect=dddd}",
            ),
            me(
                "strokeRect:",
                nsbezierpath::shim_nsbezierpath_stroke_rect as *const () as u64,
                "v@:{CGRect=dddd}",
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
    rt.register_metaclass_addr(metaclass_addr, "NSBezierPath");
}
