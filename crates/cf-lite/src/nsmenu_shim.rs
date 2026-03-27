//! NSMenu / NSMenuItem stubs for AppKit.
//!
//! Minimal shims to prevent crashes when apps create menus
//! or when the system asks for the main menu during launch.

// ── NSMenu ─────────────────────────────────────────────────────

/// `-[NSMenu initWithTitle:]`
pub extern "C" fn shim_nsmenu_init_with_title(obj: u64, _cmd: u64, _title: u64) -> u64 {
    obj
}

/// `-[NSMenu addItem:]` — no-op (we don't track menu items).
pub extern "C" fn shim_nsmenu_add_item(_obj: u64, _cmd: u64, _item: u64) {}

/// `-[NSMenu addItemWithTitle:action:keyEquivalent:]`
pub extern "C" fn shim_nsmenu_add_item_with_title(
    _obj: u64,
    _cmd: u64,
    _title: u64,
    _action: u64,
    _key_equiv: u64,
) -> u64 {
    0
}

/// `-[NSMenu numberOfItems]` → 0.
pub extern "C" fn shim_nsmenu_number_of_items(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSMenu title]` → empty string.
pub extern "C" fn shim_nsmenu_title(_obj: u64, _cmd: u64) -> u64 {
    crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr())
}

/// `-[NSMenu setTitle:]` — no-op.
pub extern "C" fn shim_nsmenu_set_title(_obj: u64, _cmd: u64, _title: u64) {}

/// `-[NSMenu setSubmenu:forItem:]` — no-op.
pub extern "C" fn shim_nsmenu_set_submenu(_obj: u64, _cmd: u64, _submenu: u64, _item: u64) {}

// ── NSMenuItem ─────────────────────────────────────────────────

/// `+[NSMenuItem separatorItem]` → nil.
pub extern "C" fn shim_nsmenuitem_separator(_cls: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSMenuItem initWithTitle:action:keyEquivalent:]`
pub extern "C" fn shim_nsmenuitem_init_with_title(
    obj: u64,
    _cmd: u64,
    _title: u64,
    _action: u64,
    _key_equiv: u64,
) -> u64 {
    obj
}

/// `-[NSMenuItem setSubmenu:]` — no-op.
pub extern "C" fn shim_nsmenuitem_set_submenu(_obj: u64, _cmd: u64, _submenu: u64) {}

/// `-[NSMenuItem submenu]` → nil.
pub extern "C" fn shim_nsmenuitem_submenu(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSMenuItem title]` → empty string.
pub extern "C" fn shim_nsmenuitem_title(_obj: u64, _cmd: u64) -> u64 {
    crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr())
}

/// `-[NSMenuItem setTitle:]` — no-op.
pub extern "C" fn shim_nsmenuitem_set_title(_obj: u64, _cmd: u64, _title: u64) {}

/// `-[NSMenuItem setTarget:]` — no-op.
pub extern "C" fn shim_nsmenuitem_set_target(_obj: u64, _cmd: u64, _target: u64) {}

/// `-[NSMenuItem target]` → nil.
pub extern "C" fn shim_nsmenuitem_target(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSMenuItem setAction:]` — no-op.
pub extern "C" fn shim_nsmenuitem_set_action(_obj: u64, _cmd: u64, _action: u64) {}

/// `-[NSMenuItem isEnabled]` → YES.
pub extern "C" fn shim_nsmenuitem_is_enabled(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[NSMenuItem setEnabled:]` — no-op.
pub extern "C" fn shim_nsmenuitem_set_enabled(_obj: u64, _cmd: u64, _enabled: u8) {}
