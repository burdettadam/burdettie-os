use super::color::Color;

// ── Surface ────────────────────────────────────────────────────────

/// A CPU-rendered pixel buffer (ARGB32, row-major).
pub struct Surface {
    pixels: Vec<u32>,
    width: u32,
    height: u32,
}

impl Surface {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            pixels: vec![0; (width * height) as usize],
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn pixels_mut(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    /// Set a single pixel (bounds-checked).
    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Clear to a solid color.
    pub fn clear(&mut self, color: Color) {
        let c = color.to_argb32();
        self.pixels.fill(c);
    }
}
