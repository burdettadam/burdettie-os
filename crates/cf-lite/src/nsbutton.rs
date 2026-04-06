//! NSButton shim: push-button / checkbox control for AppKit.
//!
//! Object layout (inherits NSView at offsets 0–79):
//!
//!   offset  0: isa                  (u64)     — from NSView
//!   offset  8: superview            (u64)     — from NSView
//!   offset 16: window               (u64)     — from NSView
//!   offset 24: frame_x              (f64)     — from NSView
//!   offset 32: frame_y              (f64)     — from NSView
//!   offset 40: frame_w              (f64)     — from NSView
//!   offset 48: frame_h              (f64)     — from NSView
//!   offset 56: subviews_ptr         (u64)     — from NSView
//!   offset 64: subview_count        (u64)     — from NSView
//!   offset 72: flags (view)         (u64)     — from NSView
//!   offset 80: title_ptr            (u64 — NSString*)
//!   offset 88: alternate_title_ptr  (u64 — NSString*)
//!   offset 96: image_ptr            (u64 — NSImage*)
//!   offset 104: target_ptr          (u64)
//!   offset 112: action_sel          (u64 — SEL)
//!   offset 120: btn_flags           (u64 — bit 0: is_enabled, bit 1: is_bordered,
//!                                           bit 2: allows_mixed_state, bit 3: state_on)
//!
//! Total: 128 bytes.

const TITLE_OFF: usize = 80;
const ALT_TITLE_OFF: usize = 88;
const IMAGE_OFF: usize = 96;
const TARGET_OFF: usize = 104;
const ACTION_OFF: usize = 112;
const FLAGS_OFF: usize = 120;
pub(crate) const BUTTON_SIZE: usize = 128;

const FLAG_IS_ENABLED: u64 = 1 << 0;
const FLAG_IS_BORDERED: u64 = 1 << 1;
const FLAG_ALLOWS_MIXED_STATE: u64 = 1 << 2;
const FLAG_STATE_ON: u64 = 1 << 3;

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

fn read_flags(obj: u64) -> u64 {
    read_u64(obj, FLAGS_OFF)
}

fn write_flags(obj: u64, flags: u64) {
    write_u64(obj, FLAGS_OFF, flags);
}

fn set_flag(obj: u64, flag: u64, val: bool) {
    if obj == 0 {
        return;
    }
    let mut flags = read_flags(obj);
    if val {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_flags(obj, flags);
}

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_flags(obj) & flag != 0)
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSButton title]`
pub extern "C" fn shim_nsbutton_title(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ptr = read_u64(obj, TITLE_OFF);
    if ptr != 0 {
        return ptr;
    }
    // Return empty string if none set
    crate::cfstring::shim_cf_string_create_with_cstring(0, b"\0".as_ptr().cast(), 0)
}

/// `-[NSButton setTitle:]`
pub extern "C" fn shim_nsbutton_set_title(obj: u64, _cmd: u64, title: u64) {
    write_u64(obj, TITLE_OFF, title);
}

/// `-[NSButton alternateTitle]`
pub extern "C" fn shim_nsbutton_alternate_title(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ALT_TITLE_OFF)
}

/// `-[NSButton setAlternateTitle:]`
pub extern "C" fn shim_nsbutton_set_alternate_title(obj: u64, _cmd: u64, title: u64) {
    write_u64(obj, ALT_TITLE_OFF, title);
}

/// `-[NSButton image]`
pub extern "C" fn shim_nsbutton_image(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, IMAGE_OFF)
}

/// `-[NSButton setImage:]`
pub extern "C" fn shim_nsbutton_set_image(obj: u64, _cmd: u64, img: u64) {
    write_u64(obj, IMAGE_OFF, img);
}

/// `-[NSButton target]`
pub extern "C" fn shim_nsbutton_target(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TARGET_OFF)
}

/// `-[NSButton setTarget:]`
pub extern "C" fn shim_nsbutton_set_target(obj: u64, _cmd: u64, tgt: u64) {
    write_u64(obj, TARGET_OFF, tgt);
}

/// `-[NSButton action]`
pub extern "C" fn shim_nsbutton_action(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ACTION_OFF)
}

/// `-[NSButton setAction:]`
pub extern "C" fn shim_nsbutton_set_action(obj: u64, _cmd: u64, sel: u64) {
    write_u64(obj, ACTION_OFF, sel);
}

/// `-[NSButton state]`
pub extern "C" fn shim_nsbutton_state(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    if read_flags(obj) & FLAG_STATE_ON != 0 {
        1
    } else {
        0
    }
}

/// `-[NSButton setState:]`
pub extern "C" fn shim_nsbutton_set_state(obj: u64, _cmd: u64, state: i64) {
    set_flag(obj, FLAG_STATE_ON, state != 0);
}

/// `-[NSButton isEnabled]`
pub extern "C" fn shim_nsbutton_is_enabled(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_ENABLED)
}

/// `-[NSButton setEnabled:]`
pub extern "C" fn shim_nsbutton_set_enabled(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_ENABLED, val != 0);
}

/// `-[NSButton isBordered]`
pub extern "C" fn shim_nsbutton_is_bordered(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_BORDERED)
}

/// `-[NSButton setBordered:]`
pub extern "C" fn shim_nsbutton_set_bordered(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_BORDERED, val != 0);
}

/// `-[NSButton allowsMixedState]`
pub extern "C" fn shim_nsbutton_allows_mixed_state(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_ALLOWS_MIXED_STATE)
}

/// `-[NSButton setAllowsMixedState:]`
pub extern "C" fn shim_nsbutton_set_allows_mixed_state(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_ALLOWS_MIXED_STATE, val != 0);
}

/// `-[NSButton setButtonType:]` — no-op stub.
pub extern "C" fn shim_nsbutton_set_button_type(_obj: u64, _cmd: u64, _type: u64) {}

/// `-[NSButton setBezelStyle:]` — no-op stub.
pub extern "C" fn shim_nsbutton_set_bezel_style(_obj: u64, _cmd: u64, _style: u64) {}

/// `-[NSButton highlight:]` — no-op stub.
pub extern "C" fn shim_nsbutton_highlight(_obj: u64, _cmd: u64, _flag: u8) {}

/// `-[NSButton performClick:]` — simulates a click: toggles state and fires action.
pub extern "C" fn shim_nsbutton_perform_click(obj: u64, _cmd: u64, _sender: u64) {
    if obj == 0 {
        return;
    }
    // Toggle state for checkboxes
    let flags = read_flags(obj);
    if flags & FLAG_STATE_ON != 0 {
        write_flags(obj, flags & !FLAG_STATE_ON);
    } else {
        write_flags(obj, flags | FLAG_STATE_ON);
    }
    fire_action(obj);
}

/// `-[NSButton mouseDown:]` — handle click: fire action on mouse down.
///
/// Real AppKit tracks the mouse until mouseUp and only fires if still inside
/// the button. We simplify: fire immediately on mouseDown (single-threaded
/// model, no run loop for mouse tracking).
pub extern "C" fn shim_nsbutton_mouse_down(obj: u64, _cmd: u64, _event: u64) {
    if obj == 0 {
        return;
    }
    if read_flags(obj) & FLAG_IS_ENABLED == 0 {
        return; // disabled buttons don't respond
    }
    // Toggle state for checkboxes
    let flags = read_flags(obj);
    if flags & FLAG_STATE_ON != 0 {
        write_flags(obj, flags & !FLAG_STATE_ON);
    } else {
        write_flags(obj, flags | FLAG_STATE_ON);
    }
    fire_action(obj);
}

/// `-[NSButton acceptsFirstResponder]` — buttons accept first responder.
pub extern "C" fn shim_nsbutton_accepts_first_responder(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    u8::from(read_flags(obj) & FLAG_IS_ENABLED != 0)
}

/// Fire the button's target/action pair via NSApp sendAction:to:from:.
fn fire_action(obj: u64) {
    let action = read_u64(obj, ACTION_OFF);
    let target = read_u64(obj, TARGET_OFF);
    if action == 0 {
        return;
    }
    // Use NSApp sendAction:to:from: for proper target resolution
    let nsapp = crate::nsapplication::nsapp_global_addr();
    crate::nsapplication::shim_nsapplication_send_action_to_from(nsapp, 0, action, target, obj);
}

/// `-[NSButton sizeToFit]` — no-op stub.
pub extern "C" fn shim_nsbutton_size_to_fit(_obj: u64, _cmd: u64) {}

// ── Class method shims ─────────────────────────────────────────

/// `+[NSButton buttonWithTitle:target:action:]`
pub extern "C" fn shim_nsbutton_button_with_title(
    _cls: u64,
    _cmd: u64,
    title: u64,
    target: u64,
    action: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, BUTTON_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsbutton_class_addr();
    }
    write_u64(obj, TITLE_OFF, title);
    write_u64(obj, TARGET_OFF, target);
    write_u64(obj, ACTION_OFF, action);
    // Enabled + bordered by default
    write_flags(obj, FLAG_IS_ENABLED | FLAG_IS_BORDERED);
    obj
}

/// `+[NSButton checkboxWithTitle:target:action:]`
pub extern "C" fn shim_nsbutton_checkbox_with_title(
    _cls: u64,
    _cmd: u64,
    title: u64,
    target: u64,
    action: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, BUTTON_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsbutton_class_addr();
    }
    write_u64(obj, TITLE_OFF, title);
    write_u64(obj, TARGET_OFF, target);
    write_u64(obj, ACTION_OFF, action);
    // Enabled but NOT bordered for checkbox
    write_flags(obj, FLAG_IS_ENABLED);
    obj
}

// ── Drawing ────────────────────────────────────────────────────

/// `-[NSButton drawRect:]` — renders the button using CGContext.
///
/// Draws a bordered rounded rectangle with the button title centered inside.
pub extern "C" fn shim_nsbutton_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 { return; }

    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 { return; }

    let flags = read_flags(obj);

    // Draw button background.
    let bg_color = if flags & FLAG_IS_BORDERED != 0 {
        // Light gray rounded button background
        graphics_core::Color::new(0.92, 0.92, 0.92, 1.0)
    } else {
        // Transparent for borderless/checkbox
        graphics_core::Color::new(0.0, 0.0, 0.0, 0.0)
    };

    cg_shims::bridge::with_context(cg_ctx, |c| {
        if bg_color.a > 0.0 {
            c.fill_color = bg_color;
            c.fill_rect(x, y, w, h);
        }
        // Draw border
        if flags & FLAG_IS_BORDERED != 0 {
            c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
            c.stroke_rect(x, y, w, h);
        }
    });

    // Draw title text centered.
    let title_ptr = read_u64(obj, TITLE_OFF);
    if title_ptr != 0 {
        let utf8 = crate::cfstring::shim_nsstring_utf8string(title_ptr, 0);
        if !utf8.is_null() {
            let text = unsafe { std::ffi::CStr::from_ptr(utf8) }
                .to_str()
                .unwrap_or("");
            if !text.is_empty() {
                let text_color = if flags & FLAG_IS_ENABLED != 0 {
                    graphics_core::Color::BLACK
                } else {
                    graphics_core::Color::new(0.5, 0.5, 0.5, 1.0) // gray for disabled
                };
                cg_shims::bridge::with_context(cg_ctx, |c| {
                    c.fill_color = text_color;
                    // Center text vertically, left-pad slightly.
                    let text_y = y + (h * 0.65);
                    c.draw_text(text, x + 8.0, text_y);
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsbutton_title(0, 0), 0);
        shim_nsbutton_set_title(0, 0, 0x1234);
        assert_eq!(shim_nsbutton_alternate_title(0, 0), 0);
        assert_eq!(shim_nsbutton_image(0, 0), 0);
        assert_eq!(shim_nsbutton_target(0, 0), 0);
        assert_eq!(shim_nsbutton_action(0, 0), 0);
        assert_eq!(shim_nsbutton_state(0, 0), 0);
        shim_nsbutton_set_state(0, 0, 1);
        assert_eq!(shim_nsbutton_is_enabled(0, 0), 0);
        assert_eq!(shim_nsbutton_is_bordered(0, 0), 0);
        assert_eq!(shim_nsbutton_allows_mixed_state(0, 0), 0);
    }

    #[test]
    fn title_round_trip() {
        let btn = alloc_obj(BUTTON_SIZE, 0xDEAD);
        let title = make_string("Click Me");
        shim_nsbutton_set_title(btn, 0, title);
        assert_eq!(shim_nsbutton_title(btn, 0), title);
        assert_eq!(read_string(title), "Click Me");
        free_obj(btn);
    }

    #[test]
    fn title_returns_empty_when_unset() {
        let btn = alloc_obj(BUTTON_SIZE, 0xDEAD);
        let t = shim_nsbutton_title(btn, 0);
        assert_ne!(t, 0, "should return an empty string object, not nil");
        assert_eq!(read_string(t), "");
        free_obj(btn);
    }

    #[test]
    fn state_toggle() {
        let btn = alloc_obj(BUTTON_SIZE, 0xDEAD);
        assert_eq!(shim_nsbutton_state(btn, 0), 0);
        shim_nsbutton_set_state(btn, 0, 1);
        assert_eq!(shim_nsbutton_state(btn, 0), 1);
        shim_nsbutton_set_state(btn, 0, 0);
        assert_eq!(shim_nsbutton_state(btn, 0), 0);
        free_obj(btn);
    }

    #[test]
    fn enabled_bordered_flags() {
        let btn = alloc_obj(BUTTON_SIZE, 0xDEAD);
        // Initially all flags are 0
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 0);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 0);
        assert_eq!(shim_nsbutton_allows_mixed_state(btn, 0), 0);

        shim_nsbutton_set_enabled(btn, 0, 1);
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 1);
        shim_nsbutton_set_bordered(btn, 0, 1);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 1);
        shim_nsbutton_set_allows_mixed_state(btn, 0, 1);
        assert_eq!(shim_nsbutton_allows_mixed_state(btn, 0), 1);

        // Toggling one flag should not affect others
        shim_nsbutton_set_enabled(btn, 0, 0);
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 0);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 1);
        assert_eq!(shim_nsbutton_allows_mixed_state(btn, 0), 1);
        free_obj(btn);
    }

    #[test]
    fn button_with_title_factory() {
        let title = make_string("OK");
        let target = 0xAAAA_u64;
        let action = 0xBBBB_u64;
        let btn = shim_nsbutton_button_with_title(0, 0, title, target, action);
        assert_ne!(btn, 0);
        assert_eq!(shim_nsbutton_title(btn, 0), title);
        assert_eq!(shim_nsbutton_target(btn, 0), target);
        assert_eq!(shim_nsbutton_action(btn, 0), action);
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 1);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 1);
        free_obj(btn);
    }

    #[test]
    fn checkbox_with_title_factory() {
        let title = make_string("Accept Terms");
        let btn = shim_nsbutton_checkbox_with_title(0, 0, title, 0, 0);
        assert_ne!(btn, 0);
        assert_eq!(shim_nsbutton_title(btn, 0), title);
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 1);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 0);
        free_obj(btn);
    }
}
