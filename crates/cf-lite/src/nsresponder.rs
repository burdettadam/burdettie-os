//! NSResponder shim: base class for the responder chain.
//!
//! Object layout:
//!
//!   offset  0: isa              (u64)
//!   offset  8: next_responder   (u64 — pointer to next responder, or 0)
//!
//! Total: 16 bytes.

/// `-[NSResponder nextResponder]`
pub extern "C" fn shim_nsresponder_next_responder(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + 8) as *const u64) }
}

/// `-[NSResponder setNextResponder:]`
pub extern "C" fn shim_nsresponder_set_next_responder(obj: u64, _cmd: u64, next: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + 8) as *mut u64) = next; }
}

/// `-[NSResponder keyDown:]` — stub: forward to next responder.
pub extern "C" fn shim_nsresponder_key_down(_obj: u64, _cmd: u64, _event: u64) {
    log::debug!("NSResponder keyDown: (stub)");
}

/// `-[NSResponder keyUp:]` — stub.
pub extern "C" fn shim_nsresponder_key_up(_obj: u64, _cmd: u64, _event: u64) {
    log::debug!("NSResponder keyUp: (stub)");
}

/// `-[NSResponder mouseDown:]` — stub.
pub extern "C" fn shim_nsresponder_mouse_down(_obj: u64, _cmd: u64, _event: u64) {
    log::debug!("NSResponder mouseDown: (stub)");
}

/// `-[NSResponder mouseUp:]` — stub.
pub extern "C" fn shim_nsresponder_mouse_up(_obj: u64, _cmd: u64, _event: u64) {
    log::debug!("NSResponder mouseUp: (stub)");
}

/// `-[NSResponder acceptsFirstResponder]` — returns NO by default.
pub extern "C" fn shim_nsresponder_accepts_first_responder(_obj: u64, _cmd: u64) -> u8 {
    0
}
