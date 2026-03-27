//! Pixel-buffer image type with optional PNG/JPEG decode support.

use crate::color::Color;

/// Pixel format for image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 8-bit per channel: R, G, B, A — 4 bytes per pixel.
    Rgba8,
    /// 8-bit per channel: R, G, B — 3 bytes per pixel.
    Rgb8,
    /// 8-bit grayscale — 1 byte per pixel.
    Gray8,
}

impl PixelFormat {
    /// Bytes per pixel for this format.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            PixelFormat::Rgba8 => 4,
            PixelFormat::Rgb8 => 3,
            PixelFormat::Gray8 => 1,
        }
    }
}

/// A CPU-side pixel buffer image.
///
/// Stores pixel data in row-major order. The primary format is RGBA8 but
/// RGB8 and Gray8 are also supported. All drawing operations treat images
/// as pre-multiplied or straight alpha depending on the source.
#[derive(Debug, Clone)]
pub struct ImageBuffer {
    data: Vec<u8>,
    width: u32,
    height: u32,
    format: PixelFormat,
}

impl ImageBuffer {
    /// Create a new image filled with transparent black.
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize) * 4;
        Self {
            data: vec![0u8; len],
            width,
            height,
            format: PixelFormat::Rgba8,
        }
    }

    /// Create from raw RGBA8 bytes. Returns `None` if length doesn't match.
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let expected = (width as usize) * (height as usize) * 4;
        if data.len() != expected {
            return None;
        }
        Some(Self {
            data,
            width,
            height,
            format: PixelFormat::Rgba8,
        })
    }

    /// Create from raw RGB8 bytes (no alpha). Returns `None` if length doesn't match.
    pub fn from_rgb(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let expected = (width as usize) * (height as usize) * 3;
        if data.len() != expected {
            return None;
        }
        Some(Self {
            data,
            width,
            height,
            format: PixelFormat::Rgb8,
        })
    }

    /// Decode an image from PNG or JPEG bytes.
    ///
    /// Requires the `imaging` feature.
    #[cfg(feature = "imaging")]
    pub fn from_encoded(bytes: &[u8]) -> Option<Self> {
        use image::GenericImageView;
        let img = image::load_from_memory(bytes).ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = img.dimensions();
        Some(Self {
            data: rgba.into_raw(),
            width: w,
            height: h,
            format: PixelFormat::Rgba8,
        })
    }

    /// Decode an image from a file path.
    ///
    /// Requires the `imaging` feature.
    #[cfg(feature = "imaging")]
    pub fn from_file(path: &std::path::Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        Self::from_encoded(&bytes)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn format(&self) -> PixelFormat {
        self.format
    }

    /// Raw pixel data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Mutable raw pixel data.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Convert to RGBA8 format (in-place if already RGBA8).
    pub fn to_rgba8(&self) -> Self {
        match self.format {
            PixelFormat::Rgba8 => self.clone(),
            PixelFormat::Rgb8 => {
                let pixel_count = (self.width as usize) * (self.height as usize);
                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for chunk in self.data.chunks_exact(3) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[2]);
                    rgba.push(255);
                }
                Self {
                    data: rgba,
                    width: self.width,
                    height: self.height,
                    format: PixelFormat::Rgba8,
                }
            }
            PixelFormat::Gray8 => {
                let pixel_count = (self.width as usize) * (self.height as usize);
                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for &g in &self.data {
                    rgba.push(g);
                    rgba.push(g);
                    rgba.push(g);
                    rgba.push(255);
                }
                Self {
                    data: rgba,
                    width: self.width,
                    height: self.height,
                    format: PixelFormat::Rgba8,
                }
            }
        }
    }

    /// Get a pixel as Color at (x, y). Returns transparent black if out of bounds.
    pub fn pixel(&self, x: u32, y: u32) -> Color {
        if x >= self.width || y >= self.height {
            return Color::TRANSPARENT;
        }
        let idx = (y as usize * self.width as usize + x as usize) * self.format.bytes_per_pixel();
        match self.format {
            PixelFormat::Rgba8 => Color::new(
                self.data[idx] as f64 / 255.0,
                self.data[idx + 1] as f64 / 255.0,
                self.data[idx + 2] as f64 / 255.0,
                self.data[idx + 3] as f64 / 255.0,
            ),
            PixelFormat::Rgb8 => Color::new(
                self.data[idx] as f64 / 255.0,
                self.data[idx + 1] as f64 / 255.0,
                self.data[idx + 2] as f64 / 255.0,
                1.0,
            ),
            PixelFormat::Gray8 => {
                let g = self.data[idx] as f64 / 255.0;
                Color::new(g, g, g, 1.0)
            }
        }
    }

    /// Set a pixel at (x, y). No-op if out of bounds or format is not RGBA8.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height || self.format != PixelFormat::Rgba8 {
            return;
        }
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        self.data[idx] = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
        self.data[idx + 1] = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
        self.data[idx + 2] = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
        self.data[idx + 3] = (color.a.clamp(0.0, 1.0) * 255.0) as u8;
    }

    /// Get an ARGB32 pixel value suitable for Surface blitting.
    pub fn argb32_at(&self, x: u32, y: u32) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let bpp = self.format.bytes_per_pixel();
        let idx = (y as usize * self.width as usize + x as usize) * bpp;
        match self.format {
            PixelFormat::Rgba8 => {
                let r = self.data[idx] as u32;
                let g = self.data[idx + 1] as u32;
                let b = self.data[idx + 2] as u32;
                let a = self.data[idx + 3] as u32;
                (a << 24) | (r << 16) | (g << 8) | b
            }
            PixelFormat::Rgb8 => {
                let r = self.data[idx] as u32;
                let g = self.data[idx + 1] as u32;
                let b = self.data[idx + 2] as u32;
                (255 << 24) | (r << 16) | (g << 8) | b
            }
            PixelFormat::Gray8 => {
                let g = self.data[idx] as u32;
                (255 << 24) | (g << 16) | (g << 8) | g
            }
        }
    }
}
