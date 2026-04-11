//! NSTextStorage — backing store for the TextKit pipeline.
//!
//! Object layout: 24 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  string_ptr  (NSString backing)
//!   [16..24): delegate_ptr (layout manager or nil)
//!
//! ## Live bridge
//!
//! A global side-table (`STORAGE_VIEWS`) links each NSTextStorage to its
//! owning NSTextView(s).  When a guest mutates the storage (setString:,
//! replaceCharactersInRange:withString:), the change is forwarded to every
//! linked view's cosmic-text Editor via `editor_set_text`.  Conversely,
//! after NSTextView edits (insertText:, keyDown:), the view calls
//! `sync_storage_from_view` to push the editor text back into the storage's
//! backing NSString.

use std::collections::HashMap;
use std::sync::Mutex;

const SIZE: usize = 24;
const STRING_OFF: usize = 8;
const _DELEGATE_OFF: usize = 16;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as *const u8).add(offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, val: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as *mut u8).add(offset) as *mut u64) = val; }
}

// ── Side-table: storage ↔ view linkage ─────────────────────────

static STORAGE_VIEWS: Mutex<Option<HashMap<u64, Vec<u64>>>> = Mutex::new(None);
static VIEW_STORAGE: Mutex<Option<HashMap<u64, u64>>> = Mutex::new(None);

/// Link a storage to a text view (bidirectional).
pub fn link_storage_to_view(storage: u64, view: u64) {
    if storage == 0 || view == 0 { return; }
    {
        let mut guard = STORAGE_VIEWS.lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);
        let views = map.entry(storage).or_default();
        if !views.contains(&view) {
            views.push(view);
        }
    }
    {
        let mut guard = VIEW_STORAGE.lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(view, storage);
    }
}

/// Remove a view from its linked storage.
pub fn unlink_view(view: u64) {
    if view == 0 { return; }
    let storage = {
        let mut guard = VIEW_STORAGE.lock().unwrap();
        guard.as_mut().and_then(|m| m.remove(&view))
    };
    if let Some(s) = storage {
        let mut guard = STORAGE_VIEWS.lock().unwrap();
        if let Some(map) = guard.as_mut() {
            if let Some(views) = map.get_mut(&s) {
                views.retain(|&v| v != view);
            }
        }
    }
}

/// Return the storage linked to a view, if any.
pub fn storage_for_view(view: u64) -> Option<u64> {
    if view == 0 { return None; }
    let guard = VIEW_STORAGE.lock().unwrap();
    guard.as_ref().and_then(|m| m.get(&view).copied())
}

/// Return the views linked to a storage.
fn views_for_storage(storage: u64) -> Vec<u64> {
    if storage == 0 { return vec![]; }
    let guard = STORAGE_VIEWS.lock().unwrap();
    guard.as_ref()
        .and_then(|m| m.get(&storage))
        .cloned()
        .unwrap_or_default()
}

/// Sync the storage's backing NSString from the editor text of `view`.
/// Called by NSTextView after internal edits (insertText, keyDown, etc.).
pub fn sync_storage_from_view(view: u64) {
    let storage = match storage_for_view(view) {
        Some(s) => s,
        None => return,
    };
    let text = graphics_core::text_system::editor_get_text(view);
    let c_str = std::ffi::CString::new(text).unwrap_or_default();
    let ns = crate::cfstring::alloc_nsstring_from_cstr(c_str.as_ptr());
    write_u64(storage, STRING_OFF, ns);
}

/// Push the storage's string content into all linked views' editors.
fn sync_views_from_storage(storage: u64) {
    let views = views_for_storage(storage);
    if views.is_empty() { return; }
    let s_ptr = read_u64(storage, STRING_OFF);
    if s_ptr == 0 { return; }
    let utf8 = crate::cfstring::shim_nsstring_utf8string(s_ptr, 0);
    if utf8.is_null() { return; }
    let text = match unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str() {
        Ok(t) => t,
        Err(_) => return,
    };
    for view in views {
        graphics_core::text_system::editor_set_text(view, text);
        crate::nsview::shim_nsview_set_needs_display(view, 0, 1);
    }
}

// ── ObjC shims ─────────────────────────────────────────────────

/// `+[NSTextStorage alloc]`
pub extern "C" fn shim_nstextstorage_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = crate::nsobj::nstextstorage_class_addr(); }
    obj
}

/// `-[NSTextStorage init]`
pub extern "C" fn shim_nstextstorage_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    // Create empty backing string.
    let empty = crate::cfstring::alloc_nsstring_from_cstr(b"\0".as_ptr() as *const _);
    write_u64(obj, STRING_OFF, empty);
    obj
}

/// `-[NSTextStorage string]` → backing NSString
pub extern "C" fn shim_nstextstorage_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    read_u64(obj, STRING_OFF)
}

/// `-[NSTextStorage setString:]`
pub extern "C" fn shim_nstextstorage_set_string(obj: u64, _cmd: u64, s: u64) {
    if obj == 0 { return; }
    write_u64(obj, STRING_OFF, s);
    sync_views_from_storage(obj);
}

/// `-[NSTextStorage length]`
pub extern "C" fn shim_nstextstorage_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let s = read_u64(obj, STRING_OFF);
    if s == 0 { return 0; }
    crate::cfstring::shim_nsstring_length(s, 0)
}

/// `-[NSTextStorage replaceCharactersInRange:withString:]`
/// Simplified: replaces entire string (range ignored), then syncs to linked views.
pub extern "C" fn shim_nstextstorage_replace_chars(
    obj: u64, _cmd: u64, _loc: u64, _len: u64, replacement: u64,
) {
    if obj == 0 { return; }
    write_u64(obj, STRING_OFF, replacement);
    sync_views_from_storage(obj);
}

/// `-[NSTextStorage mutableString]` → returns self.string (simplified)
pub extern "C" fn shim_nstextstorage_mutable_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    read_u64(obj, STRING_OFF)
}

/// `-[NSTextStorage addAttribute:value:range:]` → no-op stub
pub extern "C" fn shim_nstextstorage_add_attribute(
    _obj: u64, _cmd: u64, _attr: u64, _value: u64, _loc: u64, _len: u64,
) {}

/// `-[NSTextStorage addLayoutManager:]` — sets textStorage on the layout
/// manager so the LM can reach back to read our string.
pub extern "C" fn shim_nstextstorage_add_layout_manager(
    obj: u64, _cmd: u64, lm: u64,
) {
    if obj == 0 || lm == 0 { return; }
    crate::nslayoutmanager::shim_nslayoutmanager_set_text_storage(lm, 0, obj);
}

/// `-[NSTextStorage removeLayoutManager:]` → no-op stub
pub extern "C" fn shim_nstextstorage_remove_layout_manager(
    _obj: u64, _cmd: u64, _lm: u64,
) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn alloc_init_round_trip() {
        let ts = shim_nstextstorage_alloc(0, 0);
        let ts = shim_nstextstorage_init(ts, 0);
        assert_ne!(ts, 0);
        assert_eq!(read_u64(ts, 0), crate::nsobj::nstextstorage_class_addr());
        // Empty backing string.
        assert_eq!(shim_nstextstorage_length(ts, 0), 0);
        free_obj(ts);
    }

    #[test]
    fn set_and_get_string() {
        let ts = shim_nstextstorage_alloc(0, 0);
        let ts = shim_nstextstorage_init(ts, 0);
        let s = make_string("Hello TextKit");
        shim_nstextstorage_set_string(ts, 0, s);
        assert_eq!(shim_nstextstorage_string(ts, 0), s);
        assert_eq!(shim_nstextstorage_length(ts, 0), 13);
        free_obj(ts);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstextstorage_init(0, 0), 0);
        assert_eq!(shim_nstextstorage_string(0, 0), 0);
        assert_eq!(shim_nstextstorage_length(0, 0), 0);
        shim_nstextstorage_set_string(0, 0, 42);
    }

    #[test]
    fn link_and_lookup() {
        let ts = shim_nstextstorage_alloc(0, 0);
        let ts = shim_nstextstorage_init(ts, 0);
        let fake_view: u64 = 0xDEAD_0001;

        link_storage_to_view(ts, fake_view);
        assert_eq!(storage_for_view(fake_view), Some(ts));
        assert_eq!(views_for_storage(ts), vec![fake_view]);

        unlink_view(fake_view);
        assert_eq!(storage_for_view(fake_view), None);
        assert!(views_for_storage(ts).is_empty());
        free_obj(ts);
    }

    #[test]
    fn link_nil_is_noop() {
        link_storage_to_view(0, 42);
        link_storage_to_view(42, 0);
        assert_eq!(storage_for_view(42), None);
    }

    #[test]
    fn set_string_syncs_to_linked_view() {
        // Create a text view with an editor, then link a storage to it.
        let tv = crate::nstextview::shim_nstextview_init_with_frame(0, 0, 0.0, 0.0, 200.0, 100.0);
        let ts = shim_nstextstorage_alloc(0, 0);
        let ts = shim_nstextstorage_init(ts, 0);
        link_storage_to_view(ts, tv);

        // Mutate the storage.
        let s = make_string("Bridge works");
        shim_nstextstorage_set_string(ts, 0, s);

        // The editor text should match.
        let editor_text = graphics_core::text_system::editor_get_text(tv);
        assert_eq!(editor_text, "Bridge works");

        unlink_view(tv);
        free_obj(ts);
        free_obj(tv);
    }

    #[test]
    fn replace_chars_syncs_to_linked_view() {
        let tv = crate::nstextview::shim_nstextview_init_with_frame(0, 0, 0.0, 0.0, 200.0, 100.0);
        let ts = shim_nstextstorage_alloc(0, 0);
        let ts = shim_nstextstorage_init(ts, 0);
        link_storage_to_view(ts, tv);

        let s = make_string("Replaced");
        shim_nstextstorage_replace_chars(ts, 0, 0, 0, s);

        let editor_text = graphics_core::text_system::editor_get_text(tv);
        assert_eq!(editor_text, "Replaced");

        unlink_view(tv);
        free_obj(ts);
        free_obj(tv);
    }

    #[test]
    fn add_layout_manager_wires_text_storage() {
        let ts = shim_nstextstorage_alloc(0, 0);
        let ts = shim_nstextstorage_init(ts, 0);
        let lm = crate::nslayoutmanager::shim_nslayoutmanager_alloc(0, 0);
        let lm = crate::nslayoutmanager::shim_nslayoutmanager_init(lm, 0);

        // Before wiring, lm has no text storage.
        assert_eq!(crate::nslayoutmanager::shim_nslayoutmanager_text_storage(lm, 0), 0);

        shim_nstextstorage_add_layout_manager(ts, 0, lm);

        // After wiring, lm's textStorage should be ts.
        assert_eq!(crate::nslayoutmanager::shim_nslayoutmanager_text_storage(lm, 0), ts);

        free_obj(lm);
        free_obj(ts);
    }

    #[test]
    fn add_layout_manager_nil_safety() {
        let ts = shim_nstextstorage_alloc(0, 0);
        let ts = shim_nstextstorage_init(ts, 0);
        // nil lm should not crash.
        shim_nstextstorage_add_layout_manager(ts, 0, 0);
        // nil obj should not crash.
        shim_nstextstorage_add_layout_manager(0, 0, 42);
        free_obj(ts);
    }
}
