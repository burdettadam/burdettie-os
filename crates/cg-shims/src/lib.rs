//! CoreGraphics C API stubs for guest Mach-O binaries.
//!
//! Provides stub implementations for CG functions that Xcode tools and
//! Foundation-based programs import. The real rendering is done by
//! `graphics-core`; these stubs ensure symbol resolution doesn't fail.
//!
//! # Apple Stack Mapping
//!
//! ```text
//! Apple:  CoreGraphics.framework → CGContext*, CGColorSpace*, CGImage*, CGPath*
//! Us:     cg-shims              → register_cg_shims(table) installs ~80 stubs
//! ```
// Function pointers are cast to u64 addresses for the emulated Mach-O address space.
#![allow(dead_code, clippy::fn_to_numeric_cast)]

mod affine;
pub mod bridge;
mod colorspace;
mod context;
mod data_provider;
mod gradient;
mod image;
mod path;

use libsystem_shims::table::ShimTable;

/// Install all CoreGraphics stubs into the shim table.
pub fn register_cg_shims(table: &mut ShimTable) {
    context::install(table);
    colorspace::install(table);
    image::install(table);
    path::install(table);
    affine::install(table);
    data_provider::install(table);
    gradient::install(table);

    // Wire CTLineDraw → CG context text rendering (now with font name)
    libsystem_shims::darwin_stubs::core_text::set_ct_line_draw_callback(ct_line_draw_impl);

    // Wire real font metrics from graphics-core into CoreText shims
    libsystem_shims::darwin_stubs::core_text::set_ct_font_metrics_callback(ct_font_metrics_impl);
    libsystem_shims::darwin_stubs::core_text::set_ct_text_measure_callback(ct_text_measure_impl);
    libsystem_shims::darwin_stubs::core_text::set_ct_char_advance_callback(ct_char_advance_impl);

    // Wire font family enumeration from graphics-core into CoreText shims
    libsystem_shims::darwin_stubs::core_text::set_ct_font_families_callback(ct_font_families_impl);

    log::debug!("cg-shims: registered CoreGraphics C API stubs");
}

/// CTLineDraw callback: renders text at the current text position via CgContext.
/// Now receives font name so it can resolve to the correct FontFace.
extern "C" fn ct_line_draw_impl(
    ctx: u64,
    text_ptr: *const u8,
    text_len: usize,
    font_name_ptr: *const u8,
    font_name_len: usize,
    font_size: f64,
) {
    let text =
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(text_ptr, text_len)) };
    let font_name = unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(font_name_ptr, font_name_len))
    };
    bridge::with_context(ctx, |c| {
        let old_size = c.font_size;
        if font_size > 0.0 {
            c.font_size = font_size;
        }
        let x = c.text_x;
        let y = c.text_y;
        c.draw_text_with_font(text, x, y, font_name);
        c.font_size = old_size;
    });
}

/// Font metrics callback: returns real ascent/descent/leading from ab_glyph.
extern "C" fn ct_font_metrics_impl(
    name_ptr: *const u8,
    name_len: usize,
    size: f64,
) -> libsystem_shims::darwin_stubs::core_text::FontMetricsResult {
    use libsystem_shims::darwin_stubs::core_text::FontMetricsResult;

    let name =
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len)) };
    let fm = bridge::font_manager();
    let face = fm.get(name);
    let m = face.metrics(size);

    FontMetricsResult {
        ascent: m.ascent,
        descent: m.descent,
        leading: m.line_gap,
        x_height: size * 0.5, // ab_glyph doesn't expose x_height directly; approximate
        cap_height: size * 0.7, // same approximation
        advance_width: face.char_width('M', size), // Use 'M' width as reference advance
        units_per_em: 2048,
    }
}

/// Text measurement callback: returns the advance width of a text string.
extern "C" fn ct_text_measure_impl(
    text_ptr: *const u8,
    text_len: usize,
    name_ptr: *const u8,
    name_len: usize,
    size: f64,
) -> f64 {
    let text =
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(text_ptr, text_len)) };
    let name =
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len)) };
    let m = graphics_core::text_system::measure_shaped_text(text, size, name, None);
    m.width
}

/// Per-character advance callback: returns the advance width for a Unicode codepoint.
extern "C" fn ct_char_advance_impl(
    ch: u32,
    name_ptr: *const u8,
    name_len: usize,
    size: f64,
) -> f64 {
    let name =
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len)) };
    let fm = bridge::font_manager();
    let face = fm.get(name);
    let c = char::from_u32(ch).unwrap_or(' ');
    face.char_width(c, size)
}

/// Font family enumeration callback for CTFontManagerCopyAvailableFontFamilyNames.
///
/// First call with null `out` returns the count.
/// Second call fills `out` with `malloc`'d C strings (caller frees).
extern "C" fn ct_font_families_impl(out: *mut *mut u8, cap: usize) -> usize {
    let families = bridge::font_family_names();
    let count = families.len();

    if out.is_null() || cap == 0 {
        return count;
    }

    let n = count.min(cap);
    for (i, name) in families.into_iter().take(n).enumerate() {
        let c_string = std::ffi::CString::new(name).unwrap_or_default();
        let bytes = c_string.as_bytes_with_nul();
        let ptr = unsafe { libc::malloc(bytes.len()) as *mut u8 };
        if !ptr.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            }
        }
        unsafe {
            out.add(i).write(ptr);
        }
    }
    n
}
