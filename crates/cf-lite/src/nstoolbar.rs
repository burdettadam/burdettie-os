//! NSToolbar shim: toolbar management for Cocoa windows.
//!
//! Object layout:
//!
//!   offset  0: isa                         (u64)
//!   offset  8: identifier_ptr              (u64 — NSString*)
//!   offset 16: delegate_ptr                (u64)
//!   offset 24: flags                       (u64 — bit 0: is_visible, bit 1: allows_user_customization)
//!
//! Total: 32 bytes.

const IDENTIFIER_OFF: usize = 8;
const DELEGATE_OFF: usize = 16;
const FLAGS_OFF: usize = 24;
pub(crate) const TOOLBAR_SIZE: usize = 32;

const FLAG_VISIBLE: u64 = 1 << 0;
const FLAG_ALLOWS_USER_CUSTOMIZATION: u64 = 1 << 1;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSToolbar initWithIdentifier:]`
pub extern "C" fn shim_nstoolbar_init_with_identifier(obj: u64, _cmd: u64, ident: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    write_u64(obj, IDENTIFIER_OFF, ident);
    // Visible by default
    write_u64(obj, FLAGS_OFF, FLAG_VISIBLE);
    obj
}

/// `-[NSToolbar identifier]`
pub extern "C" fn shim_nstoolbar_identifier(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, IDENTIFIER_OFF)
}

/// `-[NSToolbar delegate]`
pub extern "C" fn shim_nstoolbar_delegate(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DELEGATE_OFF)
}

/// `-[NSToolbar setDelegate:]`
pub extern "C" fn shim_nstoolbar_set_delegate(obj: u64, _cmd: u64, del: u64) {
    write_u64(obj, DELEGATE_OFF, del);
}

/// `-[NSToolbar isVisible]`
pub extern "C" fn shim_nstoolbar_is_visible(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & FLAG_VISIBLE != 0)
}

/// `-[NSToolbar setVisible:]`
pub extern "C" fn shim_nstoolbar_set_visible(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        flags |= FLAG_VISIBLE;
    } else {
        flags &= !FLAG_VISIBLE;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSToolbar allowsUserCustomization]`
pub extern "C" fn shim_nstoolbar_allows_user_customization(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & FLAG_ALLOWS_USER_CUSTOMIZATION != 0)
}

/// `-[NSToolbar setAllowsUserCustomization:]`
pub extern "C" fn shim_nstoolbar_set_allows_user_customization(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        flags |= FLAG_ALLOWS_USER_CUSTOMIZATION;
    } else {
        flags &= !FLAG_ALLOWS_USER_CUSTOMIZATION;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSToolbar items]` — stub, returns 0 (empty).
pub extern "C" fn shim_nstoolbar_items(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    0
}

/// `-[NSToolbar visibleItems]` — stub, returns 0 (empty).
pub extern "C" fn shim_nstoolbar_visible_items(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    0
}

/// `-[NSToolbar validateVisibleItems]` — no-op stub.
pub extern "C" fn shim_nstoolbar_validate_visible_items(_obj: u64, _cmd: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstoolbar_init_with_identifier(0, 0, 0x1234), 0);
        assert_eq!(shim_nstoolbar_identifier(0, 0), 0);
        assert_eq!(shim_nstoolbar_delegate(0, 0), 0);
        shim_nstoolbar_set_delegate(0, 0, 0x1234);
        assert_eq!(shim_nstoolbar_is_visible(0, 0), 0);
        shim_nstoolbar_set_visible(0, 0, 1);
        assert_eq!(shim_nstoolbar_allows_user_customization(0, 0), 0);
        shim_nstoolbar_set_allows_user_customization(0, 0, 1);
        assert_eq!(shim_nstoolbar_items(0, 0), 0);
        assert_eq!(shim_nstoolbar_visible_items(0, 0), 0);
        shim_nstoolbar_validate_visible_items(0, 0);
    }

    #[test]
    fn init_with_identifier() {
        let tb = alloc_obj(TOOLBAR_SIZE, 0xDEAD);
        let ident = make_string("MainToolbar");
        let ret = shim_nstoolbar_init_with_identifier(tb, 0, ident);
        assert_eq!(ret, tb);
        assert_eq!(shim_nstoolbar_identifier(tb, 0), ident);
        // Visible by default after init
        assert_eq!(shim_nstoolbar_is_visible(tb, 0), 1);
        free_obj(tb);
    }

    #[test]
    fn visible_toggle() {
        let tb = alloc_obj(TOOLBAR_SIZE, 0xDEAD);
        shim_nstoolbar_init_with_identifier(tb, 0, 0);
        assert_eq!(shim_nstoolbar_is_visible(tb, 0), 1);
        shim_nstoolbar_set_visible(tb, 0, 0);
        assert_eq!(shim_nstoolbar_is_visible(tb, 0), 0);
        shim_nstoolbar_set_visible(tb, 0, 1);
        assert_eq!(shim_nstoolbar_is_visible(tb, 0), 1);
        free_obj(tb);
    }

    #[test]
    fn delegate_round_trip() {
        let tb = alloc_obj(TOOLBAR_SIZE, 0xDEAD);
        assert_eq!(shim_nstoolbar_delegate(tb, 0), 0);
        let del = 0xCAFE_u64;
        shim_nstoolbar_set_delegate(tb, 0, del);
        assert_eq!(shim_nstoolbar_delegate(tb, 0), del);
        free_obj(tb);
    }

    #[test]
    fn customization_flag() {
        let tb = alloc_obj(TOOLBAR_SIZE, 0xDEAD);
        shim_nstoolbar_init_with_identifier(tb, 0, 0);
        // Not allowed by default
        assert_eq!(shim_nstoolbar_allows_user_customization(tb, 0), 0);
        shim_nstoolbar_set_allows_user_customization(tb, 0, 1);
        assert_eq!(shim_nstoolbar_allows_user_customization(tb, 0), 1);
        // Visible flag should be unaffected
        assert_eq!(shim_nstoolbar_is_visible(tb, 0), 1);
        shim_nstoolbar_set_allows_user_customization(tb, 0, 0);
        assert_eq!(shim_nstoolbar_allows_user_customization(tb, 0), 0);
        free_obj(tb);
    }
}
