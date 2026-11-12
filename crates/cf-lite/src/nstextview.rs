//! NSTextView — rich text editing view.
//!
//! Object layout: 112 bytes
//!   [0..80):   NSView fields
//!   offset 80: string_ptr        (u64 — backing text as NSString)
//!   offset 88: delegate_ptr      (u64)
//!   offset 96: flags             (u64 — is_editable, is_selectable, is_rich_text, etc.)
//!   offset 104: selection_start  (i32) + selection_length (i32) packed in u64

use crate::nsobj;

const TEXT_PADDING_X: f64 = 4.0;
const TEXT_PADDING_Y: f64 = 2.0;
const DEFAULT_FONT_SIZE: f64 = 14.0;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u8).add(offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, val: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u8).add(offset) as *mut u64) = val;
    }
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
    obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> u64 {
    let ptr = if obj == 0 {
        crate::pal_alloc().calloc(1, TEXT_VIEW_SIZE) as u64
    } else {
        obj
    };
    if ptr == 0 {
        return 0;
    }

    // Preserve ISA if already set, else set to NSTextView.
    let isa = read_u64(ptr, 0);
    if isa == 0 {
        write_u64(ptr, 0, nsobj::nstextview_class_addr());
    }

    // Set frame.
    crate::nsview::shim_nsview_set_frame(ptr, 0, x, y, w, h);

    // Default flags: editable + selectable + rich text.
    write_u64(
        ptr,
        FLAGS_OFF,
        FLAG_IS_EDITABLE | FLAG_IS_SELECTABLE | FLAG_IS_RICH_TEXT | FLAG_ALLOWS_UNDO,
    );

    // Empty string (kept as mirror for callers of -[NSTextView string]).
    let empty = crate::cfstring::alloc_nsstring_from_cstr(b"\0".as_ptr() as *const _);
    write_u64(ptr, STRING_OFF, empty);

    // Create a cosmic-text Editor for this view.
    let text_width = (w - TEXT_PADDING_X * 2.0).max(1.0);
    graphics_core::text_system::editor_create(ptr, "", DEFAULT_FONT_SIZE, text_width);

    ptr
}

/// `-[NSTextView string]` — syncs from editor buffer before returning.
pub extern "C" fn shim_nstextview_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Sync STRING_OFF from editor text.
    let text = graphics_core::text_system::editor_get_text(obj);
    if !text.is_empty() {
        let c_str = std::ffi::CString::new(text).unwrap_or_default();
        let ns = crate::cfstring::alloc_nsstring_from_cstr(c_str.as_ptr());
        write_u64(obj, STRING_OFF, ns);
    }
    read_u64(obj, STRING_OFF)
}

/// `-[NSTextView setString:]`
pub extern "C" fn shim_nstextview_set_string(obj: u64, _cmd: u64, str_obj: u64) {
    write_u64(obj, STRING_OFF, str_obj);
    // Sync editor text from the NSString.
    if str_obj != 0 {
        let utf8 = crate::cfstring::shim_nsstring_utf8string(str_obj, 0);
        if !utf8.is_null() {
            if let Ok(s) = unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str() {
                graphics_core::text_system::editor_set_text(obj, s);
            }
        }
    } else {
        graphics_core::text_system::editor_set_text(obj, "");
    }
    crate::nstextstorage::sync_storage_from_view(obj);
}

/// `-[NSTextView textStorage]` — lazily creates and links an NSTextStorage.
pub extern "C" fn shim_nstextview_text_storage(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // If already linked, return the existing storage.
    if let Some(s) = crate::nstextstorage::storage_for_view(obj) {
        // Sync storage string from editor before returning.
        crate::nstextstorage::sync_storage_from_view(obj);
        return s;
    }
    // Create a new NSTextStorage and link it.
    let ts = crate::nstextstorage::shim_nstextstorage_alloc(0, 0);
    let ts = crate::nstextstorage::shim_nstextstorage_init(ts, 0);
    crate::nstextstorage::link_storage_to_view(ts, obj);
    // Sync current editor text into storage.
    crate::nstextstorage::sync_storage_from_view(obj);
    ts
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
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_IS_EDITABLE;
    } else {
        flags &= !FLAG_IS_EDITABLE;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView isSelectable]`
pub extern "C" fn shim_nstextview_is_selectable(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_SELECTABLE != 0)
}

/// `-[NSTextView setSelectable:]`
pub extern "C" fn shim_nstextview_set_selectable(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_IS_SELECTABLE;
    } else {
        flags &= !FLAG_IS_SELECTABLE;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView isRichText]`
pub extern "C" fn shim_nstextview_is_rich_text(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_RICH_TEXT != 0)
}

/// `-[NSTextView setRichText:]`
pub extern "C" fn shim_nstextview_set_rich_text(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_IS_RICH_TEXT;
    } else {
        flags &= !FLAG_IS_RICH_TEXT;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView isFieldEditor]`
pub extern "C" fn shim_nstextview_is_field_editor(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_FIELD_EDITOR != 0)
}

/// `-[NSTextView setFieldEditor:]`
pub extern "C" fn shim_nstextview_set_field_editor(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_IS_FIELD_EDITOR;
    } else {
        flags &= !FLAG_IS_FIELD_EDITOR;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView usesFontPanel]`
pub extern "C" fn shim_nstextview_uses_font_panel(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_USES_FONT_PANEL != 0)
}

/// `-[NSTextView setUsesFontPanel:]`
pub extern "C" fn shim_nstextview_set_uses_font_panel(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_USES_FONT_PANEL;
    } else {
        flags &= !FLAG_USES_FONT_PANEL;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView allowsUndo]`
pub extern "C" fn shim_nstextview_allows_undo(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_ALLOWS_UNDO != 0)
}

/// `-[NSTextView setAllowsUndo:]`
pub extern "C" fn shim_nstextview_set_allows_undo(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_ALLOWS_UNDO;
    } else {
        flags &= !FLAG_ALLOWS_UNDO;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSTextView selectedRange]` — returns NSRange as (location, length) via x0/x1.
pub extern "C" fn shim_nstextview_selected_range(obj: u64, _cmd: u64) -> (u64, u64) {
    if obj == 0 {
        return (0, 0);
    }
    let packed = read_u64(obj, SELECTION_OFF);
    let (start, length) = unpack_selection(packed);
    (start as u64, length as u64)
}

/// `-[NSTextView setSelectedRange:]`
pub extern "C" fn shim_nstextview_set_selected_range(obj: u64, _cmd: u64, loc: u64, len: u64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, SELECTION_OFF, pack_selection(loc as i32, len as i32));
}

/// `-[NSTextView acceptsFirstResponder]`
pub extern "C" fn shim_nstextview_accepts_first_responder(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_IS_EDITABLE != 0)
}

/// `-[NSTextView insertText:]`
pub extern "C" fn shim_nstextview_insert_text(obj: u64, _cmd: u64, text: u64) {
    if obj == 0 || text == 0 {
        return;
    }

    let utf8 = crate::cfstring::shim_nsstring_utf8string(text, 0);
    if utf8.is_null() {
        return;
    }
    let typed = match unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    if typed.is_empty() {
        return;
    }

    // Insert each character through the editor at the cursor.
    use graphics_core::text_system::Action;
    for ch in typed.chars() {
        graphics_core::text_system::editor_action(obj, Action::Insert(ch));
    }
    crate::nstextstorage::sync_storage_from_view(obj);
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSTextView keyDown:]`
pub extern "C" fn shim_nstextview_key_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }

    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_IS_EDITABLE == 0 {
        crate::nsresponder::shim_nsresponder_key_down(obj, _cmd, event);
        return;
    }

    use graphics_core::text_system::{Action, Motion};

    let key_code = crate::nsevent::shim_nsevent_key_code(event, 0);
    let modifiers = crate::nsevent::shim_nsevent_modifier_flags(event, 0);
    let shift = modifiers & 0x20000 != 0; // NSEventModifierFlagShift
    let cmd = modifiers & 0x100000 != 0; // NSEventModifierFlagCommand

    let action = match key_code {
        51 => Some(Action::Backspace), // Delete/Backspace
        117 => Some(Action::Delete),   // Forward Delete
        36 => Some(Action::Enter),     // Return
        123 => {
            // Left arrow
            if cmd {
                Some(Action::Motion(Motion::Home))
            } else if shift {
                Some(Action::Motion(Motion::LeftWord))
            } else {
                Some(Action::Motion(Motion::Left))
            }
        }
        124 => {
            // Right arrow
            if cmd {
                Some(Action::Motion(Motion::End))
            } else if shift {
                Some(Action::Motion(Motion::RightWord))
            } else {
                Some(Action::Motion(Motion::Right))
            }
        }
        125 => Some(Action::Motion(Motion::Down)), // Down arrow
        126 => Some(Action::Motion(Motion::Up)),   // Up arrow
        _ => None,
    };

    if let Some(a) = action {
        graphics_core::text_system::editor_action(obj, a);
        crate::nstextstorage::sync_storage_from_view(obj);
        crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
        return;
    }

    // Cmd+A → select all.
    if cmd && key_code == 0 {
        graphics_core::text_system::editor_action(obj, Action::Motion(Motion::BufferStart));
        // No direct select-all action; move to end with selection would require
        // a different approach. For now, skip.
        crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
        return;
    }

    // Regular character input.
    let chars_ptr = crate::nsevent::shim_nsevent_characters(event, 0);
    if chars_ptr != 0 {
        shim_nstextview_insert_text(obj, 0, chars_ptr);
    }
}

/// `-[NSTextView mouseDown:]` — set cursor from click position.
pub extern "C" fn shim_nstextview_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_IS_SELECTABLE == 0 {
        return;
    }

    // Make this text view the first responder.
    let window = crate::nsview::shim_nsview_window(obj, 0);
    if window != 0 {
        crate::nswindow::shim_nswindow_make_first_responder(window, 0, obj);
    }

    let (loc_x, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let click_x = (loc_x - frame.origin.x - TEXT_PADDING_X) as i32;
    let click_y = (loc_y - frame.origin.y - TEXT_PADDING_Y) as i32;

    let click_count = crate::nsevent::shim_nsevent_click_count(event, 0);
    use graphics_core::text_system::Action;
    let action = match click_count {
        2 => Action::DoubleClick {
            x: click_x,
            y: click_y,
        },
        3 => Action::TripleClick {
            x: click_x,
            y: click_y,
        },
        _ => Action::Click {
            x: click_x,
            y: click_y,
        },
    };
    graphics_core::text_system::editor_action(obj, action);
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSTextView mouseDragged:]` — extend selection.
pub extern "C" fn shim_nstextview_mouse_dragged(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_IS_SELECTABLE == 0 {
        return;
    }

    let (loc_x, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let drag_x = (loc_x - frame.origin.x - TEXT_PADDING_X) as i32;
    let drag_y = (loc_y - frame.origin.y - TEXT_PADDING_Y) as i32;

    use graphics_core::text_system::Action;
    graphics_core::text_system::editor_action(
        obj,
        Action::Drag {
            x: drag_x,
            y: drag_y,
        },
    );
    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSTextView drawRect:]`
pub extern "C" fn shim_nstextview_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }

    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    // Update editor width in case view was resized.
    let text_width = (w - TEXT_PADDING_X * 2.0).max(1.0);
    graphics_core::text_system::editor_set_width(obj, text_width);

    // White background.
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.fill_color = graphics_core::Color::WHITE;
        c.fill_rect(x, y, w, h);
    });

    // Check if this view is the window's first responder (for cursor drawing).
    let window = crate::nsview::shim_nsview_window(obj, 0);
    let is_first_responder = if window != 0 {
        crate::nswindow::shim_nswindow_first_responder(window, 0) == obj
    } else {
        false
    };

    // Draw text + selection + cursor via the editor.
    cg_shims::bridge::with_context(cg_ctx, |c| {
        let sw = c.surface.width();
        let sh = c.surface.height();
        let sel_bg = graphics_core::Color::new(0.26, 0.52, 0.96, 0.3); // light blue selection
        graphics_core::text_system::editor_draw(
            obj,
            c.surface.pixels_mut(),
            sw,
            sh,
            x + TEXT_PADDING_X,
            y + TEXT_PADDING_Y,
            graphics_core::Color::BLACK,
            sel_bg,
            is_first_responder,
        );
    });
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
        // After the editor rewrite, string returns a fresh NSString from the
        // editor buffer, so check text content rather than pointer identity.
        let got = shim_nstextview_string(tv, 0);
        assert_ne!(got, 0);
        let utf8 = crate::cfstring::shim_nsstring_utf8string(got, 0);
        assert!(!utf8.is_null());
        let text = unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str().unwrap();
        assert_eq!(text, "Hello World");
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
        assert_eq!(shim_nstextview_text_storage(0, 0), 0);
        shim_nstextview_set_string(0, 0, 0);
        shim_nstextview_key_down(0, 0, 0);
    }

    #[test]
    fn text_storage_creates_and_links() {
        let tv = shim_nstextview_init_with_frame(0, 0, 0.0, 0.0, 200.0, 100.0);
        let s = make_string("Live bridge");
        shim_nstextview_set_string(tv, 0, s);

        // First call creates the storage.
        let ts = shim_nstextview_text_storage(tv, 0);
        assert_ne!(ts, 0);
        // Storage ISA should be NSTextStorage.
        assert_eq!(read_u64(ts, 0), crate::nsobj::nstextstorage_class_addr());
        // Storage string should match the editor text.
        let s_ptr = crate::nstextstorage::shim_nstextstorage_string(ts, 0);
        assert_ne!(s_ptr, 0);
        let utf8 = crate::cfstring::shim_nsstring_utf8string(s_ptr, 0);
        let text = unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str().unwrap();
        assert_eq!(text, "Live bridge");

        // Second call returns the same storage.
        let ts2 = shim_nstextview_text_storage(tv, 0);
        assert_eq!(ts, ts2);

        free_obj(tv);
    }

    #[test]
    fn storage_mutation_reaches_editor() {
        let tv = shim_nstextview_init_with_frame(0, 0, 0.0, 0.0, 200.0, 100.0);
        // Force creation of linked storage.
        let ts = shim_nstextview_text_storage(tv, 0);
        assert_ne!(ts, 0);

        // Mutate storage directly.
        let new_s = make_string("From storage");
        crate::nstextstorage::shim_nstextstorage_set_string(ts, 0, new_s);

        // Editor should reflect the change.
        let editor_text = graphics_core::text_system::editor_get_text(tv);
        assert_eq!(editor_text, "From storage");

        free_obj(tv);
    }
}
