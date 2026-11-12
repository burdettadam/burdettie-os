//! NSOpenGL shims: NSOpenGLPixelFormat and NSOpenGLContext.
//!
//! These classes are used by GPU-backed macOS apps during early view/context
//! setup.  Burdettie does not provide a real OpenGL implementation yet, but
//! minimal stateful shims allow apps to progress through pixel-format/context
//! creation instead of failing on missing native classes.
//!
//! ## NSOpenGLPixelFormat object layout (16 bytes)
//!
//!   offset  0: isa                 (u64)
//!   offset  8: attributes_ptr      (u64 — raw attribute array pointer)
//!
//! ## NSOpenGLContext object layout (32 bytes)
//!
//!   offset  0: isa                 (u64)
//!   offset  8: pixel_format        (u64 — NSOpenGLPixelFormat *)
//!   offset 16: share_context       (u64 — NSOpenGLContext *)
//!   offset 24: view                (u64 — NSView *)

use std::cell::Cell;

const PIXEL_FORMAT_ATTRS_OFF: usize = 8;
const PIXEL_FORMAT_SIZE: usize = 16;

const CONTEXT_PIXEL_FORMAT_OFF: usize = 8;
const CONTEXT_SHARE_CONTEXT_OFF: usize = 16;
const CONTEXT_VIEW_OFF: usize = 24;
const CONTEXT_SIZE: usize = 32;

thread_local! {
    static CURRENT_CONTEXT: Cell<u64> = const { Cell::new(0) };
}

pub(crate) fn reset_opengl_state() {
    CURRENT_CONTEXT.with(|current| current.set(0));
}

/// `-[NSOpenGLPixelFormat initWithAttributes:]`
pub extern "C" fn shim_nsopenglpixelformat_init_with_attributes(
    obj: u64,
    _cmd: u64,
    attrs: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + PIXEL_FORMAT_ATTRS_OFF) as *mut u64) = attrs;
    }
    obj
}

/// `-[NSOpenGLContext initWithFormat:shareContext:]`
pub extern "C" fn shim_nsopenglcontext_init_with_format_share_context(
    obj: u64,
    _cmd: u64,
    pixel_format: u64,
    share_context: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + CONTEXT_PIXEL_FORMAT_OFF) as *mut u64) = pixel_format;
        *((obj as usize + CONTEXT_SHARE_CONTEXT_OFF) as *mut u64) = share_context;
    }
    obj
}

/// `-[NSOpenGLContext pixelFormat]`
pub extern "C" fn shim_nsopenglcontext_pixel_format(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CONTEXT_PIXEL_FORMAT_OFF) as *const u64) }
}

/// `-[NSOpenGLContext setPixelFormat:]`
pub extern "C" fn shim_nsopenglcontext_set_pixel_format(obj: u64, _cmd: u64, pixel_format: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + CONTEXT_PIXEL_FORMAT_OFF) as *mut u64) = pixel_format;
    }
}

/// `-[NSOpenGLContext view]`
pub extern "C" fn shim_nsopenglcontext_view(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CONTEXT_VIEW_OFF) as *const u64) }
}

/// `-[NSOpenGLContext setView:]`
pub extern "C" fn shim_nsopenglcontext_set_view(obj: u64, _cmd: u64, view: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + CONTEXT_VIEW_OFF) as *mut u64) = view;
    }
}

/// `-[NSOpenGLContext makeCurrentContext]`
pub extern "C" fn shim_nsopenglcontext_make_current_context(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    CURRENT_CONTEXT.with(|current| current.set(obj));
}

/// `+[NSOpenGLContext currentContext]`
pub extern "C" fn shim_nsopenglcontext_current_context(_cls: u64, _cmd: u64) -> u64 {
    CURRENT_CONTEXT.with(Cell::get)
}

/// `+[NSOpenGLContext clearCurrentContext]`
pub extern "C" fn shim_nsopenglcontext_clear_current_context(_cls: u64, _cmd: u64) {
    CURRENT_CONTEXT.with(|current| current.set(0));
}

/// `-[NSOpenGLContext update]`
pub extern "C" fn shim_nsopenglcontext_update(_obj: u64, _cmd: u64) {}

/// `-[NSOpenGLContext flushBuffer]`
pub extern "C" fn shim_nsopenglcontext_flush_buffer(_obj: u64, _cmd: u64) {}

/// `-[NSOpenGLContext setValues:forParameter:]`
pub extern "C" fn shim_nsopenglcontext_set_values_for_parameter(
    _obj: u64,
    _cmd: u64,
    _values: u64,
    _parameter: i32,
) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsobj;
    use crate::shim_test_helpers::*;

    #[test]
    fn pixel_format_init_stores_attribute_pointer() {
        let obj = alloc_obj(PIXEL_FORMAT_SIZE, nsobj::nsopenglpixelformat_class_addr());
        let ret = shim_nsopenglpixelformat_init_with_attributes(obj, 0, 0x1234);
        assert_eq!(ret, obj);
        assert_eq!(read_field(obj, PIXEL_FORMAT_ATTRS_OFF), 0x1234);
        free_obj(obj);
    }

    #[test]
    fn context_init_stores_format_and_share_context() {
        let obj = alloc_obj(CONTEXT_SIZE, nsobj::nsopenglcontext_class_addr());
        let ret = shim_nsopenglcontext_init_with_format_share_context(obj, 0, 0x1111, 0x2222);
        assert_eq!(ret, obj);
        assert_eq!(read_field(obj, CONTEXT_PIXEL_FORMAT_OFF), 0x1111);
        assert_eq!(read_field(obj, CONTEXT_SHARE_CONTEXT_OFF), 0x2222);
        free_obj(obj);
    }

    #[test]
    fn context_setters_update_stored_state() {
        let obj = alloc_obj(CONTEXT_SIZE, nsobj::nsopenglcontext_class_addr());
        shim_nsopenglcontext_set_pixel_format(obj, 0, 0xAAAA);
        shim_nsopenglcontext_set_view(obj, 0, 0xBBBB);
        assert_eq!(shim_nsopenglcontext_pixel_format(obj, 0), 0xAAAA);
        assert_eq!(shim_nsopenglcontext_view(obj, 0), 0xBBBB);
        free_obj(obj);
    }

    #[test]
    fn current_context_round_trip_is_thread_local() {
        std::thread::spawn(|| {
            reset_opengl_state();
            let obj = alloc_obj(CONTEXT_SIZE, nsobj::nsopenglcontext_class_addr());
            assert_eq!(shim_nsopenglcontext_current_context(0, 0), 0);
            shim_nsopenglcontext_make_current_context(obj, 0);
            assert_eq!(shim_nsopenglcontext_current_context(0, 0), obj);
            shim_nsopenglcontext_clear_current_context(0, 0);
            assert_eq!(shim_nsopenglcontext_current_context(0, 0), 0);
            free_obj(obj);
        })
        .join()
        .unwrap();
    }
}
