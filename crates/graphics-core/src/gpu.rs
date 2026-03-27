//! GPU-accelerated renderer using wgpu.
//!
//! Implements the `Renderer` trait using a wgpu render pipeline for
//! quad batching (fill_rect, stroke_rect, draw_line) and a glyph atlas
//! texture for text rendering.
//!
//! This module is only compiled when the `gpu` feature is enabled.

use crate::color::Color;
use crate::font::Font;
use crate::font_manager::FontManager;
use crate::geometry::{Point, Rect};
use crate::renderer::Renderer;
use crate::surface::Surface;
use crate::text_layout::AttributedLayout;

use wgpu::util::DeviceExt;

// ── Vertex layout ──────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
    tex_coord: [f32; 2],
    use_texture: f32,
    _pad: f32,
}

impl Vertex {
    fn colored(x: f32, y: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            position: [x, y],
            color: [r, g, b, a],
            tex_coord: [0.0, 0.0],
            use_texture: 0.0,
            _pad: 0.0,
        }
    }

    fn textured(x: f32, y: f32, u: f32, v: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            position: [x, y],
            color: [r, g, b, a],
            tex_coord: [u, v],
            use_texture: 1.0,
            _pad: 0.0,
        }
    }
}

// ── Glyph atlas ────────────────────────────────────────────────────

const ATLAS_SIZE: u32 = 1024;

#[derive(Clone, Copy)]
struct GlyphEntry {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    width: f32,
    height: f32,
    offset_x: f32,
    offset_y: f32,
}

struct GlyphAtlas {
    pixels: Vec<u8>,
    entries: std::collections::HashMap<(u32, u16), GlyphEntry>, // (glyph_id, px_size) → entry
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

impl GlyphAtlas {
    fn new() -> Self {
        Self {
            pixels: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            entries: std::collections::HashMap::new(),
            cursor_x: 1,
            cursor_y: 1,
            row_height: 0,
        }
    }

    fn rasterize_glyph(
        &mut self,
        font_manager: &FontManager,
        glyph_id: u32,
        px_size: f32,
        font_index: usize,
    ) -> Option<&GlyphEntry> {
        let key = (glyph_id, px_size as u16);
        if self.entries.contains_key(&key) {
            return self.entries.get(&key);
        }

        // Get the font face and rasterize
        let face = font_manager.face_by_index(font_index)?;
        let font_inner = face.inner();

        let ab_glyph_id = ab_glyph::GlyphId(glyph_id as u16);
        let glyph = ab_glyph_id.with_scale_and_position(px_size, ab_glyph::point(0.0, 0.0));

        let outlined = ab_glyph::Font::outline_glyph(font_inner, glyph)?;
        let bounds = outlined.px_bounds();

        let gw = (bounds.max.x - bounds.min.x).ceil() as u32;
        let gh = (bounds.max.y - bounds.min.y).ceil() as u32;

        if gw == 0 || gh == 0 {
            return None;
        }

        // Advance cursor
        if self.cursor_x + gw + 1 >= ATLAS_SIZE {
            self.cursor_x = 1;
            self.cursor_y += self.row_height + 1;
            self.row_height = 0;
        }
        if self.cursor_y + gh + 1 >= ATLAS_SIZE {
            return None; // Atlas full
        }
        self.row_height = self.row_height.max(gh);

        let ox = self.cursor_x;
        let oy = self.cursor_y;

        // Rasterize into atlas
        outlined.draw(|x, y, c| {
            let px = ox + x;
            let py = oy + y;
            if px < ATLAS_SIZE && py < ATLAS_SIZE {
                self.pixels[(py * ATLAS_SIZE + px) as usize] = (c * 255.0) as u8;
            }
        });

        self.cursor_x = ox + gw + 1;

        let u0 = ox as f32 / ATLAS_SIZE as f32;
        let v0 = oy as f32 / ATLAS_SIZE as f32;
        let u1 = (ox + gw) as f32 / ATLAS_SIZE as f32;
        let v1 = (oy + gh) as f32 / ATLAS_SIZE as f32;

        self.entries.insert(key, GlyphEntry {
            u0, v0, u1, v1,
            width: gw as f32,
            height: gh as f32,
            offset_x: bounds.min.x,
            offset_y: bounds.min.y,
        });

        self.entries.get(&key)
    }
}

// ── GpuRenderer ────────────────────────────────────────────────────

/// GPU-accelerated renderer using wgpu.
///
/// Collects draw commands as batched vertex data, then flushes to the
/// GPU in a single render pass. Text is rendered via a glyph atlas
/// texture.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    atlas_texture: wgpu::Texture,
    atlas_bind_group: wgpu::BindGroup,
    vertices: Vec<Vertex>,
    target_width: u32,
    target_height: u32,
    atlas: GlyphAtlas,
    atlas_dirty: bool,
    font_manager: Option<FontManager>,
}

impl GpuRenderer {
    /// Create a new GPU renderer with the given dimensions.
    ///
    /// Initializes a wgpu device and render pipeline for offscreen rendering.
    /// Use `with_fonts()` to enable TrueType text rendering.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("GpuRenderer"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .ok()?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu_renderer_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_shader.wgsl").into()),
        });

        // Atlas texture
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpu_renderer_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gpu_renderer_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2, // position
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4, // color
                        },
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x2, // tex_coord
                        },
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32, // use_texture
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Some(Self {
            device,
            queue,
            pipeline,
            atlas_texture,
            atlas_bind_group,
            vertices: Vec::new(),
            target_width: width,
            target_height: height,
            atlas: GlyphAtlas::new(),
            atlas_dirty: false,
            font_manager: None,
        })
    }

    /// Set the font manager for TrueType text rendering.
    pub fn set_font_manager(&mut self, fm: FontManager) {
        self.font_manager = Some(fm);
    }

    /// Resize the render target.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.target_width = width;
        self.target_height = height;
    }

    /// Convert pixel coordinates to NDC (-1..1 for x, 1..-1 for y).
    fn to_ndc(&self, x: f32, y: f32) -> [f32; 2] {
        let nx = (x / self.target_width as f32) * 2.0 - 1.0;
        let ny = 1.0 - (y / self.target_height as f32) * 2.0;
        [nx, ny]
    }

    /// Push a filled quad as two triangles.
    fn push_quad(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color) {
        let [r, g, b, a] = [color.r as f32, color.g as f32, color.b as f32, color.a as f32];
        let tl = self.to_ndc(x0, y0);
        let tr = self.to_ndc(x1, y0);
        let bl = self.to_ndc(x0, y1);
        let br = self.to_ndc(x1, y1);

        // Two triangles: TL-TR-BL, TR-BR-BL
        self.vertices.push(Vertex::colored(tl[0], tl[1], r, g, b, a));
        self.vertices.push(Vertex::colored(tr[0], tr[1], r, g, b, a));
        self.vertices.push(Vertex::colored(bl[0], bl[1], r, g, b, a));
        self.vertices.push(Vertex::colored(tr[0], tr[1], r, g, b, a));
        self.vertices.push(Vertex::colored(br[0], br[1], r, g, b, a));
        self.vertices.push(Vertex::colored(bl[0], bl[1], r, g, b, a));
    }

    /// Push a textured quad for a glyph.
    fn push_textured_quad(
        &mut self,
        x0: f32, y0: f32, x1: f32, y1: f32,
        u0: f32, v0: f32, u1: f32, v1: f32,
        color: Color,
    ) {
        let [r, g, b, a] = [color.r as f32, color.g as f32, color.b as f32, color.a as f32];
        let tl = self.to_ndc(x0, y0);
        let tr = self.to_ndc(x1, y0);
        let bl = self.to_ndc(x0, y1);
        let br = self.to_ndc(x1, y1);

        self.vertices.push(Vertex::textured(tl[0], tl[1], u0, v0, r, g, b, a));
        self.vertices.push(Vertex::textured(tr[0], tr[1], u1, v0, r, g, b, a));
        self.vertices.push(Vertex::textured(bl[0], bl[1], u0, v1, r, g, b, a));
        self.vertices.push(Vertex::textured(tr[0], tr[1], u1, v0, r, g, b, a));
        self.vertices.push(Vertex::textured(br[0], br[1], u1, v1, r, g, b, a));
        self.vertices.push(Vertex::textured(bl[0], bl[1], u0, v1, r, g, b, a));
    }

    /// Flush all batched vertices to the GPU and render to a CPU Surface.
    ///
    /// This is the compatibility path — renders to an offscreen texture,
    /// then reads back to a `Surface` for use with existing `WindowBackend`.
    pub fn flush_to_surface(&mut self) -> Surface {
        let w = self.target_width;
        let h = self.target_height;

        // Upload atlas if dirty
        if self.atlas_dirty {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &self.atlas.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ATLAS_SIZE),
                    rows_per_image: Some(ATLAS_SIZE),
                },
                wgpu::Extent3d {
                    width: ATLAS_SIZE,
                    height: ATLAS_SIZE,
                    depth_or_array_layers: 1,
                },
            );
            self.atlas_dirty = false;
        }

        // Create render target texture
        let render_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_target"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create vertex buffer
        let vertex_buffer = if self.vertices.is_empty() {
            None
        } else {
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }))
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            if let Some(ref vb) = vertex_buffer {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.atlas_bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..self.vertices.len() as u32, 0..1);
            }
        }

        // Read back to CPU
        let bytes_per_row = (4 * w + 255) & !255; // Align to 256
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (bytes_per_row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let buffer_slice = readback_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let data = buffer_slice.get_mapped_range();
        let mut surface = Surface::new(w, h);
        let pixels = surface.pixels_mut();
        for y in 0..h {
            for x in 0..w {
                let src_offset = (y * bytes_per_row + x * 4) as usize;
                if src_offset + 3 < data.len() {
                    let r = data[src_offset] as u32;
                    let g = data[src_offset + 1] as u32;
                    let b = data[src_offset + 2] as u32;
                    let a = data[src_offset + 3] as u32;
                    // ARGB32 format
                    pixels[(y * w + x) as usize] = (a << 24) | (r << 16) | (g << 8) | b;
                }
            }
        }
        drop(data);
        readback_buffer.unmap();

        self.vertices.clear();
        surface
    }

    /// Returns true if a GPU device was successfully initialized.
    pub fn is_available(&self) -> bool {
        true // If construction succeeded, GPU is available
    }
}

impl Renderer for GpuRenderer {
    fn clear(&mut self, color: Color) {
        // Fill the entire viewport with the clear color
        self.push_quad(0.0, 0.0, self.target_width as f32, self.target_height as f32, color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.push_quad(
            rect.x as f32,
            rect.y as f32,
            (rect.x + rect.width) as f32,
            (rect.y + rect.height) as f32,
            color,
        );
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64) {
        let w = width as f32;
        let x0 = rect.x as f32;
        let y0 = rect.y as f32;
        let x1 = (rect.x + rect.width) as f32;
        let y1 = (rect.y + rect.height) as f32;

        // Top
        self.push_quad(x0, y0, x1, y0 + w, color);
        // Bottom
        self.push_quad(x0, y1 - w, x1, y1, color);
        // Left
        self.push_quad(x0, y0 + w, x0 + w, y1 - w, color);
        // Right
        self.push_quad(x1 - w, y0 + w, x1, y1 - w, color);
    }

    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        if self.font_manager.is_none() {
            return;
        }

        let px_size = font.size as f32;
        let mut cursor_x = origin.x as f32;
        let cursor_y = origin.y as f32 + px_size; // Baseline

        // Phase 1: rasterize glyphs (borrows font_manager + atlas)
        let mut prepared: Vec<(GlyphEntry, f32, f32)> = Vec::new();
        {
            let fm = self.font_manager.as_ref().unwrap();
            for ch in text.chars() {
                if let Some(face) = fm.face_by_index(0) {
                    let font_inner = face.inner();
                    let glyph_id = ab_glyph::Font::glyph_id(font_inner, ch);
                    let scaled = ab_glyph::Font::as_scaled(font_inner, px_size);
                    let advance = ab_glyph::ScaleFont::h_advance(&scaled, glyph_id);

                    if let Some(&entry) = self.atlas.rasterize_glyph(fm, glyph_id.0 as u32, px_size, 0) {
                        prepared.push((entry, cursor_x + entry.offset_x, cursor_y + entry.offset_y));
                    }
                    cursor_x += advance;
                }
            }
        }

        // Phase 2: push quads (fm borrow dropped)
        for (entry, gx, gy) in prepared {
            self.atlas_dirty = true;
            self.push_textured_quad(
                gx, gy,
                gx + entry.width, gy + entry.height,
                entry.u0, entry.v0, entry.u1, entry.v1,
                color,
            );
        }
    }

    fn draw_line(&mut self, from: Point, to: Point, color: Color, width: f64) {
        let dx = (to.x - from.x) as f32;
        let dy = (to.y - from.y) as f32;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }

        let hw = width as f32 * 0.5;
        let nx = -dy / len * hw;
        let ny = dx / len * hw;

        let [r, g, b, a] = [color.r as f32, color.g as f32, color.b as f32, color.a as f32];
        let p0 = self.to_ndc(from.x as f32 + nx, from.y as f32 + ny);
        let p1 = self.to_ndc(from.x as f32 - nx, from.y as f32 - ny);
        let p2 = self.to_ndc(to.x as f32 + nx, to.y as f32 + ny);
        let p3 = self.to_ndc(to.x as f32 - nx, to.y as f32 - ny);

        self.vertices.push(Vertex::colored(p0[0], p0[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p2[0], p2[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p1[0], p1[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p2[0], p2[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p3[0], p3[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p1[0], p1[1], r, g, b, a));
    }

    fn draw_attributed_text(
        &mut self,
        layout: &AttributedLayout<'_>,
        origin: Point,
        default_color: Color,
    ) {
        if self.font_manager.is_none() {
            return;
        }

        // Phase 1: collect glyph entries and missing rects
        enum GlyphOp {
            Textured { entry: GlyphEntry, gx: f32, gy: f32, color: Color },
            Missing { rect: Rect, color: Color },
        }
        let mut ops: Vec<GlyphOp> = Vec::new();
        {
            let fm = self.font_manager.as_ref().unwrap();
            for glyph in &layout.glyphs {
                let color = glyph.color.unwrap_or(default_color);
                let font_index = glyph.font_index;
                let px_size = glyph.size as f32;

                if glyph.missing {
                    let bw = px_size * 0.6;
                    let bh = px_size * 0.8;
                    ops.push(GlyphOp::Missing {
                        rect: Rect::new(origin.x + glyph.x, origin.y + glyph.y, bw as f64, bh as f64),
                        color,
                    });
                    continue;
                }

                if let Some(&entry) = self.atlas.rasterize_glyph(fm, glyph.glyph_id.0 as u32, px_size, font_index) {
                    ops.push(GlyphOp::Textured {
                        entry,
                        gx: origin.x as f32 + glyph.x as f32 + entry.offset_x,
                        gy: origin.y as f32 + glyph.y as f32 + entry.offset_y,
                        color,
                    });
                }
            }
        }

        // Phase 2: push quads (fm borrow dropped)
        for op in ops {
            match op {
                GlyphOp::Textured { entry, gx, gy, color } => {
                    self.atlas_dirty = true;
                    self.push_textured_quad(
                        gx, gy,
                        gx + entry.width, gy + entry.height,
                        entry.u0, entry.v0, entry.u1, entry.v1,
                        color,
                    );
                }
                GlyphOp::Missing { rect, color } => {
                    self.stroke_rect(rect, color, 1.0);
                }
            }
        }
    }

    fn supports_attributed_text(&self) -> bool {
        self.font_manager.is_some()
    }
}

/// Try to create a GpuRenderer. Returns None if no GPU is available.
pub fn try_create_gpu_renderer(width: u32, height: u32) -> Option<GpuRenderer> {
    GpuRenderer::new(width, height)
}
