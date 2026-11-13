//! NSBitmapImageRep — concrete image representation backed by pixel data.
//!
//! Object layout: 72 bytes  (extends NSImageRep's 40 bytes)
//!   [0..40):  NSImageRep fields
//!   [40..48): cg_handle     (u64 — CGImage handle in cg-shims IMAGES map)
//!   [48..56): bits_per_sample (u64)
//!   [56..64): samples_per_pixel (u64)
//!   [64..72): has_alpha      (u64 — 0 or 1)

pub(crate) const BITMAP_REP_SIZE: usize = 72;
const CG_HANDLE_OFF: usize = 40;
const BPS_OFF: usize = 48;
const SPP_OFF: usize = 56;
const HAS_ALPHA_OFF: usize = 64;

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

/// `+[NSBitmapImageRep alloc]`
pub extern "C" fn shim_nsbitmapimagerep_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, BITMAP_REP_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsbitmapimagerep_class_addr();
    }
    obj
}

/// `-[NSBitmapImageRep initWithBitmapDataPlanes:pixelsWide:pixelsHigh:
///     bitsPerSample:samplesPerPixel:hasAlpha:isPlanar:
///     colorSpaceName:bytesPerRow:bitsPerPixel:]`
///
/// ABI: planes is ignored (we only support non-planar RGBA).
pub extern "C" fn shim_nsbitmapimagerep_init_planes(
    obj: u64,
    _cmd: u64,
    _planes: u64,           // unsigned char** — ignored
    pixels_wide: u64,       // NSInteger
    pixels_high: u64,       // NSInteger
    bits_per_sample: u64,   // NSInteger (typically 8)
    samples_per_pixel: u64, // NSInteger (3 or 4)
    has_alpha: u64,         // BOOL
    _is_planar: u64,        // BOOL — ignored
    _color_space_name: u64, // NSString — ignored
    _bytes_per_row: u64,    // NSInteger — auto
    _bits_per_pixel: u64,   // NSInteger — auto
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let pw = pixels_wide.max(1);
    let ph = pixels_high.max(1);
    crate::nsimagerep::set_pixel_dims(obj, pw, ph);
    write_u64(obj, BPS_OFF, bits_per_sample);
    write_u64(obj, SPP_OFF, samples_per_pixel);
    write_u64(obj, HAS_ALPHA_OFF, u64::from(has_alpha != 0));

    // Create an empty CGImage of the correct size.
    let total = (pw as usize) * (ph as usize) * 4;
    let data = vec![0u8; total];
    let handle = cg_shims::bridge::image_create(
        pw as u32,
        ph as u32,
        8,               // bits per component
        32,              // bits per pixel
        (pw as u32) * 4, // bytes per row
        data.as_ptr() as u64,
    );
    write_u64(obj, CG_HANDLE_OFF, handle);
    obj
}

/// `-[NSBitmapImageRep initWithCGImage:]`
pub extern "C" fn shim_nsbitmapimagerep_init_with_cgimage(
    obj: u64,
    _cmd: u64,
    cg_image: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    if cg_image == 0 {
        return obj;
    }

    let w = cg_shims::bridge::image_get_width(cg_image);
    let h = cg_shims::bridge::image_get_height(cg_image);
    crate::nsimagerep::set_pixel_dims(obj, w as u64, h as u64);
    write_u64(obj, BPS_OFF, 8);
    write_u64(obj, SPP_OFF, 4);
    write_u64(obj, HAS_ALPHA_OFF, 1);
    write_u64(obj, CG_HANDLE_OFF, cg_image);
    obj
}

/// `-[NSBitmapImageRep initWithData:]`
pub extern "C" fn shim_nsbitmapimagerep_init_with_data(obj: u64, _cmd: u64, data_obj: u64) -> u64 {
    if obj == 0 || data_obj == 0 {
        return 0;
    }

    // Extract bytes from NSData.
    let bytes = crate::nsdata::shim_nsdata_bytes(data_obj, 0);
    let length = crate::nsdata::shim_nsdata_length(data_obj, 0) as usize;
    if bytes == 0 || length == 0 {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes as *const u8, length) };
    let Some(img) = graphics_core::ImageBuffer::from_encoded(slice) else {
        return 0;
    };
    let w = img.width();
    let h = img.height();
    let handle = cg_shims::bridge::image_create_from_buffer(img);
    crate::nsimagerep::set_pixel_dims(obj, w as u64, h as u64);
    write_u64(obj, BPS_OFF, 8);
    write_u64(obj, SPP_OFF, 4);
    write_u64(obj, HAS_ALPHA_OFF, 1);
    write_u64(obj, CG_HANDLE_OFF, handle);
    obj
}

/// `-[NSBitmapImageRep bitmapData]` → returns pointer to RGBA pixel data.
///
/// Returns a live pointer into the CGImage's RGBA8 buffer.
pub extern "C" fn shim_nsbitmapimagerep_bitmap_data(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let handle = read_u64(obj, CG_HANDLE_OFF);
    if handle == 0 {
        return 0;
    }
    cg_shims::bridge::image_data_ptr(handle)
}

/// `-[NSBitmapImageRep CGImage]` → returns the backing CGImage handle.
pub extern "C" fn shim_nsbitmapimagerep_cgimage(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_u64(obj, CG_HANDLE_OFF)
}

/// `-[NSBitmapImageRep bitsPerSample]`
pub extern "C" fn shim_nsbitmapimagerep_bits_per_sample(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, BPS_OFF)
}

/// `-[NSBitmapImageRep samplesPerPixel]`
pub extern "C" fn shim_nsbitmapimagerep_samples_per_pixel(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, SPP_OFF)
}

/// `-[NSBitmapImageRep hasAlpha]`
pub extern "C" fn shim_nsbitmapimagerep_has_alpha(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_u64(obj, HAS_ALPHA_OFF) != 0)
}

/// `-[NSBitmapImageRep isPlanar]` — always NO.
pub extern "C" fn shim_nsbitmapimagerep_is_planar(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[NSBitmapImageRep bytesPerRow]`
pub extern "C" fn shim_nsbitmapimagerep_bytes_per_row(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::nsimagerep::shim_nsimagerep_pixels_wide(obj, 0) * 4
}

/// `-[NSBitmapImageRep bitsPerPixel]`
pub extern "C" fn shim_nsbitmapimagerep_bits_per_pixel(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let spp = read_u64(obj, SPP_OFF);
    let bps = read_u64(obj, BPS_OFF);
    if spp == 0 || bps == 0 {
        32
    } else {
        spp * bps
    }
}

/// `-[NSBitmapImageRep colorSpaceName]` → `NSCalibratedRGBColorSpace`
pub extern "C" fn shim_nsbitmapimagerep_color_space_name(_obj: u64, _cmd: u64) -> u64 {
    crate::cfstring::alloc_nsstring_from_cstr(c"NSCalibratedRGBColorSpace".as_ptr())
}

/// `-[NSBitmapImageRep representationUsingType:properties:]` → stub returning nil.
pub extern "C" fn shim_nsbitmapimagerep_representation(
    _obj: u64,
    _cmd: u64,
    _storage_type: u64,
    _properties: u64,
) -> u64 {
    0
}

/// `-[NSBitmapImageRep TIFFRepresentation]` → stub returning nil.
pub extern "C" fn shim_nsbitmapimagerep_tiff(_obj: u64, _cmd: u64) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn alloc_init_planes() {
        let rep = shim_nsbitmapimagerep_alloc(0, 0);
        let rep = shim_nsbitmapimagerep_init_planes(
            rep, 0, 0,  // planes
            64, // pixels wide
            32, // pixels high
            8,  // bps
            4,  // spp
            1,  // has alpha
            0,  // is planar
            0,  // colorSpaceName
            0,  // bytes per row (auto)
            0,  // bits per pixel (auto)
        );
        assert_ne!(rep, 0);
        assert_eq!(crate::nsimagerep::shim_nsimagerep_pixels_wide(rep, 0), 64);
        assert_eq!(crate::nsimagerep::shim_nsimagerep_pixels_high(rep, 0), 32);
        assert_eq!(shim_nsbitmapimagerep_bits_per_sample(rep, 0), 8);
        assert_eq!(shim_nsbitmapimagerep_samples_per_pixel(rep, 0), 4);
        assert_eq!(shim_nsbitmapimagerep_has_alpha(rep, 0), 1);
        assert_eq!(shim_nsbitmapimagerep_bytes_per_row(rep, 0), 256);
        assert_eq!(shim_nsbitmapimagerep_bits_per_pixel(rep, 0), 32);
        assert_ne!(shim_nsbitmapimagerep_cgimage(rep, 0), 0);
        free_obj(rep);
    }

    #[test]
    fn bitmap_data_points_to_pixels() {
        let rep = shim_nsbitmapimagerep_alloc(0, 0);
        let rep = shim_nsbitmapimagerep_init_planes(rep, 0, 0, 4, 4, 8, 4, 1, 0, 0, 0, 0);
        let data_ptr = shim_nsbitmapimagerep_bitmap_data(rep, 0);
        assert_ne!(data_ptr, 0);
        free_obj(rep);
    }

    #[test]
    fn init_with_cgimage() {
        // Create a small CGImage.
        let pixels = vec![255u8; 8 * 8 * 4]; // 8×8 RGBA
        let handle = cg_shims::bridge::image_create(8, 8, 8, 32, 32, pixels.as_ptr() as u64);
        assert_ne!(handle, 0);

        let rep = shim_nsbitmapimagerep_alloc(0, 0);
        let rep = shim_nsbitmapimagerep_init_with_cgimage(rep, 0, handle);
        assert_ne!(rep, 0);
        assert_eq!(crate::nsimagerep::shim_nsimagerep_pixels_wide(rep, 0), 8);
        assert_eq!(crate::nsimagerep::shim_nsimagerep_pixels_high(rep, 0), 8);
        assert_eq!(shim_nsbitmapimagerep_cgimage(rep, 0), handle);
        free_obj(rep);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(
            shim_nsbitmapimagerep_init_planes(0, 0, 0, 4, 4, 8, 4, 1, 0, 0, 0, 0),
            0
        );
        assert_eq!(shim_nsbitmapimagerep_bitmap_data(0, 0), 0);
        assert_eq!(shim_nsbitmapimagerep_cgimage(0, 0), 0);
        assert_eq!(shim_nsbitmapimagerep_has_alpha(0, 0), 0);
    }

    #[test]
    fn is_planar_always_false() {
        let rep = shim_nsbitmapimagerep_alloc(0, 0);
        let rep = shim_nsbitmapimagerep_init_planes(rep, 0, 0, 4, 4, 8, 4, 1, 0, 0, 0, 0);
        assert_eq!(shim_nsbitmapimagerep_is_planar(rep, 0), 0);
        free_obj(rep);
    }
}
