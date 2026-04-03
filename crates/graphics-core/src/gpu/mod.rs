//! GPU-accelerated renderer using wgpu.
//!
//! Implements the `Renderer` trait using a wgpu render pipeline for
//! quad batching (fill_rect, stroke_rect, draw_line) and a glyph atlas
//! texture for text rendering.
//!
//! This module is only compiled when the `gpu` feature is enabled.

mod atlas;
mod renderer;
mod renderer_flush;
mod renderer_ops;
mod vertex;

pub use renderer::GpuRenderer;
pub use renderer_ops::try_create_gpu_renderer;
