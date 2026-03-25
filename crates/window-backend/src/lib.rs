//! Windowing abstraction trait.
//!
//! Defines a `WindowBackend` trait that platform-specific backends implement
//! to create windows, handle input events, and present rendered surfaces.
//! This abstraction allows the GUI framework (app-kit-lite, ui-controls)
//! to work identically on any platform.

mod types;

pub use types::*;

use graphics_core::Surface;

// ── Backend trait ──────────────────────────────────────────────────

/// Trait that platform window backends implement.
///
/// This is the core abstraction that makes the GUI framework backend-agnostic.
/// Implementations handle platform-specific window creation, event delivery,
/// and surface presentation.
pub trait WindowBackend: Send + Sync {
    /// Create a new window and return its identifier.
    fn create_window(&self, config: WindowConfig) -> WindowId;

    /// Destroy a previously created window.
    fn destroy_window(&self, id: WindowId);

    /// Set the title of a window.
    fn set_title(&self, id: WindowId, title: &str);

    /// Resize a window.
    fn set_size(&self, id: WindowId, width: f64, height: f64);

    /// Present a rendered surface to the window. The backend is responsible
    /// for scaling to the physical resolution (HiDPI).
    fn present_surface(&self, id: WindowId, surface: &Surface);

    /// Drain pending events from the backend. Returns all events since
    /// the last call.
    fn poll_events(&self) -> Vec<WindowEvent>;

    /// Request that the window be redrawn. The backend will deliver a
    /// `RedrawRequested` event at the appropriate time.
    fn request_redraw(&self, id: WindowId);

    /// Query the window's current logical size. Returns (width, height).
    fn window_size(&self, id: WindowId) -> (f64, f64) {
        let _ = id;
        (0.0, 0.0)
    }

    /// Query the display scale factor for the window. Returns 1.0 by default.
    fn scale_factor(&self, id: WindowId) -> f64 {
        let _ = id;
        1.0
    }
}

// ── Headless backend (for testing) ─────────────────────────────────

use std::sync::Mutex;

struct HeadlessWindow {
    config: WindowConfig,
}

/// A no-op window backend for headless testing.
///
/// Supports event injection, window size tracking, and configurable
/// scale factor — everything needed to test GUI logic without a display.
pub struct HeadlessBackend {
    next_id: Mutex<u64>,
    events: Mutex<Vec<WindowEvent>>,
    windows: Mutex<Vec<(WindowId, HeadlessWindow)>>,
    scale: f64,
}

impl HeadlessBackend {
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
            events: Mutex::new(Vec::new()),
            windows: Mutex::new(Vec::new()),
            scale: 1.0,
        }
    }

    /// Create a headless backend with a custom scale factor.
    pub fn with_scale(scale: f64) -> Self {
        Self {
            next_id: Mutex::new(1),
            events: Mutex::new(Vec::new()),
            windows: Mutex::new(Vec::new()),
            scale,
        }
    }

    /// Inject an event into the queue (for testing).
    pub fn inject_event(&self, event: WindowEvent) {
        self.events.lock().unwrap_or_else(|e| e.into_inner()).push(event);
    }
}

impl Default for HeadlessBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowBackend for HeadlessBackend {
    fn create_window(&self, config: WindowConfig) -> WindowId {
        let mut id = self.next_id.lock().unwrap_or_else(|e| e.into_inner());
        let wid = WindowId(*id);
        *id += 1;
        self.windows
            .lock()
            .unwrap()
            .push((wid, HeadlessWindow { config }));
        wid
    }

    fn destroy_window(&self, id: WindowId) {
        self.windows.lock().unwrap_or_else(|e| e.into_inner()).retain(|(wid, _)| *wid != id);
    }

    fn set_title(&self, id: WindowId, title: &str) {
        if let Some((_, w)) = self
            .windows
            .lock()
            .unwrap()
            .iter_mut()
            .find(|(wid, _)| *wid == id)
        {
            w.config.title = title.to_string();
        }
    }

    fn set_size(&self, id: WindowId, width: f64, height: f64) {
        if let Some((_, w)) = self
            .windows
            .lock()
            .unwrap()
            .iter_mut()
            .find(|(wid, _)| *wid == id)
        {
            w.config.width = width;
            w.config.height = height;
        }
    }

    fn present_surface(&self, _id: WindowId, _surface: &Surface) {}

    fn poll_events(&self) -> Vec<WindowEvent> {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *events)
    }

    fn request_redraw(&self, _id: WindowId) {}

    fn window_size(&self, id: WindowId) -> (f64, f64) {
        self.windows
            .lock()
            .unwrap()
            .iter()
            .find(|(wid, _)| *wid == id)
            .map(|(_, w)| (w.config.width, w.config.height))
            .unwrap_or((0.0, 0.0))
    }

    fn scale_factor(&self, _id: WindowId) -> f64 {
        self.scale
    }
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

    #[test]
    fn headless_window_size() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig {
            title: "test".into(),
            width: 400.0,
            height: 300.0,
            resizable: false,
        });
        assert_eq!(backend.window_size(wid), (400.0, 300.0));
    }

    #[test]
    fn headless_set_size() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.set_size(wid, 1024.0, 768.0);
        assert_eq!(backend.window_size(wid), (1024.0, 768.0));
    }

    #[test]
    fn headless_set_title() {
        let backend = HeadlessBackend::new();
        let _wid = backend.create_window(WindowConfig::default());
        // Just verifying no panic
        backend.set_title(_wid, "New Title");
    }

    #[test]
    fn headless_destroy_window() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        assert_eq!(backend.window_size(wid), (800.0, 600.0));
        backend.destroy_window(wid);
        assert_eq!(backend.window_size(wid), (0.0, 0.0));
    }

    #[test]
    fn headless_scale_factor() {
        let backend = HeadlessBackend::with_scale(2.0);
        let wid = backend.create_window(WindowConfig::default());
        assert_eq!(backend.scale_factor(wid), 2.0);
    }

    #[test]
    fn headless_modifiers_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::ModifiersChanged(
            wid,
            Modifiers {
                command: true,
                ..Default::default()
            },
        ));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], WindowEvent::ModifiersChanged(_, _)));
    }

    #[test]
    fn headless_scroll_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::ScrollWheel(wid, 0.0, -10.0));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn headless_character_input() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::CharacterInput(wid, "a".into()));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], WindowEvent::CharacterInput(_, _)));
    }
}
