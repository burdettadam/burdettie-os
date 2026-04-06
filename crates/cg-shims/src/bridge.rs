//! Bridge from CoreGraphics C API stubs to `graphics-core` real objects.
//!
//! Provides heap-allocated drawing contexts, paths, and colors that are backed
//! by real `graphics-core` types. CG function stubs call into this module
//! to perform actual drawing on a `Surface` pixel buffer.

use std::collections::HashMap;
use std::sync::Mutex;

use graphics_core::{BezierPath, Color, Font, FontManager, Rect, Surface, SoftwareRenderer, Renderer, Point};

/// Global shared font manager (TrueType via ab_glyph).
static FONT_MANAGER: std::sync::OnceLock<FontManager> = std::sync::OnceLock::new();

fn font_manager() -> &'static FontManager {
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
        }
    }

    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.fill_rect(Rect::new(x, y, w, h), self.fill_color);
    }

    pub fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.stroke_rect(Rect::new(x, y, w, h), self.stroke_color, self.line_width);
    }

    pub fn fill_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.fill_ellipse(Rect::new(x, y, w, h), self.fill_color);
    }

    pub fn fill_path(&mut self) {
        let path = std::mem::take(&mut self.path);
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.fill_path(&path, self.fill_color);
    }

    pub fn stroke_path(&mut self) {
        let path = std::mem::take(&mut self.path);
        let mut renderer = SoftwareRenderer::new(&mut self.surface);
        renderer.stroke_path(&path, self.stroke_color, self.line_width);
    }

    /// Draw text at (x, y) using the current fill color and font size.
    /// After drawing, advances text_x by the width of the rendered string.
    pub fn draw_text(&mut self, text: &str, x: f64, y: f64) {
        let font = Font::system(self.font_size);
        let fm = font_manager();
        let mut renderer = SoftwareRenderer::with_fonts(&mut self.surface, fm);
        renderer.draw_text(text, Point::new(x, y), &font, self.fill_color);
        // Advance text position by the measured width.
        let advance = fm.system().measure_text(text, self.font_size);
        self.text_x = x + advance;
        self.text_y = y;
    }
}

// ── Handle table ───────────────────────────────────────────────

static NEXT_HANDLE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0x4347_4358_1000);

static CONTEXTS: Mutex<Option<HashMap<u64, Box<CgContext>>>> = Mutex::new(None);
static PATHS: Mutex<Option<HashMap<u64, Box<BezierPath>>>> = Mutex::new(None);
static COLORS: Mutex<Option<HashMap<u64, Color>>> = Mutex::new(None);

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
    if let Some(m) = guard.as_mut() { m.remove(&handle); }
}

/// Get the raw pixel data pointer for `CGBitmapContextGetData`.
pub fn context_get_data(handle: u64) -> u64 {
    let guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()
        .and_then(|m| m.get(&handle))
        .map(|ctx| ctx.surface.pixels().as_ptr() as u64)
        .unwrap_or(0)
}

/// Get width for `CGBitmapContextGetWidth`.
pub fn context_get_width(handle: u64) -> u64 {
    let guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()
        .and_then(|m| m.get(&handle))
        .map(|ctx| ctx.surface.width() as u64)
        .unwrap_or(0)
}

/// Get height for `CGBitmapContextGetHeight`.
pub fn context_get_height(handle: u64) -> u64 {
    let guard = CONTEXTS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()
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
    if let Some(m) = guard.as_mut() { m.remove(&handle); }
}

pub fn path_is_empty(handle: u64) -> bool {
    let guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()
        .and_then(|m| m.get(&handle))
        .map(|p| p.elements().is_empty())
        .unwrap_or(true)
}

/// Copy path elements from a standalone path handle into a context's path.
pub fn copy_path_to_context(path_handle: u64, ctx_handle: u64) {
    // Collect elements under the path lock first, then apply under the context lock.
    let elements: Vec<graphics_core::PathElement> = {
        let guard = PATHS.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_ref()
            .and_then(|m| m.get(&path_handle))
            .map(|p| p.elements().to_vec())
            .unwrap_or_default()
    };
    if elements.is_empty() { return; }
    with_context(ctx_handle, |c| {
        for elem in &elements {
            match *elem {
                graphics_core::PathElement::MoveTo(p) => { c.path.move_to(p); }
                graphics_core::PathElement::LineTo(p) => { c.path.line_to(p); }
                graphics_core::PathElement::QuadTo { ctrl, end } => { c.path.quad_to(ctrl, end); }
                graphics_core::PathElement::CubicTo { ctrl1, ctrl2, end } => { c.path.cubic_to(ctrl1, ctrl2, end); }
                graphics_core::PathElement::Close => { c.path.close(); }
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
    if let Some(m) = guard.as_mut() { m.remove(&handle); }
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
