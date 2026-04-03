//! CGImage stubs.

use libsystem_shims::table::ShimTable;

static CG_IMAGE_SENTINEL: u64 = 0x4347_494D_0001;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(table,
        ("_CGImageCreate", shim_cg_image_create as u64, "CGImageCreate (stub → sentinel)"),
        ("_CGImageRetain", shim_cg_image_retain as u64, "CGImageRetain (stub → same)"),
        ("_CGImageRelease", shim_cg_image_release as u64, "CGImageRelease (stub → void)"),
        ("_CGImageGetWidth", shim_cg_image_get_zero as u64, "CGImageGetWidth (stub → 0)"),
        ("_CGImageGetHeight", shim_cg_image_get_zero as u64, "CGImageGetHeight (stub → 0)"),
        ("_CGImageGetBitsPerComponent", shim_cg_image_get_zero as u64, "CGImageGetBitsPerComponent (stub → 0)"),
        ("_CGImageGetBitsPerPixel", shim_cg_image_get_zero as u64, "CGImageGetBitsPerPixel (stub → 0)"),
        ("_CGImageGetBytesPerRow", shim_cg_image_get_zero as u64, "CGImageGetBytesPerRow (stub → 0)"),
        ("_CGImageGetColorSpace", shim_cg_image_get_zero as u64, "CGImageGetColorSpace (stub → NULL)"),
        ("_CGImageGetDataProvider", shim_cg_image_get_zero as u64, "CGImageGetDataProvider (stub → NULL)"),
        ("_CGImageCreateWithImageInRect", shim_cg_image_retain as u64, "CGImageCreateWithImageInRect (stub → same)")
    );
}

pub extern "C" fn shim_cg_image_create(
    _w: u64, _h: u64, _bpc: u64, _bpp: u64, _bpr: u64, _cs: u64, _info: u32, _prov: u64,
    _decode: u64, _interp: i32, _intent: i32,
) -> u64 { &CG_IMAGE_SENTINEL as *const u64 as u64 }

pub extern "C" fn shim_cg_image_retain(img: u64) -> u64 { img }
pub extern "C" fn shim_cg_image_release(_img: u64) {}
pub extern "C" fn shim_cg_image_get_zero(_img: u64) -> u64 { 0 }
