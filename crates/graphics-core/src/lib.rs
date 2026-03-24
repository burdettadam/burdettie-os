//! 2D drawing primitives and renderer trait.
//!
//! Provides geometry types (`Rect`, `Point`, `Color`, `Font`), a pixel
//! buffer (`Surface`), and a `Renderer` trait for drawing shapes and text.
//! Backends implement `Renderer` for their specific rendering target.

// ── Geometry types ─────────────────────────────────────────────────

/// A 2D point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Origin point (top-left).
    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Whether the rect contains a point.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    /// Intersect with another rect.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Inset the rect by a uniform amount on all sides.
    pub fn inset(&self, dx: f64, dy: f64) -> Rect {
        Rect::new(
            self.x + dx,
            self.y + dy,
            (self.width - 2.0 * dx).max(0.0),
            (self.height - 2.0 * dy).max(0.0),
        )
    }
}

// ── Color ──────────────────────────────────────────────────────────

/// An RGBA color with components in 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Convert to 32-bit ARGB pixel value.
    pub fn to_argb32(&self) -> u32 {
        let a = (self.a * 255.0) as u32;
        let r = (self.r * 255.0) as u32;
        let g = (self.g * 255.0) as u32;
        let b = (self.b * 255.0) as u32;
        (a << 24) | (r << 16) | (g << 8) | b
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

// ── Font ───────────────────────────────────────────────────────────

/// A font descriptor.
#[derive(Debug, Clone)]
pub struct Font {
    pub name: String,
    pub size: f64,
}

impl Font {
    pub fn new(name: impl Into<String>, size: f64) -> Self {
        Self {
            name: name.into(),
            size,
        }
    }

    pub fn system(size: f64) -> Self {
        Self::new("System", size)
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::system(14.0)
    }
}

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

// ── Renderer trait ─────────────────────────────────────────────────

/// 2D drawing operations. Backends implement this for their target surface.
pub trait Renderer {
    fn clear(&mut self, color: Color);
    fn fill_rect(&mut self, rect: Rect, color: Color);
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64);
    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color);
    fn draw_line(&mut self, from: Point, to: Point, color: Color, width: f64);
}

// ── Software renderer ──────────────────────────────────────────────

// 5×7 bitmap font for printable ASCII (32–126).
// Each glyph is 7 rows; each row has 5 bits (bit 4 = leftmost pixel).
#[rustfmt::skip]
const FONT_5X7: [[u8; 7]; 95] = [
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00], // 32 ' '
    [0x04,0x04,0x04,0x04,0x04,0x00,0x04], // 33 '!'
    [0x0A,0x0A,0x0A,0x00,0x00,0x00,0x00], // 34 '"'
    [0x0A,0x0A,0x1F,0x0A,0x1F,0x0A,0x0A], // 35 '#'
    [0x04,0x0F,0x14,0x0E,0x05,0x1E,0x04], // 36 '$'
    [0x18,0x19,0x02,0x04,0x08,0x13,0x03], // 37 '%'
    [0x0C,0x12,0x14,0x08,0x15,0x12,0x0D], // 38 '&'
    [0x06,0x04,0x08,0x00,0x00,0x00,0x00], // 39 "'"
    [0x02,0x04,0x08,0x08,0x08,0x04,0x02], // 40 '('
    [0x08,0x04,0x02,0x02,0x02,0x04,0x08], // 41 ')'
    [0x00,0x04,0x15,0x0E,0x15,0x04,0x00], // 42 '*'
    [0x00,0x04,0x04,0x1F,0x04,0x04,0x00], // 43 '+'
    [0x00,0x00,0x00,0x00,0x06,0x04,0x08], // 44 ','
    [0x00,0x00,0x00,0x1F,0x00,0x00,0x00], // 45 '-'
    [0x00,0x00,0x00,0x00,0x00,0x06,0x06], // 46 '.'
    [0x00,0x01,0x02,0x04,0x08,0x10,0x00], // 47 '/'
    [0x0E,0x11,0x13,0x15,0x19,0x11,0x0E], // 48 '0'
    [0x04,0x0C,0x04,0x04,0x04,0x04,0x0E], // 49 '1'
    [0x0E,0x11,0x01,0x02,0x04,0x08,0x1F], // 50 '2'
    [0x1F,0x02,0x04,0x02,0x01,0x11,0x0E], // 51 '3'
    [0x02,0x06,0x0A,0x12,0x1F,0x02,0x02], // 52 '4'
    [0x1F,0x10,0x1E,0x01,0x01,0x11,0x0E], // 53 '5'
    [0x06,0x08,0x10,0x1E,0x11,0x11,0x0E], // 54 '6'
    [0x1F,0x01,0x02,0x04,0x08,0x08,0x08], // 55 '7'
    [0x0E,0x11,0x11,0x0E,0x11,0x11,0x0E], // 56 '8'
    [0x0E,0x11,0x11,0x0F,0x01,0x02,0x0C], // 57 '9'
    [0x00,0x06,0x06,0x00,0x06,0x06,0x00], // 58 ':'
    [0x00,0x06,0x06,0x00,0x06,0x04,0x08], // 59 ';'
    [0x02,0x04,0x08,0x10,0x08,0x04,0x02], // 60 '<'
    [0x00,0x00,0x1F,0x00,0x1F,0x00,0x00], // 61 '='
    [0x08,0x04,0x02,0x01,0x02,0x04,0x08], // 62 '>'
    [0x0E,0x11,0x01,0x02,0x04,0x00,0x04], // 63 '?'
    [0x0E,0x11,0x17,0x15,0x17,0x10,0x0E], // 64 '@'
    [0x0E,0x11,0x11,0x1F,0x11,0x11,0x11], // 65 'A'
    [0x1E,0x11,0x11,0x1E,0x11,0x11,0x1E], // 66 'B'
    [0x0E,0x11,0x10,0x10,0x10,0x11,0x0E], // 67 'C'
    [0x1C,0x12,0x11,0x11,0x11,0x12,0x1C], // 68 'D'
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x1F], // 69 'E'
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x10], // 70 'F'
    [0x0E,0x11,0x10,0x17,0x11,0x11,0x0F], // 71 'G'
    [0x11,0x11,0x11,0x1F,0x11,0x11,0x11], // 72 'H'
    [0x0E,0x04,0x04,0x04,0x04,0x04,0x0E], // 73 'I'
    [0x07,0x02,0x02,0x02,0x02,0x12,0x0C], // 74 'J'
    [0x11,0x12,0x14,0x18,0x14,0x12,0x11], // 75 'K'
    [0x10,0x10,0x10,0x10,0x10,0x10,0x1F], // 76 'L'
    [0x11,0x1B,0x15,0x15,0x11,0x11,0x11], // 77 'M'
    [0x11,0x11,0x19,0x15,0x13,0x11,0x11], // 78 'N'
    [0x0E,0x11,0x11,0x11,0x11,0x11,0x0E], // 79 'O'
    [0x1E,0x11,0x11,0x1E,0x10,0x10,0x10], // 80 'P'
    [0x0E,0x11,0x11,0x11,0x15,0x12,0x0D], // 81 'Q'
    [0x1E,0x11,0x11,0x1E,0x14,0x12,0x11], // 82 'R'
    [0x0E,0x11,0x10,0x0E,0x01,0x11,0x0E], // 83 'S'
    [0x1F,0x04,0x04,0x04,0x04,0x04,0x04], // 84 'T'
    [0x11,0x11,0x11,0x11,0x11,0x11,0x0E], // 85 'U'
    [0x11,0x11,0x11,0x11,0x11,0x0A,0x04], // 86 'V'
    [0x11,0x11,0x11,0x15,0x15,0x15,0x0A], // 87 'W'
    [0x11,0x11,0x0A,0x04,0x0A,0x11,0x11], // 88 'X'
    [0x11,0x11,0x0A,0x04,0x04,0x04,0x04], // 89 'Y'
    [0x1F,0x01,0x02,0x04,0x08,0x10,0x1F], // 90 'Z'
    [0x0E,0x08,0x08,0x08,0x08,0x08,0x0E], // 91 '['
    [0x00,0x10,0x08,0x04,0x02,0x01,0x00], // 92 '\'
    [0x0E,0x02,0x02,0x02,0x02,0x02,0x0E], // 93 ']'
    [0x04,0x0A,0x11,0x00,0x00,0x00,0x00], // 94 '^'
    [0x00,0x00,0x00,0x00,0x00,0x00,0x1F], // 95 '_'
    [0x08,0x04,0x02,0x00,0x00,0x00,0x00], // 96 '`'
    [0x00,0x00,0x0E,0x01,0x0F,0x11,0x0F], // 97 'a'
    [0x10,0x10,0x16,0x19,0x11,0x11,0x1E], // 98 'b'
    [0x00,0x00,0x0E,0x10,0x10,0x11,0x0E], // 99 'c'
    [0x01,0x01,0x0D,0x13,0x11,0x11,0x0F], // 100 'd'
    [0x00,0x00,0x0E,0x11,0x1F,0x10,0x0E], // 101 'e'
    [0x06,0x09,0x08,0x1C,0x08,0x08,0x08], // 102 'f'
    [0x00,0x00,0x0F,0x11,0x0F,0x01,0x0E], // 103 'g'
    [0x10,0x10,0x16,0x19,0x11,0x11,0x11], // 104 'h'
    [0x04,0x00,0x0C,0x04,0x04,0x04,0x0E], // 105 'i'
    [0x02,0x00,0x06,0x02,0x02,0x12,0x0C], // 106 'j'
    [0x10,0x10,0x12,0x14,0x18,0x14,0x12], // 107 'k'
    [0x0C,0x04,0x04,0x04,0x04,0x04,0x0E], // 108 'l'
    [0x00,0x00,0x1A,0x15,0x15,0x11,0x11], // 109 'm'
    [0x00,0x00,0x16,0x19,0x11,0x11,0x11], // 110 'n'
    [0x00,0x00,0x0E,0x11,0x11,0x11,0x0E], // 111 'o'
    [0x00,0x00,0x1E,0x11,0x1E,0x10,0x10], // 112 'p'
    [0x00,0x00,0x0D,0x13,0x0F,0x01,0x01], // 113 'q'
    [0x00,0x00,0x16,0x19,0x10,0x10,0x10], // 114 'r'
    [0x00,0x00,0x0E,0x10,0x0E,0x01,0x1E], // 115 's'
    [0x08,0x08,0x1C,0x08,0x08,0x09,0x06], // 116 't'
    [0x00,0x00,0x11,0x11,0x11,0x13,0x0D], // 117 'u'
    [0x00,0x00,0x11,0x11,0x11,0x0A,0x04], // 118 'v'
    [0x00,0x00,0x11,0x11,0x15,0x15,0x0A], // 119 'w'
    [0x00,0x00,0x11,0x0A,0x04,0x0A,0x11], // 120 'x'
    [0x00,0x00,0x11,0x11,0x0F,0x01,0x0E], // 121 'y'
    [0x00,0x00,0x1F,0x02,0x04,0x08,0x1F], // 122 'z'
    [0x02,0x04,0x04,0x08,0x04,0x04,0x02], // 123 '{'
    [0x04,0x04,0x04,0x04,0x04,0x04,0x04], // 124 '|'
    [0x08,0x04,0x04,0x02,0x04,0x04,0x08], // 125 '}'
    [0x00,0x00,0x0D,0x12,0x00,0x00,0x00], // 126 '~'
];

/// Look up a glyph for a character, returning a filled rectangle for unknowns.
fn glyph_for(ch: char) -> &'static [u8; 7] {
    let idx = ch as u32;
    if (32..=126).contains(&idx) {
        &FONT_5X7[(idx - 32) as usize]
    } else {
        // Unknown character → filled block
        static BLOCK: [u8; 7] = [0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F];
        &BLOCK
    }
}

/// A basic software renderer that draws into a `Surface`.
pub struct SoftwareRenderer<'a> {
    surface: &'a mut Surface,
}

impl<'a> SoftwareRenderer<'a> {
    pub fn new(surface: &'a mut Surface) -> Self {
        Self { surface }
    }

    fn fill_pixel_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;

        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(sw);
        let y1 = (y + h).min(sh);

        for py in y0..y1 {
            for px in x0..x1 {
                self.surface.set_pixel(px as u32, py as u32, color);
            }
        }
    }
}

impl Renderer for SoftwareRenderer<'_> {
    fn clear(&mut self, color: Color) {
        self.surface.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let c = color.to_argb32();
        self.fill_pixel_rect(
            rect.x as i32,
            rect.y as i32,
            rect.width as i32,
            rect.height as i32,
            c,
        );
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64) {
        let c = color.to_argb32();
        let w = width as i32;
        let x = rect.x as i32;
        let y = rect.y as i32;
        let rw = rect.width as i32;
        let rh = rect.height as i32;

        // Top
        self.fill_pixel_rect(x, y, rw, w, c);
        // Bottom
        self.fill_pixel_rect(x, y + rh - w, rw, w, c);
        // Left
        self.fill_pixel_rect(x, y, w, rh, c);
        // Right
        self.fill_pixel_rect(x + rw - w, y, w, rh, c);
    }

    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        let c = color.to_argb32();
        let scale = (font.size / 7.0).max(1.0).round() as i32;
        let char_w = 5 * scale;
        let spacing = char_w + scale; // 1-pixel gap scaled

        for (i, ch) in text.chars().enumerate() {
            let glyph = glyph_for(ch);
            let bx = origin.x as i32 + (i as i32) * spacing;
            let by = origin.y as i32;
            for (row, &bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        let px = bx + col * scale;
                        let py = by + (row as i32) * scale;
                        self.fill_pixel_rect(px, py, scale, scale, c);
                    }
                }
            }
        }
    }

    fn draw_line(&mut self, from: Point, to: Point, color: Color, _width: f64) {
        // Bresenham's line algorithm
        let c = color.to_argb32();
        let mut x0 = from.x as i32;
        let mut y0 = from.y as i32;
        let x1 = to.x as i32;
        let y1 = to.y as i32;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.surface.set_pixel(x0 as u32, y0 as u32, c);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_basic() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(r.contains(Point::new(50.0, 30.0)));
        assert!(!r.contains(Point::new(5.0, 30.0)));
        assert!(!r.contains(Point::new(50.0, 61.0)));
        // Edge: top-left is inside
        assert!(r.contains(Point::new(10.0, 10.0)));
        // Edge: bottom-right is outside (exclusive)
        assert!(!r.contains(Point::new(110.0, 60.0)));
    }

    #[test]
    fn rect_intersects() {
        let a = Rect::new(0.0, 0.0, 50.0, 50.0);
        let b = Rect::new(25.0, 25.0, 50.0, 50.0);
        let c = Rect::new(100.0, 100.0, 10.0, 10.0);
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn rect_inset() {
        let r = Rect::new(10.0, 10.0, 100.0, 80.0);
        let inset = r.inset(5.0, 10.0);
        assert_eq!(inset.x, 15.0);
        assert_eq!(inset.y, 20.0);
        assert_eq!(inset.width, 90.0);
        assert_eq!(inset.height, 60.0);
    }

    #[test]
    fn color_to_argb32() {
        assert_eq!(Color::WHITE.to_argb32(), 0xFFFFFFFF);
        assert_eq!(Color::BLACK.to_argb32(), 0xFF000000);
        assert_eq!(Color::RED.to_argb32(), 0xFFFF0000);
        assert_eq!(Color::TRANSPARENT.to_argb32(), 0x00000000);
    }

    #[test]
    fn surface_clear_and_pixel() {
        let mut s = Surface::new(10, 10);
        s.clear(Color::RED);
        assert_eq!(s.pixels()[0], Color::RED.to_argb32());

        s.set_pixel(5, 5, Color::BLUE.to_argb32());
        assert_eq!(s.pixels()[55], Color::BLUE.to_argb32());
    }

    #[test]
    fn software_renderer_fill_rect() {
        let mut s = Surface::new(100, 100);
        {
            let mut r = SoftwareRenderer::new(&mut s);
            r.clear(Color::WHITE);
            r.fill_rect(Rect::new(10.0, 10.0, 5.0, 5.0), Color::RED);
        }
        // Check a pixel inside the filled rect
        assert_eq!(s.pixels()[(12 * 100 + 12) as usize], Color::RED.to_argb32());
        // Check a pixel outside
        assert_eq!(s.pixels()[0], Color::WHITE.to_argb32());
    }

    #[test]
    fn software_renderer_stroke_rect() {
        let mut s = Surface::new(100, 100);
        {
            let mut r = SoftwareRenderer::new(&mut s);
            r.clear(Color::WHITE);
            r.stroke_rect(Rect::new(10.0, 10.0, 20.0, 20.0), Color::BLACK, 1.0);
        }
        // Top edge
        assert_eq!(
            s.pixels()[(10 * 100 + 15) as usize],
            Color::BLACK.to_argb32()
        );
        // Interior should still be white
        assert_eq!(
            s.pixels()[(20 * 100 + 20) as usize],
            Color::WHITE.to_argb32()
        );
    }

    #[test]
    fn font_default() {
        let f = Font::default();
        assert_eq!(f.name, "System");
        assert_eq!(f.size, 14.0);
    }
}
