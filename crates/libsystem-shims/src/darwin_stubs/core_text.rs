//! CoreText framework stubs.
//!
//! macOS text rendering API. On Linux there is no CoreText, so we provide
//! handle-based stubs that store lightweight `CtFont` records in a global
//! registry.  Font metrics are derived from the requested size so callers
//! that probe metrics get plausible proportional values.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::table::ShimTable;

// ---------------------------------------------------------------------------
// Handle allocator (shared across CTFont / CTFontDescriptor / CTParagraphStyle / CTLine)
// ---------------------------------------------------------------------------

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn new_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// CtFont registry
// ---------------------------------------------------------------------------

struct CtFont {
    #[allow(dead_code)]
    name: String,
    size: f64,
    ascent: f64,
    descent: f64,
    leading: f64,
    x_height: f64,
    cap_height: f64,
    units_per_em: u32,
}

impl CtFont {
    fn new(name: String, size: f64) -> Self {
        Self {
            name,
            size,
            ascent: size * 0.8,
            descent: size * 0.2,
            leading: size * 0.1,
            x_height: size * 0.5,
            cap_height: size * 0.7,
            units_per_em: 2048,
        }
    }
}

static FONT_REGISTRY: Mutex<Option<HashMap<u64, CtFont>>> = Mutex::new(None);

fn with_registry<R>(f: impl FnOnce(&mut HashMap<u64, CtFont>) -> R) -> R {
    let mut guard = FONT_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

fn lookup_font<R>(handle: u64, f: impl FnOnce(&CtFont) -> R, default: R) -> R {
    with_registry(|map| map.get(&handle).map_or(default, f))
}

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

pub struct CoreTextStubs;

impl CoreTextStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            // -- CTFontManager (existing, still NULL stubs) --
            ("_CTFontManagerCopyAvailableFontFamilyNames", shim_ct_null_stub as *const () as u64, "CTFontManagerCopyAvailableFontFamilyNames (stub → NULL)"),
            ("_CTFontManagerCopyAvailablePostScriptNames", shim_ct_null_stub as *const () as u64, "CTFontManagerCopyAvailablePostScriptNames (stub → NULL)"),

            // -- CTFont creation --
            ("_CTFontCreateWithName", shim_ct_font_create_with_name as *const () as u64, "CTFontCreateWithName (handle-based)"),
            ("_CTFontCopyFullName", shim_ct_font_copy_full_name as *const () as u64, "CTFontCopyFullName (stub → NULL)"),
            ("_CTFontGetGlyphCount", shim_ct_zero_stub as *const () as u64, "CTFontGetGlyphCount (stub → 0)"),

            // -- CTFont metrics --
            ("_CTFontGetAscent", shim_ct_font_get_ascent as *const () as u64, "CTFontGetAscent"),
            ("_CTFontGetDescent", shim_ct_font_get_descent as *const () as u64, "CTFontGetDescent"),
            ("_CTFontGetLeading", shim_ct_font_get_leading as *const () as u64, "CTFontGetLeading"),
            ("_CTFontGetXHeight", shim_ct_font_get_x_height as *const () as u64, "CTFontGetXHeight"),
            ("_CTFontGetCapHeight", shim_ct_font_get_cap_height as *const () as u64, "CTFontGetCapHeight"),
            ("_CTFontGetUnitsPerEm", shim_ct_font_get_units_per_em as *const () as u64, "CTFontGetUnitsPerEm"),
            ("_CTFontGetSize", shim_ct_font_get_size as *const () as u64, "CTFontGetSize"),
            ("_CTFontGetBoundingBox", shim_ct_font_get_bounding_box as *const () as u64, "CTFontGetBoundingBox"),

            // -- CTFontDescriptor --
            ("_CTFontDescriptorCreateWithAttributes", shim_ct_descriptor_create as *const () as u64, "CTFontDescriptorCreateWithAttributes"),
            ("_CTFontDescriptorCreateMatchingFontDescriptor", shim_ct_descriptor_match as *const () as u64, "CTFontDescriptorCreateMatchingFontDescriptor"),
            ("_CTFontDescriptorCopyAttribute", shim_ct_descriptor_copy_attr as *const () as u64, "CTFontDescriptorCopyAttribute"),

            // -- CTParagraphStyle --
            ("_CTParagraphStyleCreate", shim_ct_paragraph_style_create as *const () as u64, "CTParagraphStyleCreate"),
            ("_CTParagraphStyleGetValueForSpecifier", shim_ct_paragraph_style_get_value as *const () as u64, "CTParagraphStyleGetValueForSpecifier"),

            // -- CTLine --
            ("_CTLineCreateWithAttributedString", shim_ct_line_create as *const () as u64, "CTLineCreateWithAttributedString (handle-based)"),

            // -- CTRun --
            ("_CTRunGetGlyphCount", shim_ct_run_get_glyph_count as *const () as u64, "CTRunGetGlyphCount (stub → 0)"),
            ("_CTRunGetPositions", shim_ct_run_get_positions as *const () as u64, "CTRunGetPositions (no-op)"),
        );
        log::debug!("CoreTextStubs: registered 21 shims");
    }
}

// ---------------------------------------------------------------------------
// Generic null / zero stubs (kept for manager + glyph-count)
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_null_stub(_arg0: u64, _arg1: u64, _arg2: u64) -> u64 { 0 }
pub extern "C" fn shim_ct_zero_stub(_arg0: u64) -> u64 { 0 }

// ---------------------------------------------------------------------------
// CTFont creation
// ---------------------------------------------------------------------------

/// `CTFontCreateWithName(name: CFStringRef, size: f64, matrix: u64) -> CTFontRef`
///
/// `name` is treated as a C-string pointer.  If NULL we default to "Helvetica".
/// `size` arrives as a raw f64 in a float register — the caller passes it via
/// the second integer-register slot so we receive the bit pattern in `size_bits`.
pub extern "C" fn shim_ct_font_create_with_name(name_ptr: u64, size_bits: u64, _matrix: u64) -> u64 {
    let name = if name_ptr == 0 {
        "Helvetica".to_string()
    } else {
        let cstr = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const std::ffi::c_char) };
        cstr.to_string_lossy().into_owned()
    };

    let size = f64::from_bits(size_bits);
    let size = if size <= 0.0 { 12.0 } else { size };

    let handle = new_handle();
    with_registry(|map| {
        map.insert(handle, CtFont::new(name, size));
    });
    handle
}

/// `CTFontCopyFullName(font: u64) -> CFStringRef`  — stub returns NULL.
pub extern "C" fn shim_ct_font_copy_full_name(_font: u64) -> u64 { 0 }

// ---------------------------------------------------------------------------
// CTFont metrics
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_font_get_ascent(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.ascent, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_descent(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.descent, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_leading(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.leading, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_x_height(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.x_height, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_cap_height(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.cap_height, 0.0);
    v.to_bits()
}

pub extern "C" fn shim_ct_font_get_units_per_em(font: u64) -> u32 {
    lookup_font(font, |f| f.units_per_em, 0)
}

pub extern "C" fn shim_ct_font_get_size(font: u64) -> u64 {
    let v = lookup_font(font, |f| f.size, 0.0);
    v.to_bits()
}

/// `CTFontGetBoundingBox(font, rect_out) -> rect_out`
///
/// Writes a CGRect `{x: 0, y: -descent, width: size*0.6*units_per_em, height: ascent+descent}`
/// to `rect_out` and returns `rect_out`.
pub extern "C" fn shim_ct_font_get_bounding_box(font: u64, rect_out: u64) -> u64 {
    if rect_out == 0 {
        return 0;
    }
    let (x, y, w, h) = lookup_font(font, |f| {
        (
            0.0_f64,
            -f.descent,
            f.size * 0.6 * f.units_per_em as f64,
            f.ascent + f.descent,
        )
    }, (0.0, 0.0, 0.0, 0.0));

    let ptr = rect_out as *mut f64;
    unsafe {
        ptr.write(x);
        ptr.add(1).write(y);
        ptr.add(2).write(w);
        ptr.add(3).write(h);
    }
    rect_out
}

// ---------------------------------------------------------------------------
// CTFontDescriptor
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_descriptor_create(_attrs: u64) -> u64 {
    new_handle()
}

pub extern "C" fn shim_ct_descriptor_match(descriptor: u64, _mandatory_attrs: u64) -> u64 {
    descriptor
}

pub extern "C" fn shim_ct_descriptor_copy_attr(_descriptor: u64, _attr_name: u64) -> u64 {
    0
}

// ---------------------------------------------------------------------------
// CTParagraphStyle
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_paragraph_style_create(_settings: u64, _count: usize) -> u64 {
    new_handle()
}

pub extern "C" fn shim_ct_paragraph_style_get_value(_style: u64, _spec: u32, _value_size: usize, _value_out: u64) -> u8 {
    0 // false — value not available
}

// ---------------------------------------------------------------------------
// CTLine
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_line_create(_attr_str: u64) -> u64 {
    new_handle()
}

// ---------------------------------------------------------------------------
// CTRun
// ---------------------------------------------------------------------------

pub extern "C" fn shim_ct_run_get_glyph_count(_run: u64) -> i64 {
    0
}

pub extern "C" fn shim_ct_run_get_positions(_run: u64, _range_loc: i64, _range_len: i64, _positions: u64) {
    // no-op
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_create_returns_handle() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        assert_ne!(h, 0, "CTFontCreateWithName should return a non-zero handle");
    }

    #[test]
    fn test_font_metrics_proportional() {
        let size = 20.0_f64;
        let h = shim_ct_font_create_with_name(0, size.to_bits(), 0);

        let ascent = f64::from_bits(shim_ct_font_get_ascent(h));
        let descent = f64::from_bits(shim_ct_font_get_descent(h));
        let leading = f64::from_bits(shim_ct_font_get_leading(h));

        assert!((ascent - 16.0).abs() < 1e-9, "ascent should be 16.0, got {ascent}");
        assert!((descent - 4.0).abs() < 1e-9, "descent should be 4.0, got {descent}");
        assert!((leading - 2.0).abs() < 1e-9, "leading should be 2.0, got {leading}");
    }

    #[test]
    fn test_font_get_size() {
        let size = 14.0_f64;
        let h = shim_ct_font_create_with_name(0, size.to_bits(), 0);
        let got = f64::from_bits(shim_ct_font_get_size(h));
        assert!((got - 14.0).abs() < 1e-9, "size should be 14.0, got {got}");
    }

    #[test]
    fn test_font_get_units_per_em() {
        let h = shim_ct_font_create_with_name(0, f64::to_bits(12.0), 0);
        assert_eq!(shim_ct_font_get_units_per_em(h), 2048);
    }

    #[test]
    fn test_descriptor_create() {
        let h = shim_ct_descriptor_create(0);
        assert_ne!(h, 0, "descriptor handle should be non-zero");
    }

    #[test]
    fn test_descriptor_copy_attribute_null() {
        let h = shim_ct_descriptor_create(0);
        assert_eq!(shim_ct_descriptor_copy_attr(h, 0), 0);
    }

    #[test]
    fn test_paragraph_style_create() {
        let h = shim_ct_paragraph_style_create(0, 0);
        assert_ne!(h, 0, "paragraph style handle should be non-zero");
    }

    #[test]
    fn test_line_create() {
        let h = shim_ct_line_create(0);
        assert_ne!(h, 0, "line handle should be non-zero");
    }

    #[test]
    fn test_run_glyph_count_zero() {
        assert_eq!(shim_ct_run_get_glyph_count(0), 0);
    }

    #[test]
    fn test_font_manager_available() {
        let result = shim_ct_null_stub(0, 0, 0);
        assert_eq!(result, 0);
    }
}
