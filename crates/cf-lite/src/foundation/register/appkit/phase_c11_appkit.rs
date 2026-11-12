//! AppKit Phase C11: Layout Anchors, Layout Guide, VFL, intrinsic content size,
//! and contentLayoutGuide registrations.

use super::me;
use crate::{nslayoutanchor, nslayoutconstraint_vfl, nslayoutguide, nsobj, nsview, nswindow};
use objc_runtime::ClassInfo;
use std::collections::HashMap;

// ── NSLayoutAnchor ─────────────────────────────────────────────

pub fn register_nslayoutanchor_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nslayoutanchor_class_addr();
    let metaclass_addr = nsobj::nslayoutanchor_metaclass_addr();
    let info = ClassInfo {
        name: "NSLayoutAnchor".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "constraintEqualToAnchor:",
                nslayoutanchor::shim_nslayoutanchor_constraint_eq as *const () as u64,
                "@@:@",
            ),
            me(
                "constraintEqualToAnchor:constant:",
                nslayoutanchor::shim_nslayoutanchor_constraint_eq_constant as *const () as u64,
                "@@:@d",
            ),
            me(
                "constraintGreaterThanOrEqualToAnchor:",
                nslayoutanchor::shim_nslayoutanchor_constraint_ge as *const () as u64,
                "@@:@",
            ),
            me(
                "constraintGreaterThanOrEqualToAnchor:constant:",
                nslayoutanchor::shim_nslayoutanchor_constraint_ge_constant as *const () as u64,
                "@@:@d",
            ),
            me(
                "constraintLessThanOrEqualToAnchor:",
                nslayoutanchor::shim_nslayoutanchor_constraint_le as *const () as u64,
                "@@:@",
            ),
            me(
                "constraintLessThanOrEqualToAnchor:constant:",
                nslayoutanchor::shim_nslayoutanchor_constraint_le_constant as *const () as u64,
                "@@:@d",
            ),
            me(
                "item",
                nslayoutanchor::shim_nslayoutanchor_item as *const () as u64,
                "@@:",
            ),
            // NSLayoutDimension extras
            me(
                "constraintEqualToConstant:",
                nslayoutanchor::shim_nslayoutdimension_constraint_eq_constant as *const () as u64,
                "@@:d",
            ),
            me(
                "constraintGreaterThanOrEqualToConstant:",
                nslayoutanchor::shim_nslayoutdimension_constraint_ge_constant as *const () as u64,
                "@@:d",
            ),
            me(
                "constraintLessThanOrEqualToConstant:",
                nslayoutanchor::shim_nslayoutdimension_constraint_le_constant as *const () as u64,
                "@@:d",
            ),
            me(
                "constraintEqualToAnchor:multiplier:",
                nslayoutanchor::shim_nslayoutdimension_constraint_eq_anchor_multiplier as *const ()
                    as u64,
                "@@:@d",
            ),
            me(
                "constraintEqualToAnchor:multiplier:constant:",
                nslayoutanchor::shim_nslayoutdimension_constraint_eq_anchor_multiplier_constant
                    as *const () as u64,
                "@@:@dd",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nslayoutanchor::ANCHOR_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSLayoutAnchor");
}

// ── NSLayoutGuide ──────────────────────────────────────────────

pub fn register_nslayoutguide_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nslayoutguide_class_addr();
    let metaclass_addr = nsobj::nslayoutguide_metaclass_addr();
    let info = ClassInfo {
        name: "NSLayoutGuide".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "owningView",
                nslayoutguide::shim_nslayoutguide_owning_view as *const () as u64,
                "@@:",
            ),
            me(
                "identifier",
                nslayoutguide::shim_nslayoutguide_identifier as *const () as u64,
                "@@:",
            ),
            me(
                "setIdentifier:",
                nslayoutguide::shim_nslayoutguide_set_identifier as *const () as u64,
                "v@:@",
            ),
            me(
                "leadingAnchor",
                nslayoutguide::shim_nslayoutguide_leading_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "trailingAnchor",
                nslayoutguide::shim_nslayoutguide_trailing_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "leftAnchor",
                nslayoutguide::shim_nslayoutguide_left_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "rightAnchor",
                nslayoutguide::shim_nslayoutguide_right_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "topAnchor",
                nslayoutguide::shim_nslayoutguide_top_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "bottomAnchor",
                nslayoutguide::shim_nslayoutguide_bottom_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "widthAnchor",
                nslayoutguide::shim_nslayoutguide_width_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "heightAnchor",
                nslayoutguide::shim_nslayoutguide_height_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "centerXAnchor",
                nslayoutguide::shim_nslayoutguide_center_x_anchor as *const () as u64,
                "@@:",
            ),
            me(
                "centerYAnchor",
                nslayoutguide::shim_nslayoutguide_center_y_anchor as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nslayoutguide::LAYOUT_GUIDE_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSLayoutGuide");
}

// ── Additional NSView methods (anchors + intrinsic content size) ───

pub fn register_nsview_anchor_methods(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsview_class_addr();

    let methods = vec![
        // Anchor accessors
        me(
            "leadingAnchor",
            nslayoutanchor::shim_nsview_leading_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "trailingAnchor",
            nslayoutanchor::shim_nsview_trailing_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "leftAnchor",
            nslayoutanchor::shim_nsview_left_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "rightAnchor",
            nslayoutanchor::shim_nsview_right_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "topAnchor",
            nslayoutanchor::shim_nsview_top_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "bottomAnchor",
            nslayoutanchor::shim_nsview_bottom_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "widthAnchor",
            nslayoutanchor::shim_nsview_width_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "heightAnchor",
            nslayoutanchor::shim_nsview_height_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "centerXAnchor",
            nslayoutanchor::shim_nsview_center_x_anchor as *const () as u64,
            "@@:",
        ),
        me(
            "centerYAnchor",
            nslayoutanchor::shim_nsview_center_y_anchor as *const () as u64,
            "@@:",
        ),
        // Intrinsic content size
        me(
            "intrinsicContentSize",
            nsview::shim_nsview_intrinsic_content_size_w as *const () as u64,
            "d@:",
        ),
        me(
            "invalidateIntrinsicContentSize",
            nsview::shim_nsview_invalidate_intrinsic_content_size as *const () as u64,
            "v@:",
        ),
        me(
            "contentHuggingPriorityForOrientation:",
            nsview::shim_nsview_content_hugging_priority as *const () as u64,
            "d@:q",
        ),
        me(
            "setContentHuggingPriority:forOrientation:",
            nsview::shim_nsview_set_content_hugging_priority as *const () as u64,
            "v@:dq",
        ),
        me(
            "contentCompressionResistancePriorityForOrientation:",
            nsview::shim_nsview_content_compression_resistance_priority as *const () as u64,
            "d@:q",
        ),
        me(
            "setContentCompressionResistancePriority:forOrientation:",
            nsview::shim_nsview_set_content_compression_resistance_priority as *const () as u64,
            "v@:dq",
        ),
        me(
            "fittingSize",
            nsview::shim_nsview_fitting_size_w as *const () as u64,
            "d@:",
        ),
    ];

    for m in methods {
        rt.add_method_to_class(class_addr, m);
    }
}

// ── Additional NSWindow methods (contentLayoutGuide) ───────────

pub fn register_nswindow_layout_guide_method(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nswindow_class_addr();
    let m = me(
        "contentLayoutGuide",
        nswindow::shim_nswindow_content_layout_guide as *const () as u64,
        "@@:",
    );
    rt.add_method_to_class(class_addr, m);
}

// ── Additional NSLayoutConstraint class method (VFL) ───────────

pub fn register_nslayoutconstraint_vfl_method(rt: &mut objc_runtime::ObjcRuntime) {
    let metaclass_addr = nsobj::nslayoutconstraint_metaclass_addr();
    let m = me(
        "constraintsWithVisualFormat:options:metrics:views:",
        nslayoutconstraint_vfl::shim_nslayoutconstraint_constraints_with_visual_format as *const ()
            as u64,
        "@@:@Q@@",
    );
    rt.add_method_to_class(metaclass_addr, m);
}
