//! AppKit GUI class method registrations (Phase C9 AppKit).
//!
//! NSButton, NSPopUpButton, NSAlert, NSPanel, NSScrollView, NSBox,
//! NSSplitView, NSStackView, NSTabView, NSTabViewItem, NSToolbar, NSToolbarItem.

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{
    nsalert, nsbox, nsbutton, nsobj, nspanel, nspopupbutton,
    nsscrollview, nssplitview, nsstackview, nstabview, nstabviewitem,
    nstoolbar, nstoolbaritem,
};

// ── NSButton ───────────────────────────────────────────────────

pub fn register_nsbutton_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbutton_class_addr();
    let metaclass_addr = nsobj::nsbutton_metaclass_addr();
    let info = ClassInfo {
        name: "NSButton".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("title", nsbutton::shim_nsbutton_title as u64, "@@:"),
            me("setTitle:", nsbutton::shim_nsbutton_set_title as u64, "v@:@"),
            me("alternateTitle", nsbutton::shim_nsbutton_alternate_title as u64, "@@:"),
            me("setAlternateTitle:", nsbutton::shim_nsbutton_set_alternate_title as u64, "v@:@"),
            me("image", nsbutton::shim_nsbutton_image as u64, "@@:"),
            me("setImage:", nsbutton::shim_nsbutton_set_image as u64, "v@:@"),
            me("target", nsbutton::shim_nsbutton_target as u64, "@@:"),
            me("setTarget:", nsbutton::shim_nsbutton_set_target as u64, "v@:@"),
            me("action", nsbutton::shim_nsbutton_action as u64, ":@:"),
            me("setAction:", nsbutton::shim_nsbutton_set_action as u64, "v@::"),
            me("state", nsbutton::shim_nsbutton_state as u64, "q@:"),
            me("setState:", nsbutton::shim_nsbutton_set_state as u64, "v@:q"),
            me("isEnabled", nsbutton::shim_nsbutton_is_enabled as u64, "B@:"),
            me("setEnabled:", nsbutton::shim_nsbutton_set_enabled as u64, "v@:B"),
            me("isBordered", nsbutton::shim_nsbutton_is_bordered as u64, "B@:"),
            me("setBordered:", nsbutton::shim_nsbutton_set_bordered as u64, "v@:B"),
            me("allowsMixedState", nsbutton::shim_nsbutton_allows_mixed_state as u64, "B@:"),
            me("setAllowsMixedState:", nsbutton::shim_nsbutton_set_allows_mixed_state as u64, "v@:B"),
            me("setButtonType:", nsbutton::shim_nsbutton_set_button_type as u64, "v@:Q"),
            me("setBezelStyle:", nsbutton::shim_nsbutton_set_bezel_style as u64, "v@:Q"),
            me("highlight:", nsbutton::shim_nsbutton_highlight as u64, "v@:B"),
            me("performClick:", nsbutton::shim_nsbutton_perform_click as u64, "v@:@"),
            me("sizeToFit", nsbutton::shim_nsbutton_size_to_fit as u64, "v@:"),
        ],
        class_methods: vec![
            me("buttonWithTitle:target:action:", nsbutton::shim_nsbutton_button_with_title as u64, "@@:@@:"),
            me("checkboxWithTitle:target:action:", nsbutton::shim_nsbutton_checkbox_with_title as u64, "@@:@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsbutton::BUTTON_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscontrol_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSButton");
}

// ── NSPopUpButton ──────────────────────────────────────────────

pub fn register_nspopupbutton_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nspopupbutton_class_addr();
    let metaclass_addr = nsobj::nspopupbutton_metaclass_addr();
    let info = ClassInfo {
        name: "NSPopUpButton".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("numberOfItems", nspopupbutton::shim_nspopupbutton_number_of_items as u64, "q@:"),
            me("addItemWithTitle:", nspopupbutton::shim_nspopupbutton_add_item_with_title as u64, "v@:@"),
            me("removeAllItems", nspopupbutton::shim_nspopupbutton_remove_all_items as u64, "v@:"),
            me("indexOfSelectedItem", nspopupbutton::shim_nspopupbutton_index_of_selected_item as u64, "q@:"),
            me("selectItemAtIndex:", nspopupbutton::shim_nspopupbutton_select_item_at_index as u64, "v@:q"),
            me("titleOfSelectedItem", nspopupbutton::shim_nspopupbutton_title_of_selected_item as u64, "@@:"),
            me("itemTitleAtIndex:", nspopupbutton::shim_nspopupbutton_item_title_at_index as u64, "@@:q"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nspopupbutton::POPUP_BUTTON_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsbutton_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSPopUpButton");
}

// ── NSAlert ────────────────────────────────────────────────────

pub fn register_nsalert_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsalert_class_addr();
    let metaclass_addr = nsobj::nsalert_metaclass_addr();
    let info = ClassInfo {
        name: "NSAlert".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("messageText", nsalert::shim_nsalert_message_text as u64, "@@:"),
            me("setMessageText:", nsalert::shim_nsalert_set_message_text as u64, "v@:@"),
            me("informativeText", nsalert::shim_nsalert_informative_text as u64, "@@:"),
            me("setInformativeText:", nsalert::shim_nsalert_set_informative_text as u64, "v@:@"),
            me("alertStyle", nsalert::shim_nsalert_alert_style as u64, "Q@:"),
            me("setAlertStyle:", nsalert::shim_nsalert_set_alert_style as u64, "v@:Q"),
            me("addButtonWithTitle:", nsalert::shim_nsalert_add_button_with_title as u64, "v@:@"),
            me("runModal", nsalert::shim_nsalert_run_modal as u64, "q@:"),
            me("window", nsalert::shim_nsalert_window as u64, "@@:"),
            me("buttons", nsalert::shim_nsalert_buttons as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsalert::ALERT_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSAlert");
}

// ── NSPanel ────────────────────────────────────────────────────

pub fn register_nspanel_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nspanel_class_addr();
    let metaclass_addr = nsobj::nspanel_metaclass_addr();
    let info = ClassInfo {
        name: "NSPanel".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("isFloatingPanel", nspanel::shim_nspanel_is_floating_panel as u64, "B@:"),
            me("setFloatingPanel:", nspanel::shim_nspanel_set_floating_panel as u64, "v@:B"),
            me("becomesKeyOnlyIfNeeded", nspanel::shim_nspanel_becomes_key_only_if_needed as u64, "B@:"),
            me("setBecomesKeyOnlyIfNeeded:", nspanel::shim_nspanel_set_becomes_key_only_if_needed as u64, "v@:B"),
            me("worksWhenModal", nspanel::shim_nspanel_works_when_modal as u64, "B@:"),
            me("setWorksWhenModal:", nspanel::shim_nspanel_set_works_when_modal as u64, "v@:B"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nspanel::PANEL_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nswindow_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSPanel");
}

// ── NSScrollView ───────────────────────────────────────────────

pub fn register_nsscrollview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsscrollview_class_addr();
    let metaclass_addr = nsobj::nsscrollview_metaclass_addr();
    let info = ClassInfo {
        name: "NSScrollView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("documentView", nsscrollview::shim_nsscrollview_document_view as u64, "@@:"),
            me("setDocumentView:", nsscrollview::shim_nsscrollview_set_document_view as u64, "v@:@"),
            me("hasVerticalScroller", nsscrollview::shim_nsscrollview_has_vertical_scroller as u64, "B@:"),
            me("setHasVerticalScroller:", nsscrollview::shim_nsscrollview_set_has_vertical_scroller as u64, "v@:B"),
            me("hasHorizontalScroller", nsscrollview::shim_nsscrollview_has_horizontal_scroller as u64, "B@:"),
            me("setHasHorizontalScroller:", nsscrollview::shim_nsscrollview_set_has_horizontal_scroller as u64, "v@:B"),
            me("autohidesScrollers", nsscrollview::shim_nsscrollview_autohides_scrollers as u64, "B@:"),
            me("setAutohidesScrollers:", nsscrollview::shim_nsscrollview_set_autohides_scrollers as u64, "v@:B"),
            me("drawsBackground", nsscrollview::shim_nsscrollview_draws_background as u64, "B@:"),
            me("setDrawsBackground:", nsscrollview::shim_nsscrollview_set_draws_background as u64, "v@:B"),
            me("contentSize", nsscrollview::shim_nsscrollview_content_size as u64, "@@:"),
            me("tile", nsscrollview::shim_nsscrollview_tile as u64, "v@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsscrollview::SCROLL_VIEW_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSScrollView");
}

// ── NSBox ──────────────────────────────────────────────────────

pub fn register_nsbox_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbox_class_addr();
    let metaclass_addr = nsobj::nsbox_metaclass_addr();
    let info = ClassInfo {
        name: "NSBox".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("title", nsbox::shim_nsbox_title as u64, "@@:"),
            me("setTitle:", nsbox::shim_nsbox_set_title as u64, "v@:@"),
            me("contentView", nsbox::shim_nsbox_content_view as u64, "@@:"),
            me("setContentView:", nsbox::shim_nsbox_set_content_view as u64, "v@:@"),
            me("boxType", nsbox::shim_nsbox_box_type as u64, "Q@:"),
            me("setBoxType:", nsbox::shim_nsbox_set_box_type as u64, "v@:Q"),
            me("isTransparent", nsbox::shim_nsbox_is_transparent as u64, "B@:"),
            me("setTransparent:", nsbox::shim_nsbox_set_transparent as u64, "v@:B"),
            me("sizeToFit", nsbox::shim_nsbox_size_to_fit as u64, "v@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsbox::BOX_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSBox");
}

// ── NSSplitView ────────────────────────────────────────────────

pub fn register_nssplitview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nssplitview_class_addr();
    let metaclass_addr = nsobj::nssplitview_metaclass_addr();
    let info = ClassInfo {
        name: "NSSplitView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("isVertical", nssplitview::shim_nssplitview_is_vertical as u64, "B@:"),
            me("setVertical:", nssplitview::shim_nssplitview_set_vertical as u64, "v@:B"),
            me("delegate", nssplitview::shim_nssplitview_delegate as u64, "@@:"),
            me("setDelegate:", nssplitview::shim_nssplitview_set_delegate as u64, "v@:@"),
            me("dividerStyle", nssplitview::shim_nssplitview_divider_style as u64, "Q@:"),
            me("setDividerStyle:", nssplitview::shim_nssplitview_set_divider_style as u64, "v@:Q"),
            me("adjustSubviews", nssplitview::shim_nssplitview_adjust_subviews as u64, "v@:"),
            me("dividerThickness", nssplitview::shim_nssplitview_divider_thickness as u64, "d@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nssplitview::SPLIT_VIEW_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSSplitView");
}

// ── NSStackView ────────────────────────────────────────────────

pub fn register_nsstackview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsstackview_class_addr();
    let metaclass_addr = nsobj::nsstackview_metaclass_addr();
    let info = ClassInfo {
        name: "NSStackView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("orientation", nsstackview::shim_nsstackview_orientation as u64, "Q@:"),
            me("setOrientation:", nsstackview::shim_nsstackview_set_orientation as u64, "v@:Q"),
            me("spacing", nsstackview::shim_nsstackview_spacing as u64, "d@:"),
            me("setSpacing:", nsstackview::shim_nsstackview_set_spacing as u64, "v@:d"),
            me("distribution", nsstackview::shim_nsstackview_distribution as u64, "Q@:"),
            me("setDistribution:", nsstackview::shim_nsstackview_set_distribution as u64, "v@:Q"),
            me("alignment", nsstackview::shim_nsstackview_alignment as u64, "Q@:"),
            me("setAlignment:", nsstackview::shim_nsstackview_set_alignment as u64, "v@:Q"),
            me("addArrangedSubview:", nsstackview::shim_nsstackview_add_arranged_subview as u64, "v@:@"),
            me("removeArrangedSubview:", nsstackview::shim_nsstackview_remove_arranged_subview as u64, "v@:@"),
        ],
        class_methods: vec![
            me("stackViewWithViews:", nsstackview::shim_nsstackview_stack_view_with_views as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsstackview::STACK_VIEW_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSStackView");
}

// ── NSTabView ──────────────────────────────────────────────────

pub fn register_nstabview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstabview_class_addr();
    let metaclass_addr = nsobj::nstabview_metaclass_addr();
    let info = ClassInfo {
        name: "NSTabView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("numberOfTabViewItems", nstabview::shim_nstabview_number_of_tab_view_items as u64, "q@:"),
            me("delegate", nstabview::shim_nstabview_delegate as u64, "@@:"),
            me("setDelegate:", nstabview::shim_nstabview_set_delegate as u64, "v@:@"),
            me("selectTabViewItemAtIndex:", nstabview::shim_nstabview_select_tab_view_item_at_index as u64, "v@:q"),
            me("selectedTabViewItem", nstabview::shim_nstabview_selected_tab_view_item as u64, "@@:"),
            me("addTabViewItem:", nstabview::shim_nstabview_add_tab_view_item as u64, "v@:@"),
            me("removeTabViewItem:", nstabview::shim_nstabview_remove_tab_view_item as u64, "v@:@"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nstabview::TAB_VIEW_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTabView");
}

// ── NSTabViewItem ──────────────────────────────────────────────

pub fn register_nstabviewitem_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstabviewitem_class_addr();
    let metaclass_addr = nsobj::nstabviewitem_metaclass_addr();
    let info = ClassInfo {
        name: "NSTabViewItem".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("label", nstabviewitem::shim_nstabviewitem_label as u64, "@@:"),
            me("setLabel:", nstabviewitem::shim_nstabviewitem_set_label as u64, "v@:@"),
            me("view", nstabviewitem::shim_nstabviewitem_view as u64, "@@:"),
            me("setView:", nstabviewitem::shim_nstabviewitem_set_view as u64, "v@:@"),
            me("identifier", nstabviewitem::shim_nstabviewitem_identifier as u64, "@@:"),
            me("setIdentifier:", nstabviewitem::shim_nstabviewitem_set_identifier as u64, "v@:@"),
            me("initWithIdentifier:", nstabviewitem::shim_nstabviewitem_init_with_identifier as u64, "@@:@"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nstabviewitem::TAB_VIEW_ITEM_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTabViewItem");
}

// ── NSToolbar ──────────────────────────────────────────────────

pub fn register_nstoolbar_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstoolbar_class_addr();
    let metaclass_addr = nsobj::nstoolbar_metaclass_addr();
    let info = ClassInfo {
        name: "NSToolbar".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithIdentifier:", nstoolbar::shim_nstoolbar_init_with_identifier as u64, "@@:@"),
            me("identifier", nstoolbar::shim_nstoolbar_identifier as u64, "@@:"),
            me("delegate", nstoolbar::shim_nstoolbar_delegate as u64, "@@:"),
            me("setDelegate:", nstoolbar::shim_nstoolbar_set_delegate as u64, "v@:@"),
            me("isVisible", nstoolbar::shim_nstoolbar_is_visible as u64, "B@:"),
            me("setVisible:", nstoolbar::shim_nstoolbar_set_visible as u64, "v@:B"),
            me("allowsUserCustomization", nstoolbar::shim_nstoolbar_allows_user_customization as u64, "B@:"),
            me("setAllowsUserCustomization:", nstoolbar::shim_nstoolbar_set_allows_user_customization as u64, "v@:B"),
            me("items", nstoolbar::shim_nstoolbar_items as u64, "@@:"),
            me("visibleItems", nstoolbar::shim_nstoolbar_visible_items as u64, "@@:"),
            me("validateVisibleItems", nstoolbar::shim_nstoolbar_validate_visible_items as u64, "v@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nstoolbar::TOOLBAR_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSToolbar");
}

// ── NSToolbarItem ──────────────────────────────────────────────

pub fn register_nstoolbaritem_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstoolbaritem_class_addr();
    let metaclass_addr = nsobj::nstoolbaritem_metaclass_addr();
    let info = ClassInfo {
        name: "NSToolbarItem".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithItemIdentifier:", nstoolbaritem::shim_nstoolbaritem_init_with_item_identifier as u64, "@@:@"),
            me("itemIdentifier", nstoolbaritem::shim_nstoolbaritem_item_identifier as u64, "@@:"),
            me("label", nstoolbaritem::shim_nstoolbaritem_label as u64, "@@:"),
            me("setLabel:", nstoolbaritem::shim_nstoolbaritem_set_label as u64, "v@:@"),
            me("image", nstoolbaritem::shim_nstoolbaritem_image as u64, "@@:"),
            me("setImage:", nstoolbaritem::shim_nstoolbaritem_set_image as u64, "v@:@"),
            me("target", nstoolbaritem::shim_nstoolbaritem_target as u64, "@@:"),
            me("setTarget:", nstoolbaritem::shim_nstoolbaritem_set_target as u64, "v@:@"),
            me("action", nstoolbaritem::shim_nstoolbaritem_action as u64, ":@:"),
            me("setAction:", nstoolbaritem::shim_nstoolbaritem_set_action as u64, "v@::"),
            me("isEnabled", nstoolbaritem::shim_nstoolbaritem_is_enabled as u64, "B@:"),
            me("setEnabled:", nstoolbaritem::shim_nstoolbaritem_set_enabled as u64, "v@:B"),
            me("validate", nstoolbaritem::shim_nstoolbaritem_validate as u64, "v@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nstoolbaritem::TOOLBAR_ITEM_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSToolbarItem");
}
