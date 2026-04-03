//! NSScreen shim: screen geometry queries.
//!
//! Object layout (singleton):
//!
//!   offset  0: isa    (u64)
//!   offset  8: width  (f64)
//!   offset 16: height (f64)
//!
//! Total: 24 bytes.
//!
//! On macOS, queries the real main display size via CoreGraphics FFI.
//! On Linux, returns a sensible default (1920×1080) or reads
//! `$DISPLAY_WIDTH` / `$DISPLAY_HEIGHT` env vars if set.

use std::sync::OnceLock;

const WIDTH_OFF: usize = 8;
const HEIGHT_OFF: usize = 16;

static MAIN_SCREEN: OnceLock<u64> = OnceLock::new();

/// Query real display dimensions from the operating system.
fn query_display_size() -> (f64, f64) {
    #[cfg(target_os = "macos")]
    {
        // Dynamically look up CoreGraphics symbols to avoid link-time dependency
        type CGMainDisplayIDFn = unsafe extern "C" fn() -> u32;
        type CGDisplayPixelsFn = unsafe extern "C" fn(u32) -> u64;

        unsafe {
            let cg = libc::dlopen(
                b"/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics\0".as_ptr() as *const _,
                libc::RTLD_LAZY,
            );
            if !cg.is_null() {
                let main_id: Option<CGMainDisplayIDFn> = std::mem::transmute(
                    libc::dlsym(cg, b"CGMainDisplayID\0".as_ptr() as *const _),
                );
                let pixels_wide: Option<CGDisplayPixelsFn> = std::mem::transmute(
                    libc::dlsym(cg, b"CGDisplayPixelsWide\0".as_ptr() as *const _),
                );
                let pixels_high: Option<CGDisplayPixelsFn> = std::mem::transmute(
                    libc::dlsym(cg, b"CGDisplayPixelsHigh\0".as_ptr() as *const _),
                );
                if let (Some(mid), Some(pw), Some(ph)) = (main_id, pixels_wide, pixels_high) {
                    let display = mid();
                    let w = pw(display) as f64;
                    let h = ph(display) as f64;
                    libc::dlclose(cg);
                    if w > 0.0 && h > 0.0 {
                        return (w, h);
                    }
                }
                libc::dlclose(cg);
            }
        }
    }

    // Fallback: check env vars, then default to 1920×1080
    let w = std::env::var("DISPLAY_WIDTH")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1920.0);
    let h = std::env::var("DISPLAY_HEIGHT")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1080.0);
    (w, h)
}

fn get_or_create_main_screen() -> u64 {
    *MAIN_SCREEN.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 24) as u64;
        if obj == 0 { return 0; }
        let (w, h) = query_display_size();
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsscreen_class_addr();
            *((obj as usize + WIDTH_OFF) as *mut f64) = w;
            *((obj as usize + HEIGHT_OFF) as *mut f64) = h;
        }
        obj
    })
}

/// `+[NSScreen mainScreen]`
pub extern "C" fn shim_nsscreen_main_screen(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_main_screen()
}

/// `+[NSScreen screens]` — returns array with main screen.
pub extern "C" fn shim_nsscreen_screens(_cls: u64, _cmd: u64) -> u64 {
    let main = get_or_create_main_screen();
    crate::nsarray::make_nsarray(&[main])
}

/// `-[NSScreen frame]` — returns CGRect via indirect struct return.
/// ARM64 ABI passes x8 as the indirect result pointer for large structs.
/// For simplicity, return the width (first component callers typically use).
pub extern "C" fn shim_nsscreen_frame(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + WIDTH_OFF) as *const f64) }
}

/// `-[NSScreen visibleFrame]` — same as frame minus menu bar height (25pt).
pub extern "C" fn shim_nsscreen_visible_frame(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + WIDTH_OFF) as *const f64) }
}

/// `-[NSScreen backingScaleFactor]` — returns 2.0 (Retina) on macOS, 1.0 elsewhere.
pub extern "C" fn shim_nsscreen_backing_scale_factor(_obj: u64, _cmd: u64) -> f64 {
    #[cfg(target_os = "macos")]
    { 2.0 }
    #[cfg(not(target_os = "macos"))]
    { 1.0 }
}

/// `-[NSScreen deviceDescription]` — returns empty NSDictionary.
pub extern "C" fn shim_nsscreen_device_description(_obj: u64, _cmd: u64) -> u64 {
    crate::nsdictionary::make_nsdictionary(&[], &[])
}

/// Helper: read the stored width for a screen object.
pub fn nsscreen_width(obj: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + WIDTH_OFF) as *const f64) }
}

/// Helper: read the stored height for a screen object.
pub fn nsscreen_height(obj: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + HEIGHT_OFF) as *const f64) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // --- shim_nsscreen_main_screen ---

    #[test]
    fn main_screen_returns_nonzero() {
        let screen = shim_nsscreen_main_screen(0, 0);
        assert_ne!(screen, 0, "mainScreen must return a valid object");
    }

    #[test]
    fn main_screen_returns_same_singleton() {
        let a = shim_nsscreen_main_screen(0, 0);
        let b = shim_nsscreen_main_screen(0, 0);
        assert_eq!(a, b, "mainScreen should return the same singleton");
    }

    #[test]
    fn main_screen_has_nsscreen_isa() {
        let screen = shim_nsscreen_main_screen(0, 0);
        let isa = read_field(screen, 0);
        assert_eq!(isa, crate::nsobj::nsscreen_class_addr());
    }

    // --- shim_nsscreen_screens ---

    #[test]
    fn screens_returns_nonzero_array() {
        let arr = shim_nsscreen_screens(0, 0);
        assert_ne!(arr, 0, "screens must return a valid NSArray");
        // Array count should be 1
        let count = read_field(arr, 8); // NSARRAY_COUNT_OFF = 8
        assert_eq!(count, 1, "screens array should contain exactly 1 element");
    }

    #[test]
    fn screens_contains_main_screen() {
        let arr = shim_nsscreen_screens(0, 0);
        let main = shim_nsscreen_main_screen(0, 0);
        // Read element pointer buffer
        let elems_ptr = read_field(arr, 16); // NSARRAY_ELEMS_OFF = 16
        assert_ne!(elems_ptr, 0);
        let first_elem = unsafe { *(elems_ptr as *const u64) };
        assert_eq!(first_elem, main, "first element should be the main screen singleton");
    }

    // --- shim_nsscreen_frame ---

    #[test]
    fn frame_returns_positive_width() {
        let screen = shim_nsscreen_main_screen(0, 0);
        let val = shim_nsscreen_frame(screen, 0);
        assert!(val > 0.0, "frame width should be positive, got {val}");
    }

    #[test]
    fn frame_nil_safety() {
        let val = shim_nsscreen_frame(0, 0);
        assert_eq!(val, 0.0);
    }

    // --- shim_nsscreen_visible_frame ---

    #[test]
    fn visible_frame_returns_positive_width() {
        let screen = shim_nsscreen_main_screen(0, 0);
        let val = shim_nsscreen_visible_frame(screen, 0);
        assert!(val > 0.0, "visible frame width should be positive, got {val}");
    }

    #[test]
    fn visible_frame_nil_safety() {
        let val = shim_nsscreen_visible_frame(0, 0);
        assert_eq!(val, 0.0);
    }

    // --- nsscreen_width / nsscreen_height ---

    #[test]
    fn screen_dimensions_positive() {
        let screen = shim_nsscreen_main_screen(0, 0);
        assert!(nsscreen_width(screen) > 0.0);
        assert!(nsscreen_height(screen) > 0.0);
    }

    // --- backing scale factor ---

    #[test]
    fn backing_scale_factor_positive() {
        let screen = shim_nsscreen_main_screen(0, 0);
        let scale = shim_nsscreen_backing_scale_factor(screen, 0);
        assert!(scale >= 1.0);
    }
}
