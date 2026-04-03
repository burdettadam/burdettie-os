//! 2D drawing primitives and renderer trait.
//!
//! Provides geometry types (`Rect`, `Point`, `Color`, `Font`), a pixel
//! buffer (`Surface`), and a `Renderer` trait for drawing shapes and text.
//! Backends implement `Renderer` for their specific rendering target.

mod geometry;
mod color;
mod font;
mod font_manager;
mod text_layout;
mod surface;
mod renderer;
mod software;
mod image_buf;
mod path;
mod gradient;
mod shadow;
pub mod unicode;
pub mod gpu_stub;

#[cfg(feature = "gpu")]
pub mod gpu;

pub use geometry::*;
pub use color::*;
pub use font::*;
pub use font_manager::*;
pub use text_layout::*;
pub use surface::*;
pub use renderer::*;
pub use software::*;
pub use image_buf::*;
pub use path::*;
pub use gradient::*;
pub use shadow::*;
pub use gpu_stub::GpuStubRenderer;

#[cfg(feature = "gpu")]
pub use gpu::{GpuRenderer, try_create_gpu_renderer};

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
            let surface = Surface::new(width, height);
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
        RendererBackend::Gpu => {
            Box::new(gpu_stub::GpuStubRenderer::new(width, height))
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────


#[cfg(test)]
mod tests;
