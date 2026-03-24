//! Host-backed GUI backend using `winit` + `softbuffer`.
//!
//! Provides [`GuiApp`] — a convenience wrapper that opens a native window,
//! renders [`Label`]s and [`Button`]s through the `graphics-core` software
//! renderer, and handles mouse and keyboard input.

use std::num::NonZeroU32;
use std::sync::Arc;

use graphics_core::{Color, Point, Renderer as _, SoftwareRenderer, Surface};
use ui_controls::{Button, Label, TextField};
use window_backend::KeyCode;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

pub use graphics_core::Rect;
pub use window_backend::WindowConfig;

// ── Key / mouse mapping ───────────────────────────────────────────

/// Map a `winit` physical key code to our [`KeyCode`].
pub fn map_winit_key(key: winit::keyboard::KeyCode) -> KeyCode {
    use winit::keyboard::KeyCode as WK;
    match key {
        WK::KeyA => KeyCode::A,
        WK::KeyB => KeyCode::B,
        WK::KeyC => KeyCode::C,
        WK::KeyD => KeyCode::D,
        WK::KeyE => KeyCode::E,
        WK::KeyF => KeyCode::F,
        WK::KeyG => KeyCode::G,
        WK::KeyH => KeyCode::H,
        WK::KeyI => KeyCode::I,
        WK::KeyJ => KeyCode::J,
        WK::KeyK => KeyCode::K,
        WK::KeyL => KeyCode::L,
        WK::KeyM => KeyCode::M,
        WK::KeyN => KeyCode::N,
        WK::KeyO => KeyCode::O,
        WK::KeyP => KeyCode::P,
        WK::KeyQ => KeyCode::Q,
        WK::KeyR => KeyCode::R,
        WK::KeyS => KeyCode::S,
        WK::KeyT => KeyCode::T,
        WK::KeyU => KeyCode::U,
        WK::KeyV => KeyCode::V,
        WK::KeyW => KeyCode::W,
        WK::KeyX => KeyCode::X,
        WK::KeyY => KeyCode::Y,
        WK::KeyZ => KeyCode::Z,
        WK::Digit0 => KeyCode::Num0,
        WK::Digit1 => KeyCode::Num1,
        WK::Digit2 => KeyCode::Num2,
        WK::Digit3 => KeyCode::Num3,
        WK::Digit4 => KeyCode::Num4,
        WK::Digit5 => KeyCode::Num5,
        WK::Digit6 => KeyCode::Num6,
        WK::Digit7 => KeyCode::Num7,
        WK::Digit8 => KeyCode::Num8,
        WK::Digit9 => KeyCode::Num9,
        WK::Space => KeyCode::Space,
        WK::Enter => KeyCode::Return,
        WK::Escape => KeyCode::Escape,
        WK::Tab => KeyCode::Tab,
        WK::Backspace => KeyCode::Backspace,
        WK::Delete => KeyCode::Delete,
        WK::ArrowLeft => KeyCode::Left,
        WK::ArrowRight => KeyCode::Right,
        WK::ArrowUp => KeyCode::Up,
        WK::ArrowDown => KeyCode::Down,
        other => KeyCode::Unknown(other as u32),
    }
}

/// Map a `winit` mouse button to our [`MouseButton`](window_backend::MouseButton).
pub fn map_winit_mouse(button: winit::event::MouseButton) -> window_backend::MouseButton {
    match button {
        winit::event::MouseButton::Left => window_backend::MouseButton::Left,
        winit::event::MouseButton::Right => window_backend::MouseButton::Right,
        winit::event::MouseButton::Middle => window_backend::MouseButton::Middle,
        _ => window_backend::MouseButton::Left,
    }
}

// ── GuiApp ─────────────────────────────────────────────────────────

/// Button click handler: receives button index and mutable access to labels/textfields.
type ButtonClickHandler = Box<dyn FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>)>;

/// A simple GUI application backed by a native window.
///
/// Add labels and buttons, set a click handler, then call [`run()`](GuiApp::run)
/// to enter the event loop.
pub struct GuiApp {
    config: WindowConfig,
    background: Color,
    labels: Vec<Label>,
    buttons: Vec<Button>,
    textfields: Vec<TextField>,
    on_button_click: Option<ButtonClickHandler>,
}

impl GuiApp {
    pub fn new(config: WindowConfig) -> Self {
        Self {
            config,
            background: Color::WHITE,
            labels: Vec::new(),
            buttons: Vec::new(),
            textfields: Vec::new(),
            on_button_click: None,
        }
    }

    pub fn set_background(&mut self, color: Color) {
        self.background = color;
    }

    pub fn add_label(&mut self, label: Label) {
        self.labels.push(label);
    }

    pub fn add_button(&mut self, button: Button) {
        self.buttons.push(button);
    }

    pub fn add_textfield(&mut self, tf: TextField) {
        self.textfields.push(tf);
    }

    /// Register a handler called when any button is clicked.
    ///
    /// The handler receives the button index and mutable access to labels
    /// and text fields so it can read/update them.
    pub fn on_button_click(
        &mut self,
        handler: impl FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>) + 'static,
    ) {
        self.on_button_click = Some(Box::new(handler));
    }

    /// Enter the main event loop. This blocks until the window is closed.
    pub fn run(self) {
        let event_loop = EventLoop::new().expect("failed to create event loop");
        let mut state = AppState {
            config: self.config,
            background: self.background,
            labels: self.labels,
            buttons: self.buttons,
            textfields: self.textfields,
            on_button_click: self.on_button_click,
            focused_textfield: None,
            window: None,
            _context: None,
            sb_surface: None,
            cursor_pos: (0.0, 0.0),
        };
        event_loop.run_app(&mut state).expect("event loop error");
    }
}

// ── Internal `ApplicationHandler` ──────────────────────────────────

struct AppState {
    config: WindowConfig,
    background: Color,
    labels: Vec<Label>,
    buttons: Vec<Button>,
    textfields: Vec<TextField>,
    on_button_click: Option<ButtonClickHandler>,
    focused_textfield: Option<usize>,

    // winit / softbuffer state
    window: Option<Arc<Window>>,
    _context: Option<softbuffer::Context<Arc<Window>>>,
    sb_surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    cursor_pos: (f64, f64),
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
                        let _ = surface.resize(
                            NonZeroU32::new(size.width).unwrap(),
                            NonZeroU32::new(size.height).unwrap(),
                        );
                    }
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
                button: winit::event::MouseButton::Left,
                ..
            } => {
                self.handle_click();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    self.handle_key_input(&event);
                }
            }
            _ => {}
        }
    }
}

impl AppState {
    /// Render all views into the software surface and present.
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

        // Render at logical resolution
        let mut surface = Surface::new(lw, lh);
        {
            let mut renderer = SoftwareRenderer::new(&mut surface);
            renderer.clear(self.background);

            for label in &mut self.labels {
                label.draw(&mut renderer);
            }
            for button in &mut self.buttons {
                button.draw(&mut renderer);
            }
            for tf in &mut self.textfields {
                tf.draw(&mut renderer);
            }
        }

        // Present to softbuffer
        let _ = sb_surface.resize(NonZeroU32::new(pw).unwrap(), NonZeroU32::new(ph).unwrap());
        let mut buffer = sb_surface.buffer_mut().expect("failed to get buffer");
        let src = surface.pixels();

        if lw == pw && lh == ph {
            // 1:1 — direct copy (strip alpha for softbuffer's 0x00RRGGBB format)
            for (dst, &pixel) in buffer.iter_mut().zip(src.iter()) {
                *dst = pixel & 0x00FF_FFFF;
            }
        } else {
            // HiDPI — nearest-neighbour scale-up
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

        buffer.present().expect("failed to present buffer");
    }

    /// Find which button the cursor is over and fire its action.
    fn handle_click(&mut self) {
        let point = Point::new(self.cursor_pos.0, self.cursor_pos.1);

        // Check textfield focus
        self.focused_textfield = self.textfields.iter().enumerate().find_map(|(i, tf)| {
            if tf.is_editable() && tf.view().frame.contains(point) {
                Some(i)
            } else {
                None
            }
        });

        let clicked_idx = self.buttons.iter().enumerate().find_map(|(i, btn)| {
            if btn.is_enabled() && btn.view().frame.contains(point) {
                Some(i)
            } else {
                None
            }
        });

        if let Some(idx) = clicked_idx {
            self.buttons[idx].perform_click();

            if let Some(ref mut handler) = self.on_button_click {
                handler(idx, &mut self.labels, &mut self.textfields);
            }

            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }
    }

    /// Handle keyboard input — dispatch to focused text field.
    fn handle_key_input(&mut self, event: &winit::event::KeyEvent) {
        let Some(idx) = self.focused_textfield else {
            return;
        };
        if idx >= self.textfields.len() {
            return;
        }

        // Try logical text first
        if let winit::keyboard::Key::Character(ref text) = event.logical_key {
            for ch in text.chars() {
                self.textfields[idx].insert_char(ch);
            }
        } else if let PhysicalKey::Code(code) = event.physical_key {
            if map_winit_key(code) == KeyCode::Backspace {
                self.textfields[idx].delete_backward();
            }
        }

        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_key_letters() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::KeyA), KeyCode::A);
        assert_eq!(map_winit_key(WK::KeyZ), KeyCode::Z);
    }

    #[test]
    fn map_key_digits() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::Digit0), KeyCode::Num0);
        assert_eq!(map_winit_key(WK::Digit9), KeyCode::Num9);
    }

    #[test]
    fn map_key_special() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::Space), KeyCode::Space);
        assert_eq!(map_winit_key(WK::Enter), KeyCode::Return);
        assert_eq!(map_winit_key(WK::Escape), KeyCode::Escape);
        assert_eq!(map_winit_key(WK::Tab), KeyCode::Tab);
        assert_eq!(map_winit_key(WK::Backspace), KeyCode::Backspace);
        assert_eq!(map_winit_key(WK::Delete), KeyCode::Delete);
        assert_eq!(map_winit_key(WK::ArrowLeft), KeyCode::Left);
        assert_eq!(map_winit_key(WK::ArrowRight), KeyCode::Right);
        assert_eq!(map_winit_key(WK::ArrowUp), KeyCode::Up);
        assert_eq!(map_winit_key(WK::ArrowDown), KeyCode::Down);
    }

    #[test]
    fn map_mouse_buttons() {
        use window_backend::MouseButton;
        assert_eq!(
            map_winit_mouse(winit::event::MouseButton::Left),
            MouseButton::Left
        );
        assert_eq!(
            map_winit_mouse(winit::event::MouseButton::Right),
            MouseButton::Right
        );
        assert_eq!(
            map_winit_mouse(winit::event::MouseButton::Middle),
            MouseButton::Middle
        );
    }

    #[test]
    fn gui_app_builder() {
        let mut app = GuiApp::new(WindowConfig::default());
        app.set_background(Color::WHITE);
        app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Test"));
        app.add_button(Button::new(Rect::new(0.0, 30.0, 80.0, 25.0), "OK"));
        assert_eq!(app.labels.len(), 1);
        assert_eq!(app.buttons.len(), 1);
    }
}
