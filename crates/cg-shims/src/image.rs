//! CGImage shims — handle-based image objects backed by `ImageBuffer`.

use crate::bridge;
use libsystem_shims::table::ShimTable;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(table,
        ("_CGImageCreate", shim_cg_image_create as *const () as u64, "CGImageCreate (bridge → real)"),
        ("_CGImageRetain", shim_cg_image_retain as *const () as u64, "CGImageRetain (bridge → same)"),
        ("_CGImageRelease", shim_cg_image_release as *const () as u64, "CGImageRelease (bridge → real)"),
        ("_CGImageGetWidth", shim_cg_image_get_width as *const () as u64, "CGImageGetWidth (bridge → real)"),
        ("_CGImageGetHeight", shim_cg_image_get_height as *const () as u64, "CGImageGetHeight (bridge → real)"),
        ("_CGImageGetBitsPerComponent", shim_cg_image_get_bpc as *const () as u64, "CGImageGetBitsPerComponent (stub → 8)"),
        ("_CGImageGetBitsPerPixel", shim_cg_image_get_bpp as *const () as u64, "CGImageGetBitsPerPixel (stub → 32)"),
        ("_CGImageGetBytesPerRow", shim_cg_image_get_bpr as *const () as u64, "CGImageGetBytesPerRow (bridge → real)"),
        ("_CGImageGetColorSpace", shim_cg_image_get_zero as *const () as u64, "CGImageGetColorSpace (stub → NULL)"),
        ("_CGImageGetDataProvider", shim_cg_image_get_zero as *const () as u64, "CGImageGetDataProvider (stub → NULL)"),
        ("_CGImageCreateWithImageInRect", shim_cg_image_retain as *const () as u64, "CGImageCreateWithImageInRect (stub → same)")
    );
}

/// CGImageCreate(w, h, bpc, bpp, bpr, cs, info, provider, decode, interp, intent) → handle
pub extern "C" fn shim_cg_image_create(
    w: u64, h: u64, bpc: u64, bpp: u64, bpr: u64, _cs: u64, _info: u32, provider: u64,
    _decode: u64, _interp: i32, _intent: i32,
) -> u64 {
    // `provider` is typically a CGDataProvider handle pointing to pixel data.
    // For now, treat it as a raw pointer to the pixel data if non-zero.
    bridge::image_create(w as u32, h as u32, bpc as u32, bpp as u32, bpr as u32, provider)
}

pub extern "C" fn shim_cg_image_retain(img: u64) -> u64 { img }

pub extern "C" fn shim_cg_image_release(img: u64) {
    bridge::image_release(img);
}

pub extern "C" fn shim_cg_image_get_width(img: u64) -> u64 {
    bridge::image_get_width(img) as u64
}

pub extern "C" fn shim_cg_image_get_height(img: u64) -> u64 {
    bridge::image_get_height(img) as u64
}

pub extern "C" fn shim_cg_image_get_bpc(_img: u64) -> u64 { 8 }
pub extern "C" fn shim_cg_image_get_bpp(_img: u64) -> u64 { 32 }

pub extern "C" fn shim_cg_image_get_bpr(img: u64) -> u64 {
    bridge::image_get_width(img) as u64 * 4
}

pub extern "C" fn shim_cg_image_get_zero(_img: u64) -> u64 { 0 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_stubs_registered() {
        let mut table = libsystem_shims::table::ShimTable::new();
        install(&mut table);
        assert!(table.resolve("_CGImageCreate").is_some());
        assert!(table.resolve("_CGImageGetWidth").is_some());
    }
}
