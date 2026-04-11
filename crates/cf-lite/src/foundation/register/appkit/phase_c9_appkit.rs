//! AppKit GUI class method registrations (Phase C9 AppKit).
//!
//! NSButton, NSPopUpButton, NSAlert, NSPanel, NSScrollView, NSBox,
//! NSSplitView, NSStackView, NSTabView, NSTabViewItem, NSToolbar, NSToolbarItem.

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{
    nsalert, nsbox, nsbutton, nsclipview, nscolorpanel, nscursor, nsobj, nspanel,
    nspopupbutton, nsscrollview, nssplitview, nsstackview, nsstepper, nstabview,
    nstabviewitem, nstextview, nstoolbar, nstoolbaritem, nstrackingarea,
};
#[cfg(target_os = "macos")]
use crate::nsopenpanel;

// ── NSButton ───────────────────────────────────────────────────

pub fn register_nsbutton_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsbutton_class_addr();
    let metaclass_addr = nsobj::nsbutton_metaclass_addr();
    let info = ClassInfo {
        name: "NSButton".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("title", nsbutton::shim_nsbutton_title as *const () as u64, "@@:"),
            me("setTitle:", nsbutton::shim_nsbutton_set_title as *const () as u64, "v@:@"),
            me("alternateTitle", nsbutton::shim_nsbutton_alternate_title as *const () as u64, "@@:"),
            me("setAlternateTitle:", nsbutton::shim_nsbutton_set_alternate_title as *const () as u64, "v@:@"),
            me("image", nsbutton::shim_nsbutton_image as *const () as u64, "@@:"),
            me("setImage:", nsbutton::shim_nsbutton_set_image as *const () as u64, "v@:@"),
            me("target", nsbutton::shim_nsbutton_target as *const () as u64, "@@:"),
            me("setTarget:", nsbutton::shim_nsbutton_set_target as *const () as u64, "v@:@"),
            me("action", nsbutton::shim_nsbutton_action as *const () as u64, ":@:"),
            me("setAction:", nsbutton::shim_nsbutton_set_action as *const () as u64, "v@::"),
            me("state", nsbutton::shim_nsbutton_state as *const () as u64, "q@:"),
            me("setState:", nsbutton::shim_nsbutton_set_state as *const () as u64, "v@:q"),
            me("isEnabled", nsbutton::shim_nsbutton_is_enabled as *const () as u64, "B@:"),
            me("setEnabled:", nsbutton::shim_nsbutton_set_enabled as *const () as u64, "v@:B"),
            me("isBordered", nsbutton::shim_nsbutton_is_bordered as *const () as u64, "B@:"),
            me("setBordered:", nsbutton::shim_nsbutton_set_bordered as *const () as u64, "v@:B"),
            me("allowsMixedState", nsbutton::shim_nsbutton_allows_mixed_state as *const () as u64, "B@:"),
            me("setAllowsMixedState:", nsbutton::shim_nsbutton_set_allows_mixed_state as *const () as u64, "v@:B"),
            me("setButtonType:", nsbutton::shim_nsbutton_set_button_type as *const () as u64, "v@:Q"),
            me("setBezelStyle:", nsbutton::shim_nsbutton_set_bezel_style as *const () as u64, "v@:Q"),
            me("highlight:", nsbutton::shim_nsbutton_highlight as *const () as u64, "v@:B"),
            me("performClick:", nsbutton::shim_nsbutton_perform_click as *const () as u64, "v@:@"),
            me("sizeToFit", nsbutton::shim_nsbutton_size_to_fit as *const () as u64, "v@:"),
            me("drawRect:", nsbutton::shim_nsbutton_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("mouseDown:", nsbutton::shim_nsbutton_mouse_down as *const () as u64, "v@:@"),
            me("acceptsFirstResponder", nsbutton::shim_nsbutton_accepts_first_responder as *const () as u64, "B@:"),
        ],
        class_methods: vec![
            me("buttonWithTitle:target:action:", nsbutton::shim_nsbutton_button_with_title as *const () as u64, "@@:@@:"),
            me("checkboxWithTitle:target:action:", nsbutton::shim_nsbutton_checkbox_with_title as *const () as u64, "@@:@@:"),
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
            me("numberOfItems", nspopupbutton::shim_nspopupbutton_number_of_items as *const () as u64, "q@:"),
            me("addItemWithTitle:", nspopupbutton::shim_nspopupbutton_add_item_with_title as *const () as u64, "v@:@"),
            me("removeAllItems", nspopupbutton::shim_nspopupbutton_remove_all_items as *const () as u64, "v@:"),
            me("indexOfSelectedItem", nspopupbutton::shim_nspopupbutton_index_of_selected_item as *const () as u64, "q@:"),
            me("selectItemAtIndex:", nspopupbutton::shim_nspopupbutton_select_item_at_index as *const () as u64, "v@:q"),
            me("titleOfSelectedItem", nspopupbutton::shim_nspopupbutton_title_of_selected_item as *const () as u64, "@@:"),
            me("itemTitleAtIndex:", nspopupbutton::shim_nspopupbutton_item_title_at_index as *const () as u64, "@@:q"),
            me("drawRect:", nspopupbutton::shim_nspopupbutton_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("mouseDown:", nspopupbutton::shim_nspopupbutton_mouse_down as *const () as u64, "v@:@"),
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
            me("messageText", nsalert::shim_nsalert_message_text as *const () as u64, "@@:"),
            me("setMessageText:", nsalert::shim_nsalert_set_message_text as *const () as u64, "v@:@"),
            me("informativeText", nsalert::shim_nsalert_informative_text as *const () as u64, "@@:"),
            me("setInformativeText:", nsalert::shim_nsalert_set_informative_text as *const () as u64, "v@:@"),
            me("alertStyle", nsalert::shim_nsalert_alert_style as *const () as u64, "Q@:"),
            me("setAlertStyle:", nsalert::shim_nsalert_set_alert_style as *const () as u64, "v@:Q"),
            me("addButtonWithTitle:", nsalert::shim_nsalert_add_button_with_title as *const () as u64, "v@:@"),
            me("runModal", nsalert::shim_nsalert_run_modal as *const () as u64, "q@:"),
            me("window", nsalert::shim_nsalert_window as *const () as u64, "@@:"),
            me("buttons", nsalert::shim_nsalert_buttons as *const () as u64, "@@:"),
            me("beginSheetModalForWindow:completionHandler:", nsalert::shim_nsalert_begin_sheet_modal as *const () as u64, "v@:@@"),
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
            me("isFloatingPanel", nspanel::shim_nspanel_is_floating_panel as *const () as u64, "B@:"),
            me("setFloatingPanel:", nspanel::shim_nspanel_set_floating_panel as *const () as u64, "v@:B"),
            me("becomesKeyOnlyIfNeeded", nspanel::shim_nspanel_becomes_key_only_if_needed as *const () as u64, "B@:"),
            me("setBecomesKeyOnlyIfNeeded:", nspanel::shim_nspanel_set_becomes_key_only_if_needed as *const () as u64, "v@:B"),
            me("worksWhenModal", nspanel::shim_nspanel_works_when_modal as *const () as u64, "B@:"),
            me("setWorksWhenModal:", nspanel::shim_nspanel_set_works_when_modal as *const () as u64, "v@:B"),
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
            me("documentView", nsscrollview::shim_nsscrollview_document_view as *const () as u64, "@@:"),
            me("setDocumentView:", nsscrollview::shim_nsscrollview_set_document_view as *const () as u64, "v@:@"),
            me("hasVerticalScroller", nsscrollview::shim_nsscrollview_has_vertical_scroller as *const () as u64, "B@:"),
            me("setHasVerticalScroller:", nsscrollview::shim_nsscrollview_set_has_vertical_scroller as *const () as u64, "v@:B"),
            me("hasHorizontalScroller", nsscrollview::shim_nsscrollview_has_horizontal_scroller as *const () as u64, "B@:"),
            me("setHasHorizontalScroller:", nsscrollview::shim_nsscrollview_set_has_horizontal_scroller as *const () as u64, "v@:B"),
            me("autohidesScrollers", nsscrollview::shim_nsscrollview_autohides_scrollers as *const () as u64, "B@:"),
            me("setAutohidesScrollers:", nsscrollview::shim_nsscrollview_set_autohides_scrollers as *const () as u64, "v@:B"),
            me("drawsBackground", nsscrollview::shim_nsscrollview_draws_background as *const () as u64, "B@:"),
            me("setDrawsBackground:", nsscrollview::shim_nsscrollview_set_draws_background as *const () as u64, "v@:B"),
            me("contentSize", nsscrollview::shim_nsscrollview_content_size as *const () as u64, "@@:"),
            me("tile", nsscrollview::shim_nsscrollview_tile as *const () as u64, "v@:"),
            me("contentView", nsscrollview::shim_nsscrollview_content_view as *const () as u64, "@@:"),
            me("drawRect:", nsscrollview::shim_nsscrollview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("scrollWheel:", nsscrollview::shim_nsscrollview_scroll_wheel as *const () as u64, "v@:@"),
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
            me("title", nsbox::shim_nsbox_title as *const () as u64, "@@:"),
            me("setTitle:", nsbox::shim_nsbox_set_title as *const () as u64, "v@:@"),
            me("contentView", nsbox::shim_nsbox_content_view as *const () as u64, "@@:"),
            me("setContentView:", nsbox::shim_nsbox_set_content_view as *const () as u64, "v@:@"),
            me("boxType", nsbox::shim_nsbox_box_type as *const () as u64, "Q@:"),
            me("setBoxType:", nsbox::shim_nsbox_set_box_type as *const () as u64, "v@:Q"),
            me("isTransparent", nsbox::shim_nsbox_is_transparent as *const () as u64, "B@:"),
            me("setTransparent:", nsbox::shim_nsbox_set_transparent as *const () as u64, "v@:B"),
            me("sizeToFit", nsbox::shim_nsbox_size_to_fit as *const () as u64, "v@:"),
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
            me("isVertical", nssplitview::shim_nssplitview_is_vertical as *const () as u64, "B@:"),
            me("setVertical:", nssplitview::shim_nssplitview_set_vertical as *const () as u64, "v@:B"),
            me("delegate", nssplitview::shim_nssplitview_delegate as *const () as u64, "@@:"),
            me("setDelegate:", nssplitview::shim_nssplitview_set_delegate as *const () as u64, "v@:@"),
            me("dividerStyle", nssplitview::shim_nssplitview_divider_style as *const () as u64, "Q@:"),
            me("setDividerStyle:", nssplitview::shim_nssplitview_set_divider_style as *const () as u64, "v@:Q"),
            me("adjustSubviews", nssplitview::shim_nssplitview_adjust_subviews as *const () as u64, "v@:"),
            me("dividerThickness", nssplitview::shim_nssplitview_divider_thickness as *const () as u64, "d@:"),
            me("drawRect:", nssplitview::shim_nssplitview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("mouseDown:", nssplitview::shim_nssplitview_mouse_down as *const () as u64, "v@:@"),
            me("mouseDragged:", nssplitview::shim_nssplitview_mouse_dragged as *const () as u64, "v@:@"),
            me("mouseUp:", nssplitview::shim_nssplitview_mouse_up as *const () as u64, "v@:@"),
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
            me("orientation", nsstackview::shim_nsstackview_orientation as *const () as u64, "Q@:"),
            me("setOrientation:", nsstackview::shim_nsstackview_set_orientation as *const () as u64, "v@:Q"),
            me("spacing", nsstackview::shim_nsstackview_spacing as *const () as u64, "d@:"),
            me("setSpacing:", nsstackview::shim_nsstackview_set_spacing as *const () as u64, "v@:d"),
            me("distribution", nsstackview::shim_nsstackview_distribution as *const () as u64, "Q@:"),
            me("setDistribution:", nsstackview::shim_nsstackview_set_distribution as *const () as u64, "v@:Q"),
            me("alignment", nsstackview::shim_nsstackview_alignment as *const () as u64, "Q@:"),
            me("setAlignment:", nsstackview::shim_nsstackview_set_alignment as *const () as u64, "v@:Q"),
            me("addArrangedSubview:", nsstackview::shim_nsstackview_add_arranged_subview as *const () as u64, "v@:@"),
            me("removeArrangedSubview:", nsstackview::shim_nsstackview_remove_arranged_subview as *const () as u64, "v@:@"),
            me("arrangedSubviews", nsstackview::shim_nsstackview_arranged_subviews as *const () as u64, "@@:"),
            me("insertArrangedSubview:atIndex:", nsstackview::shim_nsstackview_insert_arranged_subview as *const () as u64, "v@:@Q"),
        ],
        class_methods: vec![
            me("stackViewWithViews:", nsstackview::shim_nsstackview_stack_view_with_views as *const () as u64, "@@:@"),
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
            me("numberOfTabViewItems", nstabview::shim_nstabview_number_of_tab_view_items as *const () as u64, "q@:"),
            me("delegate", nstabview::shim_nstabview_delegate as *const () as u64, "@@:"),
            me("setDelegate:", nstabview::shim_nstabview_set_delegate as *const () as u64, "v@:@"),
            me("selectTabViewItemAtIndex:", nstabview::shim_nstabview_select_tab_view_item_at_index as *const () as u64, "v@:q"),
            me("selectedTabViewItem", nstabview::shim_nstabview_selected_tab_view_item as *const () as u64, "@@:"),
            me("tabViewItemAtIndex:", nstabview::shim_nstabview_tab_view_item_at_index as *const () as u64, "@@:q"),
            me("indexOfTabViewItem:", nstabview::shim_nstabview_index_of_tab_view_item as *const () as u64, "q@:@"),
            me("addTabViewItem:", nstabview::shim_nstabview_add_tab_view_item as *const () as u64, "v@:@"),
            me("removeTabViewItem:", nstabview::shim_nstabview_remove_tab_view_item as *const () as u64, "v@:@"),
            me("drawRect:", nstabview::shim_nstabview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("mouseDown:", nstabview::shim_nstabview_mouse_down as *const () as u64, "v@:@"),
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
            me("label", nstabviewitem::shim_nstabviewitem_label as *const () as u64, "@@:"),
            me("setLabel:", nstabviewitem::shim_nstabviewitem_set_label as *const () as u64, "v@:@"),
            me("view", nstabviewitem::shim_nstabviewitem_view as *const () as u64, "@@:"),
            me("setView:", nstabviewitem::shim_nstabviewitem_set_view as *const () as u64, "v@:@"),
            me("identifier", nstabviewitem::shim_nstabviewitem_identifier as *const () as u64, "@@:"),
            me("setIdentifier:", nstabviewitem::shim_nstabviewitem_set_identifier as *const () as u64, "v@:@"),
            me("initWithIdentifier:", nstabviewitem::shim_nstabviewitem_init_with_identifier as *const () as u64, "@@:@"),
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
            me("initWithIdentifier:", nstoolbar::shim_nstoolbar_init_with_identifier as *const () as u64, "@@:@"),
            me("identifier", nstoolbar::shim_nstoolbar_identifier as *const () as u64, "@@:"),
            me("delegate", nstoolbar::shim_nstoolbar_delegate as *const () as u64, "@@:"),
            me("setDelegate:", nstoolbar::shim_nstoolbar_set_delegate as *const () as u64, "v@:@"),
            me("isVisible", nstoolbar::shim_nstoolbar_is_visible as *const () as u64, "B@:"),
            me("setVisible:", nstoolbar::shim_nstoolbar_set_visible as *const () as u64, "v@:B"),
            me("allowsUserCustomization", nstoolbar::shim_nstoolbar_allows_user_customization as *const () as u64, "B@:"),
            me("setAllowsUserCustomization:", nstoolbar::shim_nstoolbar_set_allows_user_customization as *const () as u64, "v@:B"),
            me("items", nstoolbar::shim_nstoolbar_items as *const () as u64, "@@:"),
            me("visibleItems", nstoolbar::shim_nstoolbar_visible_items as *const () as u64, "@@:"),
            me("validateVisibleItems", nstoolbar::shim_nstoolbar_validate_visible_items as *const () as u64, "v@:"),
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
            me("initWithItemIdentifier:", nstoolbaritem::shim_nstoolbaritem_init_with_item_identifier as *const () as u64, "@@:@"),
            me("itemIdentifier", nstoolbaritem::shim_nstoolbaritem_item_identifier as *const () as u64, "@@:"),
            me("label", nstoolbaritem::shim_nstoolbaritem_label as *const () as u64, "@@:"),
            me("setLabel:", nstoolbaritem::shim_nstoolbaritem_set_label as *const () as u64, "v@:@"),
            me("image", nstoolbaritem::shim_nstoolbaritem_image as *const () as u64, "@@:"),
            me("setImage:", nstoolbaritem::shim_nstoolbaritem_set_image as *const () as u64, "v@:@"),
            me("target", nstoolbaritem::shim_nstoolbaritem_target as *const () as u64, "@@:"),
            me("setTarget:", nstoolbaritem::shim_nstoolbaritem_set_target as *const () as u64, "v@:@"),
            me("action", nstoolbaritem::shim_nstoolbaritem_action as *const () as u64, ":@:"),
            me("setAction:", nstoolbaritem::shim_nstoolbaritem_set_action as *const () as u64, "v@::"),
            me("isEnabled", nstoolbaritem::shim_nstoolbaritem_is_enabled as *const () as u64, "B@:"),
            me("setEnabled:", nstoolbaritem::shim_nstoolbaritem_set_enabled as *const () as u64, "v@:B"),
            me("validate", nstoolbaritem::shim_nstoolbaritem_validate as *const () as u64, "v@:"),
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

// ── NSSavePanel ────────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub fn register_nssavepanel_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nssavepanel_class_addr();
    let metaclass_addr = nsobj::nssavepanel_metaclass_addr();
    let info = ClassInfo {
        name: "NSSavePanel".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("title", nsopenpanel::shim_nssavepanel_title as *const () as u64, "@@:"),
            me("setTitle:", nsopenpanel::shim_nssavepanel_set_title as *const () as u64, "v@:@"),
            me("canCreateDirectories", nsopenpanel::shim_nssavepanel_can_create_directories as *const () as u64, "B@:"),
            me("setCanCreateDirectories:", nsopenpanel::shim_nssavepanel_set_can_create_directories as *const () as u64, "v@:B"),
            me("showsHiddenFiles", nsopenpanel::shim_nssavepanel_shows_hidden_files as *const () as u64, "B@:"),
            me("setShowsHiddenFiles:", nsopenpanel::shim_nssavepanel_set_shows_hidden_files as *const () as u64, "v@:B"),
            me("allowedContentTypes", nsopenpanel::shim_nssavepanel_allowed_content_types as *const () as u64, "@@:"),
            me("setAllowedContentTypes:", nsopenpanel::shim_nssavepanel_set_allowed_content_types as *const () as u64, "v@:@"),
            me("URL", nsopenpanel::shim_nssavepanel_url as *const () as u64, "@@:"),
            me("runModal", nsopenpanel::shim_nssavepanel_run_modal as *const () as u64, "q@:"),
        ],
        class_methods: vec![
            me("savePanel", nsopenpanel::shim_nssavepanel_save_panel as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsopenpanel::PANEL_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nspanel_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSSavePanel");
}

// ── NSOpenPanel ────────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub fn register_nsopenpanel_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsopenpanel_class_addr();
    let metaclass_addr = nsobj::nsopenpanel_metaclass_addr();
    let info = ClassInfo {
        name: "NSOpenPanel".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("canChooseFiles", nsopenpanel::shim_nsopenpanel_can_choose_files as *const () as u64, "B@:"),
            me("setCanChooseFiles:", nsopenpanel::shim_nsopenpanel_set_can_choose_files as *const () as u64, "v@:B"),
            me("canChooseDirectories", nsopenpanel::shim_nsopenpanel_can_choose_directories as *const () as u64, "B@:"),
            me("setCanChooseDirectories:", nsopenpanel::shim_nsopenpanel_set_can_choose_directories as *const () as u64, "v@:B"),
            me("allowsMultipleSelection", nsopenpanel::shim_nsopenpanel_allows_multiple_selection as *const () as u64, "B@:"),
            me("setAllowsMultipleSelection:", nsopenpanel::shim_nsopenpanel_set_allows_multiple_selection as *const () as u64, "v@:B"),
            me("URLs", nsopenpanel::shim_nsopenpanel_urls as *const () as u64, "@@:"),
            me("runModal", nsopenpanel::shim_nsopenpanel_run_modal as *const () as u64, "q@:"),
        ],
        class_methods: vec![
            me("openPanel", nsopenpanel::shim_nsopenpanel_open_panel as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsopenpanel::PANEL_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nspanel_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSOpenPanel");
}

// ── NSClipView ─────────────────────────────────────────────────

pub fn register_nsclipview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsclipview_class_addr();
    let metaclass_addr = nsobj::nsclipview_metaclass_addr();
    let info = ClassInfo {
        name: "NSClipView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsclipview::shim_nsclipview_init as *const () as u64, "@@:"),
            me("documentView", nsclipview::shim_nsclipview_document_view as *const () as u64, "@@:"),
            me("setDocumentView:", nsclipview::shim_nsclipview_set_document_view as *const () as u64, "v@:@"),
            me("scrollToPoint:", nsclipview::shim_nsclipview_scroll_to_point as *const () as u64, "v@:{CGPoint=dd}"),
            me("drawRect:", nsclipview::shim_nsclipview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsclipview::CLIP_VIEW_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSClipView");
}

// ── NSTextView ─────────────────────────────────────────────────

pub fn register_nstextview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstextview_class_addr();
    let metaclass_addr = nsobj::nstextview_metaclass_addr();
    let info = ClassInfo {
        name: "NSTextView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithFrame:", nstextview::shim_nstextview_init_with_frame as *const () as u64, "@@:{CGRect=dddd}"),
            me("string", nstextview::shim_nstextview_string as *const () as u64, "@@:"),
            me("setString:", nstextview::shim_nstextview_set_string as *const () as u64, "v@:@"),
            me("delegate", nstextview::shim_nstextview_delegate as *const () as u64, "@@:"),
            me("setDelegate:", nstextview::shim_nstextview_set_delegate as *const () as u64, "v@:@"),
            me("isEditable", nstextview::shim_nstextview_is_editable as *const () as u64, "B@:"),
            me("setEditable:", nstextview::shim_nstextview_set_editable as *const () as u64, "v@:B"),
            me("isSelectable", nstextview::shim_nstextview_is_selectable as *const () as u64, "B@:"),
            me("setSelectable:", nstextview::shim_nstextview_set_selectable as *const () as u64, "v@:B"),
            me("isRichText", nstextview::shim_nstextview_is_rich_text as *const () as u64, "B@:"),
            me("setRichText:", nstextview::shim_nstextview_set_rich_text as *const () as u64, "v@:B"),
            me("isFieldEditor", nstextview::shim_nstextview_is_field_editor as *const () as u64, "B@:"),
            me("setFieldEditor:", nstextview::shim_nstextview_set_field_editor as *const () as u64, "v@:B"),
            me("usesFontPanel", nstextview::shim_nstextview_uses_font_panel as *const () as u64, "B@:"),
            me("setUsesFontPanel:", nstextview::shim_nstextview_set_uses_font_panel as *const () as u64, "v@:B"),
            me("allowsUndo", nstextview::shim_nstextview_allows_undo as *const () as u64, "B@:"),
            me("setAllowsUndo:", nstextview::shim_nstextview_set_allows_undo as *const () as u64, "v@:B"),
            me("selectedRange", nstextview::shim_nstextview_selected_range as *const () as u64, "{NSRange=QQ}@:"),
            me("setSelectedRange:", nstextview::shim_nstextview_set_selected_range as *const () as u64, "v@:{NSRange=QQ}"),
            me("acceptsFirstResponder", nstextview::shim_nstextview_accepts_first_responder as *const () as u64, "B@:"),
            me("insertText:", nstextview::shim_nstextview_insert_text as *const () as u64, "v@:@"),
            me("keyDown:", nstextview::shim_nstextview_key_down as *const () as u64, "v@:@"),
            me("mouseDown:", nstextview::shim_nstextview_mouse_down as *const () as u64, "v@:@"),
            me("mouseDragged:", nstextview::shim_nstextview_mouse_dragged as *const () as u64, "v@:@"),
            me("drawRect:", nstextview::shim_nstextview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("textStorage", nstextview::shim_nstextview_text_storage as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nstextview::TEXT_VIEW_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTextView");
}

// ── NSTrackingArea ─────────────────────────────────────────────

pub fn register_nstrackingarea_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstrackingarea_class_addr();
    let metaclass_addr = nsobj::nstrackingarea_metaclass_addr();
    let info = ClassInfo {
        name: "NSTrackingArea".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithRect:options:owner:userInfo:", nstrackingarea::shim_nstrackingarea_init as *const () as u64, "@@:{CGRect=dddd}Q@@"),
            me("rect", nstrackingarea::shim_nstrackingarea_rect_x as *const () as u64, "{CGRect={CGPoint=dd}{CGSize=dd}}@:"),
            me("options", nstrackingarea::shim_nstrackingarea_options as *const () as u64, "Q@:"),
            me("owner", nstrackingarea::shim_nstrackingarea_owner as *const () as u64, "@@:"),
            me("userInfo", nstrackingarea::shim_nstrackingarea_user_info as *const () as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nstrackingarea::TRACKING_AREA_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTrackingArea");
}

// ── NSCursor ───────────────────────────────────────────────────

pub fn register_nscursor_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscursor_class_addr();
    let metaclass_addr = nsobj::nscursor_metaclass_addr();
    let info = ClassInfo {
        name: "NSCursor".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("set", nscursor::shim_nscursor_set as *const () as u64, "v@:"),
            me("push", nscursor::shim_nscursor_push as *const () as u64, "v@:"),
            me("image", nscursor::shim_nscursor_image as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("arrowCursor", nscursor::shim_nscursor_arrow_cursor as *const () as u64, "@@:"),
            me("IBeamCursor", nscursor::shim_nscursor_ibeam_cursor as *const () as u64, "@@:"),
            me("crosshairCursor", nscursor::shim_nscursor_crosshair_cursor as *const () as u64, "@@:"),
            me("pointingHandCursor", nscursor::shim_nscursor_pointing_hand_cursor as *const () as u64, "@@:"),
            me("openHandCursor", nscursor::shim_nscursor_open_hand_cursor as *const () as u64, "@@:"),
            me("closedHandCursor", nscursor::shim_nscursor_closed_hand_cursor as *const () as u64, "@@:"),
            me("resizeLeftRightCursor", nscursor::shim_nscursor_resize_left_right_cursor as *const () as u64, "@@:"),
            me("resizeUpDownCursor", nscursor::shim_nscursor_resize_up_down_cursor as *const () as u64, "@@:"),
            me("resizeLeftCursor", nscursor::shim_nscursor_resize_left_cursor as *const () as u64, "@@:"),
            me("resizeRightCursor", nscursor::shim_nscursor_resize_right_cursor as *const () as u64, "@@:"),
            me("resizeUpCursor", nscursor::shim_nscursor_resize_up_cursor as *const () as u64, "@@:"),
            me("resizeDownCursor", nscursor::shim_nscursor_resize_down_cursor as *const () as u64, "@@:"),
            me("disappearingItemCursor", nscursor::shim_nscursor_disappearing_item_cursor as *const () as u64, "@@:"),
            me("operationNotAllowedCursor", nscursor::shim_nscursor_operation_not_allowed_cursor as *const () as u64, "@@:"),
            me("dragLinkCursor", nscursor::shim_nscursor_drag_link_cursor as *const () as u64, "@@:"),
            me("dragCopyCursor", nscursor::shim_nscursor_drag_copy_cursor as *const () as u64, "@@:"),
            me("contextualMenuCursor", nscursor::shim_nscursor_contextual_menu_cursor as *const () as u64, "@@:"),
            me("currentCursor", nscursor::shim_nscursor_current_cursor as *const () as u64, "@@:"),
            me("hide", nscursor::shim_nscursor_hide as *const () as u64, "v@:"),
            me("unhide", nscursor::shim_nscursor_unhide as *const () as u64, "v@:"),
            me("pop", nscursor::shim_nscursor_pop as *const () as u64, "v@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nscursor::CURSOR_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCursor");
}

// ── NSColorPanel ───────────────────────────────────────────────

pub fn register_nscolorpanel_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscolorpanel_class_addr();
    let metaclass_addr = nsobj::nscolorpanel_metaclass_addr();
    let info = ClassInfo {
        name: "NSColorPanel".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("orderFront:", nscolorpanel::shim_nscolorpanel_order_front as *const () as u64, "v@:@"),
            me("close", nscolorpanel::shim_nscolorpanel_close as *const () as u64, "v@:"),
            me("color", nscolorpanel::shim_nscolorpanel_color as *const () as u64, "@@:"),
        ],
        class_methods: vec![
            me("sharedColorPanel", nscolorpanel::shim_nscolorpanel_shared as *const () as u64, "@@:"),
            me("sharedColorPanelExists", nscolorpanel::shim_nscolorpanel_shared_exists as *const () as u64, "c@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 8, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nspanel_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSColorPanel");
}

// ── NSFontPanel ────────────────────────────────────────────────

pub fn register_nsfontpanel_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsfontpanel_class_addr();
    let metaclass_addr = nsobj::nsfontpanel_metaclass_addr();
    let info = ClassInfo {
        name: "NSFontPanel".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("orderFront:", nscolorpanel::shim_nsfontpanel_order_front as *const () as u64, "v@:@"),
            me("close", nscolorpanel::shim_nsfontpanel_close as *const () as u64, "v@:"),
        ],
        class_methods: vec![
            me("sharedFontPanel", nscolorpanel::shim_nsfontpanel_shared as *const () as u64, "@@:"),
            me("sharedFontPanelExists", nscolorpanel::shim_nsfontpanel_shared_exists as *const () as u64, "c@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 8, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nspanel_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSFontPanel");
}

// ── NSStepper ──────────────────────────────────────────────────

pub fn register_nsstepper_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsstepper_class_addr();
    let metaclass_addr = nsobj::nsstepper_metaclass_addr();
    let info = ClassInfo {
        name: "NSStepper".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithFrame:", nsstepper::shim_nsstepper_init_with_frame as *const () as u64, "@@:{CGRect={CGPoint=dd}{CGSize=dd}}"),
            me("minValue", nsstepper::shim_nsstepper_min_value as *const () as u64, "d@:"),
            me("setMinValue:", nsstepper::shim_nsstepper_set_min_value as *const () as u64, "v@:d"),
            me("maxValue", nsstepper::shim_nsstepper_max_value as *const () as u64, "d@:"),
            me("setMaxValue:", nsstepper::shim_nsstepper_set_max_value as *const () as u64, "v@:d"),
            me("increment", nsstepper::shim_nsstepper_increment as *const () as u64, "d@:"),
            me("setIncrement:", nsstepper::shim_nsstepper_set_increment as *const () as u64, "v@:d"),
            me("autorepeat", nsstepper::shim_nsstepper_autorepeat as *const () as u64, "c@:"),
            me("setAutorepeat:", nsstepper::shim_nsstepper_set_autorepeat as *const () as u64, "v@:c"),
            me("valueWraps", nsstepper::shim_nsstepper_value_wraps as *const () as u64, "c@:"),
            me("setValueWraps:", nsstepper::shim_nsstepper_set_value_wraps as *const () as u64, "v@:c"),
            me("integerValue", nsstepper::shim_nsstepper_integer_value as *const () as u64, "q@:"),
            me("setIntegerValue:", nsstepper::shim_nsstepper_set_integer_value as *const () as u64, "v@:q"),
            me("doubleValue", nsstepper::shim_nsstepper_double_value as *const () as u64, "d@:"),
            me("setDoubleValue:", nsstepper::shim_nsstepper_set_double_value as *const () as u64, "v@:d"),
            me("drawRect:", nsstepper::shim_nsstepper_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("mouseDown:", nsstepper::shim_nsstepper_mouse_down as *const () as u64, "v@:@"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsstepper::STEPPER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nscontrol_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSStepper");
}
