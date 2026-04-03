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
#![allow(clippy::fn_to_numeric_cast)]

mod context;
mod colorspace;
mod image;
mod path;
mod affine;
mod data_provider;
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
    log::debug!("cg-shims: registered CoreGraphics C API stubs");
}
