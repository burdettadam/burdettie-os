//! GpuRenderer struct and inherent method implementations.

use crate::color::Color;
use crate::font_manager::FontManager;
use crate::surface::Surface;

use super::atlas::{GlyphAtlas, ATLAS_SIZE};
use super::vertex::Vertex;

use wgpu::util::DeviceExt;

/// GPU-accelerated renderer using wgpu.
///
/// Collects draw commands as batched vertex data, then flushes to the
/// GPU in a single render pass. Text is rendered via a glyph atlas
/// texture.
pub struct GpuRenderer {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) atlas_texture: wgpu::Texture,
    pub(super) atlas_bind_group: wgpu::BindGroup,
    pub(super) vertices: Vec<Vertex>,
    pub(super) target_width: u32,
    pub(super) target_height: u32,
    pub(super) atlas: GlyphAtlas,
    pub(super) atlas_dirty: bool,
    pub(super) font_manager: Option<FontManager>,
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
            source: wgpu::ShaderSource::Wgsl(include_str!("../gpu_shader.wgsl").into()),
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
    pub(super) fn to_ndc(&self, x: f32, y: f32) -> [f32; 2] {
        let nx = (x / self.target_width as f32) * 2.0 - 1.0;
        let ny = 1.0 - (y / self.target_height as f32) * 2.0;
        [nx, ny]
    }

    /// Push a filled quad as two triangles.
    pub(super) fn push_quad(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color) {
        let [r, g, b, a] = [
            color.r as f32,
            color.g as f32,
            color.b as f32,
            color.a as f32,
        ];
        let tl = self.to_ndc(x0, y0);
        let tr = self.to_ndc(x1, y0);
        let bl = self.to_ndc(x0, y1);
        let br = self.to_ndc(x1, y1);

        // Two triangles: TL-TR-BL, TR-BR-BL
        self.vertices
            .push(Vertex::colored(tl[0], tl[1], r, g, b, a));
        self.vertices
            .push(Vertex::colored(tr[0], tr[1], r, g, b, a));
        self.vertices
            .push(Vertex::colored(bl[0], bl[1], r, g, b, a));
        self.vertices
            .push(Vertex::colored(tr[0], tr[1], r, g, b, a));
        self.vertices
            .push(Vertex::colored(br[0], br[1], r, g, b, a));
        self.vertices
            .push(Vertex::colored(bl[0], bl[1], r, g, b, a));
    }

    /// Push a textured quad for a glyph.
    pub(super) fn push_textured_quad(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: Color,
    ) {
        let [r, g, b, a] = [
            color.r as f32,
            color.g as f32,
            color.b as f32,
            color.a as f32,
        ];
        let tl = self.to_ndc(x0, y0);
        let tr = self.to_ndc(x1, y0);
        let bl = self.to_ndc(x0, y1);
        let br = self.to_ndc(x1, y1);

        self.vertices
            .push(Vertex::textured(tl[0], tl[1], u0, v0, r, g, b, a));
        self.vertices
            .push(Vertex::textured(tr[0], tr[1], u1, v0, r, g, b, a));
        self.vertices
            .push(Vertex::textured(bl[0], bl[1], u0, v1, r, g, b, a));
        self.vertices
            .push(Vertex::textured(tr[0], tr[1], u1, v0, r, g, b, a));
        self.vertices
            .push(Vertex::textured(br[0], br[1], u1, v1, r, g, b, a));
        self.vertices
            .push(Vertex::textured(bl[0], bl[1], u0, v1, r, g, b, a));
    }

    /// Flush all batched vertices to the GPU and render to a CPU Surface.
    ///
    /// This is the compatibility path — renders to an offscreen texture,
    pub fn is_available(&self) -> bool {
        true // If construction succeeded, GPU is available
    }
}
