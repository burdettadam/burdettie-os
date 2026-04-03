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
    #[allow(dead_code)]
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

/// Remove the metadata entry for an NSImage (cleanup helper).
pub fn nsimage_remove(obj: u64) {
    let mut guard = store();
    if let Some(map) = guard.as_mut() {
        map.remove(&obj);
    }
}

/// Query the stored file path for an NSImage object (used by rendering layer).
#[allow(dead_code)]
pub fn nsimage_file_path(obj: u64) -> Option<String> {
    let guard = store();
    guard
        .as_ref()
        .and_then(|map| map.get(&obj))
        .and_then(|info| info.file_path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // NSImage uses a global HashMap (IMAGE_STORE) keyed by object address,
    // so object layout/ISA doesn't matter for most shims. We use alloc_obj_fake
    // with a minimal 8-byte allocation just to get a unique, valid address.
    const OBJ_SIZE: usize = 8;

    // ── shim_nsimage_image_named ──

    #[test]
    fn image_named_returns_nil() {
        let cls = alloc_obj_fake(OBJ_SIZE);
        let name = make_string("NSApplicationIcon");
        let result = shim_nsimage_image_named(cls, 0, name);
        assert_eq!(result, 0, "imageNamed: stub should always return nil");
        free_obj(name);
        free_obj(cls);
    }

    #[test]
    fn image_named_nil_name() {
        let result = shim_nsimage_image_named(0, 0, 0);
        assert_eq!(result, 0);
    }

    // ── shim_nsimage_init_with_contents_of_file ──

    #[test]
    fn init_with_contents_of_file_returns_self() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let path = make_string("/tmp/test.png");
        let result = shim_nsimage_init_with_contents_of_file(obj, 0, path);
        assert_eq!(result, obj, "initWithContentsOfFile: should return self");
        // Verify the path was stored.
        assert_eq!(nsimage_file_path(obj), Some("/tmp/test.png".to_string()));
        free_obj(path);
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn init_with_contents_of_file_nil_path() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let result = shim_nsimage_init_with_contents_of_file(obj, 0, 0);
        assert_eq!(result, obj, "should still return self even with nil path");
        // Path should be None since extract_cfstring_cstr(0) returns None.
        assert_eq!(nsimage_file_path(obj), None);
        nsimage_remove(obj);
        free_obj(obj);
    }

    // ── shim_nsimage_init_with_data ──

    #[test]
    fn init_with_data_returns_self() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let result = shim_nsimage_init_with_data(obj, 0, 0x1234);
        assert_eq!(result, obj, "initWithData: should return self");
        // No file path for data-based init.
        assert_eq!(nsimage_file_path(obj), None);
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn init_with_data_nil_data() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let result = shim_nsimage_init_with_data(obj, 0, 0);
        assert_eq!(result, obj);
        nsimage_remove(obj);
        free_obj(obj);
    }

    // ── shim_nsimage_size / shim_nsimage_set_size ──

    #[test]
    fn size_default_zero() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        shim_nsimage_init_with_data(obj, 0, 0);
        let w = shim_nsimage_size(obj, 0);
        assert_eq!(w, 0.0, "default size should be 0");
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn set_size_and_read_back() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        shim_nsimage_init_with_data(obj, 0, 0);
        shim_nsimage_set_size(obj, 0, 128.0, 256.0);
        let w = shim_nsimage_size(obj, 0);
        assert_eq!(w, 128.0, "size should return stored width");
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn set_size_creates_entry_if_missing() {
        // set_size on an obj that was never init'd — should create the entry.
        let obj = alloc_obj_fake(OBJ_SIZE);
        shim_nsimage_set_size(obj, 0, 64.0, 32.0);
        let w = shim_nsimage_size(obj, 0);
        assert_eq!(w, 64.0);
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn size_unknown_object_returns_zero() {
        // An obj pointer that was never registered in the store.
        let obj = alloc_obj_fake(OBJ_SIZE);
        let w = shim_nsimage_size(obj, 0);
        assert_eq!(w, 0.0, "unknown object should return 0.0");
        free_obj(obj);
    }

    // ── shim_nsimage_name ──

    #[test]
    fn name_returns_nil() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let result = shim_nsimage_name(obj, 0);
        assert_eq!(result, 0, "name should return nil");
        free_obj(obj);
    }

    #[test]
    fn name_nil_obj() {
        let result = shim_nsimage_name(0, 0);
        assert_eq!(result, 0);
    }

    // ── shim_nsimage_draw_in_rect ──

    #[test]
    fn draw_in_rect_does_not_crash() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        // Just verify no panic.
        shim_nsimage_draw_in_rect(obj, 0, 0.0, 0.0, 100.0, 100.0, 0.0, 0.0, 100.0, 100.0, 0, 1.0);
        free_obj(obj);
    }

    #[test]
    fn draw_in_rect_nil_obj() {
        // Should not crash with nil obj.
        shim_nsimage_draw_in_rect(0, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0, 0.0);
    }

    // ── nsimage_file_path ──

    #[test]
    fn file_path_none_for_unknown() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        assert_eq!(nsimage_file_path(obj), None);
        free_obj(obj);
    }

    #[test]
    fn file_path_none_for_zero() {
        assert_eq!(nsimage_file_path(0), None);
    }
}
