//! Bridge from CoreGraphics C API stubs to `graphics-core` real objects.
//!
//! Provides heap-allocated drawing contexts, paths, and colors that are backed
//! by real `graphics-core` types. CG function stubs call into this module
//! to perform actual drawing on a `Surface` pixel buffer.

use std::collections::HashMap;
use std::sync::Mutex;

use graphics_core::{
    BezierPath, Color, FontManager, ImageBuffer, Point, Rect, Renderer, SoftwareRenderer, Surface,
};

/// Intersect two axis-aligned rectangles. Returns a zero-area rect if no overlap.
fn intersect_rects(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    Rect::new(x, y, (right - x).max(0.0), (bottom - y).max(0.0))
}

/// Global shared font manager (TrueType via ab_glyph).
static FONT_MANAGER: std::sync::OnceLock<FontManager> = std::sync::OnceLock::new();

pub fn font_manager() -> &'static FontManager {
    FONT_MANAGER.get_or_init(FontManager::new)
}

/// A heap-allocated CGContext backed by a `Surface` + `SoftwareRenderer`.
pub struct CgContext {
    pub surface: Surface,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub line_width: f64,
    pub alpha: f64,
    pub path: BezierPath,
    pub font_size: f64,
    pub text_x: f64,
    pub text_y: f64,
    /// Current Transformation Matrix — stored as [a, b, c, d, tx, ty].
    /// Identity = [1, 0, 0, 1, 0, 0].
    pub ctm: [f64; 6],
    /// Clipping rectangle (None = no clip).
    pub clip_rect: Option<Rect>,
    /// Line dash pattern (empty = solid line).
    pub line_dash_lengths: Vec<f64>,
    /// Line dash phase offset.
    pub line_dash_phase: f64,
    state_stack: Vec<ContextState>,
}

#[derive(Clone)]
struct ContextState {
    fill_color: Color,
    stroke_color: Color,
    line_width: f64,
    alpha: f64,
    font_size: f64,
    text_x: f64,
    text_y: f64,
    ctm: [f64; 6],
    clip_rect: Option<Rect>,
    line_dash_lengths: Vec<f64>,
    line_dash_phase: f64,
}

impl CgContext {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            surface: Surface::new(width, height),
            fill_color: Color::BLACK,
            stroke_color: Color::BLACK,
            line_width: 1.0,
            alpha: 1.0,
            path: BezierPath::new(),
            font_size: 12.0,
            text_x: 0.0,
            text_y: 0.0,
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            clip_rect: None,
            line_dash_lengths: Vec::new(),
            line_dash_phase: 0.0,
            state_stack: Vec::new(),
        }
    }

    pub fn save_state(&mut self) {
        self.state_stack.push(ContextState {
            fill_color: self.fill_color,
            stroke_color: self.stroke_color,
            line_width: self.line_width,
            alpha: self.alpha,
            font_size: self.font_size,
            text_x: self.text_x,
            text_y: self.text_y,
            ctm: self.ctm,
            clip_rect: self.clip_rect,
            line_dash_lengths: self.line_dash_lengths.clone(),
            line_dash_phase: self.line_dash_phase,
        });
    }

    pub fn restore_state(&mut self) {
        if let Some(s) = self.state_stack.pop() {
            self.fill_color = s.fill_color;
            self.stroke_color = s.stroke_color;
            self.line_width = s.line_width;
            self.alpha = s.alpha;
            self.font_size = s.font_size;
            self.text_x = s.text_x;
            self.text_y = s.text_y;
            self.ctm = s.ctm;
            self.clip_rect = s.clip_rect;
            self.line_dash_lengths = s.line_dash_lengths;
            self.line_dash_phase = s.line_dash_phase;
        }
    }

    /// Apply the current CTM to a point: p' = M * p
    #[inline]
    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, tx, ty] = self.ctm;
        (a * x + c * y + tx, b * x + d * y + ty)
    }

    /// Apply the current CTM to a rect (axis-aligned approximation).
    #[inline]
    pub fn transform_rect(&self, x: f64, y: f64, w: f64, h: f64) -> (f64, f64, f64, f64) {
        let (x0, y0) = self.transform_point(x, y);
        let (x1, y1) = self.transform_point(x + w, y + h);
        let rx = x0.min(x1);
        let ry = y0.min(y1);
        ((rx), (ry), (x0 - x1).abs(), (y0 - y1).abs())
    }

    /// Concatenate a 2D affine matrix onto the CTM: CTM = CTM * M.
    pub fn concat_ctm(&mut self, m: [f64; 6]) {
        let [a1, b1, c1, d1, tx1, ty1] = self.ctm;
        let [a2, b2, c2, d2, tx2, ty2] = m;
        self.ctm = [
            a1 * a2 + b1 * c2,
            a1 * b2 + b1 * d2,
            c1 * a2 + d1 * c2,
            c1 * b2 + d1 * d2,
            tx1 * a2 + ty1 * c2 + tx2,
            tx1 * b2 + ty1 * d2 + ty2,
        ];
    }

    pub fn translate_ctm(&mut self, tx: f64, ty: f64) {
        self.concat_ctm([1.0, 0.0, 0.0, 1.0, tx, ty]);
    }

    pub fn scale_ctm(&mut self, sx: f64, sy: f64) {
        self.concat_ctm([sx, 0.0, 0.0, sy, 0.0, 0.0]);
    }

    pub fn rotate_ctm(&mut self, angle: f64) {
        let (s, c) = angle.sin_cos();
        self.concat_ctm([c, s, -s, c, 0.0, 0.0]);
    }

    pub fn clip_to_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let (tx, ty, tw, th) = self.transform_rect(x, y, w, h);
        let new_clip = Rect::new(tx, ty, tw, th);
        self.clip_rect = Some(match self.clip_rect {
            Some(existing) => intersect_rects(existing, new_clip),
            None => new_clip,
        });
    }

    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let (tx, ty, tw, th) = self.transform_rect(x, y, w, h);
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.fill_rect(Rect::new(tx, ty, tw, th), self.fill_color);
    }

    pub fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let (tx, ty, tw, th) = self.transform_rect(x, y, w, h);
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.stroke_rect(
            Rect::new(tx, ty, tw, th),
            self.stroke_color,
            self.line_width,
        );
    }

    pub fn fill_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let (tx, ty, tw, th) = self.transform_rect(x, y, w, h);
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.fill_ellipse(Rect::new(tx, ty, tw, th), self.fill_color);
    }

    pub fn fill_path(&mut self) {
        let path = std::mem::take(&mut self.path);
        let transformed = self.transform_path(&path);
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.fill_path(&transformed, self.fill_color);
    }

    pub fn stroke_path(&mut self) {
        let path = std::mem::take(&mut self.path);
        let transformed = self.transform_path(&path);
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.stroke_path(&transformed, self.stroke_color, self.line_width);
    }

    /// Transform all points in a path through the current CTM.
    fn transform_path(&self, path: &BezierPath) -> BezierPath {
        // Fast path: identity CTM → no transform needed.
        if (self.ctm[0] - 1.0).abs() < 1e-12
            && self.ctm[1].abs() < 1e-12
            && self.ctm[2].abs() < 1e-12
            && (self.ctm[3] - 1.0).abs() < 1e-12
            && self.ctm[4].abs() < 1e-12
            && self.ctm[5].abs() < 1e-12
        {
            return path.clone();
        }
        let mut out = BezierPath::new();
        for elem in path.elements() {
            match *elem {
                graphics_core::PathElement::MoveTo(p) => {
                    let (x, y) = self.transform_point(p.x, p.y);
                    out.move_to(Point::new(x, y));
                }
                graphics_core::PathElement::LineTo(p) => {
                    let (x, y) = self.transform_point(p.x, p.y);
                    out.line_to(Point::new(x, y));
                }
                graphics_core::PathElement::QuadTo { ctrl, end } => {
                    let (cx, cy) = self.transform_point(ctrl.x, ctrl.y);
                    let (ex, ey) = self.transform_point(end.x, end.y);
                    out.quad_to(Point::new(cx, cy), Point::new(ex, ey));
                }
                graphics_core::PathElement::CubicTo { ctrl1, ctrl2, end } => {
                    let (c1x, c1y) = self.transform_point(ctrl1.x, ctrl1.y);
                    let (c2x, c2y) = self.transform_point(ctrl2.x, ctrl2.y);
                    let (ex, ey) = self.transform_point(end.x, end.y);
                    out.cubic_to(
                        Point::new(c1x, c1y),
                        Point::new(c2x, c2y),
                        Point::new(ex, ey),
                    );
                }
                graphics_core::PathElement::Close => {
                    out.close();
                }
            }
        }
        out
    }

    /// Draw text at (x, y) using the current fill color and font size.
    /// After drawing, advances text_x by the width of the rendered string.
    pub fn draw_text(&mut self, text: &str, x: f64, y: f64) {
        self.draw_text_with_font(text, x, y, "System");
    }

    /// Draw text at (x, y) using a specific font name.
    /// After drawing, advances text_x by the width of the rendered string.
    pub fn draw_text_with_font(&mut self, text: &str, x: f64, y: f64, font_name: &str) {
        let (tx, ty) = self.transform_point(x, y);
        let sw = self.surface.width();
        let sh = self.surface.height();
        let advance = graphics_core::text_system::draw_shaped_text(
            self.surface.pixels_mut(),
            sw,
            sh,
            text,
            tx,
            ty - self.font_size, // cosmic-text origin is top-left; CG y is baseline
            self.font_size,
            font_name,
            self.fill_color,
            None, // no word wrap for CG calls
        );
        self.text_x = x + advance;
        self.text_y = y;
    }

    /// Draw an image into the given destination rect, applying CTM.
    pub fn draw_image(&mut self, image: &ImageBuffer, x: f64, y: f64, w: f64, h: f64) {
        let (tx, ty, tw, th) = self.transform_rect(x, y, w, h);
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.draw_image(image, Rect::new(tx, ty, tw, th));
    }
}

// ── Handle table ───────────────────────────────────────────────

static NEXT_HANDLE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0x4347_4358_1000);

static CONTEXTS: Mutex<Option<HashMap<u64, Box<CgContext>>>> = Mutex::new(None);
static PATHS: Mutex<Option<HashMap<u64, Box<BezierPath>>>> = Mutex::new(None);
static COLORS: Mutex<Option<HashMap<u64, Color>>> = Mutex::new(None);
static IMAGES: Mutex<Option<HashMap<u64, Box<ImageBuffer>>>> = Mutex::new(None);

fn next_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// ── Context API ────────────────────────────────────────────────

/// Allocate a new context backed by a `Surface`.
pub fn context_create(width: u32, height: u32) -> u64 {
    let ctx = Box::new(CgContext::new(width, height));
    let handle = next_handle();
    let mut guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(HashMap::new).insert(handle, ctx);
    handle
}

/// Access a context mutably by handle.
pub fn with_context<R>(handle: u64, f: impl FnOnce(&mut CgContext) -> R) -> Option<R> {
    let mut guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut()?.get_mut(&handle).map(|ctx| f(ctx))
}

/// Release a context.
pub fn context_release(handle: u64) {
    let mut guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = guard.as_mut() {
        m.remove(&handle);
    }
}

/// Get the raw pixel data pointer for `CGBitmapContextGetData`.
pub fn context_get_data(handle: u64) -> u64 {
    let guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&handle))
        .map(|ctx| ctx.surface.pixels().as_ptr() as u64)
        .unwrap_or(0)
}

/// Get width for `CGBitmapContextGetWidth`.
pub fn context_get_width(handle: u64) -> u64 {
    let guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&handle))
        .map(|ctx| ctx.surface.width() as u64)
        .unwrap_or(0)
}

/// Get height for `CGBitmapContextGetHeight`.
pub fn context_get_height(handle: u64) -> u64 {
    let guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&handle))
        .map(|ctx| ctx.surface.height() as u64)
        .unwrap_or(0)
}

// ── Path API ───────────────────────────────────────────────────

pub fn path_create() -> u64 {
    let path = Box::new(BezierPath::new());
    let handle = next_handle();
    let mut guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(HashMap::new).insert(handle, path);
    handle
}

pub fn with_path<R>(handle: u64, f: impl FnOnce(&mut BezierPath) -> R) -> Option<R> {
    let mut guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut()?.get_mut(&handle).map(|p| f(p))
}

pub fn path_release(handle: u64) {
    let mut guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = guard.as_mut() {
        m.remove(&handle);
    }
}

pub fn path_is_empty(handle: u64) -> bool {
    let guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&handle))
        .map(|p| p.elements().is_empty())
        .unwrap_or(true)
}

/// Append all elements from `src` path into `dst` path.
pub fn path_append_path(dst: u64, src: u64) {
    // Collect elements from src first to avoid nested locks.
    let elements: Vec<graphics_core::PathElement> = {
        let guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .and_then(|m| m.get(&src))
            .map(|p| p.elements().to_vec())
            .unwrap_or_default()
    };
    if elements.is_empty() {
        return;
    }
    let mut guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = guard.as_mut() {
        if let Some(p) = m.get_mut(&dst) {
            for elem in &elements {
                match *elem {
                    graphics_core::PathElement::MoveTo(pt) => {
                        p.move_to(pt);
                    }
                    graphics_core::PathElement::LineTo(pt) => {
                        p.line_to(pt);
                    }
                    graphics_core::PathElement::QuadTo { ctrl, end } => {
                        p.quad_to(ctrl, end);
                    }
                    graphics_core::PathElement::CubicTo { ctrl1, ctrl2, end } => {
                        p.cubic_to(ctrl1, ctrl2, end);
                    }
                    graphics_core::PathElement::Close => {
                        p.close();
                    }
                }
            }
        }
    }
}

/// Copy path elements from a standalone path handle into a context's path.
pub fn copy_path_to_context(path_handle: u64, ctx_handle: u64) {
    // Collect elements under the path lock first, then apply under the context lock.
    let elements: Vec<graphics_core::PathElement> = {
        let guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .and_then(|m| m.get(&path_handle))
            .map(|p| p.elements().to_vec())
            .unwrap_or_default()
    };
    if elements.is_empty() {
        return;
    }
    with_context(ctx_handle, |c| {
        for elem in &elements {
            match *elem {
                graphics_core::PathElement::MoveTo(p) => {
                    c.path.move_to(p);
                }
                graphics_core::PathElement::LineTo(p) => {
                    c.path.line_to(p);
                }
                graphics_core::PathElement::QuadTo { ctrl, end } => {
                    c.path.quad_to(ctrl, end);
                }
                graphics_core::PathElement::CubicTo { ctrl1, ctrl2, end } => {
                    c.path.cubic_to(ctrl1, ctrl2, end);
                }
                graphics_core::PathElement::Close => {
                    c.path.close();
                }
            }
        }
    });
}

// ── Color API ──────────────────────────────────────────────────

pub fn color_create_rgb(r: f64, g: f64, b: f64, a: f64) -> u64 {
    let color = Color::new(r, g, b, a);
    let handle = next_handle();
    let mut guard = COLORS.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(HashMap::new).insert(handle, color);
    handle
}

pub fn color_get(handle: u64) -> Option<Color> {
    let guard = COLORS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()?.get(&handle).copied()
}

pub fn color_get_alpha(handle: u64) -> f64 {
    color_get(handle).map(|c| c.a).unwrap_or(1.0)
}

pub fn color_release(handle: u64) {
    let mut guard = COLORS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = guard.as_mut() {
        m.remove(&handle);
    }
}

// ── Image API ──────────────────────────────────────────────────

/// Create a CGImage from raw pixel data. Stores an `ImageBuffer` in the handle table.
pub fn image_create(
    width: u32,
    height: u32,
    bits_per_component: u32,
    bits_per_pixel: u32,
    bytes_per_row: u32,
    data_ptr: u64,
) -> u64 {
    let bpp = if bits_per_pixel == 0 {
        32
    } else {
        bits_per_pixel
    };
    let bpc = if bits_per_component == 0 {
        8
    } else {
        bits_per_component
    };
    let bpr = if bytes_per_row == 0 {
        width * (bpp / 8)
    } else {
        bytes_per_row
    };

    if width == 0 || height == 0 || data_ptr == 0 {
        // No pixel data — return a placeholder 1x1 transparent image.
        let img = ImageBuffer::new(1, 1);
        let handle = next_handle();
        let mut guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .get_or_insert_with(HashMap::new)
            .insert(handle, Box::new(img));
        return handle;
    }

    // Read raw pixels and convert to RGBA8 ImageBuffer.
    let channels = (bpp / bpc) as usize;
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    for row in 0..height as usize {
        for col in 0..width as usize {
            let src_off = row * bpr as usize + col * channels;
            let dst_off = (row * width as usize + col) * 4;
            let src = data_ptr as *const u8;
            match channels {
                4 => unsafe {
                    // Assume ARGB or RGBA — standard CG bitmap format is premultiplied ARGB.
                    // CG default: ARGB (alpha first). Re-order to RGBA for ImageBuffer.
                    let a = *src.add(src_off);
                    let r = *src.add(src_off + 1);
                    let g = *src.add(src_off + 2);
                    let b = *src.add(src_off + 3);
                    rgba[dst_off] = r;
                    rgba[dst_off + 1] = g;
                    rgba[dst_off + 2] = b;
                    rgba[dst_off + 3] = a;
                },
                3 => unsafe {
                    rgba[dst_off] = *src.add(src_off);
                    rgba[dst_off + 1] = *src.add(src_off + 1);
                    rgba[dst_off + 2] = *src.add(src_off + 2);
                    rgba[dst_off + 3] = 255;
                },
                1 => unsafe {
                    let v = *src.add(src_off);
                    rgba[dst_off] = v;
                    rgba[dst_off + 1] = v;
                    rgba[dst_off + 2] = v;
                    rgba[dst_off + 3] = 255;
                },
                _ => {
                    rgba[dst_off + 3] = 255; // opaque black for unknown format
                }
            }
        }
    }
    let img = ImageBuffer::from_rgba(width, height, rgba)
        .unwrap_or_else(|| ImageBuffer::new(width, height));
    let handle = next_handle();
    let mut guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, Box::new(img));
    handle
}

/// Create a CGImage handle from an already-decoded `ImageBuffer`.
pub fn image_create_from_buffer(img: ImageBuffer) -> u64 {
    let handle = next_handle();
    let mut guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, Box::new(img));
    handle
}

/// Create a CGImage from a CGBitmapContext — copies current pixel data.
pub fn image_create_from_context(ctx_handle: u64) -> u64 {
    let guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ctx) = guard.as_ref().and_then(|m| m.get(&ctx_handle)) {
        let w = ctx.surface.width();
        let h = ctx.surface.height();
        let pixels = ctx.surface.pixels();
        // Surface pixels are ARGB32. Convert to RGBA8.
        let mut rgba = vec![0u8; (w as usize) * (h as usize) * 4];
        for (i, &px) in pixels.iter().enumerate() {
            let off = i * 4;
            rgba[off] = ((px >> 16) & 0xFF) as u8; // R
            rgba[off + 1] = ((px >> 8) & 0xFF) as u8; // G
            rgba[off + 2] = (px & 0xFF) as u8; // B
            rgba[off + 3] = ((px >> 24) & 0xFF) as u8; // A
        }
        drop(guard);
        let img = ImageBuffer::from_rgba(w, h, rgba).unwrap_or_else(|| ImageBuffer::new(w, h));
        let handle = next_handle();
        let mut ig = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
        ig.get_or_insert_with(HashMap::new)
            .insert(handle, Box::new(img));
        handle
    } else {
        drop(guard);
        0
    }
}

pub fn image_get_width(handle: u64) -> u32 {
    let guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&handle))
        .map(|i| i.width())
        .unwrap_or(0)
}

pub fn image_get_height(handle: u64) -> u32 {
    let guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&handle))
        .map(|i| i.height())
        .unwrap_or(0)
}

pub fn image_release(handle: u64) {
    let mut guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = guard.as_mut() {
        m.remove(&handle);
    }
}

/// Get the RGBA8 pixel data, width, and height of a CGImage.
/// Returns `None` if the handle is invalid.
pub fn image_get_rgba_data(handle: u64) -> Option<(u32, u32, Vec<u8>)> {
    let guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().and_then(|m| m.get(&handle)).map(|img| {
        let rgba = img.to_rgba8();
        (rgba.width(), rgba.height(), rgba.data().to_vec())
    })
}

/// Get a raw pointer to the RGBA8 pixel data of a CGImage.
///
/// Returns 0 if the handle is invalid.  The pointer is valid only while
/// the IMAGES map entry is alive (i.e. until `image_release`).
pub fn image_data_ptr(handle: u64) -> u64 {
    let mut guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_mut().and_then(|m| m.get_mut(&handle)) {
        Some(img) => img.data_mut().as_mut_ptr() as u64,
        None => 0,
    }
}

/// Draw a CGImage into a CGContext at the given dest rect.
pub fn draw_image_in_context(ctx_handle: u64, image_handle: u64, x: f64, y: f64, w: f64, h: f64) {
    // Read image data under the image lock first.
    let img: Option<ImageBuffer> = {
        let guard = IMAGES.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .and_then(|m| m.get(&image_handle))
            .map(|i| (**i).clone())
    };
    if let Some(img) = img {
        with_context(ctx_handle, |c| c.draw_image(&img, x, y, w, h));
    }
}

// ── Font enumeration ───────────────────────────────────────────

/// Return all available font family names (embedded + system).
pub fn font_family_names() -> Vec<String> {
    graphics_core::text_system::available_font_families()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_create_and_query() {
        let h = context_create(100, 200);
        assert_ne!(h, 0);
        assert_eq!(context_get_width(h), 100);
        assert_eq!(context_get_height(h), 200);
        assert_ne!(context_get_data(h), 0);
        context_release(h);
    }

    #[test]
    fn context_fill_rect() {
        let h = context_create(10, 10);
        with_context(h, |ctx| {
            ctx.fill_color = Color::RED;
            ctx.fill_rect(0.0, 0.0, 10.0, 10.0);
        });
        context_release(h);
    }

    #[test]
    fn path_create_and_build() {
        let h = path_create();
        assert!(path_is_empty(h));
        with_path(h, |p| {
            p.move_to(Point::new(0.0, 0.0));
            p.line_to(Point::new(10.0, 10.0));
        });
        assert!(!path_is_empty(h));
        path_release(h);
    }

    #[test]
    fn color_create_and_query() {
        let h = color_create_rgb(1.0, 0.0, 0.0, 0.5);
        assert_eq!(color_get_alpha(h), 0.5);
        let c = color_get(h).expect("color should exist");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        color_release(h);
    }
}
