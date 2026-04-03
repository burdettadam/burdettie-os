//! NSViewController shim: view controller for Cocoa GUI apps.
//!
//! Object layout:
//!
//!   offset  0: isa                  (u64)
//!   offset  8: view_ptr             (u64 — NSView*, or 0)
//!   offset 16: title_ptr            (u64 — NSString*, or 0)
//!   offset 24: represented_object   (u64 — id, or 0)
//!   offset 32: flags                (u64 — bit 0: view_loaded)
//!
//! Total: 40 bytes.

const VIEW_OFF: usize = 8;
const TITLE_OFF: usize = 16;
const REPRESENTED_OBJECT_OFF: usize = 24;
const FLAGS_OFF: usize = 32;
pub(crate) const VIEWCONTROLLER_SIZE: usize = 40;

const FLAG_VIEW_LOADED: u64 = 1 << 0;

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

/// `-[NSViewController view]`
pub extern "C" fn shim_nsviewcontroller_view(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_VIEW_LOADED == 0 {
        // Trigger loadView if not yet loaded
        shim_nsviewcontroller_load_view(obj, 0);
    }
    read_u64(obj, VIEW_OFF)
}

/// `-[NSViewController setView:]`
pub extern "C" fn shim_nsviewcontroller_set_view(obj: u64, _cmd: u64, view: u64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, VIEW_OFF, view);
    let flags = read_u64(obj, FLAGS_OFF);
    write_u64(obj, FLAGS_OFF, flags | FLAG_VIEW_LOADED);
}

/// `-[NSViewController title]`
pub extern "C" fn shim_nsviewcontroller_title(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TITLE_OFF)
}

/// `-[NSViewController setTitle:]`
pub extern "C" fn shim_nsviewcontroller_set_title(obj: u64, _cmd: u64, title: u64) {
    write_u64(obj, TITLE_OFF, title);
}

/// `-[NSViewController representedObject]`
pub extern "C" fn shim_nsviewcontroller_represented_object(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, REPRESENTED_OBJECT_OFF)
}

/// `-[NSViewController setRepresentedObject:]`
pub extern "C" fn shim_nsviewcontroller_set_represented_object(obj: u64, _cmd: u64, rep: u64) {
    write_u64(obj, REPRESENTED_OBJECT_OFF, rep);
}

/// `-[NSViewController loadView]` — stub: creates a default empty view if none set.
pub extern "C" fn shim_nsviewcontroller_load_view(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_VIEW_LOADED != 0 {
        return;
    }
    // Create a default empty NSView (0,0,0,0)
    let view = crate::pal_alloc().calloc(1, crate::nsview::VIEW_SIZE);
    if !view.is_null() {
        let view_addr = view as u64;
        unsafe {
            *(view_addr as *mut u64) = crate::nsobj::nsview_class_addr();
        }
        write_u64(obj, VIEW_OFF, view_addr);
    }
    write_u64(obj, FLAGS_OFF, flags | FLAG_VIEW_LOADED);
}

/// `-[NSViewController viewDidLoad]` — no-op stub (subclass override point).
pub extern "C" fn shim_nsviewcontroller_view_did_load(_obj: u64, _cmd: u64) {}

/// `-[NSViewController viewWillAppear]` — no-op stub.
pub extern "C" fn shim_nsviewcontroller_view_will_appear(_obj: u64, _cmd: u64) {}

/// `-[NSViewController viewDidAppear]` — no-op stub.
pub extern "C" fn shim_nsviewcontroller_view_did_appear(_obj: u64, _cmd: u64) {}

/// `-[NSViewController viewWillDisappear]` — no-op stub.
pub extern "C" fn shim_nsviewcontroller_view_will_disappear(_obj: u64, _cmd: u64) {}

/// `-[NSViewController viewDidDisappear]` — no-op stub.
pub extern "C" fn shim_nsviewcontroller_view_did_disappear(_obj: u64, _cmd: u64) {}

/// `-[NSViewController isViewLoaded]`
pub extern "C" fn shim_nsviewcontroller_is_view_loaded(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & FLAG_VIEW_LOADED != 0)
}

/// `-[NSViewController nibName]` — returns nil (no nib).
pub extern "C" fn shim_nsviewcontroller_nib_name(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSViewController nibBundle]` — returns nil.
pub extern "C" fn shim_nsviewcontroller_nib_bundle(_obj: u64, _cmd: u64) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsviewcontroller_view(0, 0), 0);
        shim_nsviewcontroller_set_view(0, 0, 0x1234);
        assert_eq!(shim_nsviewcontroller_title(0, 0), 0);
        shim_nsviewcontroller_set_title(0, 0, 0x1234);
        assert_eq!(shim_nsviewcontroller_represented_object(0, 0), 0);
        shim_nsviewcontroller_set_represented_object(0, 0, 0x1234);
        shim_nsviewcontroller_load_view(0, 0);
        shim_nsviewcontroller_view_did_load(0, 0);
        assert_eq!(shim_nsviewcontroller_is_view_loaded(0, 0), 0);
        assert_eq!(shim_nsviewcontroller_nib_name(0, 0), 0);
        assert_eq!(shim_nsviewcontroller_nib_bundle(0, 0), 0);
    }

    #[test]
    fn view_access_triggers_load_view() {
        let vc = alloc_obj(VIEWCONTROLLER_SIZE, 0xDEAD);
        // Initially not loaded
        assert_eq!(shim_nsviewcontroller_is_view_loaded(vc, 0), 0);
        // Accessing view triggers loadView
        let view = shim_nsviewcontroller_view(vc, 0);
        assert_ne!(view, 0, "loadView should create a default view");
        assert_eq!(shim_nsviewcontroller_is_view_loaded(vc, 0), 1);
        // Free the auto-created view
        free_obj(view);
        free_obj(vc);
    }

    #[test]
    fn set_view_marks_loaded() {
        let vc = alloc_obj(VIEWCONTROLLER_SIZE, 0xDEAD);
        let fake_view = 0x4000_u64;
        shim_nsviewcontroller_set_view(vc, 0, fake_view);
        assert_eq!(shim_nsviewcontroller_is_view_loaded(vc, 0), 1);
        assert_eq!(shim_nsviewcontroller_view(vc, 0), fake_view);
        free_obj(vc);
    }

    #[test]
    fn title_round_trip() {
        let vc = alloc_obj(VIEWCONTROLLER_SIZE, 0xDEAD);
        let title = make_string("My Controller");
        shim_nsviewcontroller_set_title(vc, 0, title);
        assert_eq!(shim_nsviewcontroller_title(vc, 0), title);
        free_obj(vc);
    }

    #[test]
    fn represented_object_round_trip() {
        let vc = alloc_obj(VIEWCONTROLLER_SIZE, 0xDEAD);
        let obj = 0x9999_u64;
        shim_nsviewcontroller_set_represented_object(vc, 0, obj);
        assert_eq!(shim_nsviewcontroller_represented_object(vc, 0), obj);
        free_obj(vc);
    }

    #[test]
    fn lifecycle_stubs_are_noop() {
        let vc = alloc_obj(VIEWCONTROLLER_SIZE, 0xDEAD);
        shim_nsviewcontroller_view_did_load(vc, 0);
        shim_nsviewcontroller_view_will_appear(vc, 0);
        shim_nsviewcontroller_view_did_appear(vc, 0);
        shim_nsviewcontroller_view_will_disappear(vc, 0);
        shim_nsviewcontroller_view_did_disappear(vc, 0);
        free_obj(vc);
    }

    #[test]
    fn nib_returns_nil() {
        let vc = alloc_obj(VIEWCONTROLLER_SIZE, 0xDEAD);
        assert_eq!(shim_nsviewcontroller_nib_name(vc, 0), 0);
        assert_eq!(shim_nsviewcontroller_nib_bundle(vc, 0), 0);
        free_obj(vc);
    }
}
