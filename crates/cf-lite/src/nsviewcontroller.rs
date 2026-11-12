//! NSViewController shim: view controller for Cocoa GUI apps.
//!
//! Object layout:
//!
//!   offset  0: isa                  (u64)
//!   offset  8: view_ptr             (u64 — NSView*, or 0)
//!   offset 16: title_ptr            (u64 — NSString*, or 0)
//!   offset 24: represented_object   (u64 — id, or 0)
//!   offset 32: nib_name             (u64 — NSString*, or 0)
//!   offset 40: nib_bundle           (u64 — NSBundle*, or 0 = mainBundle)
//!   offset 48: flags                (u64 — bit 0: view_loaded)
//!
//! Total: 56 bytes.

const VIEW_OFF: usize = 8;
const TITLE_OFF: usize = 16;
const REPRESENTED_OBJECT_OFF: usize = 24;
const NIB_NAME_OFF: usize = 32;
const NIB_BUNDLE_OFF: usize = 40;
const FLAGS_OFF: usize = 48;
pub(crate) const VIEWCONTROLLER_SIZE: usize = 56;

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

fn object_is_kind_of(obj: u64, class_addr: u64) -> bool {
    if obj == 0 || class_addr == 0 {
        return false;
    }

    (unsafe { *(obj as *const u64) == class_addr })
        || crate::nsobj::shim_nsobject_is_kind_of_class(obj, 0, class_addr) != 0
}

fn first_top_level_object_kind_of(top_level: u64, class_addr: u64) -> u64 {
    if top_level == 0 {
        return 0;
    }

    if object_is_kind_of(top_level, class_addr) {
        return top_level;
    }

    let isa = unsafe { *(top_level as *const u64) };
    if isa != crate::nsobj::nsarray_class_addr() && isa != crate::nsobj::nsmutablearray_class_addr()
    {
        return 0;
    }

    let count = crate::nsarray::shim_nsarray_count(top_level, 0);
    for index in 0..count {
        let item = crate::nsarray::shim_nsarray_object_at_index(top_level, 0, index);
        if object_is_kind_of(item, class_addr) {
            return item;
        }
    }
    0
}

fn load_nib_named(owner: u64, nib_name: u64, nib_bundle: u64) -> u64 {
    if nib_name == 0 {
        return 0;
    }

    let bundle = if nib_bundle == 0 {
        crate::nsbundle::get_or_create_main_bundle()
    } else {
        nib_bundle
    };
    if bundle == 0 {
        return 0;
    }

    let mut top_level = 0_u64;
    let ok = crate::nsbundle::shim_nsbundle_load_nib_named(
        bundle,
        0,
        nib_name,
        owner,
        (&mut top_level as *mut u64) as u64,
    );
    if ok != 0 {
        top_level
    } else {
        0
    }
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSViewController initWithNibName:bundle:]`
pub extern "C" fn shim_nsviewcontroller_init_with_nib_name_bundle(
    obj: u64,
    _cmd: u64,
    nib_name: u64,
    bundle: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    write_u64(obj, NIB_NAME_OFF, nib_name);
    write_u64(obj, NIB_BUNDLE_OFF, bundle);
    write_u64(obj, FLAGS_OFF, 0);
    obj
}

/// `-[NSViewController initWithCoder:]`
pub extern "C" fn shim_nsviewcontroller_init_with_coder(obj: u64, _cmd: u64, coder: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    write_u64(obj, FLAGS_OFF, 0);

    let view = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSView", "NSViewControllerView", "view", "_view"],
    );
    if view != 0 {
        write_u64(obj, VIEW_OFF, view);
        write_u64(obj, FLAGS_OFF, FLAG_VIEW_LOADED);
    }

    let title = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSTitle", "NSViewControllerTitle", "title", "_title"],
    );
    if title != 0 {
        write_u64(obj, TITLE_OFF, title);
    }

    let represented = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &[
            "NSRepresentedObject",
            "representedObject",
            "_representedObject",
        ],
    );
    if represented != 0 {
        write_u64(obj, REPRESENTED_OBJECT_OFF, represented);
    }

    let nib_name = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &[
            "NSNibName",
            "NSViewControllerNibName",
            "nibName",
            "_nibName",
        ],
    );
    if nib_name != 0 {
        write_u64(obj, NIB_NAME_OFF, nib_name);
    }

    let nib_bundle = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &[
            "NSNibBundle",
            "NSViewControllerNibBundle",
            "nibBundle",
            "_nibBundle",
        ],
    );
    if nib_bundle != 0 {
        write_u64(obj, NIB_BUNDLE_OFF, nib_bundle);
    }

    obj
}

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

/// `-[NSViewController loadView]`
///
/// Loads a configured nib through NSBundle when present, then falls back to
/// a default empty NSView if the nib did not connect or provide one.
pub extern "C" fn shim_nsviewcontroller_load_view(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_VIEW_LOADED != 0 {
        return;
    }

    let nib_name = read_u64(obj, NIB_NAME_OFF);
    let nib_bundle = read_u64(obj, NIB_BUNDLE_OFF);
    if nib_name != 0 {
        let top_level = load_nib_named(obj, nib_name, nib_bundle);
        if read_u64(obj, VIEW_OFF) == 0 {
            let top_view =
                first_top_level_object_kind_of(top_level, crate::nsobj::nsview_class_addr());
            if top_view != 0 {
                write_u64(obj, VIEW_OFF, top_view);
            }
        }
    }

    if read_u64(obj, VIEW_OFF) == 0 {
        let view = crate::pal_alloc().calloc(1, crate::nsview::VIEW_SIZE);
        if !view.is_null() {
            let view_addr = view as u64;
            unsafe {
                *(view_addr as *mut u64) = crate::nsobj::nsview_class_addr();
            }
            write_u64(obj, VIEW_OFF, view_addr);
        }
    }

    write_u64(obj, FLAGS_OFF, flags | FLAG_VIEW_LOADED);
    shim_nsviewcontroller_view_did_load(obj, 0);
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

/// `-[NSViewController nibName]`
pub extern "C" fn shim_nsviewcontroller_nib_name(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, NIB_NAME_OFF)
}

/// `-[NSViewController nibBundle]`
pub extern "C" fn shim_nsviewcontroller_nib_bundle(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, NIB_BUNDLE_OFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn make_coder() -> u64 {
        crate::nskeyedarchiver::shim_nskeyedarchiver_alloc(0, 0)
    }

    fn encode_object(coder: u64, key: &str, value: u64) {
        crate::nskeyedarchiver::shim_nskeyedarchiver_encode_object_for_key(
            coder,
            0,
            value,
            make_string(key),
        );
    }

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
        assert_eq!(
            shim_nsviewcontroller_init_with_nib_name_bundle(0, 0, 0, 0),
            0
        );
        assert_eq!(shim_nsviewcontroller_init_with_coder(0, 0, 0), 0);
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
    fn nib_name_and_bundle_round_trip() {
        let vc = alloc_obj(VIEWCONTROLLER_SIZE, 0xDEAD);
        let name = make_string("MainView");
        let bundle = alloc_obj_fake(16);
        let ret = shim_nsviewcontroller_init_with_nib_name_bundle(vc, 0, name, bundle);
        assert_eq!(ret, vc);
        assert_eq!(shim_nsviewcontroller_nib_name(vc, 0), name);
        assert_eq!(shim_nsviewcontroller_nib_bundle(vc, 0), bundle);
        free_obj(bundle);
        free_obj(vc);
    }

    #[test]
    fn init_with_coder_decodes_archived_view_and_metadata() {
        let vc = alloc_obj(
            VIEWCONTROLLER_SIZE,
            crate::nsobj::nsviewcontroller_class_addr(),
        );
        let view = crate::nsview::alloc_view_with_frame(0.0, 0.0, 320.0, 200.0);
        let title = make_string("Archived VC");
        let represented = alloc_obj_fake(16);
        let nib_name = make_string("ArchivedScene");
        let nib_bundle = alloc_obj_fake(16);
        let coder = make_coder();

        encode_object(coder, "NSView", view);
        encode_object(coder, "NSTitle", title);
        encode_object(coder, "NSRepresentedObject", represented);
        encode_object(coder, "NSNibName", nib_name);
        encode_object(coder, "NSNibBundle", nib_bundle);

        assert_eq!(shim_nsviewcontroller_init_with_coder(vc, 0, coder), vc);
        assert_eq!(shim_nsviewcontroller_view(vc, 0), view);
        assert_eq!(shim_nsviewcontroller_is_view_loaded(vc, 0), 1);
        assert_eq!(shim_nsviewcontroller_title(vc, 0), title);
        assert_eq!(shim_nsviewcontroller_represented_object(vc, 0), represented);
        assert_eq!(shim_nsviewcontroller_nib_name(vc, 0), nib_name);
        assert_eq!(shim_nsviewcontroller_nib_bundle(vc, 0), nib_bundle);

        free_obj(view);
        free_obj(represented);
        free_obj(nib_bundle);
        free_obj(vc);
    }

    #[test]
    fn load_view_with_missing_nib_falls_back_to_default_view() {
        let vc = alloc_obj(VIEWCONTROLLER_SIZE, 0xDEAD);
        let name = make_string("DefinitelyMissingNibForUnitTest");
        shim_nsviewcontroller_init_with_nib_name_bundle(vc, 0, name, 0);

        let view = shim_nsviewcontroller_view(vc, 0);
        assert_ne!(view, 0, "missing nib should still create a default view");
        assert_eq!(shim_nsviewcontroller_is_view_loaded(vc, 0), 1);

        free_obj(view);
        free_obj(vc);
    }
}
