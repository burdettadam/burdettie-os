//! NSEvent shim: event stub for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa             (u64)
//!   offset  8: event_type      (u64)
//!   offset 16: location_x      (f64)
//!   offset 24: location_y      (f64)
//!   offset 32: modifier_flags  (u64)
//!   offset 40: key_code        (u64)
//!   offset 48: characters_ptr  (u64 — NSString*)
//!
//! Total: 56 bytes.

const EVENT_TYPE_OFF: usize = 8;
const LOCATION_X_OFF: usize = 16;
const MODIFIER_FLAGS_OFF: usize = 32;
const KEY_CODE_OFF: usize = 40;
const CHARACTERS_OFF: usize = 48;

/// `-[NSEvent type]`
pub extern "C" fn shim_nsevent_type(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + EVENT_TYPE_OFF) as *const u64) }
}

/// `-[NSEvent locationInWindow]` — returns x coordinate as f64 stub.
pub extern "C" fn shim_nsevent_location_in_window(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + LOCATION_X_OFF) as *const f64) }
}

/// `-[NSEvent modifierFlags]`
pub extern "C" fn shim_nsevent_modifier_flags(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + MODIFIER_FLAGS_OFF) as *const u64) }
}

/// `-[NSEvent keyCode]`
pub extern "C" fn shim_nsevent_key_code(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + KEY_CODE_OFF) as *const u64) }
}

/// `-[NSEvent characters]`
pub extern "C" fn shim_nsevent_characters(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + CHARACTERS_OFF) as *const u64) }
}

/// `-[NSEvent charactersIgnoringModifiers]` — same as characters for stub.
pub extern "C" fn shim_nsevent_characters_ignoring_modifiers(obj: u64, _cmd: u64) -> u64 {
    shim_nsevent_characters(obj, _cmd)
}

/// `-[NSEvent window]` — stub, returns 0.
pub extern "C" fn shim_nsevent_window(_obj: u64, _cmd: u64) -> u64 {
    0
}
