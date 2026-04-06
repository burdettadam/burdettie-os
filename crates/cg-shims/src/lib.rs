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

mod context;
mod colorspace;
mod image;
mod path;
mod affine;
mod data_provider;
mod gradient;
pub mod bridge;

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

    // Wire CTLineDraw → CG context text rendering
    libsystem_shims::darwin_stubs::core_text::set_ct_line_draw_callback(ct_line_draw_impl);

    log::debug!("cg-shims: registered CoreGraphics C API stubs");
}

/// CTLineDraw callback: renders text at the current text position via CgContext.
extern "C" fn ct_line_draw_impl(ctx: u64, text_ptr: *const u8, text_len: usize, font_size: f64) {
    let text = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(text_ptr, text_len)) };
    bridge::with_context(ctx, |c| {
        let old_size = c.font_size;
        if font_size > 0.0 {
            c.font_size = font_size;
        }
        let x = c.text_x;
        let y = c.text_y;
        c.draw_text(text, x, y);
        c.font_size = old_size;
    });
}
