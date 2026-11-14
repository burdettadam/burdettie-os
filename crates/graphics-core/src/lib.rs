//! 2D drawing primitives and renderer trait.
//!
//! Provides geometry types (`Rect`, `Point`, `Color`, `Font`), a pixel
//! buffer (`Surface`), and a `Renderer` trait for drawing shapes and text.
//! Backends implement `Renderer` for their specific rendering target.

mod color;
mod font;
mod font_backend;
mod font_manager;
mod geometry;
pub mod gpu_stub;
mod gradient;
mod image_buf;
mod path;
mod renderer;
mod shadow;
mod software;
mod surface;
mod text_layout;
pub mod text_system;
pub mod unicode;

#[cfg(feature = "gpu")]
pub mod gpu;

pub use color::*;
pub use font::*;
pub use font_backend::GlyphId;
pub use font_manager::*;
pub use geometry::*;
pub use gpu_stub::GpuStubRenderer;
pub use gradient::*;
pub use image_buf::*;
pub use path::*;
pub use renderer::*;
pub use shadow::*;
pub use software::*;
pub use surface::*;
pub use text_layout::*;

#[cfg(feature = "gpu")]
pub use gpu::{try_create_gpu_renderer, GpuRenderer};

// ── Renderer factory ───────────────────────────────────────────────

/// Selects which renderer backend to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererBackend {
    Software,
    Gpu,
}

/// Create a boxed renderer for the chosen backend.
///
/// `Gpu` currently returns a [`GpuStubRenderer`] (software-backed placeholder).
pub fn create_renderer(backend: RendererBackend, width: u32, height: u32) -> Box<dyn Renderer> {
    match backend {
        RendererBackend::Software => {
            let _surface = Surface::new(width, height);
            // We need an owned renderer — wrap in a GpuStubRenderer-style
            // owned surface. For the Software path we reuse GpuStubRenderer
            // (which delegates to SoftwareRenderer) since SoftwareRenderer
            // borrows its surface.
            //
            // True software path: callers that need SoftwareRenderer directly
            // should construct it with their own Surface.  This factory is
            // intentionally simple.
            Box::new(gpu_stub::GpuStubRenderer::new(width, height))
        }
        RendererBackend::Gpu => Box::new(gpu_stub::GpuStubRenderer::new(width, height)),
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
