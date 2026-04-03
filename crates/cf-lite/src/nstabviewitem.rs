//! NSTabViewItem shim: individual tab item for NSTabView.
//!
//! Object layout (inherits NSObject):
//!
//!   offset  0: isa                  (u64)
//!   offset  8: label_ptr            (u64 — NSString*, or 0)
//!   offset 16: view_ptr             (u64 — NSView*, or 0)
//!   offset 24: identifier           (u64 — id, or 0)
//!
//! Total: 32 bytes.

const LABEL_OFF: usize = 8;
const VIEW_OFF: usize = 16;
const IDENTIFIER_OFF: usize = 24;
pub(crate) const TAB_VIEW_ITEM_SIZE: usize = 32;

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

/// `-[NSTabViewItem label]`
pub extern "C" fn shim_nstabviewitem_label(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, LABEL_OFF)
}

/// `-[NSTabViewItem setLabel:]`
pub extern "C" fn shim_nstabviewitem_set_label(obj: u64, _cmd: u64, label: u64) {
    write_u64(obj, LABEL_OFF, label);
}

/// `-[NSTabViewItem view]`
pub extern "C" fn shim_nstabviewitem_view(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, VIEW_OFF)
}

/// `-[NSTabViewItem setView:]`
pub extern "C" fn shim_nstabviewitem_set_view(obj: u64, _cmd: u64, view: u64) {
    write_u64(obj, VIEW_OFF, view);
}

/// `-[NSTabViewItem identifier]`
pub extern "C" fn shim_nstabviewitem_identifier(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, IDENTIFIER_OFF)
}

/// `-[NSTabViewItem setIdentifier:]`
pub extern "C" fn shim_nstabviewitem_set_identifier(obj: u64, _cmd: u64, ident: u64) {
    write_u64(obj, IDENTIFIER_OFF, ident);
}

/// `-[NSTabViewItem initWithIdentifier:]` — instance init; stores identifier, returns obj.
pub extern "C" fn shim_nstabviewitem_init_with_identifier(obj: u64, _cmd: u64, ident: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    write_u64(obj, IDENTIFIER_OFF, ident);
    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstabviewitem_label(0, 0), 0);
        shim_nstabviewitem_set_label(0, 0, 0x1234);
        assert_eq!(shim_nstabviewitem_view(0, 0), 0);
        shim_nstabviewitem_set_view(0, 0, 0x1234);
        assert_eq!(shim_nstabviewitem_identifier(0, 0), 0);
        shim_nstabviewitem_set_identifier(0, 0, 0x1234);
        assert_eq!(shim_nstabviewitem_init_with_identifier(0, 0, 0x1234), 0);
    }

    #[test]
    fn label_round_trip() {
        let item = alloc_obj(TAB_VIEW_ITEM_SIZE, 0xDEAD);
        let label = make_string("Tab One");
        shim_nstabviewitem_set_label(item, 0, label);
        assert_eq!(shim_nstabviewitem_label(item, 0), label);
        free_obj(item);
    }

    #[test]
    fn view_round_trip() {
        let item = alloc_obj(TAB_VIEW_ITEM_SIZE, 0xDEAD);
        let fake_view = 0x5000_u64;
        shim_nstabviewitem_set_view(item, 0, fake_view);
        assert_eq!(shim_nstabviewitem_view(item, 0), fake_view);
        free_obj(item);
    }

    #[test]
    fn init_with_identifier() {
        let item = alloc_obj(TAB_VIEW_ITEM_SIZE, 0xDEAD);
        let ident = 0xCAFE_u64;
        let ret = shim_nstabviewitem_init_with_identifier(item, 0, ident);
        assert_eq!(ret, item);
        assert_eq!(shim_nstabviewitem_identifier(item, 0), ident);
        // Can update via setter too
        shim_nstabviewitem_set_identifier(item, 0, 0xF00D);
        assert_eq!(shim_nstabviewitem_identifier(item, 0), 0xF00D);
        free_obj(item);
    }
}
