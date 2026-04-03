//! NSTabView shim: tabbed container view for Cocoa GUI apps.
//!
//! Object layout (inherits NSView, 72 bytes):
//!
//!   offset  0: isa                  (u64) — from NSView
//!   offset  8..72: inherited NSView fields
//!   offset 72: delegate_ptr         (u64 — id, or 0)
//!   offset 80: tab_view_data        (u64 — selected_index as i64)
//!
//! Total: 88 bytes.

const DELEGATE_OFF: usize = 72;
const TAB_VIEW_DATA_OFF: usize = 80;
pub(crate) const TAB_VIEW_SIZE: usize = 88;

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

/// `-[NSTabView numberOfTabViewItems]` — stub returning 0.
pub extern "C" fn shim_nstabview_number_of_tab_view_items(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    0
}

/// `-[NSTabView delegate]`
pub extern "C" fn shim_nstabview_delegate(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DELEGATE_OFF)
}

/// `-[NSTabView setDelegate:]`
pub extern "C" fn shim_nstabview_set_delegate(obj: u64, _cmd: u64, del: u64) {
    write_u64(obj, DELEGATE_OFF, del);
}

/// `-[NSTabView selectTabViewItemAtIndex:]`
pub extern "C" fn shim_nstabview_select_tab_view_item_at_index(obj: u64, _cmd: u64, idx: i64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, TAB_VIEW_DATA_OFF, idx as u64);
}

/// `-[NSTabView selectedTabViewItem]` — stub returning nil.
pub extern "C" fn shim_nstabview_selected_tab_view_item(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    0
}

/// `-[NSTabView addTabViewItem:]` — no-op stub.
pub extern "C" fn shim_nstabview_add_tab_view_item(_obj: u64, _cmd: u64, _item: u64) {}

/// `-[NSTabView removeTabViewItem:]` — no-op stub.
pub extern "C" fn shim_nstabview_remove_tab_view_item(_obj: u64, _cmd: u64, _item: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstabview_number_of_tab_view_items(0, 0), 0);
        assert_eq!(shim_nstabview_delegate(0, 0), 0);
        shim_nstabview_set_delegate(0, 0, 0x1234);
        shim_nstabview_select_tab_view_item_at_index(0, 0, 5);
        assert_eq!(shim_nstabview_selected_tab_view_item(0, 0), 0);
        shim_nstabview_add_tab_view_item(0, 0, 0x1234);
        shim_nstabview_remove_tab_view_item(0, 0, 0x1234);
    }

    #[test]
    fn delegate_round_trip() {
        let tv = alloc_obj(TAB_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nstabview_delegate(tv, 0), 0);
        let fake_del = 0xBEEF_u64;
        shim_nstabview_set_delegate(tv, 0, fake_del);
        assert_eq!(shim_nstabview_delegate(tv, 0), fake_del);
        free_obj(tv);
    }

    #[test]
    fn select_tab_index() {
        let tv = alloc_obj(TAB_VIEW_SIZE, 0xDEAD);
        shim_nstabview_select_tab_view_item_at_index(tv, 0, 3);
        let stored = read_field(tv, TAB_VIEW_DATA_OFF);
        assert_eq!(stored, 3);
        shim_nstabview_select_tab_view_item_at_index(tv, 0, 0);
        let stored2 = read_field(tv, TAB_VIEW_DATA_OFF);
        assert_eq!(stored2, 0);
        free_obj(tv);
    }

    #[test]
    fn stubs_return_defaults() {
        let tv = alloc_obj(TAB_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nstabview_number_of_tab_view_items(tv, 0), 0);
        assert_eq!(shim_nstabview_selected_tab_view_item(tv, 0), 0);
        free_obj(tv);
    }
}
