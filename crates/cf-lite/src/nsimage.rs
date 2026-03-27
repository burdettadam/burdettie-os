//! NSImage shim: image factory and instance methods for AppKit.
//!
//! Maintains a backing store of image metadata (file path, dimensions)
//! keyed by object pointer so that NSImage objects can be created,
//! passed around, and queried for size.

use std::collections::HashMap;
use std::sync::Mutex;

/// Per-object metadata for allocated NSImage instances.
struct ImageInfo {
    /// Original file path (if loaded from file).
    file_path: Option<String>,
    width: f64,
    height: f64,
}

static IMAGE_STORE: Mutex<Option<HashMap<u64, ImageInfo>>> = Mutex::new(None);

fn store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, ImageInfo>>> {
    IMAGE_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

fn ensure_store(guard: &mut Option<HashMap<u64, ImageInfo>>) -> &mut HashMap<u64, ImageInfo> {
    guard.get_or_insert_with(HashMap::new)
}

/// `+[NSImage imageNamed:]` — stub, returns nil for named images.
pub extern "C" fn shim_nsimage_image_named(_cls: u64, _cmd: u64, _name: u64) -> u64 {
    log::debug!("NSImage imageNamed: (stub, returning nil)");
    0
}

/// `-[NSImage initWithContentsOfFile:]` — records the file path and returns self.
pub extern "C" fn shim_nsimage_init_with_contents_of_file(
    obj: u64,
    _cmd: u64,
    path_ptr: u64,
) -> u64 {
    // Extract NSString content from the runtime if possible.
    let path_str = crate::cfstring::extract_cfstring_cstr(path_ptr).map(|s| s.to_string());
    log::debug!("NSImage initWithContentsOfFile: {:?}", path_str);

    let mut guard = store();
    let map = ensure_store(&mut guard);
    map.insert(
        obj,
        ImageInfo {
            file_path: path_str,
            width: 0.0,
            height: 0.0,
        },
    );
    obj
}

/// `-[NSImage initWithData:]` — records allocation, returns self.
pub extern "C" fn shim_nsimage_init_with_data(obj: u64, _cmd: u64, _data: u64) -> u64 {
    log::debug!("NSImage initWithData: (stub)");
    let mut guard = store();
    let map = ensure_store(&mut guard);
    map.insert(
        obj,
        ImageInfo {
            file_path: None,
            width: 0.0,
            height: 0.0,
        },
    );
    obj
}

/// `-[NSImage size]` — returns stored width (compact struct return).
pub extern "C" fn shim_nsimage_size(_obj: u64, _cmd: u64) -> f64 {
    let guard = store();
    if let Some(map) = guard.as_ref() {
        if let Some(info) = map.get(&_obj) {
            return info.width;
        }
    }
    0.0
}

/// `-[NSImage setSize:]` — stores dimensions.
pub extern "C" fn shim_nsimage_set_size(obj: u64, _cmd: u64, w: f64, h: f64) {
    log::debug!("NSImage setSize: {}×{}", w, h);
    let mut guard = store();
    let map = ensure_store(&mut guard);
    if let Some(info) = map.get_mut(&obj) {
        info.width = w;
        info.height = h;
    } else {
        map.insert(
            obj,
            ImageInfo {
                file_path: None,
                width: w,
                height: h,
            },
        );
    }
}

/// `-[NSImage name]` — returns nil.
pub extern "C" fn shim_nsimage_name(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSImage drawInRect:fromRect:operation:fraction:]` — no-op stub.
pub extern "C" fn shim_nsimage_draw_in_rect(
    _obj: u64,
    _cmd: u64,
    _dx: f64,
    _dy: f64,
    _dw: f64,
    _dh: f64,
    _sx: f64,
    _sy: f64,
    _sw: f64,
    _sh: f64,
    _op: u64,
    _fraction: f64,
) {
    log::debug!("NSImage drawInRect:fromRect:operation:fraction: (stub)");
}

/// Query the stored file path for an NSImage object (used by rendering layer).
pub fn nsimage_file_path(obj: u64) -> Option<String> {
    let guard = store();
    guard
        .as_ref()
        .and_then(|map| map.get(&obj))
        .and_then(|info| info.file_path.clone())
}
