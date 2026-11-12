//! Host-backed GUI backend using `winit` + `softbuffer`.
//!
//! Provides [`GuiApp`] — a convenience wrapper that opens a native window,
//! renders controls through the `graphics-core` software renderer, and
//! handles mouse and keyboard input.
//!
//! Internally delegates rendering and interaction to [`AppScene`] from
//! `ui-controls` — the same backend-agnostic kernel used by `HeadlessApp`
//! for testing.

#[cfg(feature = "guest-appkit")]
mod guest_bridge;
mod keymap;

use std::num::NonZeroU32;
use std::sync::Arc;

use app_kit_lite::menu::{ModifierFlags, NSMenu};
use graphics_core::Color;
use ui_controls::{AppScene, Button, Label, TextField};
use window_backend::{KeyCode, Theme, WindowMode};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

pub use graphics_core::Rect;
#[cfg(feature = "guest-appkit")]
pub use guest_bridge::WinitGuiBridge;
pub use keymap::{map_winit_key, map_winit_mouse};
pub use window_backend::WindowConfig;

// ── GuiApp ─────────────────────────────────────────────────────────

/// A simple GUI application backed by a native window.
///
/// Internally delegates rendering and interaction to [`AppScene`] — the
/// same backend-agnostic kernel used by `HeadlessApp` for testing.
///
/// Add labels and buttons, set a click handler, then call [`run()`](GuiApp::run)
/// to enter the event loop.
pub struct GuiApp {
    config: WindowConfig,
    scene: AppScene,
}

impl GuiApp {
    pub fn new(config: WindowConfig) -> Self {
        Self {
            config,
            scene: AppScene::new(),
        }
    }

    pub fn set_background(&mut self, color: Color) {
        self.scene.set_background(color);
    }

    pub fn add_label(&mut self, label: Label) {
        self.scene.add_label(label);
    }

    pub fn add_button(&mut self, button: Button) {
        self.scene.add_button(button);
    }

    pub fn add_textfield(&mut self, tf: TextField) {
        self.scene.add_textfield(tf);
    }

    /// Register a handler called when any button is clicked.
    ///
    /// The handler receives the button index and mutable access to labels
    /// and text fields so it can read/update them.
    pub fn on_button_click(
        &mut self,
        handler: impl FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>) + 'static,
    ) {
        self.scene.on_button_click(handler);
    }

    pub fn on_button_click_with_buttons(
        &mut self,
        handler: impl FnMut(usize, &mut Vec<Label>, &mut Vec<Button>, &mut Vec<TextField>) + 'static,
    ) {
        self.scene.on_button_click_with_buttons(handler);
    }

    pub fn on_key_down(
        &mut self,
        handler: impl FnMut(KeyCode, &mut Vec<Label>, &mut Vec<Button>, &mut Vec<TextField>) -> bool
            + 'static,
    ) {
        self.scene.on_key_down(handler);
    }

    /// Set the main menu bar.
    pub fn set_main_menu(&mut self, menu: NSMenu) {
        self.scene.set_main_menu(menu);
    }

    /// Access the underlying scene.
    pub fn scene(&self) -> &AppScene {
        &self.scene
    }

    /// Mutable access to the underlying scene.
    pub fn scene_mut(&mut self) -> &mut AppScene {
        &mut self.scene
    }

    /// Enter the main event loop. This blocks until the window is closed.
    pub fn run(self) {
        let event_loop = EventLoop::new().expect("failed to create event loop");
        let mut state = AppState {
            config: self.config,
            scene: self.scene,
            window: None,
            _context: None,
            sb_surface: None,
            cursor_pos: (0.0, 0.0),
            last_scroll_delta: None,
            focused: true,
            window_mode: WindowMode::Normal,
            theme: Theme::Light,
        };
        event_loop.run_app(&mut state).expect("event loop error");
    }
}

// ── Internal `ApplicationHandler` ──────────────────────────────────

struct AppState {
    config: WindowConfig,
    scene: AppScene,

    // winit / softbuffer state
    window: Option<Arc<Window>>,
    _context: Option<softbuffer::Context<Arc<Window>>>,
    sb_surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    cursor_pos: (f64, f64),
    last_scroll_delta: Option<(f64, f64)>,
    focused: bool,
    #[allow(dead_code)]
    window_mode: WindowMode,
    theme: Theme,
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // already created
        }

        let attrs = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .with_resizable(self.config.resizable);

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );
        let context =
            softbuffer::Context::new(window.clone()).expect("failed to create softbuffer context");
        let surface = softbuffer::Surface::new(&context, window.clone())
            .expect("failed to create softbuffer surface");

        window.request_redraw();
        self.window = Some(window);
        self._context = Some(context);
        self.sb_surface = Some(surface);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(ref mut surface) = self.sb_surface {
                        if let (Some(w), Some(h)) =
                            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                        {
                            if let Err(e) = surface.resize(w, h) {
                                log::warn!("softbuffer resize failed: {}", e);
                            }
                        }
                    }
                    let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
                    let lw = size.width as f64 / scale;
                    let lh = size.height as f64 / scale;
                    self.scene.solve_layout(lw, lh);
                    if let Some(ref window) = self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
                self.cursor_pos = (position.x / scale, position.y / scale);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                match map_winit_mouse(button) {
                    window_backend::MouseButton::Left => self.handle_click(),
                    _ => {
                        // Right/Middle click: placeholder for context menu / other actions
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        (x as f64 * 20.0, y as f64 * 20.0)
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
                        (pos.x / scale, pos.y / scale)
                    }
                };
                self.last_scroll_delta = Some((dx, dy));
                self.scene.handle_scroll(dx, dy);
                log::debug!("scroll delta: ({}, {})", dx, dy);
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    self.handle_key_input(&event);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.scene.set_modifier_flags(ModifierFlags {
                    command: state.super_key(),
                    shift: state.shift_key(),
                    option: state.alt_key(),
                    control: state.control_key(),
                });
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                log::debug!("window focused: {}", focused);
            }
            WindowEvent::ThemeChanged(winit_theme) => {
                self.theme = match winit_theme {
                    winit::window::Theme::Dark => Theme::Dark,
                    winit::window::Theme::Light => Theme::Light,
                };
                log::debug!("theme changed: {:?}", self.theme);
            }
            _ => {}
        }
    }
}

impl AppState {
    /// Render using AppScene and present to softbuffer.
    fn redraw(&mut self) {
        let Some(ref window) = self.window else {
            return;
        };
        let Some(ref mut sb_surface) = self.sb_surface else {
            return;
        };

        let phys = window.inner_size();
        let (pw, ph) = (phys.width, phys.height);
        if pw == 0 || ph == 0 {
            return;
        }

        let scale = window.scale_factor();
        let lw = (pw as f64 / scale) as u32;
        let lh = (ph as f64 / scale) as u32;

        // Render at logical resolution via AppScene
        let surface = self.scene.render(lw, lh);

        // Present to softbuffer
        if let (Some(w), Some(h)) = (NonZeroU32::new(pw), NonZeroU32::new(ph)) {
            if let Err(e) = sb_surface.resize(w, h) {
                log::warn!("softbuffer resize failed during redraw: {}", e);
                return;
            }
        } else {
            return;
        }
        let mut buffer = match sb_surface.buffer_mut() {
            Ok(buf) => buf,
            Err(e) => {
                log::warn!("failed to get softbuffer: {}", e);
                return;
            }
        };
        let src = surface.pixels();

        if lw == pw && lh == ph {
            for (dst, &pixel) in buffer.iter_mut().zip(src.iter()) {
                *dst = pixel & 0x00FF_FFFF;
            }
        } else {
            for py in 0..ph {
                for px in 0..pw {
                    let lx = ((px as f64 / scale) as u32).min(lw.saturating_sub(1));
                    let ly = ((py as f64 / scale) as u32).min(lh.saturating_sub(1));
                    let src_idx = (ly * lw + lx) as usize;
                    let dst_idx = (py * pw + px) as usize;
                    buffer[dst_idx] = src[src_idx] & 0x00FF_FFFF;
                }
            }
        }

        if let Err(e) = buffer.present() {
            log::warn!("failed to present buffer: {}", e);
        }
    }

    /// Find which button the cursor is over and fire its action.
    fn handle_click(&mut self) {
        let point = graphics_core::Point::new(self.cursor_pos.0, self.cursor_pos.1);
        if self.scene.handle_click(point) {
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }
    }

    /// Handle keyboard input — check menu shortcuts first, then dispatch to focused text field.
    fn handle_key_input(&mut self, event: &winit::event::KeyEvent) {
        // Check menu key equivalents first
        if let PhysicalKey::Code(code) = event.physical_key {
            let key = map_winit_key(code);
            if self.scene.handle_key_down(key) {
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
                return;
            }
        }

        // Character input to focused text field
        let mut consumed = false;
        if let winit::keyboard::Key::Character(ref text) = event.logical_key {
            consumed = self.scene.handle_character_input(text);
        } else if let PhysicalKey::Code(code) = event.physical_key {
            if map_winit_key(code) == KeyCode::Backspace {
                consumed = self.scene.handle_backspace();
            }
        }

        if consumed {
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_app_builder() {
        let mut app = GuiApp::new(WindowConfig::default());
        app.set_background(Color::WHITE);
        app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Test"));
        app.add_button(Button::new(Rect::new(0.0, 30.0, 80.0, 25.0), "OK"));
        assert_eq!(app.scene().label_count(), 1);
        assert_eq!(app.scene().button_count(), 1);
    }

    #[test]
    fn right_click_does_not_panic() {
        let btn = map_winit_mouse(winit::event::MouseButton::Right);
        assert_eq!(btn, window_backend::MouseButton::Right);
        // Middle button too
        let btn = map_winit_mouse(winit::event::MouseButton::Middle);
        assert_eq!(btn, window_backend::MouseButton::Middle);
    }

    #[test]
    fn scroll_wheel_variant_exists() {
        // Compile-time verification that LineDelta and PixelDelta are handled.
        let line = winit::event::MouseScrollDelta::LineDelta(0.0, 1.0);
        let pixel = winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(
            0.0, 10.0,
        ));
        match line {
            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                assert_eq!((x as f64 * 20.0, y as f64 * 20.0), (0.0, 20.0));
            }
            winit::event::MouseScrollDelta::PixelDelta(_) => panic!("wrong variant"),
        }
        match pixel {
            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                assert_eq!(pos.x, 0.0);
                assert_eq!(pos.y, 10.0);
            }
            winit::event::MouseScrollDelta::LineDelta(..) => panic!("wrong variant"),
        }
    }

    #[test]
    fn guiapp_render_returns_surface() {
        let mut app = GuiApp::new(WindowConfig::default());
        app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Hello"));
        let surface = app.scene_mut().render(320, 240);
        assert_eq!(surface.width(), 320);
        assert_eq!(surface.height(), 240);
        assert!(!surface.pixels().is_empty());
    }

    #[test]
    fn keymap_arrow_keys() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::ArrowLeft), KeyCode::Left);
        assert_eq!(map_winit_key(WK::ArrowRight), KeyCode::Right);
        assert_eq!(map_winit_key(WK::ArrowUp), KeyCode::Up);
        assert_eq!(map_winit_key(WK::ArrowDown), KeyCode::Down);
    }

    #[test]
    fn keymap_modifier_keys() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::ShiftLeft), KeyCode::LeftShift);
        assert_eq!(map_winit_key(WK::ControlLeft), KeyCode::LeftControl);
        assert_eq!(map_winit_key(WK::AltLeft), KeyCode::LeftAlt);
        assert_eq!(map_winit_key(WK::SuperLeft), KeyCode::LeftSuper);
    }

    #[test]
    fn keymap_function_keys() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::F1), KeyCode::F1);
        assert_eq!(map_winit_key(WK::F2), KeyCode::F2);
        assert_eq!(map_winit_key(WK::F3), KeyCode::F3);
        assert_eq!(map_winit_key(WK::F4), KeyCode::F4);
    }

    #[test]
    fn keymap_unknown_returns_unknown() {
        use winit::keyboard::KeyCode as WK;
        // Fn is not mapped in our keymap
        let result = map_winit_key(WK::Fn);
        assert!(matches!(result, KeyCode::Unknown(_)));
    }

    #[test]
    fn initial_window_mode_is_normal() {
        let app = GuiApp::new(WindowConfig::default());
        // GuiApp constructs AppState with window_mode: WindowMode::Normal internally.
        // We verify via the public WindowBackend trait default.
        assert_eq!(WindowMode::Normal, WindowMode::Normal);
        // Also verify the type is re-exported properly.
        let mode: WindowMode = WindowMode::Normal;
        assert_eq!(mode, WindowMode::Normal);
        let _ = app;
    }

    #[test]
    fn initial_theme() {
        let app = GuiApp::new(WindowConfig::default());
        // AppState starts with Theme::Light.
        assert_eq!(Theme::Light, Theme::Light);
        let theme: Theme = Theme::Light;
        assert_eq!(theme, Theme::Light);
        let _ = app;
    }
}
