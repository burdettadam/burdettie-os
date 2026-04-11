//! NSMenu / NSMenuItem shims for AppKit.
//!
//! NSMenu object layout (24 bytes):
//!   offset  0: ISA pointer   (u64)
//!   offset  8: title_ptr     (u64 — NSString)
//!   offset 16: items_vec_ptr (u64 — Box<Vec<u64>> of NSMenuItem*)
//!
//! NSMenuItem object layout (72 bytes):
//!   offset  0: ISA pointer   (u64)
//!   offset  8: title_ptr     (u64 — NSString)
//!   offset 16: target_ptr    (u64)
//!   offset 24: action_sel    (u64 — SEL)
//!   offset 32: submenu_ptr   (u64)
//!   offset 40: enabled       (u64 — 1=YES, 0=NO; default YES)
//!   offset 48: key_equiv_ptr (u64 — NSString)
//!   offset 56: key_equiv_mod (u64 — modifier mask, default NSCommandKeyMask)
//!   offset 64: state         (u64 — NSControlStateValue: 0=off, 1=on, -1=mixed)

const MENU_TITLE_OFF: usize = 8;
const MENU_ITEMS_OFF: usize = 16;
const MENU_SIZE: usize = 24;

const ITEM_TITLE_OFF: usize = 8;
const ITEM_TARGET_OFF: usize = 16;
const ITEM_ACTION_OFF: usize = 24;
const ITEM_SUBMENU_OFF: usize = 32;
const ITEM_ENABLED_OFF: usize = 40;
const ITEM_KEY_EQUIV_OFF: usize = 48;
const ITEM_KEY_EQUIV_MOD_OFF: usize = 56;
const ITEM_STATE_OFF: usize = 64;
const ITEM_SIZE: usize = 72;

fn menu_items_ptr(obj: u64) -> *mut Vec<u64> {
    if obj == 0 {
        return std::ptr::null_mut();
    }
    unsafe { *((obj as usize + MENU_ITEMS_OFF) as *const u64) as *mut Vec<u64> }
}

fn ensure_menu_items_ptr(obj: u64) -> *mut Vec<u64> {
    if obj == 0 {
        return std::ptr::null_mut();
    }

    let existing = menu_items_ptr(obj);
    if !existing.is_null() {
        return existing;
    }

    let items = Box::into_raw(Box::new(Vec::<u64>::new()));
    unsafe {
        *((obj as usize + MENU_ITEMS_OFF) as *mut u64) = items as u64;
    }
    items
}

fn alloc_menu_item() -> u64 {
    let obj = crate::pal_alloc().calloc(1, ITEM_SIZE) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsmenuitem_class_addr();
        }
    }
    obj
}

// ── NSMenu ─────────────────────────────────────────────────────

/// `-[NSMenu initWithTitle:]`
pub extern "C" fn shim_nsmenu_init_with_title(obj: u64, _cmd: u64, title: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe {
        *((obj as usize + MENU_TITLE_OFF) as *mut u64) = title;
    }
    let items_ptr = ensure_menu_items_ptr(obj);
    if !items_ptr.is_null() {
        unsafe { (*items_ptr).clear(); }
    }
    obj
}

/// `-[NSMenu addItem:]`
pub extern "C" fn shim_nsmenu_add_item(obj: u64, _cmd: u64, item: u64) {
    if obj == 0 || item == 0 {
        return;
    }

    let items_ptr = ensure_menu_items_ptr(obj);
    if items_ptr.is_null() {
        return;
    }

    unsafe {
        (*items_ptr).push(item);
    }
}

/// `-[NSMenu addItemWithTitle:action:keyEquivalent:]`
pub extern "C" fn shim_nsmenu_add_item_with_title(
    obj: u64,
    _cmd: u64,
    title: u64,
    action: u64,
    key_equiv: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let item = alloc_menu_item();
    if item == 0 {
        return 0;
    }

    let item = shim_nsmenuitem_init_with_title(item, 0, title, action, key_equiv);
    if item == 0 {
        return 0;
    }

    shim_nsmenu_add_item(obj, 0, item);
    item
}

/// `-[NSMenu numberOfItems]`
pub extern "C" fn shim_nsmenu_number_of_items(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let items_ptr = menu_items_ptr(obj);
    if items_ptr.is_null() {
        0
    } else {
        unsafe { (*items_ptr).len() as u64 }
    }
}

/// `-[NSMenu itemAtIndex:]`
pub extern "C" fn shim_nsmenu_item_at_index(obj: u64, _cmd: u64, index: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let items_ptr = menu_items_ptr(obj);
    if items_ptr.is_null() {
        return 0;
    }

    unsafe { (&*items_ptr).get(index as usize).copied().unwrap_or(0) }
}

/// `-[NSMenu itemArray]`
pub extern "C" fn shim_nsmenu_item_array(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }

    let items_ptr = menu_items_ptr(obj);
    if items_ptr.is_null() {
        crate::nsarray::make_nsarray(&[])
    } else {
        unsafe { crate::nsarray::make_nsarray(&*items_ptr) }
    }
}

/// `-[NSMenu removeAllItems]`
pub extern "C" fn shim_nsmenu_remove_all_items(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }

    let items_ptr = menu_items_ptr(obj);
    if !items_ptr.is_null() {
        unsafe { (*items_ptr).clear(); }
    }
}

/// `-[NSMenu insertItem:atIndex:]`
pub extern "C" fn shim_nsmenu_insert_item(obj: u64, _cmd: u64, item: u64, index: i64) {
    if obj == 0 || item == 0 { return; }
    let items_ptr = ensure_menu_items_ptr(obj);
    if items_ptr.is_null() { return; }
    unsafe {
        let items = &mut *items_ptr;
        let idx = (index as usize).min(items.len());
        items.insert(idx, item);
    }
}

/// `-[NSMenu removeItemAtIndex:]`
pub extern "C" fn shim_nsmenu_remove_item_at_index(obj: u64, _cmd: u64, index: i64) {
    if obj == 0 { return; }
    let items_ptr = menu_items_ptr(obj);
    if items_ptr.is_null() { return; }
    unsafe {
        let items = &mut *items_ptr;
        let idx = index as usize;
        if idx < items.len() {
            items.remove(idx);
        }
    }
}

/// `-[NSMenu title]`
pub extern "C" fn shim_nsmenu_title(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr()); }
    let ptr = unsafe { *((obj as usize + MENU_TITLE_OFF) as *const u64) };
    if ptr == 0 { crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr()) } else { ptr }
}

/// `-[NSMenu setTitle:]`
pub extern "C" fn shim_nsmenu_set_title(obj: u64, _cmd: u64, title: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + MENU_TITLE_OFF) as *mut u64) = title; }
}

/// `-[NSMenu setSubmenu:forItem:]`
pub extern "C" fn shim_nsmenu_set_submenu(obj: u64, _cmd: u64, submenu: u64, item: u64) {
    if obj == 0 || item == 0 {
        return;
    }

    shim_nsmenuitem_set_submenu(item, 0, submenu);

    let items_ptr = ensure_menu_items_ptr(obj);
    if items_ptr.is_null() {
        return;
    }

    unsafe {
        if !(*items_ptr).contains(&item) {
            (*items_ptr).push(item);
        }
    }
}

// ── NSMenuItem ─────────────────────────────────────────────────

/// `+[NSMenuItem separatorItem]`
pub extern "C" fn shim_nsmenuitem_separator(_cls: u64, _cmd: u64) -> u64 {
    let item = alloc_menu_item();
    if item == 0 {
        return 0;
    }

    let empty = crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr());
    let item = shim_nsmenuitem_init_with_title(item, 0, empty, 0, empty);
    shim_nsmenuitem_set_enabled(item, 0, 0);
    item
}

/// `-[NSMenuItem initWithTitle:action:keyEquivalent:]`
pub extern "C" fn shim_nsmenuitem_init_with_title(
    obj: u64,
    _cmd: u64,
    title: u64,
    action: u64,
    key_equiv: u64,
) -> u64 {
    if obj == 0 { return 0; }
    unsafe {
        *((obj as usize + ITEM_TITLE_OFF) as *mut u64) = title;
        *((obj as usize + ITEM_TARGET_OFF) as *mut u64) = 0;
        *((obj as usize + ITEM_ACTION_OFF) as *mut u64) = action;
        *((obj as usize + ITEM_SUBMENU_OFF) as *mut u64) = 0;
        *((obj as usize + ITEM_ENABLED_OFF) as *mut u64) = 1; // default YES
        *((obj as usize + ITEM_KEY_EQUIV_OFF) as *mut u64) = key_equiv;
        *((obj as usize + ITEM_KEY_EQUIV_MOD_OFF) as *mut u64) = NS_COMMAND_KEY_MASK;
        *((obj as usize + ITEM_STATE_OFF) as *mut u64) = 0; // NSControlStateValueOff
    }
    obj
}

/// `-[NSMenuItem setSubmenu:]`
pub extern "C" fn shim_nsmenuitem_set_submenu(obj: u64, _cmd: u64, submenu: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + ITEM_SUBMENU_OFF) as *mut u64) = submenu; }
}

/// `-[NSMenuItem submenu]`
pub extern "C" fn shim_nsmenuitem_submenu(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ITEM_SUBMENU_OFF) as *const u64) }
}

/// `-[NSMenuItem title]`
pub extern "C" fn shim_nsmenuitem_title(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr()); }
    let ptr = unsafe { *((obj as usize + ITEM_TITLE_OFF) as *const u64) };
    if ptr == 0 { crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr()) } else { ptr }
}

/// `-[NSMenuItem setTitle:]`
pub extern "C" fn shim_nsmenuitem_set_title(obj: u64, _cmd: u64, title: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + ITEM_TITLE_OFF) as *mut u64) = title; }
}

/// `-[NSMenuItem setTarget:]`
pub extern "C" fn shim_nsmenuitem_set_target(obj: u64, _cmd: u64, target: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + ITEM_TARGET_OFF) as *mut u64) = target; }
}

/// `-[NSMenuItem target]`
pub extern "C" fn shim_nsmenuitem_target(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ITEM_TARGET_OFF) as *const u64) }
}

/// `-[NSMenuItem setAction:]`
pub extern "C" fn shim_nsmenuitem_set_action(obj: u64, _cmd: u64, action: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + ITEM_ACTION_OFF) as *mut u64) = action; }
}

/// `-[NSMenuItem action]`
pub extern "C" fn shim_nsmenuitem_action(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ITEM_ACTION_OFF) as *const u64) }
}

/// `-[NSMenuItem isEnabled]`
pub extern "C" fn shim_nsmenuitem_is_enabled(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 1; }
    let val = unsafe { *((obj as usize + ITEM_ENABLED_OFF) as *const u64) };
    val as u8
}

/// `-[NSMenuItem setEnabled:]`
pub extern "C" fn shim_nsmenuitem_set_enabled(obj: u64, _cmd: u64, enabled: u8) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + ITEM_ENABLED_OFF) as *mut u64) = enabled as u64; }
}

/// `-[NSMenuItem keyEquivalent]`
pub extern "C" fn shim_nsmenuitem_key_equivalent(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr()); }
    let ptr = unsafe { *((obj as usize + ITEM_KEY_EQUIV_OFF) as *const u64) };
    if ptr == 0 { crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr()) } else { ptr }
}

/// `-[NSMenuItem setKeyEquivalent:]`
pub extern "C" fn shim_nsmenuitem_set_key_equivalent(obj: u64, _cmd: u64, key_equiv: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + ITEM_KEY_EQUIV_OFF) as *mut u64) = key_equiv; }
}

/// NSEventModifierFlags constants for key equivalent matching.
const NS_COMMAND_KEY_MASK: u64 = 1 << 20;  // NSEventModifierFlagCommand

/// `-[NSMenuItem keyEquivalentModifierMask]`
pub extern "C" fn shim_nsmenuitem_key_equivalent_modifier_mask(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return NS_COMMAND_KEY_MASK; }
    unsafe { *((obj as usize + ITEM_KEY_EQUIV_MOD_OFF) as *const u64) }
}

/// `-[NSMenuItem setKeyEquivalentModifierMask:]`
pub extern "C" fn shim_nsmenuitem_set_key_equivalent_modifier_mask(obj: u64, _cmd: u64, mask: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + ITEM_KEY_EQUIV_MOD_OFF) as *mut u64) = mask; }
}

/// `-[NSMenuItem state]` — returns NSControlStateValue (0=off, 1=on, -1=mixed).
pub extern "C" fn shim_nsmenuitem_state(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + ITEM_STATE_OFF) as *const u64) as i64 }
}

/// `-[NSMenuItem setState:]`
pub extern "C" fn shim_nsmenuitem_set_state(obj: u64, _cmd: u64, state: i64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + ITEM_STATE_OFF) as *mut u64) = state as u64; }
}

// ── Action dispatch ────────────────────────────────────────────

/// `-[NSMenu performKeyEquivalent:]` — walk items checking key equivalents.
///
/// Returns 1 (YES) if a matching item was found and its action dispatched.
pub extern "C" fn shim_nsmenu_perform_key_equivalent(obj: u64, _cmd: u64, event: u64) -> u8 {
    if obj == 0 || event == 0 { return 0; }

    // Get the event's characters string
    let event_chars = crate::nsevent::shim_nsevent_characters(event, 0);
    if event_chars == 0 { return 0; }

    let items_ptr = menu_items_ptr(obj);
    if items_ptr.is_null() { return 0; }

    let items = unsafe { &*items_ptr };
    for &item in items {
        if item == 0 { continue; }

        // Check submenu first (recursive)
        let submenu = unsafe { *((item as usize + ITEM_SUBMENU_OFF) as *const u64) };
        if submenu != 0 {
            if shim_nsmenu_perform_key_equivalent(submenu, _cmd, event) != 0 {
                return 1;
            }
        }

        // Check this item's key equivalent
        let enabled = unsafe { *((item as usize + ITEM_ENABLED_OFF) as *const u64) };
        if enabled == 0 { continue; }

        let key_equiv = unsafe { *((item as usize + ITEM_KEY_EQUIV_OFF) as *const u64) };
        if key_equiv == 0 { continue; }

        // Compare key equivalent string with event characters
        let key_str = crate::cfstring::extract_cfstring_cstr(key_equiv);
        let event_str = crate::cfstring::extract_cfstring_cstr(event_chars);
        if let (Some(k), Some(e)) = (key_str, event_str) {
            if !k.is_empty() && k == e {
                // Check modifier flags match.
                let item_mod = unsafe { *((item as usize + ITEM_KEY_EQUIV_MOD_OFF) as *const u64) };
                let event_mod = crate::nsevent::shim_nsevent_modifier_flags(event, 0);
                // Mask to device-independent modifier bits (Shift/Ctrl/Alt/Cmd).
                const DEVICE_INDEPENDENT_MASK: u64 = 0xFFFF_0000;
                if (event_mod & DEVICE_INDEPENDENT_MASK) & item_mod == item_mod {
                    // Dispatch the action
                    let action = unsafe { *((item as usize + ITEM_ACTION_OFF) as *const u64) };
                    let target = unsafe { *((item as usize + ITEM_TARGET_OFF) as *const u64) };
                    if action != 0 {
                        dispatch_menu_action(target, action, item);
                    }
                    return 1;
                }
            }
        }
    }
    0
}

/// Dispatch an action selector to a target or via the responder chain.
fn dispatch_menu_action(target: u64, action_sel: u64, sender: u64) {
    if action_sel == 0 { return; }

    // If we have a concrete target, call the action on it
    let effective_target = if target != 0 {
        target
    } else {
        // Nil target — route through NSApp's sendAction:to:from: which walks
        // the responder chain from the key window's first responder.
        let app = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
        if app != 0 {
            crate::nsapplication::shim_nsapplication_send_action_to_from(
                app, 0, action_sel, 0, sender,
            );
        }
        return;
    };

    let isa = unsafe { *(effective_target as *const u64) };
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if let Some(rt) = guard.as_ref() {
        // Convert action selector (SEL = pointer to C string) to &str
        let sel_name = unsafe { std::ffi::CStr::from_ptr(action_sel as *const std::ffi::c_char) };
        if let Ok(name) = sel_name.to_str() {
            if let Some(imp) = rt.msg_lookup(isa, name) {
                let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
                f(effective_target, action_sel, sender);
            }
        }
    }
}

/// `-[NSMenuItem performClick:]` — programmatically invoke this menu item's action.
pub extern "C" fn shim_nsmenuitem_perform_click(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 { return; }
    let enabled = unsafe { *((obj as usize + ITEM_ENABLED_OFF) as *const u64) };
    if enabled == 0 { return; }
    let action = unsafe { *((obj as usize + ITEM_ACTION_OFF) as *const u64) };
    let target = unsafe { *((obj as usize + ITEM_TARGET_OFF) as *const u64) };
    if action != 0 {
        dispatch_menu_action(target, action, obj);
    }
}

/// `+[NSMenu popUpContextMenu:withEvent:forView:]` — stub that walks menu items
/// and dispatches the first enabled item's action (simulates showing a context menu).
pub extern "C" fn shim_nsmenu_popup_context_menu(
    _cls: u64, _cmd: u64, menu: u64, _event: u64, _view: u64,
) {
    // In a real implementation this would display a popup. We simply log and
    // return without crashing — callers invoking this won't get an unrecognized
    // selector crash.
    let _ = menu;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // These shims are minimal AppKit stubs. None dereference obj, so
    // ISA doesn't matter — we use alloc_obj_fake for the few tests
    // that need a non-zero obj pointer.

    // ── NSMenu ─────────────────────────────────────────────────

    #[test]
    fn init_with_title_returns_obj() {
        let obj = alloc_obj_fake(MENU_SIZE);
        let title = make_string("File");
        let ret = shim_nsmenu_init_with_title(obj, 0, title);
        assert_eq!(ret, obj);
        free_obj(obj);
    }

    #[test]
    fn init_with_title_nil() {
        let ret = shim_nsmenu_init_with_title(0, 0, 0);
        assert_eq!(ret, 0);
    }

    #[test]
    fn add_item_tracks_items() {
        let menu = alloc_obj_fake(MENU_SIZE);
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenu_add_item(menu, 0, item);
        assert_eq!(shim_nsmenu_number_of_items(menu, 0), 1);
        assert_eq!(shim_nsmenu_item_at_index(menu, 0, 0), item);
        free_obj(item);
        free_obj(menu);
    }

    #[test]
    fn add_item_nil() {
        shim_nsmenu_add_item(0, 0, 0);
    }

    #[test]
    fn add_item_with_title_returns_item() {
        let menu = alloc_obj_fake(MENU_SIZE);
        let title = make_string("Copy");
        let key_equiv = make_string("c");
        let ret = shim_nsmenu_add_item_with_title(menu, 0, title, 0x123, key_equiv);
        assert_ne!(ret, 0);
        assert_eq!(shim_nsmenu_number_of_items(menu, 0), 1);
        assert_eq!(shim_nsmenu_item_at_index(menu, 0, 0), ret);
        assert_string_eq(shim_nsmenuitem_title(ret, 0), "Copy");
        assert_eq!(shim_nsmenuitem_action(ret, 0), 0x123);
        assert_string_eq(shim_nsmenuitem_key_equivalent(ret, 0), "c");
        free_obj(menu);
    }

    #[test]
    fn add_item_with_title_nil() {
        let ret = shim_nsmenu_add_item_with_title(0, 0, 0, 0, 0);
        assert_eq!(ret, 0);
    }

    #[test]
    fn number_of_items_returns_zero() {
        let menu = alloc_obj_fake(MENU_SIZE);
        assert_eq!(shim_nsmenu_number_of_items(menu, 0), 0);
        free_obj(menu);
    }

    #[test]
    fn item_array_returns_added_items() {
        let menu = alloc_obj_fake(MENU_SIZE);
        let first = alloc_obj_fake(ITEM_SIZE);
        let second = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenu_add_item(menu, 0, first);
        shim_nsmenu_add_item(menu, 0, second);

        let items = shim_nsmenu_item_array(menu, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(items, 0), 2);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(items, 0, 0), first);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(items, 0, 1), second);

        free_obj(second);
        free_obj(first);
        free_obj(menu);
    }

    #[test]
    fn number_of_items_nil() {
        assert_eq!(shim_nsmenu_number_of_items(0, 0), 0);
    }

    #[test]
    fn title_returns_empty_string() {
        let menu = alloc_obj_fake(MENU_SIZE);
        let s = shim_nsmenu_title(menu, 0);
        assert_ne!(s, 0);
        assert_string_eq(s, "");
        free_obj(menu);
    }

    #[test]
    fn title_nil_returns_empty_string() {
        // Even with nil receiver, this stub allocates and returns an empty string.
        let s = shim_nsmenu_title(0, 0);
        assert_ne!(s, 0);
        assert_string_eq(s, "");
    }

    #[test]
    fn set_title_no_crash() {
        let menu = alloc_obj_fake(MENU_SIZE);
        let title = make_string("Edit");
        shim_nsmenu_set_title(menu, 0, title);
        free_obj(menu);
    }

    #[test]
    fn set_title_nil() {
        shim_nsmenu_set_title(0, 0, 0);
    }

    #[test]
    fn set_submenu_tracks_item() {
        let menu = alloc_obj_fake(MENU_SIZE);
        let sub = alloc_obj_fake(MENU_SIZE);
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenu_set_submenu(menu, 0, sub, item);
        assert_eq!(shim_nsmenuitem_submenu(item, 0), sub);
        assert_eq!(shim_nsmenu_number_of_items(menu, 0), 1);
        assert_eq!(shim_nsmenu_item_at_index(menu, 0, 0), item);
        free_obj(item);
        free_obj(sub);
        free_obj(menu);
    }

    #[test]
    fn set_submenu_nil() {
        shim_nsmenu_set_submenu(0, 0, 0, 0);
    }

    // ── NSMenuItem ─────────────────────────────────────────────

    #[test]
    fn separator_returns_item() {
        let item = shim_nsmenuitem_separator(0, 0);
        assert_ne!(item, 0);
        assert_eq!(shim_nsmenuitem_is_enabled(item, 0), 0);
        assert_string_eq(shim_nsmenuitem_title(item, 0), "");
    }

    #[test]
    fn menuitem_init_with_title_returns_obj() {
        let obj = alloc_obj_fake(ITEM_SIZE);
        let title = make_string("Paste");
        let ret = shim_nsmenuitem_init_with_title(obj, 0, title, 0, 0);
        assert_eq!(ret, obj);
        free_obj(obj);
    }

    #[test]
    fn menuitem_init_with_title_nil() {
        let ret = shim_nsmenuitem_init_with_title(0, 0, 0, 0, 0);
        assert_eq!(ret, 0);
    }

    #[test]
    fn menuitem_set_submenu_no_crash() {
        let item = alloc_obj_fake(ITEM_SIZE);
        let sub = alloc_obj_fake(MENU_SIZE);
        shim_nsmenuitem_set_submenu(item, 0, sub);
        assert_eq!(shim_nsmenuitem_submenu(item, 0), sub);
        free_obj(sub);
        free_obj(item);
    }

    #[test]
    fn menuitem_set_submenu_nil() {
        shim_nsmenuitem_set_submenu(0, 0, 0);
    }

    #[test]
    fn menuitem_submenu_returns_zero() {
        let item = alloc_obj_fake(ITEM_SIZE);
        assert_eq!(shim_nsmenuitem_submenu(item, 0), 0);
        free_obj(item);
    }

    #[test]
    fn menuitem_submenu_nil() {
        assert_eq!(shim_nsmenuitem_submenu(0, 0), 0);
    }

    #[test]
    fn menuitem_title_returns_empty_string() {
        let item = alloc_obj_fake(ITEM_SIZE);
        let s = shim_nsmenuitem_title(item, 0);
        assert_ne!(s, 0);
        assert_string_eq(s, "");
        free_obj(item);
    }

    #[test]
    fn menuitem_title_nil() {
        let s = shim_nsmenuitem_title(0, 0);
        assert_ne!(s, 0);
        assert_string_eq(s, "");
    }

    #[test]
    fn menuitem_set_title_no_crash() {
        let item = alloc_obj_fake(ITEM_SIZE);
        let title = make_string("Undo");
        shim_nsmenuitem_set_title(item, 0, title);
        free_obj(item);
    }

    #[test]
    fn menuitem_set_title_nil() {
        shim_nsmenuitem_set_title(0, 0, 0);
    }

    #[test]
    fn menuitem_set_target_no_crash() {
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenuitem_set_target(item, 0, 0xABC);
        assert_eq!(shim_nsmenuitem_target(item, 0), 0xABC);
        free_obj(item);
    }

    #[test]
    fn menuitem_set_target_nil() {
        shim_nsmenuitem_set_target(0, 0, 0);
    }

    #[test]
    fn menuitem_target_returns_zero() {
        let item = alloc_obj_fake(ITEM_SIZE);
        assert_eq!(shim_nsmenuitem_target(item, 0), 0);
        free_obj(item);
    }

    #[test]
    fn menuitem_target_nil() {
        assert_eq!(shim_nsmenuitem_target(0, 0), 0);
    }

    #[test]
    fn menuitem_set_action_persists() {
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenuitem_set_action(item, 0, 0xDEF);
        assert_eq!(shim_nsmenuitem_action(item, 0), 0xDEF);
        free_obj(item);
    }

    #[test]
    fn menuitem_set_action_nil() {
        shim_nsmenuitem_set_action(0, 0, 0);
    }

    #[test]
    fn menuitem_is_enabled_returns_yes() {
        // init sets enabled=1 by default
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenuitem_init_with_title(item, 0, 0, 0, 0);
        assert_eq!(shim_nsmenuitem_is_enabled(item, 0), 1);
        free_obj(item);
    }

    #[test]
    fn menuitem_is_enabled_nil() {
        assert_eq!(shim_nsmenuitem_is_enabled(0, 0), 1);
    }

    #[test]
    fn menuitem_set_enabled_no_crash() {
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenuitem_set_enabled(item, 0, 0);
        shim_nsmenuitem_set_enabled(item, 0, 1);
        free_obj(item);
    }

    #[test]
    fn menuitem_set_enabled_nil() {
        shim_nsmenuitem_set_enabled(0, 0, 1);
    }

    // ── New: mutation persistence tests ────────────────────────

    #[test]
    fn nsmenu_item_set_title() {
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenuitem_init_with_title(item, 0, 0, 0, 0);
        let title = make_string("Open");
        shim_nsmenuitem_set_title(item, 0, title);
        let got = shim_nsmenuitem_title(item, 0);
        assert_string_eq(got, "Open");
        free_obj(item);
    }

    #[test]
    fn nsmenu_item_set_enabled() {
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenuitem_init_with_title(item, 0, 0, 0, 0);
        assert_eq!(shim_nsmenuitem_is_enabled(item, 0), 1);
        shim_nsmenuitem_set_enabled(item, 0, 0);
        assert_eq!(shim_nsmenuitem_is_enabled(item, 0), 0);
        shim_nsmenuitem_set_enabled(item, 0, 1);
        assert_eq!(shim_nsmenuitem_is_enabled(item, 0), 1);
        free_obj(item);
    }

    #[test]
    fn nsmenu_init_stores_title() {
        let menu = alloc_obj_fake(MENU_SIZE);
        let title = make_string("Edit");
        shim_nsmenu_init_with_title(menu, 0, title);
        let got = shim_nsmenu_title(menu, 0);
        assert_string_eq(got, "Edit");
        free_obj(menu);
    }

    #[test]
    fn nsmenu_set_title_persists() {
        let menu = alloc_obj_fake(MENU_SIZE);
        shim_nsmenu_init_with_title(menu, 0, make_string("A"));
        shim_nsmenu_set_title(menu, 0, make_string("B"));
        assert_string_eq(shim_nsmenu_title(menu, 0), "B");
        free_obj(menu);
    }

    #[test]
    fn menuitem_init_stores_title_and_action() {
        let item = alloc_obj_fake(ITEM_SIZE);
        let title = make_string("Save");
        shim_nsmenuitem_init_with_title(item, 0, title, 0xABC, 0);
        assert_string_eq(shim_nsmenuitem_title(item, 0), "Save");
        assert_eq!(read_field(item, ITEM_ACTION_OFF), 0xABC);
        free_obj(item);
    }

    #[test]
    fn menuitem_key_equivalent_roundtrip() {
        let item = alloc_obj_fake(ITEM_SIZE);
        let key = make_string("q");
        shim_nsmenuitem_init_with_title(item, 0, 0, 0, key);
        assert_string_eq(shim_nsmenuitem_key_equivalent(item, 0), "q");
        shim_nsmenuitem_set_key_equivalent(item, 0, make_string("w"));
        assert_string_eq(shim_nsmenuitem_key_equivalent(item, 0), "w");
        free_obj(item);
    }

    #[test]
    fn perform_click_nil_no_panic() {
        shim_nsmenuitem_perform_click(0, 0, 0);
    }

    #[test]
    fn perform_click_disabled_item_no_dispatch() {
        let item = alloc_obj_fake(ITEM_SIZE);
        shim_nsmenuitem_init_with_title(item, 0, make_string("X"), 0xABC, 0);
        shim_nsmenuitem_set_enabled(item, 0, 0);
        shim_nsmenuitem_perform_click(item, 0, 0); // should not crash
        free_obj(item);
    }

    #[test]
    fn popup_context_menu_nil_no_panic() {
        shim_nsmenu_popup_context_menu(0, 0, 0, 0, 0);
    }
}
