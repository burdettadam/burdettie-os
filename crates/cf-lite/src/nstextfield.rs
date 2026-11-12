//! NSTextField shim: text label and input control.
//!
//! Object layout (inherits NSView at offsets 0–79):
//!
//!   offset  0: isa               (u64)     — from NSView
//!   offset  8: superview         (u64)     — from NSView
//!   offset 16: window            (u64)     — from NSView
//!   offset 24: frame_x           (f64)     — from NSView
//!   offset 32: frame_y           (f64)     — from NSView
//!   offset 40: frame_w           (f64)     — from NSView
//!   offset 48: frame_h           (f64)     — from NSView
//!   offset 56: subviews_ptr      (u64)     — from NSView
//!   offset 64: subview_count     (u64)     — from NSView
//!   offset 72: flags (view)      (u64)     — from NSView
//!   offset 80: string_value_ptr  (u64 — NSString*, or 0)
//!   offset 88: target_ptr        (u64 — id, or 0)
//!   offset 96: action_sel        (u64 — SEL addr, or 0)
//!   offset 104: placeholder_ptr  (u64 — NSString*, or 0)
//!   offset 112: tf_flags         (u64 — bit 0: is_enabled, bit 1: is_editable,
//!                                        bit 2: is_bezeled, bit 3: draws_background,
//!                                        bit 4: is_selectable)
//!
//! Total: 120 bytes.

const STRING_VALUE_OFF: usize = 80;
const TARGET_OFF: usize = 88;
const ACTION_OFF: usize = 96;
const PLACEHOLDER_OFF: usize = 104;
const FLAGS_OFF: usize = 112;
pub(crate) const TEXTFIELD_SIZE: usize = 120;

const FLAG_IS_ENABLED: u64 = 1 << 0;
const FLAG_IS_EDITABLE: u64 = 1 << 1;
const FLAG_IS_BEZELED: u64 = 1 << 2;
const FLAG_DRAWS_BACKGROUND: u64 = 1 << 3;
const FLAG_IS_SELECTABLE: u64 = 1 << 4;

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

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & flag != 0)
}

fn set_flag(obj: u64, flag: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// Set up a text field with label defaults (non-editable, non-bezeled, no bg).
fn init_as_label(obj: u64, string_value: u64) {
    write_u64(obj, STRING_VALUE_OFF, string_value);
    // Label: enabled + selectable, but NOT editable, NOT bezeled, NO bg
    write_u64(obj, FLAGS_OFF, FLAG_IS_ENABLED | FLAG_IS_SELECTABLE);
}

/// Set up a text field with editable defaults.
fn init_as_editable(obj: u64, string_value: u64) {
    write_u64(obj, STRING_VALUE_OFF, string_value);
    // Editable: enabled + editable + bezeled + draws background + selectable
    write_u64(
        obj,
        FLAGS_OFF,
        FLAG_IS_ENABLED
            | FLAG_IS_EDITABLE
            | FLAG_IS_BEZELED
            | FLAG_DRAWS_BACKGROUND
            | FLAG_IS_SELECTABLE,
    );
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSTextField stringValue]`
pub extern "C" fn shim_nstextfield_string_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ptr = read_u64(obj, STRING_VALUE_OFF);
    if ptr != 0 {
        return ptr;
    }
    crate::cfstring::shim_cf_string_create_with_cstring(0, b"\0".as_ptr().cast(), 0)
}

/// `-[NSTextField setStringValue:]`
pub extern "C" fn shim_nstextfield_set_string_value(obj: u64, _cmd: u64, value: u64) {
    write_u64(obj, STRING_VALUE_OFF, value);
}

/// `-[NSTextField isEditable]`
pub extern "C" fn shim_nstextfield_is_editable(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_EDITABLE)
}

/// `-[NSTextField setEditable:]`
pub extern "C" fn shim_nstextfield_set_editable(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_EDITABLE, val);
}

/// `-[NSTextField isBezeled]`
pub extern "C" fn shim_nstextfield_is_bezeled(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_BEZELED)
}

/// `-[NSTextField setBezeled:]`
pub extern "C" fn shim_nstextfield_set_bezeled(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_BEZELED, val);
}

/// `-[NSTextField drawsBackground]`
pub extern "C" fn shim_nstextfield_draws_background(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_DRAWS_BACKGROUND)
}

/// `-[NSTextField setDrawsBackground:]`
pub extern "C" fn shim_nstextfield_set_draws_background(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_DRAWS_BACKGROUND, val);
}

/// `-[NSTextField isSelectable]`
pub extern "C" fn shim_nstextfield_is_selectable(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_SELECTABLE)
}

/// `-[NSTextField setSelectable:]`
pub extern "C" fn shim_nstextfield_set_selectable(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_SELECTABLE, val);
}

/// `-[NSTextField isEnabled]`
pub extern "C" fn shim_nstextfield_is_enabled(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_ENABLED)
}

/// `-[NSTextField setEnabled:]`
pub extern "C" fn shim_nstextfield_set_enabled(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_ENABLED, val);
}

/// `-[NSTextField placeholderString]`
pub extern "C" fn shim_nstextfield_placeholder_string(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, PLACEHOLDER_OFF)
}

/// `-[NSTextField setPlaceholderString:]`
pub extern "C" fn shim_nstextfield_set_placeholder_string(obj: u64, _cmd: u64, value: u64) {
    write_u64(obj, PLACEHOLDER_OFF, value);
}

/// `-[NSTextField target]`
pub extern "C" fn shim_nstextfield_target(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TARGET_OFF)
}

/// `-[NSTextField setTarget:]`
pub extern "C" fn shim_nstextfield_set_target(obj: u64, _cmd: u64, target: u64) {
    write_u64(obj, TARGET_OFF, target);
}

/// `-[NSTextField action]`
pub extern "C" fn shim_nstextfield_action(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ACTION_OFF)
}

/// `-[NSTextField setAction:]`
pub extern "C" fn shim_nstextfield_set_action(obj: u64, _cmd: u64, sel: u64) {
    write_u64(obj, ACTION_OFF, sel);
}

// ── Class method shims ─────────────────────────────────────────

/// `+[NSTextField labelWithString:]` — creates a non-editable label.
pub extern "C" fn shim_nstextfield_label_with_string(_cls: u64, _cmd: u64, string: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, TEXTFIELD_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nstextfield_class_addr();
    }
    init_as_label(obj, string);
    obj
}

/// `+[NSTextField wrappingLabelWithString:]` — creates a wrapping label.
pub extern "C" fn shim_nstextfield_wrapping_label_with_string(
    _cls: u64,
    _cmd: u64,
    string: u64,
) -> u64 {
    // Same as labelWithString: for our purposes
    shim_nstextfield_label_with_string(_cls, _cmd, string)
}

/// `+[NSTextField textFieldWithString:]` — creates an editable text field.
pub extern "C" fn shim_nstextfield_text_field_with_string(
    _cls: u64,
    _cmd: u64,
    string: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, TEXTFIELD_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nstextfield_class_addr();
    }
    init_as_editable(obj, string);
    obj
}

// ── Drawing ────────────────────────────────────────────────────

/// `-[NSTextField drawRect:]` — renders the text field using CGContext.
///
/// Draws optional background/bezel, then the string value (or placeholder)
/// in the current fill color. Uses the CG context from the current
/// NSGraphicsContext (set by `display_view_recursive`).
pub extern "C" fn shim_nstextfield_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }

    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    // Draw background if enabled.
    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_DRAWS_BACKGROUND != 0 {
        // White background
        cg_shims::bridge::with_context(cg_ctx, |c| {
            c.fill_color = graphics_core::Color::WHITE;
            c.fill_rect(x, y, w, h);
        });
    }

    // Draw bezel border if enabled.
    if flags & FLAG_IS_BEZELED != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| {
            c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
            c.stroke_rect(x, y, w, h);
        });
    }

    // Draw cursor if this text field is the first responder and is editable.
    let is_editing = flags & FLAG_IS_EDITABLE != 0 && is_first_responder(obj);

    // Draw text content.
    let string_ptr = read_u64(obj, STRING_VALUE_OFF);
    let text_ptr = if string_ptr != 0 {
        string_ptr
    } else {
        read_u64(obj, PLACEHOLDER_OFF)
    };

    if text_ptr != 0 {
        let utf8 = crate::cfstring::shim_nsstring_utf8string(text_ptr, 0);
        if !utf8.is_null() {
            let text = unsafe { std::ffi::CStr::from_ptr(utf8) }
                .to_str()
                .unwrap_or("");
            if !text.is_empty() {
                let text_color = if string_ptr != 0 {
                    graphics_core::Color::BLACK
                } else {
                    // Placeholder text in gray
                    graphics_core::Color::new(0.6, 0.6, 0.6, 1.0)
                };
                cg_shims::bridge::with_context(cg_ctx, |c| {
                    let font_size = c.font_size.max(12.0);
                    let sw = c.surface.width();
                    let sh = c.surface.height();
                    // Vertically center: place text top at y + (h - font_size) / 2
                    let text_top = y + ((h - font_size * 1.2) / 2.0).max(0.0);
                    let advance = graphics_core::text_system::draw_shaped_text(
                        c.surface.pixels_mut(),
                        sw,
                        sh,
                        text,
                        x + 4.0,
                        text_top,
                        font_size,
                        "System",
                        text_color,
                        None, // single-line, no wrap
                    );
                    c.text_x = x + 4.0 + advance;

                    // Draw a blinking cursor after the text if editing.
                    if is_editing {
                        let cursor_x = c.text_x + 1.0;
                        c.fill_color = graphics_core::Color::BLACK;
                        c.fill_rect(cursor_x, y + 4.0, 1.0, h - 8.0);
                    }
                });
                return;
            }
        }
    }

    // Draw cursor at the beginning if field is empty but editing.
    if is_editing {
        cg_shims::bridge::with_context(cg_ctx, |c| {
            c.fill_color = graphics_core::Color::BLACK;
            c.fill_rect(x + 5.0, y + 4.0, 1.0, h - 8.0);
        });
    }
}

// ── Text editing ───────────────────────────────────────────────

/// Check if this view is the first responder of its window.
fn is_first_responder(obj: u64) -> bool {
    let window = read_u64(obj, 16); // WINDOW_OFF from nsview
    if window == 0 {
        return false;
    }
    crate::nswindow::shim_nswindow_first_responder(window, 0) == obj
}

/// `-[NSTextField acceptsFirstResponder]` — returns YES if editable.
pub extern "C" fn shim_nstextfield_accepts_first_responder(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & FLAG_IS_EDITABLE != 0)
}

/// `-[NSTextField keyDown:]` — handles keyboard input for editable text fields.
///
/// When the text field is editable and is the first responder, keyDown:
/// reads the event's characters and appends them to the string value.
/// Backspace (keyCode 51) deletes the last character.
pub extern "C" fn shim_nstextfield_key_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }

    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_IS_EDITABLE == 0 {
        // Not editable — forward up the responder chain.
        crate::nsresponder::shim_nsresponder_key_down(obj, _cmd, event);
        return;
    }

    let key_code = crate::nsevent::shim_nsevent_key_code(event, 0);

    if key_code == 51 {
        // Backspace — delete last character.
        let string_ptr = read_u64(obj, STRING_VALUE_OFF);
        if string_ptr != 0 {
            let utf8 = crate::cfstring::shim_nsstring_utf8string(string_ptr, 0);
            if !utf8.is_null() {
                let s = unsafe { std::ffi::CStr::from_ptr(utf8) }
                    .to_str()
                    .unwrap_or("");
                if !s.is_empty() {
                    // Remove the last character.
                    let mut chars: Vec<char> = s.chars().collect();
                    chars.pop();
                    let new_str: String = chars.into_iter().collect();
                    let c_str = std::ffi::CString::new(new_str).unwrap_or_default();
                    let new_nsstring = crate::cfstring::alloc_nsstring_from_cstr(c_str.as_ptr());
                    write_u64(obj, STRING_VALUE_OFF, new_nsstring);
                }
            }
        }
        // Request redraw.
        crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
        notify_text_did_change(obj);
        return;
    }

    if key_code == 36 {
        // Return key — notify delegate, then fire action.
        notify_text_did_end_editing(obj);
        let target = read_u64(obj, TARGET_OFF);
        let action = read_u64(obj, ACTION_OFF);
        if target != 0 && action != 0 {
            crate::nscontrol::shim_nscontrol_send_action(obj, 0, action, target);
        }
        return;
    }

    // Regular character input — read event characters and append.
    let chars_ptr = crate::nsevent::shim_nsevent_characters(event, 0);
    if chars_ptr == 0 {
        return;
    }

    let utf8 = crate::cfstring::shim_nsstring_utf8string(chars_ptr, 0);
    if utf8.is_null() {
        return;
    }
    let typed = unsafe { std::ffi::CStr::from_ptr(utf8) }
        .to_str()
        .unwrap_or("");
    if typed.is_empty() {
        return;
    }

    // Filter non-printable characters (control chars).
    let typed: String = typed.chars().filter(|c| !c.is_control()).collect();
    if typed.is_empty() {
        return;
    }

    // Get current string value.
    let string_ptr = read_u64(obj, STRING_VALUE_OFF);
    let current = if string_ptr != 0 {
        let utf8 = crate::cfstring::shim_nsstring_utf8string(string_ptr, 0);
        if utf8.is_null() {
            String::new()
        } else {
            unsafe { std::ffi::CStr::from_ptr(utf8) }
                .to_str()
                .unwrap_or("")
                .to_string()
        }
    } else {
        String::new()
    };

    // Append typed characters.
    let new_str = format!("{}{}", current, typed);
    let c_str = std::ffi::CString::new(new_str).unwrap_or_default();
    let new_nsstring = crate::cfstring::alloc_nsstring_from_cstr(c_str.as_ptr());
    write_u64(obj, STRING_VALUE_OFF, new_nsstring);

    // Request redraw.
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);

    // Notify delegate: controlTextDidChange:
    notify_text_did_change(obj);
}

/// `-[NSTextField mouseDown:]` — make this text field the first responder.
pub extern "C" fn shim_nstextfield_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 {
        return;
    }

    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_IS_EDITABLE == 0 && flags & FLAG_IS_SELECTABLE == 0 {
        return;
    }

    let window = read_u64(obj, 16); // WINDOW_OFF from nsview
    if window != 0 {
        crate::nswindow::shim_nswindow_make_first_responder(window, 0, obj);
    }
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
    let _ = event; // consumed
}

/// Dispatch `controlTextDidChange:` to the delegate (if any).
fn notify_text_did_change(obj: u64) {
    // NSTextField inherits NSControl, whose delegate field sits at the same
    // position as NSView's window+8, but our shim wires the delegate via
    // the `delegate` selector.  For simplicity, use the target as the
    // delegate receiver — many real apps set delegate == target.
    let delegate = read_u64(obj, TARGET_OFF);
    if delegate != 0 {
        let _ = crate::nsapplication::dispatch_selector_named_to_target(
            delegate,
            "controlTextDidChange:",
            obj,
        );
    }
}

/// Dispatch `controlTextDidEndEditing:` to the delegate (if any).
fn notify_text_did_end_editing(obj: u64) {
    let delegate = read_u64(obj, TARGET_OFF);
    if delegate != 0 {
        let _ = crate::nsapplication::dispatch_selector_named_to_target(
            delegate,
            "controlTextDidEndEditing:",
            obj,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstextfield_string_value(0, 0), 0);
        shim_nstextfield_set_string_value(0, 0, 0x1234);
        assert_eq!(shim_nstextfield_is_editable(0, 0), 0);
        shim_nstextfield_set_editable(0, 0, 1);
        assert_eq!(shim_nstextfield_is_bezeled(0, 0), 0);
        shim_nstextfield_set_bezeled(0, 0, 1);
        assert_eq!(shim_nstextfield_draws_background(0, 0), 0);
        shim_nstextfield_set_draws_background(0, 0, 1);
        assert_eq!(shim_nstextfield_is_selectable(0, 0), 0);
        shim_nstextfield_set_selectable(0, 0, 1);
        assert_eq!(shim_nstextfield_is_enabled(0, 0), 0);
        shim_nstextfield_set_enabled(0, 0, 1);
        assert_eq!(shim_nstextfield_placeholder_string(0, 0), 0);
        shim_nstextfield_set_placeholder_string(0, 0, 0x1234);
        assert_eq!(shim_nstextfield_target(0, 0), 0);
        shim_nstextfield_set_target(0, 0, 0x1234);
        assert_eq!(shim_nstextfield_action(0, 0), 0);
        shim_nstextfield_set_action(0, 0, 0x1234);
        shim_nstextfield_mouse_down(0, 0, 0);
    }

    #[test]
    fn label_factory() {
        let s = make_string("Hello");
        let tf = shim_nstextfield_label_with_string(0, 0, s);
        assert_ne!(tf, 0);
        assert_eq!(shim_nstextfield_string_value(tf, 0), s);
        // Label: enabled, selectable, NOT editable, NOT bezeled, NO bg
        assert_eq!(shim_nstextfield_is_enabled(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_selectable(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 0);
        assert_eq!(shim_nstextfield_is_bezeled(tf, 0), 0);
        assert_eq!(shim_nstextfield_draws_background(tf, 0), 0);
        free_obj(tf);
    }

    #[test]
    fn editable_factory() {
        let s = make_string("Edit me");
        let tf = shim_nstextfield_text_field_with_string(0, 0, s);
        assert_ne!(tf, 0);
        assert_eq!(shim_nstextfield_string_value(tf, 0), s);
        // Editable: all flags on
        assert_eq!(shim_nstextfield_is_enabled(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_bezeled(tf, 0), 1);
        assert_eq!(shim_nstextfield_draws_background(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_selectable(tf, 0), 1);
        free_obj(tf);
    }

    #[test]
    fn string_value_round_trip() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        let s = make_string("test");
        shim_nstextfield_set_string_value(tf, 0, s);
        assert_eq!(shim_nstextfield_string_value(tf, 0), s);
        free_obj(tf);
    }

    #[test]
    fn placeholder_round_trip() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        let p = make_string("Enter text...");
        shim_nstextfield_set_placeholder_string(tf, 0, p);
        assert_eq!(shim_nstextfield_placeholder_string(tf, 0), p);
        free_obj(tf);
    }

    #[test]
    fn editable_toggle() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 0);
        shim_nstextfield_set_editable(tf, 0, 1);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 1);
        shim_nstextfield_set_editable(tf, 0, 0);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 0);
        free_obj(tf);
    }

    #[test]
    fn bezeled_toggle() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        shim_nstextfield_set_bezeled(tf, 0, 1);
        assert_eq!(shim_nstextfield_is_bezeled(tf, 0), 1);
        shim_nstextfield_set_bezeled(tf, 0, 0);
        assert_eq!(shim_nstextfield_is_bezeled(tf, 0), 0);
        free_obj(tf);
    }

    #[test]
    fn target_action_round_trip() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        shim_nstextfield_set_target(tf, 0, 0xAABB);
        shim_nstextfield_set_action(tf, 0, 0xCCDD);
        assert_eq!(shim_nstextfield_target(tf, 0), 0xAABB);
        assert_eq!(shim_nstextfield_action(tf, 0), 0xCCDD);
        free_obj(tf);
    }

    #[test]
    fn wrapping_label_same_as_label() {
        let s = make_string("Wrap");
        let tf = shim_nstextfield_wrapping_label_with_string(0, 0, s);
        assert_ne!(tf, 0);
        assert_eq!(shim_nstextfield_string_value(tf, 0), s);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 0);
        free_obj(tf);
    }
}
