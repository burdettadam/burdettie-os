//! NSImageRep — abstract base for image representation classes.
//!
//! Object layout: 40 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  pixels_wide  (u64)
//!   [16..24): pixels_high  (u64)
//!   [24..32): width        (f64 — points)
//!   [32..40): height       (f64 — points)

use crate::cg_types::CGSize;

const SIZE: usize = 40;
const PIXELS_WIDE_OFF: usize = 8;
const PIXELS_HIGH_OFF: usize = 16;
const WIDTH_OFF: usize = 24;
const HEIGHT_OFF: usize = 32;

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

fn read_f64(obj: u64, offset: usize) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as *const u8).add(offset) as *const f64) }
}

fn write_f64(obj: u64, offset: usize, val: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u8).add(offset) as *mut f64) = val;
    }
}

pub(crate) const IMAGE_REP_SIZE: usize = SIZE;

/// `+[NSImageRep alloc]`
pub extern "C" fn shim_nsimagerep_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsimagerep_class_addr();
    }
    obj
}

/// `-[NSImageRep init]`
pub extern "C" fn shim_nsimagerep_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    obj
}

/// `-[NSImageRep pixelsWide]`
pub extern "C" fn shim_nsimagerep_pixels_wide(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, PIXELS_WIDE_OFF)
}

/// `-[NSImageRep setPixelsWide:]`
pub extern "C" fn shim_nsimagerep_set_pixels_wide(obj: u64, _cmd: u64, w: u64) {
    write_u64(obj, PIXELS_WIDE_OFF, w);
}

/// `-[NSImageRep pixelsHigh]`
pub extern "C" fn shim_nsimagerep_pixels_high(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, PIXELS_HIGH_OFF)
}

/// `-[NSImageRep setPixelsHigh:]`
pub extern "C" fn shim_nsimagerep_set_pixels_high(obj: u64, _cmd: u64, h: u64) {
    write_u64(obj, PIXELS_HIGH_OFF, h);
}

/// `-[NSImageRep size]` — returns the point dimensions as a C-compatible CGSize.
pub extern "C" fn shim_nsimagerep_size(obj: u64, _cmd: u64) -> CGSize {
    if obj == 0 {
        return CGSize::ZERO;
    }
    CGSize {
        width: read_f64(obj, WIDTH_OFF),
        height: read_f64(obj, HEIGHT_OFF),
    }
}

/// `-[NSImageRep setSize:]`
pub extern "C" fn shim_nsimagerep_set_size(obj: u64, _cmd: u64, w: f64, h: f64) {
    write_f64(obj, WIDTH_OFF, w);
    write_f64(obj, HEIGHT_OFF, h);
}

/// Helper: set pixel dimensions and default point size (1:1 mapping).
pub(crate) fn set_pixel_dims(obj: u64, pw: u64, ph: u64) {
    write_u64(obj, PIXELS_WIDE_OFF, pw);
    write_u64(obj, PIXELS_HIGH_OFF, ph);
    write_f64(obj, WIDTH_OFF, pw as f64);
    write_f64(obj, HEIGHT_OFF, ph as f64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn alloc_init() {
        let rep = shim_nsimagerep_alloc(0, 0);
        let rep = shim_nsimagerep_init(rep, 0);
        assert_ne!(rep, 0);
        assert_eq!(read_u64(rep, 0), crate::nsobj::nsimagerep_class_addr());
        assert_eq!(shim_nsimagerep_pixels_wide(rep, 0), 0);
        assert_eq!(shim_nsimagerep_pixels_high(rep, 0), 0);
        free_obj(rep);
    }

    #[test]
    fn pixel_dims_and_size() {
        let rep = shim_nsimagerep_alloc(0, 0);
        let rep = shim_nsimagerep_init(rep, 0);
        set_pixel_dims(rep, 100, 200);
        assert_eq!(shim_nsimagerep_pixels_wide(rep, 0), 100);
        assert_eq!(shim_nsimagerep_pixels_high(rep, 0), 200);
        let size: extern "C" fn(u64, u64) -> CGSize = shim_nsimagerep_size;
        assert_eq!(
            size(rep, 0),
            CGSize {
                width: 100.0,
                height: 200.0
            }
        );
        free_obj(rep);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsimagerep_init(0, 0), 0);
        assert_eq!(shim_nsimagerep_pixels_wide(0, 0), 0);
        assert_eq!(shim_nsimagerep_pixels_high(0, 0), 0);
        assert_eq!(shim_nsimagerep_size(0, 0), CGSize::ZERO);
    }
}
