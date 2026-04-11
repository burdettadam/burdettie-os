//! AppKit Phase C12: Safe area / layout margins, NSTraitCollection, NSAnimationContext,
//! NSTableView data source, and NSUndoManager registrations.

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{
    nsanimationcontext, nsobj, nsoutlineview, nstableview, nstraitcollection, nsundomanager, nsview,
};

// ── NSTraitCollection ──────────────────────────────────────────

pub fn register_nstraitcollection_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstraitcollection_class_addr();
    let metaclass_addr = nsobj::nstraitcollection_metaclass_addr();
    let info = ClassInfo {
        name: "NSTraitCollection".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("horizontalSizeClass", nstraitcollection::shim_nstraitcollection_horizontal_size_class as *const () as u64, "Q@:"),
            me("verticalSizeClass", nstraitcollection::shim_nstraitcollection_vertical_size_class as *const () as u64, "Q@:"),
            me("userInterfaceIdiom", nstraitcollection::shim_nstraitcollection_user_interface_idiom as *const () as u64, "Q@:"),
            me("displayScale", nstraitcollection::shim_nstraitcollection_display_scale as *const () as u64, "d@:"),
            me("hasDifferentColorAppearanceComparedToTraitCollection:", nstraitcollection::shim_nstraitcollection_has_different_color_appearance as *const () as u64, "B@:@"),
        ],
        class_methods: vec![
            me("traitCollectionWithHorizontalSizeClass:", nstraitcollection::shim_nstraitcollection_with_horizontal_size_class as *const () as u64, "@@:Q"),
            me("traitCollectionWithVerticalSizeClass:", nstraitcollection::shim_nstraitcollection_with_vertical_size_class as *const () as u64, "@@:Q"),
            me("traitCollectionWithUserInterfaceIdiom:", nstraitcollection::shim_nstraitcollection_with_user_interface_idiom as *const () as u64, "@@:Q"),
            me("traitCollectionWithDisplayScale:", nstraitcollection::shim_nstraitcollection_with_display_scale as *const () as u64, "@@:d"),
            me("currentTraitCollection", nstraitcollection::shim_nstraitcollection_current as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nstraitcollection::TRAIT_COLLECTION_SIZE as u32,
        is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTraitCollection");
}

// ── NSAnimationContext ─────────────────────────────────────────

pub fn register_nsanimationcontext_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsanimationcontext_class_addr();
    let metaclass_addr = nsobj::nsanimationcontext_metaclass_addr();
    let info = ClassInfo {
        name: "NSAnimationContext".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("duration", nsanimationcontext::shim_nsanimationcontext_duration as *const () as u64, "d@:"),
            me("setDuration:", nsanimationcontext::shim_nsanimationcontext_set_duration as *const () as u64, "v@:d"),
            me("allowsImplicitAnimation", nsanimationcontext::shim_nsanimationcontext_allows_implicit as *const () as u64, "B@:"),
            me("setAllowsImplicitAnimation:", nsanimationcontext::shim_nsanimationcontext_set_allows_implicit as *const () as u64, "v@:B"),
            me("completionHandler", nsanimationcontext::shim_nsanimationcontext_completion_handler as *const () as u64, "@@:"),
            me("setCompletionHandler:", nsanimationcontext::shim_nsanimationcontext_set_completion_handler as *const () as u64, "v@:@"),
            me("timingFunction", nsanimationcontext::shim_nsanimationcontext_timing_function as *const () as u64, "@@:"),
            me("setTimingFunction:", nsanimationcontext::shim_nsanimationcontext_set_timing_function as *const () as u64, "v@:@"),
        ],
        class_methods: vec![
            me("beginGrouping", nsanimationcontext::shim_nsanimationcontext_begin_grouping as *const () as u64, "v@:"),
            me("endGrouping", nsanimationcontext::shim_nsanimationcontext_end_grouping as *const () as u64, "v@:"),
            me("currentContext", nsanimationcontext::shim_nsanimationcontext_current_context as *const () as u64, "@@:"),
            me("runAnimationGroup:completionHandler:", nsanimationcontext::shim_nsanimationcontext_run_animation_group as *const () as u64, "v@:@@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsanimationcontext::ANIMATION_CONTEXT_SIZE as u32,
        is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSAnimationContext");
}

// ── NSTableView ────────────────────────────────────────────────

pub fn register_nstableview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstableview_class_addr();
    let metaclass_addr = nsobj::nstableview_metaclass_addr();
    let info = ClassInfo {
        name: "NSTableView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithFrame:", nstableview::shim_nstableview_init_with_frame as *const () as u64, "@@:dddd"),
            me("dataSource", nstableview::shim_nstableview_data_source as *const () as u64, "@@:"),
            me("setDataSource:", nstableview::shim_nstableview_set_data_source as *const () as u64, "v@:@"),
            me("delegate", nstableview::shim_nstableview_delegate as *const () as u64, "@@:"),
            me("setDelegate:", nstableview::shim_nstableview_set_delegate as *const () as u64, "v@:@"),
            me("numberOfRows", nstableview::shim_nstableview_number_of_rows as *const () as u64, "Q@:"),
            me("numberOfColumns", nstableview::shim_nstableview_number_of_columns as *const () as u64, "Q@:"),
            me("addTableColumn:", nstableview::shim_nstableview_add_table_column as *const () as u64, "v@:@"),
            me("removeTableColumn:", nstableview::shim_nstableview_remove_table_column as *const () as u64, "v@:@"),
            me("tableColumns", nstableview::shim_nstableview_table_columns as *const () as u64, "@@:"),
            me("reloadData", nstableview::shim_nstableview_reload_data as *const () as u64, "v@:"),
            me("superview", nstableview::shim_nstableview_superview as *const () as u64, "@@:"),
            me("selectedRow", nstableview::shim_nstableview_selected_row as *const () as u64, "q@:"),
            me("selectRowIndexes:byExtendingSelection:", nstableview::shim_nstableview_select_row_indexes as *const () as u64, "v@:@B"),
            me("deselectAll:", nstableview::shim_nstableview_deselect_all as *const () as u64, "v@:@"),
            me("rowHeight", nstableview::shim_nstableview_row_height as *const () as u64, "d@:"),
            me("setRowHeight:", nstableview::shim_nstableview_set_row_height as *const () as u64, "v@:d"),
            me("headerView", nstableview::shim_nstableview_header_view as *const () as u64, "@@:"),
            me("drawRect:", nstableview::shim_nstableview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("mouseDown:", nstableview::shim_nstableview_mouse_down as *const () as u64, "v@:@"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nstableview::TABLE_VIEW_SIZE as u32,
        is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTableView");
}

// ── NSOutlineView ──────────────────────────────────────────────

pub fn register_nsoutlineview_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsoutlineview_class_addr();
    let metaclass_addr = nsobj::nsoutlineview_metaclass_addr();
    let info = ClassInfo {
        name: "NSOutlineView".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("reloadData", nsoutlineview::shim_nsoutlineview_reload_data as *const () as u64, "v@:"),
            me("isItemExpanded:", nsoutlineview::shim_nsoutlineview_is_item_expanded as *const () as u64, "B@:@"),
            me("expandItem:", nsoutlineview::shim_nsoutlineview_expand_item as *const () as u64, "v@:@"),
            me("collapseItem:", nsoutlineview::shim_nsoutlineview_collapse_item as *const () as u64, "v@:@"),
            me("itemAtRow:", nsoutlineview::shim_nsoutlineview_item_at_row as *const () as u64, "@@:q"),
            me("rowForItem:", nsoutlineview::shim_nsoutlineview_row_for_item as *const () as u64, "q@:@"),
            me("numberOfRows", nsoutlineview::shim_nsoutlineview_number_of_rows as *const () as u64, "Q@:"),
            me("levelForItem:", nsoutlineview::shim_nsoutlineview_level_for_item as *const () as u64, "q@:@"),
            me("levelForRow:", nsoutlineview::shim_nsoutlineview_level_for_row as *const () as u64, "q@:q"),
            me("indentationPerLevel", nsoutlineview::shim_nsoutlineview_indentation_per_level as *const () as u64, "d@:"),
            me("drawRect:", nsoutlineview::shim_nsoutlineview_draw_rect as *const () as u64, "v@:{CGRect=dddd}"),
            me("mouseDown:", nsoutlineview::shim_nsoutlineview_mouse_down as *const () as u64, "v@:@"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsoutlineview::OUTLINE_VIEW_SIZE as u32,
        is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nstableview_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSOutlineView");
}

// ── NSUndoManager ──────────────────────────────────────────────

pub fn register_nsundomanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsundomanager_class_addr();
    let metaclass_addr = nsobj::nsundomanager_metaclass_addr();
    let info = ClassInfo {
        name: "NSUndoManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("registerUndoWithTarget:selector:object:", nsundomanager::shim_nsundomanager_register_undo as *const () as u64, "v@:@:@"),
            me("undo", nsundomanager::shim_nsundomanager_undo as *const () as u64, "v@:"),
            me("redo", nsundomanager::shim_nsundomanager_redo as *const () as u64, "v@:"),
            me("canUndo", nsundomanager::shim_nsundomanager_can_undo as *const () as u64, "B@:"),
            me("canRedo", nsundomanager::shim_nsundomanager_can_redo as *const () as u64, "B@:"),
            me("undoActionName", nsundomanager::shim_nsundomanager_undo_action_name as *const () as u64, "@@:"),
            me("redoActionName", nsundomanager::shim_nsundomanager_redo_action_name as *const () as u64, "@@:"),
            me("setActionName:", nsundomanager::shim_nsundomanager_set_action_name as *const () as u64, "v@:@"),
            me("removeAllActions", nsundomanager::shim_nsundomanager_remove_all_actions as *const () as u64, "v@:"),
            me("removeAllActionsWithTarget:", nsundomanager::shim_nsundomanager_remove_all_actions_with_target as *const () as u64, "v@:@"),
            me("groupsByEvent", nsundomanager::shim_nsundomanager_groups_by_event as *const () as u64, "B@:"),
            me("setGroupsByEvent:", nsundomanager::shim_nsundomanager_set_groups_by_event as *const () as u64, "v@:B"),
            me("beginUndoGrouping", nsundomanager::shim_nsundomanager_begin_undo_grouping as *const () as u64, "v@:"),
            me("endUndoGrouping", nsundomanager::shim_nsundomanager_end_undo_grouping as *const () as u64, "v@:"),
            me("groupingLevel", nsundomanager::shim_nsundomanager_grouping_level as *const () as u64, "Q@:"),
            me("isUndoing", nsundomanager::shim_nsundomanager_is_undoing as *const () as u64, "B@:"),
            me("isRedoing", nsundomanager::shim_nsundomanager_is_redoing as *const () as u64, "B@:"),
            me("levelsOfUndo", nsundomanager::shim_nsundomanager_levels_of_undo as *const () as u64, "Q@:"),
            me("setLevelsOfUndo:", nsundomanager::shim_nsundomanager_set_levels_of_undo as *const () as u64, "v@:Q"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsundomanager::UNDO_MANAGER_SIZE as u32,
        is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSUndoManager");
}

// ── Additional NSView methods: safe area, layout margins, traitCollection ──

pub fn register_nsview_safe_area_methods(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsview_class_addr();

    let methods = vec![
        me("safeAreaLayoutGuide", nsview::shim_nsview_safe_area_layout_guide as *const () as u64, "@@:"),
        me("layoutMarginsGuide", nsview::shim_nsview_layout_margins_guide as *const () as u64, "@@:"),
        me("safeAreaInsets", nsview::shim_nsview_safe_area_insets_top as *const () as u64, "d@:"),
        me("layoutMargins", nsview::shim_nsview_layout_margins_top as *const () as u64, "d@:"),
        me("addLayoutGuide:", nsview::shim_nsview_add_layout_guide as *const () as u64, "v@:@"),
        me("removeLayoutGuide:", nsview::shim_nsview_remove_layout_guide as *const () as u64, "v@:@"),
        me("traitCollection", nstraitcollection::shim_nsview_trait_collection as *const () as u64, "@@:"),
    ];

    for m in methods {
        rt.add_method_to_class(class_addr, m);
    }
}
