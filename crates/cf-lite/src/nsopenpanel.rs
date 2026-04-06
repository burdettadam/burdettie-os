//! NSOpenPanel / NSSavePanel — file dialog shims.
//!
//! NSSavePanel layout: 104 bytes
//!   [0..88):   NSPanel fields
//!   offset 88: title_ptr       (u64 — dialog title)
//!   offset 96: flags           (u64 — bit fields)
//!
//! NSOpenPanel layout: 104 bytes (same as NSSavePanel)
//!   Additional behavior: allows multiple selection, can choose directories.
//!
//! `runModal` uses `rfd` (native file dialog) when available, otherwise
//! returns NSModalResponseCancel (0).
//!
//! URLs are stored in a global map keyed by object pointer.

use crate::nsobj;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as *const u8).add(offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, val: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as *mut u8).add(offset) as *mut u64) = val; }
}

pub(crate) const PANEL_SIZE: usize = 104;
const TITLE_OFF: usize = 88;
const FLAGS_OFF: usize = 96;

// Flags
const FLAG_CAN_CHOOSE_FILES: u64 = 1 << 0;
const FLAG_CAN_CHOOSE_DIRS: u64 = 1 << 1;
const FLAG_ALLOWS_MULTIPLE: u64 = 1 << 2;
const FLAG_CAN_CREATE_DIRS: u64 = 1 << 3;
const FLAG_SHOWS_HIDDEN: u64 = 1 << 4;

// Modal response constants
const NS_MODAL_RESPONSE_OK: u64 = 1;
const NS_MODAL_RESPONSE_CANCEL: u64 = 0;

use std::collections::HashMap;
use std::sync::Mutex;

static PANEL_URLS: Mutex<Option<HashMap<u64, Vec<u64>>>> = Mutex::new(None);
static PANEL_ALLOWED_TYPES: Mutex<Option<HashMap<u64, Vec<String>>>> = Mutex::new(None);

/// Helper: create a file:// NSURL from a PathBuf.
fn file_url_from_path(path: &std::path::Path) -> u64 {
    let s = path.to_string_lossy().to_string();
    let c_str = std::ffi::CString::new(s).unwrap_or_default();
    let ns_str = crate::cfstring::alloc_nsstring_from_cstr(c_str.as_ptr());
    crate::nsurl::shim_nsurl_file_url_with_path(0, 0, ns_str)
}

fn store_urls(obj: u64, urls: Vec<u64>) {
    let mut guard = PANEL_URLS.lock().unwrap();
    guard.get_or_insert_with(HashMap::new).insert(obj, urls);
}

fn get_urls(obj: u64) -> Vec<u64> {
    let guard = PANEL_URLS.lock().unwrap();
    guard.as_ref().and_then(|m| m.get(&obj)).cloned().unwrap_or_default()
}

fn store_allowed_types(obj: u64, types: Vec<String>) {
    let mut guard = PANEL_ALLOWED_TYPES.lock().unwrap();
    guard.get_or_insert_with(HashMap::new).insert(obj, types);
}

fn get_allowed_types(obj: u64) -> Vec<String> {
    let guard = PANEL_ALLOWED_TYPES.lock().unwrap();
    guard.as_ref().and_then(|m| m.get(&obj)).cloned().unwrap_or_default()
}

// ── NSSavePanel ──────────────────────────────────────────────────

/// `+[NSSavePanel savePanel]`
pub extern "C" fn shim_nssavepanel_save_panel(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, PANEL_SIZE) as u64;
    if obj == 0 { return 0; }
    write_u64(obj, 0, nsobj::nssavepanel_class_addr());
    write_u64(obj, FLAGS_OFF, FLAG_CAN_CREATE_DIRS);
    obj
}

/// `-[NSSavePanel title]`
pub extern "C" fn shim_nssavepanel_title(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TITLE_OFF)
}

/// `-[NSSavePanel setTitle:]`
pub extern "C" fn shim_nssavepanel_set_title(obj: u64, _cmd: u64, title: u64) {
    write_u64(obj, TITLE_OFF, title);
}

/// `-[NSSavePanel canCreateDirectories]`
pub extern "C" fn shim_nssavepanel_can_create_directories(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_CAN_CREATE_DIRS != 0)
}

/// `-[NSSavePanel setCanCreateDirectories:]`
pub extern "C" fn shim_nssavepanel_set_can_create_directories(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_CAN_CREATE_DIRS; } else { flags &= !FLAG_CAN_CREATE_DIRS; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSSavePanel showsHiddenFiles]`
pub extern "C" fn shim_nssavepanel_shows_hidden_files(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_SHOWS_HIDDEN != 0)
}

/// `-[NSSavePanel setShowsHiddenFiles:]`
pub extern "C" fn shim_nssavepanel_set_shows_hidden_files(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_SHOWS_HIDDEN; } else { flags &= !FLAG_SHOWS_HIDDEN; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSSavePanel allowedContentTypes]` — stub returns nil.
pub extern "C" fn shim_nssavepanel_allowed_content_types(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let types = get_allowed_types(obj);
    if types.is_empty() { return 0; }
    // Return an empty NSArray as a placeholder.
    crate::nsarray::make_nsarray(&[])
}

/// `-[NSSavePanel setAllowedContentTypes:]` — stores for filter info.
pub extern "C" fn shim_nssavepanel_set_allowed_content_types(obj: u64, _cmd: u64, types: u64) {
    if obj == 0 { return; }
    // Extract type strings from the NSArray for filtering.
    let count = crate::nsarray::shim_nsarray_count(types, 0) as usize;
    let mut type_strs = Vec::new();
    for i in 0..count {
        let item = crate::nsarray::shim_nsarray_object_at_index(types, 0, i as u64);
        if item != 0 {
            let utf8 = crate::cfstring::shim_nsstring_utf8string(item, 0);
            if !utf8.is_null() {
                if let Ok(s) = unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str() {
                    type_strs.push(s.to_string());
                }
            }
        }
    }
    store_allowed_types(obj, type_strs);
}

/// `-[NSSavePanel URL]` — returns the first selected URL (or nil).
pub extern "C" fn shim_nssavepanel_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let urls = get_urls(obj);
    urls.first().copied().unwrap_or(0)
}

/// `-[NSSavePanel runModal]`
///
/// Stub implementation: always returns NSModalResponseCancel (0).
/// TODO: Wire to a native file dialog (rfd or similar) when the dependency is added.
pub extern "C" fn shim_nssavepanel_run_modal(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return NS_MODAL_RESPONSE_CANCEL; }
    log::debug!("NSSavePanel runModal: stub — returning Cancel");
    NS_MODAL_RESPONSE_CANCEL
}

// ── NSOpenPanel ──────────────────────────────────────────────────

/// `+[NSOpenPanel openPanel]`
pub extern "C" fn shim_nsopenpanel_open_panel(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, PANEL_SIZE) as u64;
    if obj == 0 { return 0; }
    write_u64(obj, 0, nsobj::nsopenpanel_class_addr());
    write_u64(obj, FLAGS_OFF, FLAG_CAN_CHOOSE_FILES);
    obj
}

/// `-[NSOpenPanel canChooseFiles]`
pub extern "C" fn shim_nsopenpanel_can_choose_files(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_CAN_CHOOSE_FILES != 0)
}

/// `-[NSOpenPanel setCanChooseFiles:]`
pub extern "C" fn shim_nsopenpanel_set_can_choose_files(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_CAN_CHOOSE_FILES; } else { flags &= !FLAG_CAN_CHOOSE_FILES; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSOpenPanel canChooseDirectories]`
pub extern "C" fn shim_nsopenpanel_can_choose_directories(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_CAN_CHOOSE_DIRS != 0)
}

/// `-[NSOpenPanel setCanChooseDirectories:]`
pub extern "C" fn shim_nsopenpanel_set_can_choose_directories(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_CAN_CHOOSE_DIRS; } else { flags &= !FLAG_CAN_CHOOSE_DIRS; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSOpenPanel allowsMultipleSelection]`
pub extern "C" fn shim_nsopenpanel_allows_multiple_selection(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_ALLOWS_MULTIPLE != 0)
}

/// `-[NSOpenPanel setAllowsMultipleSelection:]`
pub extern "C" fn shim_nsopenpanel_set_allows_multiple_selection(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_ALLOWS_MULTIPLE; } else { flags &= !FLAG_ALLOWS_MULTIPLE; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSOpenPanel URLs]`
pub extern "C" fn shim_nsopenpanel_urls(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let urls = get_urls(obj);
    crate::nsarray::make_nsarray(&urls)
}

/// `-[NSOpenPanel runModal]`
///
/// Stub implementation: always returns NSModalResponseCancel (0).
/// TODO: Wire to a native file dialog (rfd or similar) when the dependency is added.
pub extern "C" fn shim_nsopenpanel_run_modal(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return NS_MODAL_RESPONSE_CANCEL; }
    log::debug!("NSOpenPanel runModal: stub — returning Cancel");
    NS_MODAL_RESPONSE_CANCEL
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn open_panel_factory() {
        let panel = shim_nsopenpanel_open_panel(0, 0);
        assert_ne!(panel, 0);
        assert_eq!(read_u64(panel, 0), nsobj::nsopenpanel_class_addr());
        // Default: can choose files
        assert_eq!(shim_nsopenpanel_can_choose_files(panel, 0), 1);
        assert_eq!(shim_nsopenpanel_can_choose_directories(panel, 0), 0);
        assert_eq!(shim_nsopenpanel_allows_multiple_selection(panel, 0), 0);
        free_obj(panel);
    }

    #[test]
    fn save_panel_factory() {
        let panel = shim_nssavepanel_save_panel(0, 0);
        assert_ne!(panel, 0);
        assert_eq!(read_u64(panel, 0), nsobj::nssavepanel_class_addr());
        assert_eq!(shim_nssavepanel_can_create_directories(panel, 0), 1);
        free_obj(panel);
    }

    #[test]
    fn open_panel_flags() {
        let panel = shim_nsopenpanel_open_panel(0, 0);
        shim_nsopenpanel_set_can_choose_directories(panel, 0, 1);
        assert_eq!(shim_nsopenpanel_can_choose_directories(panel, 0), 1);
        shim_nsopenpanel_set_allows_multiple_selection(panel, 0, 1);
        assert_eq!(shim_nsopenpanel_allows_multiple_selection(panel, 0), 1);
        free_obj(panel);
    }

    #[test]
    fn save_panel_title() {
        let panel = shim_nssavepanel_save_panel(0, 0);
        let title = make_string("Save As…");
        shim_nssavepanel_set_title(panel, 0, title);
        assert_eq!(shim_nssavepanel_title(panel, 0), title);
        free_obj(panel);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsopenpanel_can_choose_files(0, 0), 0);
        assert_eq!(shim_nssavepanel_url(0, 0), 0);
        assert_eq!(shim_nsopenpanel_urls(0, 0), 0);
        shim_nsopenpanel_set_can_choose_files(0, 0, 1); // no crash
    }
}
