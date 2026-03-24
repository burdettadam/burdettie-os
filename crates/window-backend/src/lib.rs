//! Windowing abstraction trait.
//!
//! Defines a `WindowBackend` trait that platform-specific backends implement
//! to create windows, handle input events, and present rendered surfaces.

use graphics_core::Surface;

// ── Window identity ────────────────────────────────────────────────

/// Opaque identifier for a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

// ── Window configuration ───────────────────────────────────────────

/// Configuration for creating a new window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "burdetty-os".to_string(),
            width: 800.0,
            height: 600.0,
            resizable: true,
        }
    }
}

// ── Input types ────────────────────────────────────────────────────

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Keyboard key codes (subset covering common keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Space,
    Return,
    Escape,
    Tab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Unknown(u32),
}

// ── Window events ──────────────────────────────────────────────────

/// Events produced by the windowing system.
#[derive(Debug, Clone)]
pub enum WindowEvent {
    CloseRequested(WindowId),
    Resized(WindowId, f64, f64),
    KeyDown(WindowId, KeyCode),
    KeyUp(WindowId, KeyCode),
    MouseDown(WindowId, f64, f64, MouseButton),
    MouseUp(WindowId, f64, f64, MouseButton),
    MouseMoved(WindowId, f64, f64),
    RedrawRequested(WindowId),
}

// ── Backend trait ──────────────────────────────────────────────────

/// Trait that platform window backends implement.
pub trait WindowBackend: Send + Sync {
    fn create_window(&self, config: WindowConfig) -> WindowId;
    fn destroy_window(&self, id: WindowId);
    fn set_title(&self, id: WindowId, title: &str);
    fn set_size(&self, id: WindowId, width: f64, height: f64);
    fn present_surface(&self, id: WindowId, surface: &Surface);
    fn poll_events(&self) -> Vec<WindowEvent>;
    fn request_redraw(&self, id: WindowId);
}

// ── Headless backend (for testing) ─────────────────────────────────

use std::sync::Mutex;

/// A no-op window backend for headless testing.
pub struct HeadlessBackend {
    next_id: Mutex<u64>,
    events: Mutex<Vec<WindowEvent>>,
}

impl HeadlessBackend {
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
            events: Mutex::new(Vec::new()),
        }
    }

    /// Inject an event into the queue (for testing).
    pub fn inject_event(&self, event: WindowEvent) {
        self.events.lock().unwrap().push(event);
    }
}

impl Default for HeadlessBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowBackend for HeadlessBackend {
    fn create_window(&self, _config: WindowConfig) -> WindowId {
        let mut id = self.next_id.lock().unwrap();
        let wid = WindowId(*id);
        *id += 1;
        wid
    }

    fn destroy_window(&self, _id: WindowId) {}

    fn set_title(&self, _id: WindowId, _title: &str) {}

    fn set_size(&self, _id: WindowId, _width: f64, _height: f64) {}

    fn present_surface(&self, _id: WindowId, _surface: &Surface) {}

    fn poll_events(&self) -> Vec<WindowEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }

    fn request_redraw(&self, _id: WindowId) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_create_window() {
        let backend = HeadlessBackend::new();
        let w1 = backend.create_window(WindowConfig::default());
        let w2 = backend.create_window(WindowConfig::default());
        assert_ne!(w1, w2);
    }

    #[test]
    fn headless_inject_and_poll_events() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());

        backend.inject_event(WindowEvent::CloseRequested(wid));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);

        // Queue should be drained
        let events = backend.poll_events();
        assert!(events.is_empty());
    }

    #[test]
    fn window_config_default() {
        let cfg = WindowConfig::default();
        assert_eq!(cfg.width, 800.0);
        assert_eq!(cfg.height, 600.0);
        assert!(cfg.resizable);
    }
}
