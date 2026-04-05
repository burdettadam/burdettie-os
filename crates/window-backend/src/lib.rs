//! Windowing abstraction trait.
//!
//! Defines a `WindowBackend` trait that platform-specific backends implement
//! to create windows, handle input events, and present rendered surfaces.
//! This abstraction allows the GUI framework (app-kit-lite, ui-controls)
//! to work identically on any platform.

mod types;
pub mod keyboard_layout;

pub use keyboard_layout::KeyboardLayout;
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

    /// Get the title of a window.
    fn title(&self, _id: WindowId) -> Option<String> { None }

    /// Set whether a window is resizable.
    fn set_resizable(&self, _id: WindowId, _resizable: bool) {}

    /// Query whether a window is resizable.
    fn is_resizable(&self, _id: WindowId) -> bool { true }

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

    /// Set the cursor icon for the window.
    fn set_cursor(&mut self, _icon: CursorIcon) {}

    /// Set the window display mode.
    fn set_window_mode(&mut self, _mode: WindowMode) {}

    /// Query the current window display mode.
    fn window_mode(&self) -> WindowMode { WindowMode::Normal }

    /// Query the current UI theme.
    fn theme(&self) -> Theme { Theme::Light }
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
    cursor: Mutex<CursorIcon>,
    window_mode: Mutex<WindowMode>,
    theme: Mutex<Theme>,
}

impl HeadlessBackend {
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
            events: Mutex::new(Vec::new()),
            windows: Mutex::new(Vec::new()),
            scale: 1.0,
            cursor: Mutex::new(CursorIcon::Default),
            window_mode: Mutex::new(WindowMode::Normal),
            theme: Mutex::new(Theme::Light),
        }
    }

    /// Create a headless backend with a custom scale factor.
    pub fn with_scale(scale: f64) -> Self {
        Self {
            next_id: Mutex::new(1),
            events: Mutex::new(Vec::new()),
            windows: Mutex::new(Vec::new()),
            scale,
            cursor: Mutex::new(CursorIcon::Default),
            window_mode: Mutex::new(WindowMode::Normal),
            theme: Mutex::new(Theme::Light),
        }
    }

    /// Get the current cursor icon.
    pub fn cursor(&self) -> CursorIcon {
        *self.cursor.lock().unwrap_or_else(|e| e.into_inner())
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

    fn title(&self, id: WindowId) -> Option<String> {
        self.windows
            .lock()
            .unwrap()
            .iter()
            .find(|(wid, _)| *wid == id)
            .map(|(_, w)| w.config.title.clone())
    }

    fn set_resizable(&self, id: WindowId, resizable: bool) {
        if let Some((_, w)) = self
            .windows
            .lock()
            .unwrap()
            .iter_mut()
            .find(|(wid, _)| *wid == id)
        {
            w.config.resizable = resizable;
        }
    }

    fn is_resizable(&self, id: WindowId) -> bool {
        self.windows
            .lock()
            .unwrap()
            .iter()
            .find(|(wid, _)| *wid == id)
            .map(|(_, w)| w.config.resizable)
            .unwrap_or(true)
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

    fn set_cursor(&mut self, icon: CursorIcon) {
        *self.cursor.lock().unwrap_or_else(|e| e.into_inner()) = icon;
    }

    fn set_window_mode(&mut self, mode: WindowMode) {
        *self.window_mode.lock().unwrap_or_else(|e| e.into_inner()) = mode;
    }

    fn window_mode(&self) -> WindowMode {
        *self.window_mode.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn theme(&self) -> Theme {
        *self.theme.lock().unwrap_or_else(|e| e.into_inner())
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

    #[test]
    fn headless_mouse_moved_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::MouseMoved(wid, 120.0, 45.5));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            WindowEvent::MouseMoved(id, x, y) => {
                assert_eq!(*id, wid);
                assert_eq!(*x, 120.0);
                assert_eq!(*y, 45.5);
            }
            other => panic!("expected MouseMoved, got {:?}", other),
        }
    }

    #[test]
    fn headless_mouse_up_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::MouseUp(wid, 50.0, 60.0, MouseButton::Left));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            WindowEvent::MouseUp(id, x, y, btn) => {
                assert_eq!(*id, wid);
                assert_eq!(*x, 50.0);
                assert_eq!(*y, 60.0);
                assert_eq!(*btn, MouseButton::Left);
            }
            other => panic!("expected MouseUp, got {:?}", other),
        }
    }

    #[test]
    fn headless_key_up_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::KeyUp(wid, KeyCode::A));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            WindowEvent::KeyUp(id, key) => {
                assert_eq!(*id, wid);
                assert_eq!(*key, KeyCode::A);
            }
            other => panic!("expected KeyUp, got {:?}", other),
        }
    }

    #[test]
    fn headless_redraw_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::RedrawRequested(wid));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], WindowEvent::RedrawRequested(_)));
    }

    #[test]
    fn headless_close_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::CloseRequested(wid));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], WindowEvent::CloseRequested(_)));
    }

    #[test]
    fn headless_resize_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::Resized(wid, 1920.0, 1080.0));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            WindowEvent::Resized(id, w, h) => {
                assert_eq!(*id, wid);
                assert_eq!(*w, 1920.0);
                assert_eq!(*h, 1080.0);
            }
            other => panic!("expected Resized, got {:?}", other),
        }
    }

    #[test]
    fn headless_multiple_events() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::KeyDown(wid, KeyCode::Space));
        backend.inject_event(WindowEvent::MouseMoved(wid, 10.0, 20.0));
        backend.inject_event(WindowEvent::CloseRequested(wid));
        let events = backend.poll_events();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], WindowEvent::KeyDown(_, KeyCode::Space)));
        assert!(matches!(events[1], WindowEvent::MouseMoved(_, _, _)));
        assert!(matches!(events[2], WindowEvent::CloseRequested(_)));
    }

    #[test]
    fn headless_poll_clears_queue() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::RedrawRequested(wid));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        let events = backend.poll_events();
        assert!(events.is_empty());
    }

    #[test]
    fn headless_right_click_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::MouseDown(wid, 100.0, 200.0, MouseButton::Right));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            WindowEvent::MouseDown(id, x, y, btn) => {
                assert_eq!(*id, wid);
                assert_eq!(*x, 100.0);
                assert_eq!(*y, 200.0);
                assert_eq!(*btn, MouseButton::Right);
            }
            other => panic!("expected MouseDown Right, got {:?}", other),
        }
    }

    #[test]
    fn headless_middle_click_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::MouseDown(wid, 50.0, 75.0, MouseButton::Middle));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            WindowEvent::MouseDown(id, x, y, btn) => {
                assert_eq!(*id, wid);
                assert_eq!(*x, 50.0);
                assert_eq!(*y, 75.0);
                assert_eq!(*btn, MouseButton::Middle);
            }
            other => panic!("expected MouseDown Middle, got {:?}", other),
        }
    }

    #[test]
    fn keycode_unknown_variant_equality() {
        assert_eq!(KeyCode::Unknown(42), KeyCode::Unknown(42));
        assert_ne!(KeyCode::Unknown(42), KeyCode::Unknown(1));
        assert_ne!(KeyCode::Unknown(0), KeyCode::A);
    }

    #[test]
    fn modifiers_non_default_fields() {
        let m = Modifiers {
            command: true,
            shift: false,
            option: true,
            control: false,
        };
        assert!(m.command);
        assert!(!m.shift);
        assert!(m.option);
        assert!(!m.control);

        let def = Modifiers::default();
        assert!(!def.command);
        assert!(!def.shift);
        assert!(!def.option);
        assert!(!def.control);
        assert_ne!(m, def);
    }

    #[test]
    fn headless_set_cursor() {
        let mut backend = HeadlessBackend::new();
        assert_eq!(backend.cursor(), CursorIcon::Default);
        backend.set_cursor(CursorIcon::Pointer);
        assert_eq!(backend.cursor(), CursorIcon::Pointer);
    }

    #[test]
    fn headless_window_mode_normal() {
        let backend = HeadlessBackend::new();
        assert_eq!(backend.window_mode(), WindowMode::Normal);
    }

    #[test]
    fn headless_set_window_mode() {
        let mut backend = HeadlessBackend::new();
        backend.set_window_mode(WindowMode::Fullscreen);
        assert_eq!(backend.window_mode(), WindowMode::Fullscreen);
    }

    #[test]
    fn headless_theme_default_light() {
        let backend = HeadlessBackend::new();
        assert_eq!(backend.theme(), Theme::Light);
    }

    #[test]
    fn headless_theme_changed_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::ThemeChanged(wid, Theme::Dark));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            WindowEvent::ThemeChanged(id, theme) => {
                assert_eq!(*id, wid);
                assert_eq!(*theme, Theme::Dark);
            }
            other => panic!("expected ThemeChanged, got {:?}", other),
        }
    }

    #[test]
    fn headless_focused_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::Focused(wid, true));
        backend.inject_event(WindowEvent::Focused(wid, false));
        let events = backend.poll_events();
        assert_eq!(events.len(), 2);
        match &events[0] {
            WindowEvent::Focused(id, focused) => {
                assert_eq!(*id, wid);
                assert!(*focused);
            }
            other => panic!("expected Focused(true), got {:?}", other),
        }
        match &events[1] {
            WindowEvent::Focused(id, focused) => {
                assert_eq!(*id, wid);
                assert!(!(*focused));
            }
            other => panic!("expected Focused(false), got {:?}", other),
        }
    }

    #[test]
    fn headless_dropped_file_event() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.inject_event(WindowEvent::DroppedFile(wid, "/tmp/test.txt".into()));
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            WindowEvent::DroppedFile(id, path) => {
                assert_eq!(*id, wid);
                assert_eq!(path, "/tmp/test.txt");
            }
            other => panic!("expected DroppedFile, got {:?}", other),
        }
    }

    #[test]
    fn cursor_icon_equality() {
        assert_eq!(CursorIcon::Default, CursorIcon::Default);
        assert_ne!(CursorIcon::Default, CursorIcon::Pointer);
        assert_eq!(CursorIcon::Text, CursorIcon::Text);
        assert_ne!(CursorIcon::Move, CursorIcon::NotAllowed);
    }

    #[test]
    fn window_mode_equality() {
        assert_eq!(WindowMode::Normal, WindowMode::Normal);
        assert_ne!(WindowMode::Normal, WindowMode::Fullscreen);
        assert_eq!(WindowMode::Maximized, WindowMode::Maximized);
        assert_ne!(WindowMode::Minimized, WindowMode::Maximized);
    }

    #[test]
    fn theme_equality() {
        assert_eq!(Theme::Light, Theme::Light);
        assert_eq!(Theme::Dark, Theme::Dark);
        assert_ne!(Theme::Light, Theme::Dark);
    }

    #[test]
    fn headless_set_title_verify() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        backend.set_title(wid, "My Window");
        assert_eq!(backend.title(wid), Some("My Window".to_string()));
    }

    #[test]
    fn headless_title_default_none() {
        let backend = HeadlessBackend::new();
        let wid = WindowId(999);
        assert_eq!(backend.title(wid), None);
    }

    #[test]
    fn headless_set_resizable() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        assert!(backend.is_resizable(wid));
        backend.set_resizable(wid, false);
        assert!(!backend.is_resizable(wid));
    }

    #[test]
    fn headless_is_resizable_default_true() {
        let backend = HeadlessBackend::new();
        let wid = backend.create_window(WindowConfig::default());
        assert!(backend.is_resizable(wid));
    }

    #[test]
    fn headless_multiple_window_titles() {
        let backend = HeadlessBackend::new();
        let w1 = backend.create_window(WindowConfig::default());
        let w2 = backend.create_window(WindowConfig::default());
        backend.set_title(w1, "First");
        backend.set_title(w2, "Second");
        assert_eq!(backend.title(w1), Some("First".to_string()));
        assert_eq!(backend.title(w2), Some("Second".to_string()));
    }
}
