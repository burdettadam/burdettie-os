//! NSWindowController shim: window management controller.
//!
//! Object layout:
//!
//!   offset  0: isa          (u64)
//!   offset  8: window_ptr   (u64 — NSWindow*, or 0)
//!   offset 16: flags        (u64 — bit 0: should_cascade_windows)
//!
//! Total: 24 bytes.

const WINDOW_OFF: usize = 8;
const FLAGS_OFF: usize = 16;
pub(crate) const WINDOWCONTROLLER_SIZE: usize = 24;

const FLAG_SHOULD_CASCADE: u64 = 1 << 0;

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

/// `-[NSWindowController initWithWindow:]`
pub extern "C" fn shim_nswindowcontroller_init_with_window(obj: u64, _cmd: u64, window: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    write_u64(obj, WINDOW_OFF, window);
    // Default: shouldCascadeWindows = YES
    write_u64(obj, FLAGS_OFF, FLAG_SHOULD_CASCADE);
    obj
}

/// `-[NSWindowController window]`
pub extern "C" fn shim_nswindowcontroller_window(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, WINDOW_OFF)
}

/// `-[NSWindowController setWindow:]`
pub extern "C" fn shim_nswindowcontroller_set_window(obj: u64, _cmd: u64, window: u64) {
    write_u64(obj, WINDOW_OFF, window);
}

/// `-[NSWindowController showWindow:]` — makes the window key and orders front.
pub extern "C" fn shim_nswindowcontroller_show_window(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 {
        return;
    }
    let window = read_u64(obj, WINDOW_OFF);
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

/// `-[NSWindowController windowNibName]` — returns nil (no nib).
pub extern "C" fn shim_nswindowcontroller_window_nib_name(_obj: u64, _cmd: u64) -> u64 {
    0
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

/// `-[NSWindowController windowDidLoad]` — no-op stub (subclass override point).
pub extern "C" fn shim_nswindowcontroller_window_did_load(_obj: u64, _cmd: u64) {}

/// `-[NSWindowController document]` — returns nil (no document architecture).
pub extern "C" fn shim_nswindowcontroller_document(_obj: u64, _cmd: u64) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nswindowcontroller_init_with_window(0, 0, 0x1234), 0);
        assert_eq!(shim_nswindowcontroller_window(0, 0), 0);
        shim_nswindowcontroller_set_window(0, 0, 0x1234);
        shim_nswindowcontroller_show_window(0, 0, 0);
        shim_nswindowcontroller_close(0, 0);
        assert_eq!(shim_nswindowcontroller_window_nib_name(0, 0), 0);
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(0, 0), 1);
        shim_nswindowcontroller_set_should_cascade_windows(0, 0, 0);
        shim_nswindowcontroller_window_did_load(0, 0);
        assert_eq!(shim_nswindowcontroller_document(0, 0), 0);
    }

    #[test]
    fn init_with_window() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        let fake_win = 0x8000_u64;
        let ret = shim_nswindowcontroller_init_with_window(wc, 0, fake_win);
        assert_eq!(ret, wc, "init returns self");
        assert_eq!(shim_nswindowcontroller_window(wc, 0), fake_win);
        // Default shouldCascadeWindows = YES
        assert_eq!(shim_nswindowcontroller_should_cascade_windows(wc, 0), 1);
        free_obj(wc);
    }

    #[test]
    fn window_round_trip() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        shim_nswindowcontroller_set_window(wc, 0, 0xAAAA);
        assert_eq!(shim_nswindowcontroller_window(wc, 0), 0xAAAA);
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
    fn document_returns_nil() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        assert_eq!(shim_nswindowcontroller_document(wc, 0), 0);
        free_obj(wc);
    }

    #[test]
    fn lifecycle_stub_is_noop() {
        let wc = alloc_obj(WINDOWCONTROLLER_SIZE, 0xDEAD);
        shim_nswindowcontroller_window_did_load(wc, 0);
        free_obj(wc);
    }
}
