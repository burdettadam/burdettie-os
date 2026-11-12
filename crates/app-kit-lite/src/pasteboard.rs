//! NSPasteboard — clipboard snapshot with native macOS host integration.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const NS_PASTEBOARD_TYPE_STRING: &str = "public.utf8-plain-text";
pub const NS_PASTEBOARD_TYPE_PDF: &str = "com.adobe.pdf";
pub const NS_PASTEBOARD_TYPE_PNG: &str = "public.png";

type PasteboardContents = HashMap<String, Vec<u8>>;

/// Clipboard snapshot backed by the host pasteboard on macOS builds.
///
/// The in-memory `contents` map is the current snapshot for this instance.
/// Construct a new instance with [`NSPasteboard::general`] to refresh from the
/// host clipboard.
pub struct NSPasteboard {
    contents: PasteboardContents,
    change_count: u64,
}

impl NSPasteboard {
    /// Create a general pasteboard instance.
    pub fn general() -> Self {
        let mut pasteboard = Self {
            contents: HashMap::new(),
            change_count: 0,
        };
        pasteboard.refresh();
        pasteboard
    }

    /// Refresh this instance from the current host pasteboard snapshot.
    pub fn refresh(&mut self) {
        #[cfg(all(target_os = "macos", not(test)))]
        if native_clipboard_enabled() {
            self.change_count = native::refresh_snapshot(&mut self.contents);
            shared::store_snapshot(&self.contents, self.change_count);
            return;
        }

        // On non-macOS, pull the latest plain-text from arboard first.
        #[cfg(all(feature = "arboard", not(target_os = "macos"), not(test)))]
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                if !text.is_empty() {
                    self.contents
                        .insert(NS_PASTEBOARD_TYPE_STRING.to_string(), text.into_bytes());
                }
            }
        }

        self.change_count = shared::load_snapshot(&mut self.contents);
    }

    /// Clear all contents and increment the change count.
    pub fn clear_contents(&mut self) -> u64 {
        self.contents.clear();
        self.change_count = sync_snapshot_to_host(&self.contents, self.change_count);
        self.change_count
    }

    /// Set a string value for the given type. Returns true on success.
    pub fn set_string(&mut self, string: &str, for_type: &str) -> bool {
        self.contents
            .insert(for_type.to_string(), string.as_bytes().to_vec());
        self.change_count = sync_snapshot_to_host(&self.contents, self.change_count);
        true
    }

    /// Get a string value for the given type.
    pub fn string_for_type(&self, type_name: &str) -> Option<&str> {
        self.contents
            .get(type_name)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
    }

    /// Set raw data for the given type. Returns true on success.
    pub fn set_data(&mut self, data: &[u8], for_type: &str) -> bool {
        self.contents.insert(for_type.to_string(), data.to_vec());
        self.change_count = sync_snapshot_to_host(&self.contents, self.change_count);
        true
    }

    /// Get raw data for the given type.
    pub fn data_for_type(&self, type_name: &str) -> Option<&[u8]> {
        self.contents.get(type_name).map(|v| v.as_slice())
    }

    /// Return all type keys currently on the pasteboard.
    pub fn types(&self) -> Vec<&str> {
        self.contents.keys().map(|k| k.as_str()).collect()
    }

    /// The current change count.
    pub fn change_count(&self) -> u64 {
        self.change_count
    }
}

fn sync_snapshot_to_host(contents: &PasteboardContents, current_change_count: u64) -> u64 {
    #[cfg(all(target_os = "macos", not(test)))]
    if native_clipboard_enabled() {
        if let Some(change_count) = native::write_snapshot(contents) {
            return shared::store_snapshot(contents, change_count);
        }
    }

    // On non-macOS, mirror plain-text writes to the system clipboard via arboard
    // so that cf-lite ObjC shims and Rust-level callers share the same content.
    #[cfg(all(feature = "arboard", not(target_os = "macos"), not(test)))]
    if let Some(bytes) = contents.get(NS_PASTEBOARD_TYPE_STRING) {
        if let Ok(text) = std::str::from_utf8(bytes) {
            let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text));
        }
    }

    shared::store_snapshot(contents, current_change_count.saturating_add(1))
}

#[cfg(all(target_os = "macos", not(test)))]
fn native_clipboard_enabled() -> bool {
    std::env::var_os("DARWIN_LOADER_FORCE_IN_MEMORY_PASTEBOARD").is_none()
}

mod shared {
    use super::{Mutex, OnceLock, PasteboardContents};

    #[derive(Clone, Default)]
    struct PasteboardState {
        change_count: u64,
        contents: PasteboardContents,
    }

    static STATE: OnceLock<Mutex<PasteboardState>> = OnceLock::new();

    pub(super) fn load_snapshot(contents: &mut PasteboardContents) -> u64 {
        let state = state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *contents = state.contents.clone();
        state.change_count
    }

    pub(super) fn store_snapshot(contents: &PasteboardContents, change_count: u64) -> u64 {
        let mut state = state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.contents = contents.clone();
        state.change_count = change_count;
        state.change_count
    }

    fn state() -> &'static Mutex<PasteboardState> {
        STATE.get_or_init(|| Mutex::new(PasteboardState::default()))
    }
}

#[cfg(all(target_os = "macos", not(test)))]
mod native {
    use super::{
        PasteboardContents, NS_PASTEBOARD_TYPE_PDF, NS_PASTEBOARD_TYPE_PNG,
        NS_PASTEBOARD_TYPE_STRING,
    };
    use objc2_app_kit::NSPasteboard as NativeNSPasteboard;
    use objc2_foundation::{NSData, NSString};

    pub(super) fn refresh_snapshot(contents: &mut PasteboardContents) -> u64 {
        let pasteboard = unsafe { NativeNSPasteboard::generalPasteboard() };
        contents.clear();

        for type_name in [
            NS_PASTEBOARD_TYPE_STRING,
            NS_PASTEBOARD_TYPE_PDF,
            NS_PASTEBOARD_TYPE_PNG,
        ] {
            if let Some(data) = read_type(&pasteboard, type_name) {
                contents.insert(type_name.to_string(), data);
            }
        }

        change_count(&pasteboard)
    }

    pub(super) fn write_snapshot(contents: &PasteboardContents) -> Option<u64> {
        let pasteboard = unsafe { NativeNSPasteboard::generalPasteboard() };
        let _ = unsafe { pasteboard.clearContents() };

        for (type_name, bytes) in contents {
            if !write_type(&pasteboard, type_name, bytes) {
                return None;
            }
        }

        Some(change_count(&pasteboard))
    }

    fn read_type(pasteboard: &NativeNSPasteboard, type_name: &str) -> Option<Vec<u8>> {
        let ns_type = NSString::from_str(type_name);

        if type_name == NS_PASTEBOARD_TYPE_STRING {
            return unsafe { pasteboard.stringForType(&ns_type) }
                .map(|value| value.to_string().into_bytes());
        }

        unsafe { pasteboard.dataForType(&ns_type) }.map(|value| value.bytes().to_vec())
    }

    fn write_type(pasteboard: &NativeNSPasteboard, type_name: &str, bytes: &[u8]) -> bool {
        let ns_type = NSString::from_str(type_name);

        if type_name == NS_PASTEBOARD_TYPE_STRING {
            if let Ok(text) = std::str::from_utf8(bytes) {
                let ns_string = NSString::from_str(text);
                return unsafe { pasteboard.setString_forType(&ns_string, &ns_type) };
            }
        }

        let data = NSData::with_bytes(bytes);
        unsafe { pasteboard.setData_forType(Some(&data), &ns_type) }
    }

    fn change_count(pasteboard: &NativeNSPasteboard) -> u64 {
        u64::try_from(unsafe { pasteboard.changeCount() })
            .ok()
            .unwrap_or(0)
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_pasteboard() -> NSPasteboard {
        shared::store_snapshot(&HashMap::new(), 0);
        NSPasteboard::general()
    }

    #[test]
    fn pasteboard_clear() {
        let mut pb = fresh_pasteboard();
        pb.set_string("hello", NS_PASTEBOARD_TYPE_STRING);
        let cc = pb.clear_contents();
        assert_eq!(cc, 2);
        assert!(pb.types().is_empty());
    }

    #[test]
    fn pasteboard_set_and_get_string() {
        let mut pb = fresh_pasteboard();
        assert!(pb.set_string("clipboard text", NS_PASTEBOARD_TYPE_STRING));
        assert_eq!(
            pb.string_for_type(NS_PASTEBOARD_TYPE_STRING),
            Some("clipboard text")
        );
        assert_eq!(pb.string_for_type("nonexistent"), None);
    }

    #[test]
    fn pasteboard_set_and_get_data() {
        let mut pb = fresh_pasteboard();
        let data = b"\x89PNG\r\n\x1a\n";
        assert!(pb.set_data(data, NS_PASTEBOARD_TYPE_PNG));
        assert_eq!(
            pb.data_for_type(NS_PASTEBOARD_TYPE_PNG),
            Some(data.as_slice())
        );
        assert_eq!(pb.data_for_type("missing"), None);
    }

    #[test]
    fn pasteboard_types() {
        let mut pb = fresh_pasteboard();
        pb.set_string("text", NS_PASTEBOARD_TYPE_STRING);
        pb.set_data(b"pdf", NS_PASTEBOARD_TYPE_PDF);
        let mut types = pb.types();
        types.sort();
        assert_eq!(types, vec!["com.adobe.pdf", "public.utf8-plain-text"]);
    }

    #[test]
    fn pasteboard_change_count_increments() {
        let mut pb = fresh_pasteboard();
        assert_eq!(pb.change_count(), 0);
        pb.clear_contents();
        assert_eq!(pb.change_count(), 1);
        pb.clear_contents();
        assert_eq!(pb.change_count(), 2);
    }

    #[test]
    fn pasteboard_change_count_tracks_writes() {
        let mut pb = fresh_pasteboard();
        assert_eq!(pb.change_count(), 0);

        pb.set_string("hello", NS_PASTEBOARD_TYPE_STRING);
        assert_eq!(pb.change_count(), 1);

        pb.set_data(b"png", NS_PASTEBOARD_TYPE_PNG);
        assert_eq!(pb.change_count(), 2);
    }
}
