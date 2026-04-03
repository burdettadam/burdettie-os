//! GPU renderer flush: readback surface from GPU to CPU.

use wgpu::util::DeviceExt;

use crate::Surface;

use super::atlas::ATLAS_SIZE;
use super::renderer::GpuRenderer;

impl GpuRenderer {
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
}
