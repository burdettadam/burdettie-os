//! Bridge from CoreGraphics C API stubs to `graphics-core` real objects.
//!
//! Provides heap-allocated drawing contexts, paths, and colors that are backed
//! by real `graphics-core` types. CG function stubs call into this module
//! to perform actual drawing on a `Surface` pixel buffer.

use std::collections::HashMap;
use std::sync::Mutex;

use graphics_core::{BezierPath, Color, Point, Rect, Surface, SoftwareRenderer, Renderer};

/// A heap-allocated CGContext backed by a `Surface` + `SoftwareRenderer`.
pub struct CgContext {
    pub surface: Surface,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub line_width: f64,
    pub alpha: f64,
    pub path: BezierPath,
    state_stack: Vec<ContextState>,
}

#[derive(Clone)]
struct ContextState {
    fill_color: Color,
    stroke_color: Color,
    line_width: f64,
    alpha: f64,
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
            state_stack: Vec::new(),
        }
    }

    pub fn save_state(&mut self) {
        self.state_stack.push(ContextState {
            fill_color: self.fill_color,
            stroke_color: self.stroke_color,
            line_width: self.line_width,
            alpha: self.alpha,
        });
    }

    pub fn restore_state(&mut self) {
        if let Some(s) = self.state_stack.pop() {
            self.fill_color = s.fill_color;
            self.stroke_color = s.stroke_color;
            self.line_width = s.line_width;
            self.alpha = s.alpha;
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
