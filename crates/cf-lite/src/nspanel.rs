//! NSPanel shim: panel window subclass for AppKit.
//!
//! NSPanel inherits from NSWindow. Only the panel-specific
//! `panel_flags` field is added beyond NSWindow's layout.
//!
//! Object layout:
//!
//!   offset  0..80: inherited from NSWindow
//!   offset 80: panel_flags (u64 — bit 0: is_floating_panel,
//!                                  bit 1: becomes_key_only_if_needed,
//!                                  bit 2: works_when_modal)
//!
//! Total: 88 bytes.

const PANEL_FLAGS_OFF: usize = 80;
pub(crate) const PANEL_SIZE: usize = 88;

const FLAG_IS_FLOATING_PANEL: u64 = 1 << 0;
const FLAG_BECOMES_KEY_ONLY_IF_NEEDED: u64 = 1 << 1;
const FLAG_WORKS_WHEN_MODAL: u64 = 1 << 2;

fn get_flag(obj: u64, bit: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = unsafe { *((obj as usize + PANEL_FLAGS_OFF) as *const u64) };
    u8::from(flags & bit != 0)
}

fn set_flag(obj: u64, bit: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let ptr = (obj as usize + PANEL_FLAGS_OFF) as *mut u64;
    unsafe {
        let mut flags = *ptr;
        if val != 0 {
            flags |= bit;
        } else {
            flags &= !bit;
        }
        *ptr = flags;
    }
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSPanel isFloatingPanel]`
pub extern "C" fn shim_nspanel_is_floating_panel(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_FLOATING_PANEL)
}

/// `-[NSPanel setFloatingPanel:]`
pub extern "C" fn shim_nspanel_set_floating_panel(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_FLOATING_PANEL, val);
}

/// `-[NSPanel becomesKeyOnlyIfNeeded]`
pub extern "C" fn shim_nspanel_becomes_key_only_if_needed(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_BECOMES_KEY_ONLY_IF_NEEDED)
}

/// `-[NSPanel setBecomesKeyOnlyIfNeeded:]`
pub extern "C" fn shim_nspanel_set_becomes_key_only_if_needed(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_BECOMES_KEY_ONLY_IF_NEEDED, val);
}

/// `-[NSPanel worksWhenModal]`
pub extern "C" fn shim_nspanel_works_when_modal(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_WORKS_WHEN_MODAL)
}

/// `-[NSPanel setWorksWhenModal:]`
pub extern "C" fn shim_nspanel_set_works_when_modal(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_WORKS_WHEN_MODAL, val);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn default_flags_are_zero() {
        let panel = alloc_obj(PANEL_SIZE, 0xDA9E);
        assert_eq!(shim_nspanel_is_floating_panel(panel, 0), 0);
        assert_eq!(shim_nspanel_becomes_key_only_if_needed(panel, 0), 0);
        assert_eq!(shim_nspanel_works_when_modal(panel, 0), 0);
        free_obj(panel);
    }

    #[test]
    fn toggle_floating_panel() {
        let panel = alloc_obj(PANEL_SIZE, 0xDA9E);
        shim_nspanel_set_floating_panel(panel, 0, 1);
        assert_eq!(shim_nspanel_is_floating_panel(panel, 0), 1);
        shim_nspanel_set_floating_panel(panel, 0, 0);
        assert_eq!(shim_nspanel_is_floating_panel(panel, 0), 0);
        free_obj(panel);
    }

    #[test]
    fn toggle_becomes_key_only_if_needed() {
        let panel = alloc_obj(PANEL_SIZE, 0xDA9E);
        shim_nspanel_set_becomes_key_only_if_needed(panel, 0, 1);
        assert_eq!(shim_nspanel_becomes_key_only_if_needed(panel, 0), 1);
        shim_nspanel_set_becomes_key_only_if_needed(panel, 0, 0);
        assert_eq!(shim_nspanel_becomes_key_only_if_needed(panel, 0), 0);
        free_obj(panel);
    }

    #[test]
    fn toggle_works_when_modal() {
        let panel = alloc_obj(PANEL_SIZE, 0xDA9E);
        shim_nspanel_set_works_when_modal(panel, 0, 1);
        assert_eq!(shim_nspanel_works_when_modal(panel, 0), 1);
        shim_nspanel_set_works_when_modal(panel, 0, 0);
        assert_eq!(shim_nspanel_works_when_modal(panel, 0), 0);
        free_obj(panel);
    }
}

