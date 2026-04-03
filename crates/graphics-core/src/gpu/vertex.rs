//! Vertex type for the GPU renderer quad batcher.

/// A single vertex in the GPU renderer's vertex buffer.
///
/// Supports both solid-colour quads (`use_texture == 0`) and
/// atlas-sampled glyph quads (`use_texture == 1`).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Vertex {
    pub(super) position: [f32; 2],
    pub(super) color: [f32; 4],
    pub(super) tex_coord: [f32; 2],
    pub(super) use_texture: f32,
    pub(super) _pad: f32,
}

impl Vertex {
    pub(super) fn colored(x: f32, y: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            position: [x, y],
            color: [r, g, b, a],
            tex_coord: [0.0, 0.0],
            use_texture: 0.0,
            _pad: 0.0,
        }
    }

    pub(super) fn textured(x: f32, y: f32, u: f32, v: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            position: [x, y],
            color: [r, g, b, a],
            tex_coord: [u, v],
            use_texture: 1.0,
            _pad: 0.0,
        }
    }
}
