//! AppKit class method registrations (Phase C5).

use super::me;
use crate::{nsappearance, nsmenu_shim, nsnib, nsobj, nsusernotificationcenter};
use objc_runtime::ClassInfo;
use std::collections::HashMap;

// ── Phase C7 AppKit classes ────────────────────────────────────

/// Register `NSAppearance` — dark mode appearance queries.
pub fn register_nsappearance_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsappearance_class_addr();
    let metaclass_addr = nsobj::nsappearance_metaclass_addr();
    let info = ClassInfo {
        name: "NSAppearance".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "name",
                nsappearance::shim_nsappearance_name as *const () as u64,
                "@@:",
            ),
            me(
                "bestMatchFromAppearancesWithNames:",
                nsappearance::shim_nsappearance_best_match as *const () as u64,
                "@@:@",
            ),
        ],
        class_methods: vec![
            me(
                "currentAppearance",
                nsappearance::shim_nsappearance_current_appearance as *const () as u64,
                "@@:",
            ),
            me(
                "currentDrawingAppearance",
                nsappearance::shim_nsappearance_current_drawing_appearance as *const () as u64,
                "@@:",
            ),
            me(
                "appearanceNamed:",
                nsappearance::shim_nsappearance_appearance_named as *const () as u64,
                "@@:@",
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
    rt.register_metaclass_addr(metaclass_addr, "NSAppearance");
}

/// Register `NSUserNotificationCenter` — deprecated notification API.
pub fn register_nsusernotificationcenter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsusernotificationcenter_class_addr();
    let metaclass_addr = nsobj::nsusernotificationcenter_metaclass_addr();
    let info = ClassInfo {
        name: "NSUserNotificationCenter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "delegate",
                nsusernotificationcenter::shim_nsusernotificationcenter_delegate as *const ()
                    as u64,
                "@@:",
            ),
            me(
                "setDelegate:",
                nsusernotificationcenter::shim_nsusernotificationcenter_set_delegate as *const ()
                    as u64,
                "v@:@",
            ),
            me(
                "deliverNotification:",
                nsusernotificationcenter::shim_nsusernotificationcenter_deliver as *const () as u64,
                "v@:@",
            ),
            me(
                "removeDeliveredNotification:",
                nsusernotificationcenter::shim_nsusernotificationcenter_remove as *const () as u64,
                "v@:@",
            ),
            me(
                "removeAllDeliveredNotifications",
                nsusernotificationcenter::shim_nsusernotificationcenter_remove_all as *const ()
                    as u64,
                "v@:",
            ),
            me(
                "deliveredNotifications",
                nsusernotificationcenter::shim_nsusernotificationcenter_delivered_notifications
                    as *const () as u64,
                "@@:",
            ),
            me(
                "scheduledNotifications",
                nsusernotificationcenter::shim_nsusernotificationcenter_scheduled_notifications
                    as *const () as u64,
                "@@:",
            ),
            me(
                "setScheduledNotifications:",
                nsusernotificationcenter::shim_nsusernotificationcenter_set_scheduled as *const ()
                    as u64,
                "v@:@",
            ),
            me(
                "scheduleNotification:",
                nsusernotificationcenter::shim_nsusernotificationcenter_schedule as *const ()
                    as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![me(
            "defaultUserNotificationCenter",
            nsusernotificationcenter::shim_nsusernotificationcenter_default as *const () as u64,
            "@@:",
        )],
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
    rt.register_metaclass_addr(metaclass_addr, "NSUserNotificationCenter");
}

/// Register `NSMenu` — menu creation stubs.
pub fn register_nsmenu_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmenu_class_addr();
    let metaclass_addr = nsobj::nsmenu_metaclass_addr();
    let info = ClassInfo {
        name: "NSMenu".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithTitle:",
                nsmenu_shim::shim_nsmenu_init_with_title as *const () as u64,
                "@@:@",
            ),
            me(
                "addItem:",
                nsmenu_shim::shim_nsmenu_add_item as *const () as u64,
                "v@:@",
            ),
            me(
                "addItemWithTitle:action:keyEquivalent:",
                nsmenu_shim::shim_nsmenu_add_item_with_title as *const () as u64,
                "@@:@:@",
            ),
            me(
                "numberOfItems",
                nsmenu_shim::shim_nsmenu_number_of_items as *const () as u64,
                "q@:",
            ),
            me(
                "itemAtIndex:",
                nsmenu_shim::shim_nsmenu_item_at_index as *const () as u64,
                "@@:Q",
            ),
            me(
                "itemWithTitle:",
                nsmenu_shim::shim_nsmenu_item_with_title as *const () as u64,
                "@@:@",
            ),
            me(
                "indexOfItemWithRepresentedObject:",
                nsmenu_shim::shim_nsmenu_index_of_item_with_represented_object as *const () as u64,
                "q@:@",
            ),
            me(
                "itemArray",
                nsmenu_shim::shim_nsmenu_item_array as *const () as u64,
                "@@:",
            ),
            me(
                "removeAllItems",
                nsmenu_shim::shim_nsmenu_remove_all_items as *const () as u64,
                "v@:",
            ),
            me(
                "insertItem:atIndex:",
                nsmenu_shim::shim_nsmenu_insert_item as *const () as u64,
                "v@:@q",
            ),
            me(
                "removeItemAtIndex:",
                nsmenu_shim::shim_nsmenu_remove_item_at_index as *const () as u64,
                "v@:q",
            ),
            me(
                "title",
                nsmenu_shim::shim_nsmenu_title as *const () as u64,
                "@@:",
            ),
            me(
                "setTitle:",
                nsmenu_shim::shim_nsmenu_set_title as *const () as u64,
                "v@:@",
            ),
            me(
                "setSubmenu:forItem:",
                nsmenu_shim::shim_nsmenu_set_submenu as *const () as u64,
                "v@:@@",
            ),
            me(
                "performKeyEquivalent:",
                nsmenu_shim::shim_nsmenu_perform_key_equivalent as *const () as u64,
                "B@:@",
            ),
        ],
        class_methods: vec![me(
            "popUpContextMenu:withEvent:forView:",
            nsmenu_shim::shim_nsmenu_popup_context_menu as *const () as u64,
            "v@:@@@",
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
    rt.register_metaclass_addr(metaclass_addr, "NSMenu");
}

/// Register `NSMenuItem` — menu item stubs.
pub fn register_nsmenuitem_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmenuitem_class_addr();
    let metaclass_addr = nsobj::nsmenuitem_metaclass_addr();
    let info = ClassInfo {
        name: "NSMenuItem".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithTitle:action:keyEquivalent:",
                nsmenu_shim::shim_nsmenuitem_init_with_title as *const () as u64,
                "@@:@:@",
            ),
            me(
                "setSubmenu:",
                nsmenu_shim::shim_nsmenuitem_set_submenu as *const () as u64,
                "v@:@",
            ),
            me(
                "submenu",
                nsmenu_shim::shim_nsmenuitem_submenu as *const () as u64,
                "@@:",
            ),
            me(
                "title",
                nsmenu_shim::shim_nsmenuitem_title as *const () as u64,
                "@@:",
            ),
            me(
                "setTitle:",
                nsmenu_shim::shim_nsmenuitem_set_title as *const () as u64,
                "v@:@",
            ),
            me(
                "setTarget:",
                nsmenu_shim::shim_nsmenuitem_set_target as *const () as u64,
                "v@:@",
            ),
            me(
                "target",
                nsmenu_shim::shim_nsmenuitem_target as *const () as u64,
                "@@:",
            ),
            me(
                "setAction:",
                nsmenu_shim::shim_nsmenuitem_set_action as *const () as u64,
                "v@::",
            ),
            me(
                "action",
                nsmenu_shim::shim_nsmenuitem_action as *const () as u64,
                ":@:",
            ),
            me(
                "isEnabled",
                nsmenu_shim::shim_nsmenuitem_is_enabled as *const () as u64,
                "B@:",
            ),
            me(
                "setEnabled:",
                nsmenu_shim::shim_nsmenuitem_set_enabled as *const () as u64,
                "v@:B",
            ),
            me(
                "keyEquivalent",
                nsmenu_shim::shim_nsmenuitem_key_equivalent as *const () as u64,
                "@@:",
            ),
            me(
                "setKeyEquivalent:",
                nsmenu_shim::shim_nsmenuitem_set_key_equivalent as *const () as u64,
                "v@:@",
            ),
            me(
                "keyEquivalentModifierMask",
                nsmenu_shim::shim_nsmenuitem_key_equivalent_modifier_mask as *const () as u64,
                "Q@:",
            ),
            me(
                "setKeyEquivalentModifierMask:",
                nsmenu_shim::shim_nsmenuitem_set_key_equivalent_modifier_mask as *const () as u64,
                "v@:Q",
            ),
            me(
                "state",
                nsmenu_shim::shim_nsmenuitem_state as *const () as u64,
                "q@:",
            ),
            me(
                "setState:",
                nsmenu_shim::shim_nsmenuitem_set_state as *const () as u64,
                "v@:q",
            ),
            me(
                "toolTip",
                nsmenu_shim::shim_nsmenuitem_tool_tip as *const () as u64,
                "@@:",
            ),
            me(
                "setToolTip:",
                nsmenu_shim::shim_nsmenuitem_set_tool_tip as *const () as u64,
                "v@:@",
            ),
            me(
                "representedObject",
                nsmenu_shim::shim_nsmenuitem_represented_object as *const () as u64,
                "@@:",
            ),
            me(
                "setRepresentedObject:",
                nsmenu_shim::shim_nsmenuitem_set_represented_object as *const () as u64,
                "v@:@",
            ),
            me(
                "tag",
                nsmenu_shim::shim_nsmenuitem_tag as *const () as u64,
                "q@:",
            ),
            me(
                "setTag:",
                nsmenu_shim::shim_nsmenuitem_set_tag as *const () as u64,
                "v@:q",
            ),
            me(
                "performClick:",
                nsmenu_shim::shim_nsmenuitem_perform_click as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![me(
            "separatorItem",
            nsmenu_shim::shim_nsmenuitem_separator as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 96,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMenuItem");
}

/// Register `NSNib` — nib loading stubs.
pub fn register_nsnib_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsnib_class_addr();
    let metaclass_addr = nsobj::nsnib_metaclass_addr();
    let info = ClassInfo {
        name: "NSNib".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithNibNamed:bundle:",
                nsnib::shim_nsnib_init_with_name as *const () as u64,
                "@@:@@",
            ),
            me(
                "instantiateWithOwner:topLevelObjects:",
                nsnib::shim_nsnib_instantiate as *const () as u64,
                "B@:@^@",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nsnib::shim_nsnib_alloc as *const () as u64,
            "@@:",
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
    rt.register_metaclass_addr(metaclass_addr, "NSNib");
}
