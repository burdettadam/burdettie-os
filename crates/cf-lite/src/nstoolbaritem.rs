//! NSToolbarItem shim: individual items in an NSToolbar.
//!
//! Object layout:
//!
//!   offset  0: isa                  (u64)
//!   offset  8: item_identifier_ptr  (u64 — NSString*)
//!   offset 16: label_ptr            (u64 — NSString*)
//!   offset 24: image_ptr            (u64 — NSImage*)
//!   offset 32: target_ptr           (u64)
//!   offset 40: action_sel           (u64 — SEL)
//!
//! Total: 48 bytes.

const ITEM_IDENTIFIER_OFF: usize = 8;
const LABEL_OFF: usize = 16;
const IMAGE_OFF: usize = 24;
const TARGET_OFF: usize = 32;
const ACTION_OFF: usize = 40;
pub(crate) const TOOLBAR_ITEM_SIZE: usize = 48;

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

/// `-[NSToolbarItem initWithItemIdentifier:]`
pub extern "C" fn shim_nstoolbaritem_init_with_item_identifier(
    obj: u64,
    _cmd: u64,
    ident: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    write_u64(obj, ITEM_IDENTIFIER_OFF, ident);
    obj
}

/// `-[NSToolbarItem itemIdentifier]`
pub extern "C" fn shim_nstoolbaritem_item_identifier(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ITEM_IDENTIFIER_OFF)
}

/// `-[NSToolbarItem label]`
pub extern "C" fn shim_nstoolbaritem_label(obj: u64, _cmd: u64) -> u64 {
    let ptr = read_u64(obj, LABEL_OFF);
    if ptr != 0 {
        return ptr;
    }
    // Return empty string when no label set
    crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr())
}

/// `-[NSToolbarItem setLabel:]`
pub extern "C" fn shim_nstoolbaritem_set_label(obj: u64, _cmd: u64, label: u64) {
    write_u64(obj, LABEL_OFF, label);
}

/// `-[NSToolbarItem image]`
pub extern "C" fn shim_nstoolbaritem_image(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, IMAGE_OFF)
}

/// `-[NSToolbarItem setImage:]`
pub extern "C" fn shim_nstoolbaritem_set_image(obj: u64, _cmd: u64, img: u64) {
    write_u64(obj, IMAGE_OFF, img);
}

/// `-[NSToolbarItem target]`
pub extern "C" fn shim_nstoolbaritem_target(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TARGET_OFF)
}

/// `-[NSToolbarItem setTarget:]`
pub extern "C" fn shim_nstoolbaritem_set_target(obj: u64, _cmd: u64, tgt: u64) {
    write_u64(obj, TARGET_OFF, tgt);
}

/// `-[NSToolbarItem action]`
pub extern "C" fn shim_nstoolbaritem_action(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ACTION_OFF)
}

/// `-[NSToolbarItem setAction:]`
pub extern "C" fn shim_nstoolbaritem_set_action(obj: u64, _cmd: u64, sel: u64) {
    write_u64(obj, ACTION_OFF, sel);
}

/// `-[NSToolbarItem isEnabled]` — stub, always returns 1.
pub extern "C" fn shim_nstoolbaritem_is_enabled(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    1
}

/// `-[NSToolbarItem setEnabled:]` — no-op stub.
pub extern "C" fn shim_nstoolbaritem_set_enabled(_obj: u64, _cmd: u64, _val: u8) {}

/// `-[NSToolbarItem validate]` — no-op stub.
pub extern "C" fn shim_nstoolbaritem_validate(_obj: u64, _cmd: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstoolbaritem_init_with_item_identifier(0, 0, 0x1234), 0);
        assert_eq!(shim_nstoolbaritem_item_identifier(0, 0), 0);
        assert_eq!(shim_nstoolbaritem_image(0, 0), 0);
        shim_nstoolbaritem_set_image(0, 0, 0x1234);
        assert_eq!(shim_nstoolbaritem_target(0, 0), 0);
        shim_nstoolbaritem_set_target(0, 0, 0x1234);
        assert_eq!(shim_nstoolbaritem_action(0, 0), 0);
        shim_nstoolbaritem_set_action(0, 0, 0x1234);
        assert_eq!(shim_nstoolbaritem_is_enabled(0, 0), 0);
        shim_nstoolbaritem_set_enabled(0, 0, 1);
        shim_nstoolbaritem_validate(0, 0);
    }

    #[test]
    fn init_with_item_identifier() {
        let item = alloc_obj(TOOLBAR_ITEM_SIZE, 0xDEAD);
        let ident = make_string("com.example.item");
        let ret = shim_nstoolbaritem_init_with_item_identifier(item, 0, ident);
        assert_eq!(ret, item);
        assert_eq!(shim_nstoolbaritem_item_identifier(item, 0), ident);
        free_obj(item);
    }

    #[test]
    fn label_round_trip() {
        let item = alloc_obj(TOOLBAR_ITEM_SIZE, 0xDEAD);
        // No label set → returns empty string (non-zero)
        let empty = shim_nstoolbaritem_label(item, 0);
        assert_ne!(empty, 0);
        let label = make_string("Save");
        shim_nstoolbaritem_set_label(item, 0, label);
        assert_eq!(shim_nstoolbaritem_label(item, 0), label);
        free_obj(item);
    }

    #[test]
    fn image_round_trip() {
        let item = alloc_obj(TOOLBAR_ITEM_SIZE, 0xDEAD);
        assert_eq!(shim_nstoolbaritem_image(item, 0), 0);
        let img = 0xBEEF_u64;
        shim_nstoolbaritem_set_image(item, 0, img);
        assert_eq!(shim_nstoolbaritem_image(item, 0), img);
        free_obj(item);
    }

    #[test]
    fn target_action_round_trip() {
        let item = alloc_obj(TOOLBAR_ITEM_SIZE, 0xDEAD);
        assert_eq!(shim_nstoolbaritem_target(item, 0), 0);
        assert_eq!(shim_nstoolbaritem_action(item, 0), 0);
        let tgt = 0xAAAA_u64;
        let sel = 0xBBBB_u64;
        shim_nstoolbaritem_set_target(item, 0, tgt);
        shim_nstoolbaritem_set_action(item, 0, sel);
        assert_eq!(shim_nstoolbaritem_target(item, 0), tgt);
        assert_eq!(shim_nstoolbaritem_action(item, 0), sel);
        free_obj(item);
    }
}
