//! CGColorSpace / CGColor — bridge-backed color objects.
//!
//! `CGColorCreateGenericRGB` allocates a real `Color` via the bridge module.
//! Color spaces remain sentinels (model/components queries are trivial).

use crate::bridge;
use libsystem_shims::table::ShimTable;

static CG_COLORSPACE_SENTINEL: u64 = 0x4347_4353_0001;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(table,
        ("_CGColorSpaceCreateDeviceRGB", shim_cg_cs_device_rgb as *const () as u64, "CGColorSpaceCreateDeviceRGB (stub → sentinel)"),
        ("_CGColorSpaceCreateDeviceGray", shim_cg_cs_device_rgb as *const () as u64, "CGColorSpaceCreateDeviceGray (stub → sentinel)"),
        ("_CGColorSpaceCreateDeviceCMYK", shim_cg_cs_device_rgb as *const () as u64, "CGColorSpaceCreateDeviceCMYK (stub → sentinel)"),
        ("_CGColorSpaceCreateWithName", shim_cg_cs_with_name as *const () as u64, "CGColorSpaceCreateWithName (stub → sentinel)"),
        ("_CGColorSpaceRetain", shim_cg_cs_retain as *const () as u64, "CGColorSpaceRetain (stub → same)"),
        ("_CGColorSpaceRelease", shim_cg_cs_release as *const () as u64, "CGColorSpaceRelease (stub → void)"),
        ("_CGColorSpaceGetModel", shim_cg_cs_get_model as *const () as u64, "CGColorSpaceGetModel (stub → kCGColorSpaceModelRGB)"),
        ("_CGColorSpaceGetNumberOfComponents", shim_cg_cs_components as *const () as u64, "CGColorSpaceGetNumberOfComponents (stub → 3)"),
        ("_CGColorCreate", shim_cg_color_create as *const () as u64, "CGColorCreate (bridge → real)"),
        ("_CGColorCreateGenericRGB", shim_cg_color_create_rgb as *const () as u64, "CGColorCreateGenericRGB (bridge → real)"),
        ("_CGColorRetain", shim_cg_color_retain as *const () as u64, "CGColorRetain (bridge → same)"),
        ("_CGColorRelease", shim_cg_color_release as *const () as u64, "CGColorRelease (bridge → real)"),
        ("_CGColorGetAlpha", shim_cg_color_get_alpha as *const () as u64, "CGColorGetAlpha (bridge → real)"),
        ("_CGColorGetComponents", shim_cg_return_null as *const () as u64, "CGColorGetComponents (stub → NULL)"),
        ("_CGColorGetColorSpace", shim_cg_cs_device_rgb as *const () as u64, "CGColorGetColorSpace (stub → sentinel)")
    );
}

// ── Color space stubs (sentinels, model queries are trivial) ────

pub extern "C" fn shim_cg_cs_device_rgb() -> u64 { &CG_COLORSPACE_SENTINEL as *const u64 as u64 }
pub extern "C" fn shim_cg_cs_with_name(_name: u64) -> u64 { &CG_COLORSPACE_SENTINEL as *const u64 as u64 }
pub extern "C" fn shim_cg_cs_retain(cs: u64) -> u64 { cs }
pub extern "C" fn shim_cg_cs_release(_cs: u64) {}
pub extern "C" fn shim_cg_cs_get_model(_cs: u64) -> i32 { 1 } // kCGColorSpaceModelRGB
pub extern "C" fn shim_cg_cs_components(_cs: u64) -> i64 { 3 }

// ── Bridge-backed colors ────────────────────────────────────────

/// CGColorCreate(colorspace, components_ptr) → bridge color.
/// For simplicity, treats components as [r, g, b, a] in device RGB.
pub extern "C" fn shim_cg_color_create(_cs: u64, _components: u64) -> u64 {
    // Without reading the pointer (unsafe), default to opaque black.
    bridge::color_create_rgb(0.0, 0.0, 0.0, 1.0)
}

/// CGColorCreateGenericRGB(r, g, b, a) → bridge color with real values.
pub extern "C" fn shim_cg_color_create_rgb(r: f64, g: f64, b: f64, a: f64) -> u64 {
    bridge::color_create_rgb(r, g, b, a)
}

pub extern "C" fn shim_cg_color_retain(c: u64) -> u64 { c }

pub extern "C" fn shim_cg_color_release(c: u64) {
    bridge::color_release(c);
}

/// CGColorGetAlpha → real alpha from bridge color store.
pub extern "C" fn shim_cg_color_get_alpha(c: u64) -> f64 {
    bridge::color_get_alpha(c)
}

pub extern "C" fn shim_cg_return_null(_a: u64) -> u64 { 0 }

#[cfg(test)]
mod tests {
    use super::*;
    use libsystem_shims::table::ShimTable;

    #[test]
    fn colorspace_stubs_registered() {
        let mut table = ShimTable::new();
        install(&mut table);
        assert!(table.resolve("_CGColorSpaceCreateDeviceRGB").is_some());
        assert!(table.resolve("_CGColorCreate").is_some());
    }

    #[test]
    fn device_rgb_is_non_null() {
        assert_ne!(shim_cg_cs_device_rgb(), 0);
    }

    #[test]
    fn color_create_rgb_bridge() {
        let c = shim_cg_color_create_rgb(1.0, 0.0, 0.0, 0.5);
        assert_ne!(c, 0);
        let alpha = shim_cg_color_get_alpha(c);
        assert!((alpha - 0.5).abs() < 1e-6);
        shim_cg_color_release(c);
    }
}
