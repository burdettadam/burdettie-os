//! NSBox shim: titled box container for AppKit views.
//!
//! Object layout (96 bytes):
//!
//!   [0..72):  inherited from NSView (ISA at [0..8))
//!   [72..80): title_ptr        (u64 — NSString*)
//!   [80..88): content_view_ptr (u64 — the box's content view)
//!   [88..96): box_type         (u64 — 0=primary, 1=secondary, 2=separator,
//!                                     3=custom, 4=no border)

const ISA_OFF: usize = 0;
const TITLE_OFF: usize = 72;
const CONTENT_VIEW_OFF: usize = 80;
const BOX_TYPE_OFF: usize = 88;
pub(crate) const BOX_SIZE: usize = 96;

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

/// `-[NSBox title]` — returns the title NSString*, or an empty string if nil.
pub extern "C" fn shim_nsbox_title(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ptr = read_u64(obj, TITLE_OFF);
    if ptr == 0 {
        crate::cfstring::alloc_nsstring_from_cstr(c"".as_ptr())
    } else {
        ptr
    }
}

/// `-[NSBox setTitle:]`
pub extern "C" fn shim_nsbox_set_title(obj: u64, _cmd: u64, title: u64) {
    write_u64(obj, TITLE_OFF, title);
}

/// `-[NSBox contentView]`
pub extern "C" fn shim_nsbox_content_view(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, CONTENT_VIEW_OFF)
}

/// `-[NSBox setContentView:]`
pub extern "C" fn shim_nsbox_set_content_view(obj: u64, _cmd: u64, view: u64) {
    write_u64(obj, CONTENT_VIEW_OFF, view);
}

/// `-[NSBox boxType]`
pub extern "C" fn shim_nsbox_box_type(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, BOX_TYPE_OFF)
}

/// `-[NSBox setBoxType:]`
pub extern "C" fn shim_nsbox_set_box_type(obj: u64, _cmd: u64, btype: u64) {
    write_u64(obj, BOX_TYPE_OFF, btype);
}

/// `-[NSBox isTransparent]` — stub, returns NO.
pub extern "C" fn shim_nsbox_is_transparent(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    0
}

/// `-[NSBox setTransparent:]` — no-op stub.
pub extern "C" fn shim_nsbox_set_transparent(_obj: u64, _cmd: u64, _val: u8) {}

/// `-[NSBox sizeToFit]` — no-op stub.
pub extern "C" fn shim_nsbox_size_to_fit(_obj: u64, _cmd: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsbox_title(0, 0), 0);
        shim_nsbox_set_title(0, 0, 0x1234);
        assert_eq!(shim_nsbox_content_view(0, 0), 0);
        shim_nsbox_set_content_view(0, 0, 0x1234);
        assert_eq!(shim_nsbox_box_type(0, 0), 0);
        shim_nsbox_set_box_type(0, 0, 1);
        assert_eq!(shim_nsbox_is_transparent(0, 0), 0);
        shim_nsbox_set_transparent(0, 0, 1);
        shim_nsbox_size_to_fit(0, 0);
    }

    #[test]
    fn title_round_trip() {
        let bx = alloc_obj(BOX_SIZE, 0xDEAD);
        let title = make_string("Group Box");
        shim_nsbox_set_title(bx, 0, title);
        assert_eq!(shim_nsbox_title(bx, 0), title);
        free_obj(bx);
    }

    #[test]
    fn content_view_round_trip() {
        let bx = alloc_obj(BOX_SIZE, 0xDEAD);
        assert_eq!(shim_nsbox_content_view(bx, 0), 0);
        let fake_view = 0x5000_u64;
        shim_nsbox_set_content_view(bx, 0, fake_view);
        assert_eq!(shim_nsbox_content_view(bx, 0), fake_view);
        free_obj(bx);
    }

    #[test]
    fn box_type_round_trip() {
        let bx = alloc_obj(BOX_SIZE, 0xDEAD);
        assert_eq!(shim_nsbox_box_type(bx, 0), 0); // default: primary
        shim_nsbox_set_box_type(bx, 0, 3); // custom
        assert_eq!(shim_nsbox_box_type(bx, 0), 3);
        shim_nsbox_set_box_type(bx, 0, 4); // no border
        assert_eq!(shim_nsbox_box_type(bx, 0), 4);
        free_obj(bx);
    }
}
