//! NSPasteboard shim — per-type clipboard manager with arboard cross-platform sync.
//!
//! Object layout (32 bytes, unchanged for ABI compatibility):
//!   [0..8):   ISA pointer
//!   [8..16):  <reserved>
//!   [16..24): name: u64  — NSString (pasteboard name, e.g. "NSGeneralPboard")
//!   [24..32): <reserved>
//!
//! Item storage no longer lives inside the object.  A process-wide
//! `clipboard_manager::STORE` (keyed by pasteboard object pointer) holds a
//! `HashMap<type_name, ItemData>` per pasteboard, so multiple content types
//! (string, raw data, plist object) can coexist on one pasteboard instance.
//!
//! Plain-text writes are mirrored to the system clipboard via `arboard` when
//! the `arboard` feature is enabled, and reads fall back to arboard when the
//! in-memory store for that type is empty.

use std::sync::OnceLock;

const PASTEBOARD_SIZE: usize = 32;
const NAME_OFF: usize = 16;

static GENERAL_PASTEBOARD: OnceLock<u64> = OnceLock::new();
static DRAG_PASTEBOARD: OnceLock<u64> = OnceLock::new();
static FIND_PASTEBOARD: OnceLock<u64> = OnceLock::new();
static RULER_PASTEBOARD: OnceLock<u64> = OnceLock::new();
static FONT_PASTEBOARD: OnceLock<u64> = OnceLock::new();

fn make_pasteboard(name_str: &str) -> u64 {
    let obj = crate::pal_alloc().calloc(1, PASTEBOARD_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nspasteboard_class_addr();
    }
    let name_cstr = std::ffi::CString::new(name_str).unwrap_or_default();
    let name = crate::cfstring::alloc_nsstring_from_cstr(name_cstr.as_ptr());
    unsafe {
        *((obj as usize + NAME_OFF) as *mut u64) = name;
    }
    obj
}

// ── Clipboard manager ─────────────────────────────────────────────────────────
//
// Process-wide type-keyed store.  Keyed by pasteboard object pointer so each
// named/unique pasteboard has its own slot.  Matches the design of
// `app-kit-lite::pasteboard::shared` so the two layers share state on Linux
// via arboard plain-text sync.

pub(crate) mod clipboard_manager {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    /// UTI for plain UTF-8 text — the type arboard syncs through.
    pub const PLAIN_TEXT_TYPE: &str = "public.utf8-plain-text";
    /// Legacy Cocoa plain-text type.
    pub const LEGACY_TEXT_TYPE: &str = "NSStringPboardType";

    /// True when `type_name` should be mirrored through the system clipboard.
    pub fn is_text_type(t: &str) -> bool {
        t == PLAIN_TEXT_TYPE
            || t == LEGACY_TEXT_TYPE
            || t.contains("utf8")
            || t.to_ascii_lowercase().contains("string")
    }

    pub enum ItemData {
        /// Bytes for string / NSData items.
        Bytes(Vec<u8>),
        /// Raw ObjC guest-object pointer for NSPropertyList / writeObjects items.
        Ptr(u64),
    }

    #[derive(Default)]
    pub struct PasteboardEntry {
        pub change_count: u64,
        pub items: HashMap<String, ItemData>,
    }

    static STORE: OnceLock<Mutex<HashMap<u64, PasteboardEntry>>> = OnceLock::new();

    fn store() -> std::sync::MutexGuard<'static, HashMap<u64, PasteboardEntry>> {
        STORE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    /// Write bytes for a type; returns the new change_count.
    pub fn write_bytes(pb: u64, type_name: String, data: Vec<u8>) -> u64 {
        let mut s = store();
        let entry = s.entry(pb).or_default();
        entry.items.insert(type_name, ItemData::Bytes(data));
        entry.change_count += 1;
        entry.change_count
    }

    /// Write a raw ObjC pointer for a type (e.g. NSPropertyList / array).
    pub fn write_ptr(pb: u64, type_name: String, ptr: u64) -> u64 {
        let mut s = store();
        let entry = s.entry(pb).or_default();
        entry.items.insert(type_name, ItemData::Ptr(ptr));
        entry.change_count += 1;
        entry.change_count
    }

    /// Read stored bytes for a type, if present.
    pub fn read_bytes(pb: u64, type_name: &str) -> Option<Vec<u8>> {
        let s = store();
        match s.get(&pb)?.items.get(type_name)? {
            ItemData::Bytes(b) => Some(b.clone()),
            ItemData::Ptr(_) => None,
        }
    }

    /// Read a stored ObjC pointer for a type, if present.
    pub fn read_ptr(pb: u64, type_name: &str) -> Option<u64> {
        let s = store();
        match s.get(&pb)?.items.get(type_name)? {
            ItemData::Ptr(p) => Some(*p),
            ItemData::Bytes(_) => None,
        }
    }

    /// Clear all items; returns the new change_count.
    pub fn clear(pb: u64) -> u64 {
        let mut s = store();
        let entry = s.entry(pb).or_default();
        entry.items.clear();
        entry.change_count += 1;
        entry.change_count
    }

    /// Current change count for this pasteboard.
    pub fn change_count(pb: u64) -> u64 {
        store().get(&pb).map_or(0, |e| e.change_count)
    }

    /// All stored type-name strings.
    pub fn type_names(pb: u64) -> Vec<String> {
        store()
            .get(&pb)
            .map_or_else(Vec::new, |e| e.items.keys().cloned().collect())
    }

    /// Return the first of `candidates` that is stored, or None.
    pub fn first_available(pb: u64, candidates: &[String]) -> Option<String> {
        let s = store();
        let entry = s.get(&pb)?;
        candidates
            .iter()
            .find(|t| entry.items.contains_key(*t))
            .cloned()
    }
}

// ── Helper: extract a Rust type-name from an NSString argument ────────────────

fn type_name_of(ns_type: u64) -> String {
    if ns_type == 0 {
        return clipboard_manager::PLAIN_TEXT_TYPE.to_string();
    }
    crate::cfstring::extract_cfstring_cstr(ns_type)
        .map(|s| s.to_string())
        .unwrap_or_else(|| clipboard_manager::PLAIN_TEXT_TYPE.to_string())
}

// ── Singleton accessors ──────────────────────────────────────────────────────

/// `+[NSPasteboard generalPasteboard]`
pub extern "C" fn shim_nspasteboard_general_pasteboard(_cls: u64, _cmd: u64) -> u64 {
    *GENERAL_PASTEBOARD.get_or_init(|| make_pasteboard("NSGeneralPboard"))
}

/// `+[NSPasteboard pasteboardWithName:]`
pub extern "C" fn shim_nspasteboard_pasteboard_with_name(_cls: u64, _cmd: u64, name: u64) -> u64 {
    if name == 0 {
        return shim_nspasteboard_general_pasteboard(_cls, _cmd);
    }
    let s = match crate::cfstring::extract_cfstring_cstr(name) {
        Some(s) => s.to_string(),
        None => return shim_nspasteboard_general_pasteboard(_cls, _cmd),
    };
    match s.as_str() {
        "NSGeneralPboard" => shim_nspasteboard_general_pasteboard(_cls, _cmd),
        "NSDragPboard" => *DRAG_PASTEBOARD.get_or_init(|| make_pasteboard("NSDragPboard")),
        "NSFindPboard" => *FIND_PASTEBOARD.get_or_init(|| make_pasteboard("NSFindPboard")),
        "NSRulerPboard" => *RULER_PASTEBOARD.get_or_init(|| make_pasteboard("NSRulerPboard")),
        "NSFontPboard" => *FONT_PASTEBOARD.get_or_init(|| make_pasteboard("NSFontPboard")),
        _ => make_pasteboard(s.as_str()),
    }
}

/// `+[NSPasteboard pasteboardWithUniqueName]` — creates a new one-time pasteboard.
pub extern "C" fn shim_nspasteboard_pasteboard_with_unique_name(_cls: u64, _cmd: u64) -> u64 {
    make_pasteboard("com.apple.pasteboard.unique")
}

// ── Writing ──────────────────────────────────────────────────────────────────

/// `−[NSPasteboard declareTypes:owner:]` — clears the store and returns new changeCount.
pub extern "C" fn shim_nspasteboard_declare_types(
    obj: u64,
    _cmd: u64,
    _types: u64,
    _owner: u64,
) -> i64 {
    if obj == 0 {
        return 0;
    }
    clipboard_manager::clear(obj) as i64
}

/// `−[NSPasteboard clearContents]` — clears all items; returns new changeCount.
pub extern "C" fn shim_nspasteboard_clear_contents(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    clipboard_manager::clear(obj) as i64
}

/// `−[NSPasteboard setString:forType:]` — store a string for a given UTI type.
/// Mirrors plain-text writes to the system clipboard via arboard.
pub extern "C" fn shim_nspasteboard_set_string_for_type(
    obj: u64,
    _cmd: u64,
    string: u64,
    ns_type: u64,
) -> u8 {
    if obj == 0 {
        return 0;
    }
    let type_name = type_name_of(ns_type);
    let bytes = if string != 0 {
        crate::cfstring::extract_cfstring_cstr(string)
            .map(|s| s.as_bytes().to_vec())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    // Mirror to system clipboard for text types.
    #[cfg(feature = "arboard")]
    if clipboard_manager::is_text_type(&type_name) {
        if let Ok(text) = std::str::from_utf8(&bytes) {
            let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text));
        }
    }
    clipboard_manager::write_bytes(obj, type_name, bytes);
    1
}

/// `−[NSPasteboard setData:forType:]` — store raw NSData for a given UTI type.
pub extern "C" fn shim_nspasteboard_set_data_for_type(
    obj: u64,
    _cmd: u64,
    data: u64,
    ns_type: u64,
) -> u8 {
    if obj == 0 {
        return 0;
    }
    let type_name = type_name_of(ns_type);
    let bytes = if data != 0 {
        // Read length and bytes_ptr from NSData layout [ISA|len|ptr].
        let len = unsafe { *((data as usize + 8) as *const u64) } as usize;
        let ptr = unsafe { *((data as usize + 16) as *const u64) };
        if len > 0 && ptr != 0 {
            unsafe { std::slice::from_raw_parts(ptr as *const u8, len).to_vec() }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    clipboard_manager::write_bytes(obj, type_name, bytes);
    1
}

/// `−[NSPasteboard setPropertyList:forType:]` — store a raw ObjC plist object.
pub extern "C" fn shim_nspasteboard_set_property_list_for_type(
    obj: u64,
    _cmd: u64,
    plist: u64,
    ns_type: u64,
) -> u8 {
    if obj == 0 {
        return 0;
    }
    clipboard_manager::write_ptr(obj, type_name_of(ns_type), plist);
    1
}

/// `−[NSPasteboard writeObjects:]` — store an NSArray of NSPasteboardWriting objects.
pub extern "C" fn shim_nspasteboard_write_objects(obj: u64, _cmd: u64, objects: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    clipboard_manager::write_ptr(obj, "__writeObjects__".to_string(), objects);
    1
}

// ── Reading ──────────────────────────────────────────────────────────────────

/// `−[NSPasteboard stringForType:]` — read a stored string for a UTI type.
/// Falls back to the system clipboard (arboard) for text types when the
/// in-memory store is empty.
pub extern "C" fn shim_nspasteboard_string_for_type(obj: u64, _cmd: u64, ns_type: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let type_name = type_name_of(ns_type);

    // 1. In-memory store.
    if let Some(bytes) = clipboard_manager::read_bytes(obj, &type_name) {
        if !bytes.is_empty() {
            if let Ok(text) = std::str::from_utf8(&bytes) {
                let cstr = std::ffi::CString::new(text).unwrap_or_default();
                return crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
            }
        }
    }

    // 2. System clipboard fallback for text types.
    #[cfg(feature = "arboard")]
    if clipboard_manager::is_text_type(&type_name) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                if !text.is_empty() {
                    let mut bytes = text.into_bytes();
                    bytes.push(0);
                    return crate::cfstring::alloc_nsstring_from_cstr(
                        bytes.as_ptr() as *const std::ffi::c_char
                    );
                }
            }
        }
    }

    0
}

/// `−[NSPasteboard dataForType:]` — read stored bytes as an NSData object.
pub extern "C" fn shim_nspasteboard_data_for_type(obj: u64, _cmd: u64, ns_type: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let type_name = type_name_of(ns_type);
    clipboard_manager::read_bytes(obj, &type_name)
        .filter(|b| !b.is_empty())
        .map(|b| crate::nsdata::make_nsdata(&b))
        .unwrap_or(0)
}

/// `−[NSPasteboard propertyListForType:]` — return the stored ObjC plist pointer.
pub extern "C" fn shim_nspasteboard_property_list_for_type(
    obj: u64,
    _cmd: u64,
    ns_type: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    clipboard_manager::read_ptr(obj, &type_name_of(ns_type)).unwrap_or(0)
}

/// `−[NSPasteboard readObjectsForClasses:options:]` — returns empty NSArray.
pub extern "C" fn shim_nspasteboard_read_objects_for_classes(
    _obj: u64,
    _cmd: u64,
    _classes: u64,
    _options: u64,
) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

/// `−[NSPasteboard types]` — returns an NSArray of the stored type-name strings.
pub extern "C" fn shim_nspasteboard_types(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }
    let names = clipboard_manager::type_names(obj);
    let ns_strings: Vec<u64> = names
        .iter()
        .map(|n| {
            let cstr = std::ffi::CString::new(n.as_str()).unwrap_or_default();
            crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
        })
        .collect();
    crate::nsarray::make_nsarray(&ns_strings)
}

/// `−[NSPasteboard availableTypeFromArray:]` — first stored type matching the
/// requested list, or nil.
pub extern "C" fn shim_nspasteboard_available_type_from_array(
    obj: u64,
    _cmd: u64,
    types_array: u64,
) -> u64 {
    if obj == 0 || types_array == 0 {
        return 0;
    }
    // Walk the NSArray [ISA|count|elems_ptr] to build the candidate list.
    let count = unsafe {
        *((types_array as usize + crate::nsarray::NSARRAY_COUNT_OFF) as *const u64) as usize
    };
    let elems =
        unsafe { *((types_array as usize + crate::nsarray::NSARRAY_ELEMS_OFF) as *const u64) };
    if count == 0 || elems == 0 {
        return 0;
    }
    let candidates: Vec<String> = (0..count)
        .map(|i| unsafe { *((elems as usize + i * 8) as *const u64) })
        .map(type_name_of)
        .collect();
    if let Some(found) = clipboard_manager::first_available(obj, &candidates) {
        let cstr = std::ffi::CString::new(found.as_str()).unwrap_or_default();
        return crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
    }
    0
}

/// `−[NSPasteboard changeCount]`
pub extern "C" fn shim_nspasteboard_change_count(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    clipboard_manager::change_count(obj) as i64
}

/// `−[NSPasteboard name]`
pub extern "C" fn shim_nspasteboard_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NAME_OFF) as *const u64) }
}

/// `−[NSPasteboard releaseGlobally]` — no-op.
pub extern "C" fn shim_nspasteboard_release_globally(_obj: u64, _cmd: u64) {}

// ── NSPasteboardType constants ────────────────────────────────────────────────

use std::sync::OnceLock as PbLock;

macro_rules! pboard_const {
    ($static_name:ident, $fn_name:ident, $val:expr) => {
        static $static_name: PbLock<u64> = PbLock::new();
        pub fn $fn_name() -> u64 {
            *$static_name.get_or_init(|| {
                let cstr = std::ffi::CString::new($val).unwrap_or_default();
                crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
            })
        }
    };
}

pboard_const!(
    PB_TYPE_STRING,
    ns_pasteboard_type_string_addr,
    "public.utf8-plain-text"
);
pboard_const!(PB_TYPE_RTF, ns_pasteboard_type_rtf_addr, "public.rtf");
pboard_const!(PB_TYPE_HTML, ns_pasteboard_type_html_addr, "public.html");
pboard_const!(PB_TYPE_TIFF, ns_pasteboard_type_tiff_addr, "public.tiff");
pboard_const!(PB_TYPE_PNG, ns_pasteboard_type_png_addr, "public.png");
pboard_const!(PB_TYPE_PDF, ns_pasteboard_type_pdf_addr, "com.adobe.pdf");
pboard_const!(
    PB_TYPE_FILE_URL,
    ns_pasteboard_type_file_url_addr,
    "public.file-url"
);
pboard_const!(PB_TYPE_URL, ns_pasteboard_type_url_addr, "public.url");
pboard_const!(
    PB_NS_STRING_PBOARD,
    ns_string_pasteboard_type_addr,
    "NSStringPboardType"
);
pboard_const!(
    PB_NS_RTF_PBOARD,
    ns_rtf_pasteboard_type_addr,
    "NSRTFPboardType"
);
pboard_const!(
    PB_NS_FILENAMES_PBOARD,
    ns_file_names_pasteboard_type_addr,
    "NSFilenamesPboardType"
);

#[cfg(test)]
mod tests {
    use super::*;

    // Unique pasteboard makes each test independent.
    fn fresh() -> u64 {
        let pb = shim_nspasteboard_pasteboard_with_unique_name(0, 0);
        shim_nspasteboard_clear_contents(pb, 0);
        pb
    }

    fn make_ns_str(s: &str) -> u64 {
        let cstr = std::ffi::CString::new(s).unwrap();
        crate::cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0)
    }

    #[test]
    fn general_pasteboard_not_null() {
        assert_ne!(shim_nspasteboard_general_pasteboard(0, 0), 0);
    }

    #[test]
    fn general_pasteboard_is_singleton() {
        let a = shim_nspasteboard_general_pasteboard(0, 0);
        let b = shim_nspasteboard_general_pasteboard(0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn clear_increments_change_count() {
        let pb = fresh();
        let c0 = shim_nspasteboard_change_count(pb, 0);
        let c1 = shim_nspasteboard_clear_contents(pb, 0);
        assert!(c1 > c0);
    }

    #[test]
    fn set_and_read_string_roundtrip() {
        let pb = fresh();
        let text = make_ns_str("clipboard text");
        assert_eq!(shim_nspasteboard_set_string_for_type(pb, 0, text, 0), 1);
        let back = shim_nspasteboard_string_for_type(pb, 0, 0);
        assert_ne!(back, 0);
        // The returned NSString should carry the same text.
        let recovered = crate::cfstring::extract_cfstring_cstr(back);
        assert_eq!(recovered, Some("clipboard text"));
    }

    #[test]
    fn multiple_types_coexist() {
        let pb = fresh();
        let text_type = make_ns_str("public.utf8-plain-text");
        let html_type = make_ns_str("public.html");
        let text = make_ns_str("hello");
        let html = make_ns_str("<b>hello</b>");

        shim_nspasteboard_set_string_for_type(pb, 0, text, text_type);
        shim_nspasteboard_set_string_for_type(pb, 0, html, html_type);

        let back_text = shim_nspasteboard_string_for_type(pb, 0, text_type);
        let back_html = shim_nspasteboard_string_for_type(pb, 0, html_type);
        assert_ne!(back_text, 0);
        assert_ne!(back_html, 0);
        assert_ne!(
            back_text, back_html,
            "text and HTML must be stored separately"
        );
    }

    #[test]
    fn clear_removes_all_types() {
        let pb = fresh();
        shim_nspasteboard_set_string_for_type(
            pb,
            0,
            make_ns_str("hi"),
            make_ns_str("public.utf8-plain-text"),
        );
        shim_nspasteboard_set_string_for_type(
            pb,
            0,
            make_ns_str("<b>hi</b>"),
            make_ns_str("public.html"),
        );
        shim_nspasteboard_clear_contents(pb, 0);
        assert_eq!(
            shim_nspasteboard_string_for_type(pb, 0, make_ns_str("public.utf8-plain-text")),
            0
        );
        assert_eq!(
            shim_nspasteboard_string_for_type(pb, 0, make_ns_str("public.html")),
            0
        );
    }

    #[test]
    fn set_data_and_read_back() {
        let pb = fresh();
        let data = crate::nsdata::make_nsdata(b"\x89PNG\r\n");
        let png_type = make_ns_str("public.png");
        assert_eq!(
            shim_nspasteboard_set_data_for_type(pb, 0, data, png_type),
            1
        );
        let back = shim_nspasteboard_data_for_type(pb, 0, png_type);
        assert_ne!(back, 0);
        // NSData length should match.
        let len = unsafe { *((back as usize + 8) as *const u64) };
        assert_eq!(len, 6);
    }

    #[test]
    fn types_returns_all_stored_keys() {
        let pb = fresh();
        shim_nspasteboard_set_string_for_type(
            pb,
            0,
            make_ns_str("a"),
            make_ns_str("public.utf8-plain-text"),
        );
        shim_nspasteboard_set_string_for_type(pb, 0, make_ns_str("b"), make_ns_str("public.html"));
        let arr = shim_nspasteboard_types(pb, 0);
        let count = unsafe { *((arr as usize + crate::nsarray::NSARRAY_COUNT_OFF) as *const u64) };
        assert_eq!(count, 2);
    }

    #[test]
    fn change_count_increments_per_write() {
        let pb = fresh();
        let c0 = shim_nspasteboard_change_count(pb, 0);
        shim_nspasteboard_set_string_for_type(
            pb,
            0,
            make_ns_str("x"),
            make_ns_str("public.utf8-plain-text"),
        );
        let c1 = shim_nspasteboard_change_count(pb, 0);
        shim_nspasteboard_set_string_for_type(pb, 0, make_ns_str("y"), make_ns_str("public.html"));
        let c2 = shim_nspasteboard_change_count(pb, 0);
        assert!(c1 > c0);
        assert!(c2 > c1);
    }

    #[test]
    fn available_type_from_array() {
        let pb = fresh();
        shim_nspasteboard_set_string_for_type(pb, 0, make_ns_str("hi"), make_ns_str("public.html"));
        // Ask for string first, then html — should find html since string isn't stored.
        let candidates = crate::nsarray::make_nsarray(&[
            make_ns_str("public.utf8-plain-text"),
            make_ns_str("public.html"),
        ]);
        let found = shim_nspasteboard_available_type_from_array(pb, 0, candidates);
        assert_ne!(found, 0);
        let name = crate::cfstring::extract_cfstring_cstr(found);
        assert_eq!(name, Some("public.html"));
    }

    #[test]
    fn read_objects_returns_empty_array() {
        let arr = shim_nspasteboard_read_objects_for_classes(
            shim_nspasteboard_general_pasteboard(0, 0),
            0,
            0,
            0,
        );
        assert_ne!(arr, 0);
    }

    #[test]
    fn nil_pasteboard_safe() {
        assert_eq!(shim_nspasteboard_string_for_type(0, 0, 0), 0);
        assert_eq!(shim_nspasteboard_clear_contents(0, 0), 0);
        assert_eq!(shim_nspasteboard_change_count(0, 0), 0);
    }

    #[test]
    fn pasteboard_type_constants_not_null() {
        assert_ne!(ns_pasteboard_type_string_addr(), 0);
        assert_ne!(ns_pasteboard_type_tiff_addr(), 0);
    }
}
