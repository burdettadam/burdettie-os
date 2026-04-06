//! AppKit widget class method registrations (Phase C10 AppKit).
//!
//! NSSlider, NSProgressIndicator, NSImageView, NSSegmentedControl,
//! NSComboBox, NSColorWell.

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{
    nsslider, nsprogressindicator, nsimageview, nssegmentedcontrol,
    nscombobox, nscolorwell, nspasteboard, nslayoutconstraint, nsobj,
};

// ── NSSlider ───────────────────────────────────────────────────

pub fn register_nsslider_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsslider_class_addr();
    let metaclass_addr = nsobj::nsslider_metaclass_addr();
    let info = ClassInfo {
        name: "NSSlider".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("doubleValue", nsslider::shim_nsslider_double_value as *const () as u64, "d@:"),
            me("setDoubleValue:", nsslider::shim_nsslider_set_double_value as *const () as u64, "v@:d"),
            me("minValue", nsslider::shim_nsslider_min_value as *const () as u64, "d@:"),
            me("setMinValue:", nsslider::shim_nsslider_set_min_value as *const () as u64, "v@:d"),
            me("maxValue", nsslider::shim_nsslider_max_value as *const () as u64, "d@:"),
            me("setMaxValue:", nsslider::shim_nsslider_set_max_value as *const () as u64, "v@:d"),
            me("isContinuous", nsslider::shim_nsslider_is_continuous as *const () as u64, "B@:"),
            me("setContinuous:", nsslider::shim_nsslider_set_continuous as *const () as u64, "v@:B"),
            me("orientation", nsslider::shim_nsslider_orientation as *const () as u64, "q@:"),
            me("setOrientation:", nsslider::shim_nsslider_set_orientation as *const () as u64, "v@:q"),
            me("drawRect:", nsslider::shim_nsslider_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsslider::SLIDER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscontrol_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSSlider");
}

// ── NSProgressIndicator ────────────────────────────────────────

pub fn register_nsprogressindicator_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsprogressindicator_class_addr();
    let metaclass_addr = nsobj::nsprogressindicator_metaclass_addr();
    let info = ClassInfo {
        name: "NSProgressIndicator".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("doubleValue", nsprogressindicator::shim_nsprogressindicator_double_value as *const () as u64, "d@:"),
            me("setDoubleValue:", nsprogressindicator::shim_nsprogressindicator_set_double_value as *const () as u64, "v@:d"),
            me("minValue", nsprogressindicator::shim_nsprogressindicator_min_value as *const () as u64, "d@:"),
            me("setMinValue:", nsprogressindicator::shim_nsprogressindicator_set_min_value as *const () as u64, "v@:d"),
            me("maxValue", nsprogressindicator::shim_nsprogressindicator_max_value as *const () as u64, "d@:"),
            me("setMaxValue:", nsprogressindicator::shim_nsprogressindicator_set_max_value as *const () as u64, "v@:d"),
            me("isIndeterminate", nsprogressindicator::shim_nsprogressindicator_is_indeterminate as *const () as u64, "B@:"),
            me("setIndeterminate:", nsprogressindicator::shim_nsprogressindicator_set_indeterminate as *const () as u64, "v@:B"),
            me("isBezeled", nsprogressindicator::shim_nsprogressindicator_is_bezeled as *const () as u64, "B@:"),
            me("setBezeled:", nsprogressindicator::shim_nsprogressindicator_set_bezeled as *const () as u64, "v@:B"),
            me("startAnimation:", nsprogressindicator::shim_nsprogressindicator_start_animation as *const () as u64, "v@:@"),
            me("stopAnimation:", nsprogressindicator::shim_nsprogressindicator_stop_animation as *const () as u64, "v@:@"),
            me("drawRect:", nsprogressindicator::shim_nsprogressindicator_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsprogressindicator::PROGRESS_INDICATOR_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSProgressIndicator");
}

// ── NSImageView ────────────────────────────────────────────────

pub fn register_nsimageview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsimageview_class_addr();
    let metaclass_addr = nsobj::nsimageview_metaclass_addr();
    let info = ClassInfo {
        name: "NSImageView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("image", nsimageview::shim_nsimageview_image as *const () as u64, "@@:"),
            me("setImage:", nsimageview::shim_nsimageview_set_image as *const () as u64, "v@:@"),
            me("allowsCutCopyPaste", nsimageview::shim_nsimageview_allows_cut_copy_paste as *const () as u64, "B@:"),
            me("setAllowsCutCopyPaste:", nsimageview::shim_nsimageview_set_allows_cut_copy_paste as *const () as u64, "v@:B"),
            me("animates", nsimageview::shim_nsimageview_animates as *const () as u64, "B@:"),
            me("setAnimates:", nsimageview::shim_nsimageview_set_animates as *const () as u64, "v@:B"),
            me("isEditable", nsimageview::shim_nsimageview_is_editable as *const () as u64, "B@:"),
            me("setEditable:", nsimageview::shim_nsimageview_set_editable as *const () as u64, "v@:B"),
            me("imageAlignment", nsimageview::shim_nsimageview_image_alignment as *const () as u64, "Q@:"),
            me("setImageAlignment:", nsimageview::shim_nsimageview_set_image_alignment as *const () as u64, "v@:Q"),
            me("imageScaling", nsimageview::shim_nsimageview_image_scaling as *const () as u64, "Q@:"),
            me("setImageScaling:", nsimageview::shim_nsimageview_set_image_scaling as *const () as u64, "v@:Q"),
            me("drawRect:", nsimageview::shim_nsimageview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsimageview::IMAGE_VIEW_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscontrol_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSImageView");
}

// ── NSSegmentedControl ─────────────────────────────────────────

pub fn register_nssegmentedcontrol_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nssegmentedcontrol_class_addr();
    let metaclass_addr = nsobj::nssegmentedcontrol_metaclass_addr();
    let info = ClassInfo {
        name: "NSSegmentedControl".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("segmentCount", nssegmentedcontrol::shim_nssegmentedcontrol_segment_count as *const () as u64, "Q@:"),
            me("setSegmentCount:", nssegmentedcontrol::shim_nssegmentedcontrol_set_segment_count as *const () as u64, "v@:Q"),
            me("selectedSegment", nssegmentedcontrol::shim_nssegmentedcontrol_selected_segment as *const () as u64, "q@:"),
            me("setSelectedSegment:", nssegmentedcontrol::shim_nssegmentedcontrol_set_selected_segment as *const () as u64, "v@:q"),
            me("isSelectedForSegment:", nssegmentedcontrol::shim_nssegmentedcontrol_is_selected_for_segment as *const () as u64, "B@:q"),
            me("setSelected:forSegment:", nssegmentedcontrol::shim_nssegmentedcontrol_set_selected_for_segment as *const () as u64, "v@:Bq"),
            me("labelForSegment:", nssegmentedcontrol::shim_nssegmentedcontrol_label_for_segment as *const () as u64, "@@:q"),
            me("setLabel:forSegment:", nssegmentedcontrol::shim_nssegmentedcontrol_set_label_for_segment as *const () as u64, "v@:@q"),
            me("trackingMode", nssegmentedcontrol::shim_nssegmentedcontrol_tracking_mode as *const () as u64, "Q@:"),
            me("setTrackingMode:", nssegmentedcontrol::shim_nssegmentedcontrol_set_tracking_mode as *const () as u64, "v@:Q"),
            me("isMomentary", nssegmentedcontrol::shim_nssegmentedcontrol_is_momentary as *const () as u64, "B@:"),
            me("setMomentary:", nssegmentedcontrol::shim_nssegmentedcontrol_set_momentary as *const () as u64, "v@:B"),
            me("drawRect:", nssegmentedcontrol::shim_nssegmentedcontrol_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nssegmentedcontrol::SEGMENTED_CONTROL_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscontrol_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSSegmentedControl");
}

// ── NSComboBox ─────────────────────────────────────────────────

pub fn register_nscombobox_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscombobox_class_addr();
    let metaclass_addr = nsobj::nscombobox_metaclass_addr();
    let info = ClassInfo {
        name: "NSComboBox".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("numberOfItems", nscombobox::shim_nscombobox_number_of_items as *const () as u64, "q@:"),
            me("addItemWithObjectValue:", nscombobox::shim_nscombobox_add_item_with_object_value as *const () as u64, "v@:@"),
            me("removeAllItems", nscombobox::shim_nscombobox_remove_all_items as *const () as u64, "v@:"),
            me("selectItemAtIndex:", nscombobox::shim_nscombobox_select_item_at_index as *const () as u64, "v@:q"),
            me("indexOfSelectedItem", nscombobox::shim_nscombobox_index_of_selected_item as *const () as u64, "q@:"),
            me("objectValueOfSelectedItem", nscombobox::shim_nscombobox_object_value_of_selected_item as *const () as u64, "@@:"),
            me("itemObjectValueAtIndex:", nscombobox::shim_nscombobox_item_object_value_at_index as *const () as u64, "@@:q"),
            me("autocompletes", nscombobox::shim_nscombobox_autocompletes as *const () as u64, "B@:"),
            me("setAutocompletes:", nscombobox::shim_nscombobox_set_autocompletes as *const () as u64, "v@:B"),
            me("usesDataSource", nscombobox::shim_nscombobox_uses_data_source as *const () as u64, "B@:"),
            me("setUsesDataSource:", nscombobox::shim_nscombobox_set_uses_data_source as *const () as u64, "v@:B"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nscombobox::COMBO_BOX_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nstextfield_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSComboBox");
}

// ── NSColorWell ────────────────────────────────────────────────

pub fn register_nscolorwell_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscolorwell_class_addr();
    let metaclass_addr = nsobj::nscolorwell_metaclass_addr();
    let info = ClassInfo {
        name: "NSColorWell".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("color", nscolorwell::shim_nscolorwell_color as *const () as u64, "@@:"),
            me("setColor:", nscolorwell::shim_nscolorwell_set_color as *const () as u64, "v@:@"),
            me("takeColorFrom:", nscolorwell::shim_nscolorwell_take_color_from as *const () as u64, "v@:@"),
            me("activate:", nscolorwell::shim_nscolorwell_activate as *const () as u64, "v@:B"),
            me("isEnabled", nscolorwell::shim_nscolorwell_is_enabled as *const () as u64, "B@:"),
            me("setEnabled:", nscolorwell::shim_nscolorwell_set_enabled as *const () as u64, "v@:B"),
            me("isBordered", nscolorwell::shim_nscolorwell_is_bordered as *const () as u64, "B@:"),
            me("setBordered:", nscolorwell::shim_nscolorwell_set_bordered as *const () as u64, "v@:B"),
            me("deactivate", nscolorwell::shim_nscolorwell_deactivate as *const () as u64, "v@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nscolorwell::COLOR_WELL_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscontrol_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSColorWell");
}

// ── NSPasteboard ───────────────────────────────────────────────

pub fn register_nspasteboard_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nspasteboard_class_addr();
    let metaclass_addr = nsobj::nspasteboard_metaclass_addr();
    let info = ClassInfo {
        name: "NSPasteboard".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("types", nspasteboard::shim_nspasteboard_types as *const () as u64, "@@:"),
            me("availableTypeFromArray:", nspasteboard::shim_nspasteboard_available_type_from_array as *const () as u64, "@@:@"),
            me("changeCount", nspasteboard::shim_nspasteboard_change_count as *const () as u64, "q@:"),
            me("name", nspasteboard::shim_nspasteboard_name as *const () as u64, "@@:"),
            me("declareTypes:owner:", nspasteboard::shim_nspasteboard_declare_types as *const () as u64, "q@:@@"),
            me("clearContents", nspasteboard::shim_nspasteboard_clear_contents as *const () as u64, "q@:"),
            me("setString:forType:", nspasteboard::shim_nspasteboard_set_string_for_type as *const () as u64, "B@:@@"),
            me("setData:forType:", nspasteboard::shim_nspasteboard_set_data_for_type as *const () as u64, "B@:@@"),
            me("setPropertyList:forType:", nspasteboard::shim_nspasteboard_set_property_list_for_type as *const () as u64, "B@:@@"),
            me("writeObjects:", nspasteboard::shim_nspasteboard_write_objects as *const () as u64, "B@:@"),
            me("stringForType:", nspasteboard::shim_nspasteboard_string_for_type as *const () as u64, "@@:@"),
            me("dataForType:", nspasteboard::shim_nspasteboard_data_for_type as *const () as u64, "@@:@"),
            me("propertyListForType:", nspasteboard::shim_nspasteboard_property_list_for_type as *const () as u64, "@@:@"),
            me("readObjectsForClasses:options:", nspasteboard::shim_nspasteboard_read_objects_for_classes as *const () as u64, "@@:@@"),
            me("releaseGlobally", nspasteboard::shim_nspasteboard_release_globally as *const () as u64, "v@:"),
        ],
        class_methods: vec![
            me("generalPasteboard", nspasteboard::shim_nspasteboard_general_pasteboard as *const () as u64, "@@:"),
            me("pasteboardWithName:", nspasteboard::shim_nspasteboard_pasteboard_with_name as *const () as u64, "@@:@"),
            me("pasteboardWithUniqueName", nspasteboard::shim_nspasteboard_pasteboard_with_unique_name as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 32u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSPasteboard");
}

// ── NSLayoutConstraint (Auto Layout) ───────────────────────────

pub fn register_nslayoutconstraint_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nslayoutconstraint_class_addr();
    let metaclass_addr = nsobj::nslayoutconstraint_metaclass_addr();
    let info = ClassInfo {
        name: "NSLayoutConstraint".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("isActive", nslayoutconstraint::shim_nslayoutconstraint_is_active as *const () as u64, "B@:"),
            me("setActive:", nslayoutconstraint::shim_nslayoutconstraint_set_active as *const () as u64, "v@:B"),
            me("constant", nslayoutconstraint::shim_nslayoutconstraint_constant as *const () as u64, "d@:"),
            me("setConstant:", nslayoutconstraint::shim_nslayoutconstraint_set_constant as *const () as u64, "v@:d"),
            me("multiplier", nslayoutconstraint::shim_nslayoutconstraint_multiplier as *const () as u64, "d@:"),
            me("priority", nslayoutconstraint::shim_nslayoutconstraint_priority as *const () as u64, "d@:"),
            me("setPriority:", nslayoutconstraint::shim_nslayoutconstraint_set_priority as *const () as u64, "v@:d"),
            me("firstItem", nslayoutconstraint::shim_nslayoutconstraint_first_item as *const () as u64, "@@:"),
            me("secondItem", nslayoutconstraint::shim_nslayoutconstraint_second_item as *const () as u64, "@@:"),
            me("firstAttribute", nslayoutconstraint::shim_nslayoutconstraint_first_attribute as *const () as u64, "q@:"),
            me("secondAttribute", nslayoutconstraint::shim_nslayoutconstraint_second_attribute as *const () as u64, "q@:"),
            me("relation", nslayoutconstraint::shim_nslayoutconstraint_relation as *const () as u64, "q@:"),
            me("identifier", nslayoutconstraint::shim_nslayoutconstraint_identifier as *const () as u64, "@@:"),
            me("setIdentifier:", nslayoutconstraint::shim_nslayoutconstraint_set_identifier as *const () as u64, "v@:@"),
            me("shouldBeArchived", nslayoutconstraint::shim_nslayoutconstraint_should_be_archived as *const () as u64, "B@:"),
            me("setShouldBeArchived:", nslayoutconstraint::shim_nslayoutconstraint_set_should_be_archived as *const () as u64, "v@:B"),
        ],
        class_methods: vec![
            me("constraintWithItem:attribute:relatedBy:toItem:attribute:multiplier:constant:",
               nslayoutconstraint::shim_nslayoutconstraint_create as *const () as u64,
               "@@:@q@q@dd"),
            me("activateConstraints:", nslayoutconstraint::shim_nslayoutconstraint_activate_constraints as *const () as u64, "v@:@"),
            me("deactivateConstraints:", nslayoutconstraint::shim_nslayoutconstraint_deactivate_constraints as *const () as u64, "v@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nslayoutconstraint::CONSTRAINT_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSLayoutConstraint");
}
