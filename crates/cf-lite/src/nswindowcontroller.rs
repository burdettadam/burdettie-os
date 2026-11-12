//! NSWindowController shim: window management controller.
//!
//! Object layout:
//!
//!   offset  0: isa              (u64)
//!   offset  8: window_ptr       (u64 — NSWindow*, or 0)
//!   offset 16: window_nib_name  (u64 — NSString*, or 0)
//!   offset 24: owner_ptr        (u64 — id, or 0 = self)
//!   offset 32: document_ptr     (u64 — id, or 0)
//!   offset 40: flags            (u64 — bit 0: should_cascade_windows,
//!                                bit 1: window_loaded)
//!
//! Total: 48 bytes.

const WINDOW_OFF: usize = 8;
const WINDOW_NIB_NAME_OFF: usize = 16;
const OWNER_OFF: usize = 24;
const DOCUMENT_OFF: usize = 32;
const FLAGS_OFF: usize = 40;
pub(crate) const WINDOWCONTROLLER_SIZE: usize = 48;

const FLAG_SHOULD_CASCADE: u64 = 1 << 0;
const FLAG_WINDOW_LOADED: u64 = 1 << 1;

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

fn set_flag(obj: u64, flag: u64, enabled: bool) {
    let mut flags = read_u64(obj, FLAGS_OFF);
    if enabled {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

fn default_flags() -> u64 {
    FLAG_SHOULD_CASCADE
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

fn load_nib_named(owner: u64, nib_name: u64) -> u64 {
    if nib_name == 0 {
        return 0;
    }

    let bundle = crate::nsbundle::get_or_create_main_bundle();
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

fn set_window_and_loaded(obj: u64, window: u64) {
    write_u64(obj, WINDOW_OFF, window);
    set_flag(obj, FLAG_WINDOW_LOADED, window != 0);
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSWindowController initWithWindow:]`
pub extern "C" fn shim_nswindowcontroller_init_with_window(
    obj: u64,
    _cmd: u64,
    window: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    set_window_and_loaded(obj, window);
    write_u64(
        obj,
        FLAGS_OFF,
        default_flags() | if window != 0 { FLAG_WINDOW_LOADED } else { 0 },
    );
    obj
}

/// `-[NSWindowController initWithWindowNibName:]`
pub extern "C" fn shim_nswindowcontroller_init_with_window_nib_name(
    obj: u64,
    _cmd: u64,
    name: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    write_u64(obj, WINDOW_NIB_NAME_OFF, name);
    write_u64(obj, OWNER_OFF, 0);
    write_u64(obj, FLAGS_OFF, default_flags());
    obj
}

/// `-[NSWindowController initWithWindowNibName:owner:]`
pub extern "C" fn shim_nswindowcontroller_init_with_window_nib_name_owner(
    obj: u64,
    _cmd: u64,
    name: u64,
    owner: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    write_u64(obj, WINDOW_NIB_NAME_OFF, name);
    write_u64(obj, OWNER_OFF, owner);
    write_u64(obj, FLAGS_OFF, default_flags());
    obj
}

/// `-[NSWindowController initWithCoder:]`
pub extern "C" fn shim_nswindowcontroller_init_with_coder(obj: u64, _cmd: u64, coder: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    write_u64(obj, FLAGS_OFF, default_flags());

    let window = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSWindow", "NSWindowControllerWindow", "window", "_window"],
    );
    if window != 0 {
        set_window_and_loaded(obj, window);
    }

    let nib_name = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &[
            "NSWindowNibName",
            "NSWindowControllerNibName",
            "NSNibName",
            "windowNibName",
            "_windowNibName",
        ],
    );
    if nib_name != 0 {
        write_u64(obj, WINDOW_NIB_NAME_OFF, nib_name);
    }

    let owner = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSOwner", "NSWindowControllerOwner", "owner", "_owner"],
    );
    if owner != 0 {
        write_u64(obj, OWNER_OFF, owner);
    }

    let document = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSDocument", "document", "_document"],
    );
    if document != 0 {
        write_u64(obj, DOCUMENT_OFF, document);
    }

    if let Some(should_cascade) = crate::nskeyedarchiver::decode_bool_for_any_key(
        coder,
        &[
            "NSShouldCascadeWindows",
            "shouldCascadeWindows",
            "_shouldCascadeWindows",
        ],
    ) {
        set_flag(obj, FLAG_SHOULD_CASCADE, should_cascade);
    }

    obj
}

/// `-[NSWindowController window]`
pub extern "C" fn shim_nswindowcontroller_window(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    if read_u64(obj, WINDOW_OFF) == 0 && read_u64(obj, WINDOW_NIB_NAME_OFF) != 0 {
        shim_nswindowcontroller_load_window(obj, 0);
    }
    read_u64(obj, WINDOW_OFF)
}

/// `-[NSWindowController setWindow:]`
pub extern "C" fn shim_nswindowcontroller_set_window(obj: u64, _cmd: u64, window: u64) {
    if obj == 0 {
        return;
    }
    set_window_and_loaded(obj, window);
}

/// `-[NSWindowController showWindow:]` — makes the window key and orders front.
pub extern "C" fn shim_nswindowcontroller_show_window(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 {
        return;
    }
    let window = shim_nswindowcontroller_window(obj, 0);
    if window != 0 {
        crate::nswindow::shim_nswindow_make_key_and_order_front(window, 0, 0);
    }
}

/// `-[NSWindowController close]` — closes the managed window.
pub extern "C" fn shim_nswindowcontroller_close(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let window = read_u64(obj, WINDOW_OFF);
    if window != 0 {
        crate::nswindow::shim_nswindow_close(window, 0);
    }
}

/// `-[NSWindowController loadWindow]`
pub extern "C" fn shim_nswindowcontroller_load_window(obj: u64, _cmd: u64) {
    if obj == 0 || read_u64(obj, WINDOW_OFF) != 0 {
        return;
    }

    shim_nswindowcontroller_window_will_load(obj, 0);

    let nib_name = read_u64(obj, WINDOW_NIB_NAME_OFF);
    if nib_name != 0 {
        let owner = match read_u64(obj, OWNER_OFF) {
            0 => obj,
            owner => owner,
        };
        let top_level = load_nib_named(owner, nib_name);
        if read_u64(obj, WINDOW_OFF) == 0 {
            let top_window =
                first_top_level_object_kind_of(top_level, crate::nsobj::nswindow_class_addr());
            if top_window != 0 {
                set_window_and_loaded(obj, top_window);
            }
        }
    }

    if read_u64(obj, WINDOW_OFF) != 0 {
        set_flag(obj, FLAG_WINDOW_LOADED, true);
        shim_nswindowcontroller_window_did_load(obj, 0);
    }
}

/// `-[NSWindowController isWindowLoaded]`
pub extern "C" fn shim_nswindowcontroller_is_window_loaded(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & FLAG_WINDOW_LOADED != 0 && read_u64(obj, WINDOW_OFF) != 0)
}

/// `-[NSWindowController windowNibName]`
pub extern "C" fn shim_nswindowcontroller_window_nib_name(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, WINDOW_NIB_NAME_OFF)
}

/// `-[NSWindowController windowNibPath]` — unsupported path-based nib loading.
pub extern "C" fn shim_nswindowcontroller_window_nib_path(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// Internal owner accessor used by some nib-backed controller code.
pub extern "C" fn shim_nswindowcontroller_owner(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, OWNER_OFF)
}

/// `-[NSWindowController shouldCascadeWindows]`
pub extern "C" fn shim_nswindowcontroller_should_cascade_windows(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 1; // default YES
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & FLAG_SHOULD_CASCADE != 0)
}

/// `-[NSWindowController setShouldCascadeWindows:]`
pub extern "C" fn shim_nswindowcontroller_set_should_cascade_windows(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        flags |= FLAG_SHOULD_CASCADE;
    } else {
        flags &= !FLAG_SHOULD_CASCADE;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSWindowController windowWillLoad]` — no-op stub.
pub extern "C" fn shim_nswindowcontroller_window_will_load(_obj: u64, _cmd: u64) {}

/// `-[NSWindowController windowDidLoad]` — no-op stub (subclass override point).
pub extern "C" fn shim_nswindowcontroller_window_did_load(_obj: u64, _cmd: u64) {}

/// `-[NSWindowController document]`
pub extern "C" fn shim_nswindowcontroller_document(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DOCUMENT_OFF)
}

/// `-[NSWindowController setDocument:]`
pub extern "C" fn shim_nswindowcontroller_set_document(obj: u64, _cmd: u64, document: u64) {
    write_u64(obj, DOCUMENT_OFF, document);
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

    fn encode_bool(coder: u64, key: &str, value: bool) {
        crate::nskeyedarchiver::shim_nskeyedarchiver_encode_bool_for_key(
            coder,
            0,
            u8::from(value),
            make_string(key),
        );
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nswindowcontroller_init_with_window(0, 0, 0x1234), 0);
        assert_eq!(shim_nswindowcontroller_window(0, 0), 0);
        shim_nswindowcontroller_set_window(0, 0, 0x1234);
        shim_nswindowcontroller_show_window(0, 0, 0);
        shim_nswindowcontroller_close(0, 0);
        assert_eq!(shim_nswindowcontroller_window_nib_name(0, 0), 0);
        assert_eq!(shim_nswindowcontroller_window_nib_path(0, 0), 0);
        assert_eq!(shim_nswindowcontroller_owner(0, 0), 0);
        shim_nswindowcontroller_load_window(0, 0);
        assert_eq!(shim_nswindowcontroller_is_window_loaded(0, 0), 0);
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(0, 0), 1);
        shim_nswindowcontroller_set_should_cascade_windows(0, 0, 0);
        shim_nswindowcontroller_window_will_load(0, 0);
        shim_nswindowcontroller_window_did_load(0, 0);
        assert_eq!(shim_nswindowcontroller_document(0, 0), 0);
        shim_nswindowcontroller_set_document(0, 0, 0x1234);
        assert_eq!(
            shim_nswindowcontroller_init_with_window_nib_name(0, 0, 0),
            0
        );
        assert_eq!(
            shim_nswindowcontroller_init_with_window_nib_name_owner(0, 0, 0, 0),
            0
        );
        assert_eq!(shim_nswindowcontroller_init_with_coder(0, 0, 0), 0);
    }

    #[test]
    fn init_with_window() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        let fake_win = 0x8000_u64;
        let ret = shim_nswindowcontroller_init_with_window(wc, 0, fake_win);
        assert_eq!(ret, wc, "init returns self");
        assert_eq!(shim_nswindowcontroller_window(wc, 0), fake_win);
        assert_eq!(shim_nswindowcontroller_is_window_loaded(wc, 0), 1);
        // Default shouldCascadeWindows = YES
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(wc, 0), 1);
        free_obj(wc);
    }

    #[test]
    fn window_round_trip() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        shim_nswindowcontroller_set_window(wc, 0, 0xAAAA);
        assert_eq!(shim_nswindowcontroller_window(wc, 0), 0xAAAA);
        assert_eq!(shim_nswindowcontroller_is_window_loaded(wc, 0), 1);
        shim_nswindowcontroller_set_window(wc, 0, 0);
        assert_eq!(shim_nswindowcontroller_is_window_loaded(wc, 0), 0);
        free_obj(wc);
    }

    #[test]
    fn should_cascade_toggle() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        shim_nswindowcontroller_init_with_window(wc, 0, 0);
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(wc, 0), 1);
        shim_nswindowcontroller_set_should_cascade_windows(wc, 0, 0);
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(wc, 0), 0);
        shim_nswindowcontroller_set_should_cascade_windows(wc, 0, 1);
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(wc, 0), 1);
        free_obj(wc);
    }

    #[test]
    fn nib_name_and_owner_round_trip() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        let name = make_string("MainWindow");
        let owner = 0xCAFE_u64;
        let ret = shim_nswindowcontroller_init_with_window_nib_name_owner(wc, 0, name, owner);
        assert_eq!(ret, wc);
        assert_eq!(shim_nswindowcontroller_window_nib_name(wc, 0), name);
        assert_eq!(shim_nswindowcontroller_owner(wc, 0), owner);
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(wc, 0), 1);
        free_obj(wc);
    }

    #[test]
    fn lifecycle_stub_is_noop() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        shim_nswindowcontroller_window_will_load(wc, 0);
        shim_nswindowcontroller_window_did_load(wc, 0);
        free_obj(wc);
    }

    #[test]
    fn document_round_trip() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        assert_eq!(shim_nswindowcontroller_document(wc, 0), 0);
        shim_nswindowcontroller_set_document(wc, 0, 0x9999);
        assert_eq!(shim_nswindowcontroller_document(wc, 0), 0x9999);
        free_obj(wc);
    }

    #[test]
    fn init_with_coder_decodes_archived_window_metadata_and_cascade_flag() {
        let wc = alloc_obj(
            WINDOWCONTROLLER_SIZE,
            crate::nsobj::nswindowcontroller_class_addr(),
        );
        let window = alloc_obj_fake(128);
        let nib_name = make_string("ArchivedWindow");
        let owner = alloc_obj_fake(16);
        let document = alloc_obj_fake(16);
        let coder = make_coder();

        encode_object(coder, "NSWindow", window);
        encode_object(coder, "NSWindowNibName", nib_name);
        encode_object(coder, "NSOwner", owner);
        encode_object(coder, "NSDocument", document);
        encode_bool(coder, "NSShouldCascadeWindows", false);

        assert_eq!(shim_nswindowcontroller_init_with_coder(wc, 0, coder), wc);
        assert_eq!(shim_nswindowcontroller_window(wc, 0), window);
        assert_eq!(shim_nswindowcontroller_is_window_loaded(wc, 0), 1);
        assert_eq!(shim_nswindowcontroller_window_nib_name(wc, 0), nib_name);
        assert_eq!(shim_nswindowcontroller_owner(wc, 0), owner);
        assert_eq!(shim_nswindowcontroller_document(wc, 0), document);
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(wc, 0), 0);

        free_obj(window);
        free_obj(owner);
        free_obj(document);
        free_obj(wc);
    }

    #[test]
    fn missing_window_nib_does_not_mark_loaded() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        let name = make_string("DefinitelyMissingWindowNibForUnitTest");
        shim_nswindowcontroller_init_with_window_nib_name(wc, 0, name);
        shim_nswindowcontroller_load_window(wc, 0);
        assert_eq!(shim_nswindowcontroller_window(wc, 0), 0);
        assert_eq!(shim_nswindowcontroller_is_window_loaded(wc, 0), 0);
        free_obj(wc);
    }
}
