//! NSTextView — rich text editing view.
//!
//! Object layout: 112 bytes
//!   [0..80):   NSView fields
//!   offset 80: string_ptr        (u64 — backing text as NSString)
//!   offset 88: delegate_ptr      (u64)
//!   offset 96: flags             (u64 — is_editable, is_selectable, is_rich_text, etc.)
//!   offset 104: selection_start  (i32) + selection_length (i32) packed in u64

use crate::nsobj;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as *const u8).add(offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, val: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as *mut u8).add(offset) as *mut u64) = val; }
}

pub(crate) const TEXT_VIEW_SIZE: usize = 112;
const STRING_OFF: usize = 80;
const DELEGATE_OFF: usize = 88;
const FLAGS_OFF: usize = 96;
const SELECTION_OFF: usize = 104;

const FLAG_IS_EDITABLE: u64 = 1 << 0;
const FLAG_IS_SELECTABLE: u64 = 1 << 1;
const FLAG_IS_RICH_TEXT: u64 = 1 << 2;
const FLAG_USES_FONT_PANEL: u64 = 1 << 3;
const FLAG_ALLOWS_UNDO: u64 = 1 << 4;
const FLAG_IS_FIELD_EDITOR: u64 = 1 << 5;

fn pack_selection(start: i32, length: i32) -> u64 {
    let s = start as u32;
    let l = length as u32;
    ((l as u64) << 32) | (s as u64)
}

fn unpack_selection(packed: u64) -> (i32, i32) {
    let start = packed as u32 as i32;
    let length = (packed >> 32) as u32 as i32;
    (start, length)
}

/// `-[NSTextView initWithFrame:]`
pub extern "C" fn shim_nstextview_init_with_frame(
    obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64,
) -> u64 {
    let ptr = if obj == 0 {
        crate::pal_alloc().calloc(1, TEXT_VIEW_SIZE) as u64
    } else {
        obj
    };
    if ptr == 0 { return 0; }

    // Preserve ISA if already set, else set to NSTextView.
    let isa = read_u64(ptr, 0);
    if isa == 0 {
        write_u64(ptr, 0, nsobj::nstextview_class_addr());
    }

    // Set frame.
    crate::nsview::shim_nsview_set_frame(ptr, 0, x, y, w, h);

    // Default flags: editable + selectable + rich text.
    write_u64(ptr, FLAGS_OFF, FLAG_IS_EDITABLE | FLAG_IS_SELECTABLE | FLAG_IS_RICH_TEXT | FLAG_ALLOWS_UNDO);

    // Empty string.
    let empty = crate::cfstring::alloc_nsstring_from_cstr(b"\0".as_ptr() as *const _);
    write_u64(ptr, STRING_OFF, empty);

    ptr
}

/// `-[NSTextView string]`
pub extern "C" fn shim_nstextview_string(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, STRING_OFF)
}

/// `-[NSTextView setString:]`
pub extern "C" fn shim_nstextview_set_string(obj: u64, _cmd: u64, str_obj: u64) {
    write_u64(obj, STRING_OFF, str_obj);
}

/// `-[NSTextView delegate]`
pub extern "C" fn shim_nstextview_delegate(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DELEGATE_OFF)
}

/// `-[NSTextView setDelegate:]`
pub extern "C" fn shim_nstextview_set_delegate(obj: u64, _cmd: u64, delegate: u64) {
    write_u64(obj, DELEGATE_OFF, delegate);
}

/// `-[NSTextView isEditable]`
pub extern "C" fn shim_nstextview_is_editable(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_EDITABLE != 0)
}

/// `-[NSTextView setEditable:]`
pub extern "C" fn shim_nstextview_set_editable(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_IS_EDITABLE; } else { flags &= !FLAG_IS_EDITABLE; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView isSelectable]`
pub extern "C" fn shim_nstextview_is_selectable(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_SELECTABLE != 0)
}

/// `-[NSTextView setSelectable:]`
pub extern "C" fn shim_nstextview_set_selectable(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_IS_SELECTABLE; } else { flags &= !FLAG_IS_SELECTABLE; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView isRichText]`
pub extern "C" fn shim_nstextview_is_rich_text(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_RICH_TEXT != 0)
}

/// `-[NSTextView setRichText:]`
pub extern "C" fn shim_nstextview_set_rich_text(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_IS_RICH_TEXT; } else { flags &= !FLAG_IS_RICH_TEXT; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView isFieldEditor]`
pub extern "C" fn shim_nstextview_is_field_editor(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_FIELD_EDITOR != 0)
}

/// `-[NSTextView setFieldEditor:]`
pub extern "C" fn shim_nstextview_set_field_editor(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_IS_FIELD_EDITOR; } else { flags &= !FLAG_IS_FIELD_EDITOR; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView usesFontPanel]`
pub extern "C" fn shim_nstextview_uses_font_panel(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_USES_FONT_PANEL != 0)
}

/// `-[NSTextView setUsesFontPanel:]`
pub extern "C" fn shim_nstextview_set_uses_font_panel(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_USES_FONT_PANEL; } else { flags &= !FLAG_USES_FONT_PANEL; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView allowsUndo]`
pub extern "C" fn shim_nstextview_allows_undo(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_ALLOWS_UNDO != 0)
}

/// `-[NSTextView setAllowsUndo:]`
pub extern "C" fn shim_nstextview_set_allows_undo(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_ALLOWS_UNDO; } else { flags &= !FLAG_ALLOWS_UNDO; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView selectedRange]` — returns NSRange as (location, length) via x0/x1.
pub extern "C" fn shim_nstextview_selected_range(obj: u64, _cmd: u64) -> (u64, u64) {
    if obj == 0 { return (0, 0); }
    let packed = read_u64(obj, SELECTION_OFF);
    let (start, length) = unpack_selection(packed);
    (start as u64, length as u64)
}

/// `-[NSTextView setSelectedRange:]`
pub extern "C" fn shim_nstextview_set_selected_range(obj: u64, _cmd: u64, loc: u64, len: u64) {
    if obj == 0 { return; }
    write_u64(obj, SELECTION_OFF, pack_selection(loc as i32, len as i32));
}

/// `-[NSTextView acceptsFirstResponder]`
pub extern "C" fn shim_nstextview_accepts_first_responder(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_EDITABLE != 0)
}

/// `-[NSTextView insertText:]`
pub extern "C" fn shim_nstextview_insert_text(obj: u64, _cmd: u64, text: u64) {
    if obj == 0 || text == 0 { return; }

    let utf8 = crate::cfstring::shim_nsstring_utf8string(text, 0);
    if utf8.is_null() { return; }
    let typed = match unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    if typed.is_empty() { return; }

    // Get current text.
    let string_ptr = read_u64(obj, STRING_OFF);
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

    let new_str = format!("{}{}", current, typed);
    let c_str = std::ffi::CString::new(new_str).unwrap_or_default();
    let new_nsstring = crate::cfstring::alloc_nsstring_from_cstr(c_str.as_ptr());
    write_u64(obj, STRING_OFF, new_nsstring);
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSTextView keyDown:]`
pub extern "C" fn shim_nstextview_key_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 { return; }

    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_IS_EDITABLE == 0 {
        crate::nsresponder::shim_nsresponder_key_down(obj, _cmd, event);
        return;
    }

    let key_code = crate::nsevent::shim_nsevent_key_code(event, 0);

    if key_code == 51 {
        // Backspace.
        let string_ptr = read_u64(obj, STRING_OFF);
        if string_ptr != 0 {
            let utf8 = crate::cfstring::shim_nsstring_utf8string(string_ptr, 0);
            if !utf8.is_null() {
                let s = unsafe { std::ffi::CStr::from_ptr(utf8) }
                    .to_str()
                    .unwrap_or("");
                if !s.is_empty() {
                    let mut chars: Vec<char> = s.chars().collect();
                    chars.pop();
                    let new_str: String = chars.into_iter().collect();
                    let c_str = std::ffi::CString::new(new_str).unwrap_or_default();
                    let new_nsstring = crate::cfstring::alloc_nsstring_from_cstr(c_str.as_ptr());
                    write_u64(obj, STRING_OFF, new_nsstring);
                }
            }
        }
        crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
        return;
    }

    if key_code == 36 {
        // Return — insert newline for text view (unlike text field).
        let newline_str = crate::cfstring::alloc_nsstring_from_cstr(b"\n\0".as_ptr() as *const _);
        shim_nstextview_insert_text(obj, 0, newline_str);
        return;
    }

    // Regular character input.
    let chars_ptr = crate::nsevent::shim_nsevent_characters(event, 0);
    if chars_ptr != 0 {
        shim_nstextview_insert_text(obj, 0, chars_ptr);
    }
}

/// `-[NSTextView drawRect:]`
pub extern "C" fn shim_nstextview_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 { return; }

    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 { return; }

    // White background.
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.fill_color = graphics_core::Color::WHITE;
        c.fill_rect(x, y, w, h);
    });

    // Draw text content.
    let string_ptr = read_u64(obj, STRING_OFF);
    if string_ptr != 0 {
        let utf8 = crate::cfstring::shim_nsstring_utf8string(string_ptr, 0);
        if !utf8.is_null() {
            let text = unsafe { std::ffi::CStr::from_ptr(utf8) }
                .to_str()
                .unwrap_or("");
            if !text.is_empty() {
                // Simple line-by-line rendering.
                let line_height = 16.0;
                let mut text_y = y + line_height;
                cg_shims::bridge::with_context(cg_ctx, |c| {
                    c.fill_color = graphics_core::Color::BLACK;
                    for line in text.split('\n') {
                        if text_y > y + h { break; }
                        c.draw_text(line, x + 4.0, text_y);
                        text_y += line_height;
                    }
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
    fn init_sets_isa_and_defaults() {
        let tv = shim_nstextview_init_with_frame(0, 0, 0.0, 0.0, 300.0, 200.0);
        assert_ne!(tv, 0);
        assert_eq!(read_u64(tv, 0), nsobj::nstextview_class_addr());
        assert_eq!(shim_nstextview_is_editable(tv, 0), 1);
        assert_eq!(shim_nstextview_is_selectable(tv, 0), 1);
        assert_eq!(shim_nstextview_is_rich_text(tv, 0), 1);
        free_obj(tv);
    }

    #[test]
    fn string_round_trip() {
        let tv = shim_nstextview_init_with_frame(0, 0, 0.0, 0.0, 200.0, 100.0);
        let s = make_string("Hello World");
        shim_nstextview_set_string(tv, 0, s);
        assert_eq!(shim_nstextview_string(tv, 0), s);
        free_obj(tv);
    }

    #[test]
    fn editable_toggle() {
        let tv = shim_nstextview_init_with_frame(0, 0, 0.0, 0.0, 200.0, 100.0);
        shim_nstextview_set_editable(tv, 0, 0);
        assert_eq!(shim_nstextview_is_editable(tv, 0), 0);
        shim_nstextview_set_editable(tv, 0, 1);
        assert_eq!(shim_nstextview_is_editable(tv, 0), 1);
        free_obj(tv);
    }

    #[test]
    fn selection_pack_unpack() {
        let packed = pack_selection(10, 5);
        let (s, l) = unpack_selection(packed);
        assert_eq!(s, 10);
        assert_eq!(l, 5);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstextview_string(0, 0), 0);
        assert_eq!(shim_nstextview_is_editable(0, 0), 0);
        shim_nstextview_set_string(0, 0, 0);
        shim_nstextview_key_down(0, 0, 0);
    }
}
