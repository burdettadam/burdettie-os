//! Concrete `GuiBridge` implementation using winit + softbuffer.
//!
//! Creates real OS windows for guest Mach-O GUI binaries and converts
//! platform input events into `GuiEvent` values for the cf-lite responder chain.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cf_lite::gui_bridge::{GuiBridge, GuiEvent, WindowDesc};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ── Public bridge ──────────────────────────────────────────────────

/// A `GuiBridge` backed by winit + softbuffer.
///
/// Call `WinitGuiBridge::new()` then pass to `cf_lite::gui_bridge::set_gui_bridge()`.
type FrameUpdate = (u64, f64, f64, f64, f64);
type WindowEntry = (Arc<Window>, softbuffer::Surface<Arc<Window>, Arc<Window>>);

pub struct WinitGuiBridge {
    /// Windows requested before the event loop starts.
    pending: Mutex<Vec<WindowDesc>>,
    /// Title updates for live windows.
    title_updates: Mutex<Vec<(u64, String)>>,
    /// Frame updates for live windows.
    frame_updates: Mutex<Vec<FrameUpdate>>,
    /// Hide requests for live windows.
    hide_requests: Mutex<Vec<u64>>,
    /// Whether `request_stop` has been called.
    stop: AtomicBool,
}

impl WinitGuiBridge {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            title_updates: Mutex::new(Vec::new()),
            frame_updates: Mutex::new(Vec::new()),
            hide_requests: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        }
    }
}

impl GuiBridge for WinitGuiBridge {
    fn show_window(&self, desc: &WindowDesc) {
        lock(&self.pending).push(desc.clone());
    }

    fn hide_window(&self, guest_window: u64) {
        lock(&self.hide_requests).push(guest_window);
    }

    fn set_window_title(&self, guest_window: u64, title: &str) {
        lock(&self.title_updates).push((guest_window, title.to_string()));
    }

    fn set_window_frame(&self, guest_window: u64, x: f64, y: f64, w: f64, h: f64) {
        lock(&self.frame_updates).push((guest_window, x, y, w, h));
    }

    #[allow(clippy::expect_used)] // Event loop creation/run is fatal if it fails
    fn run_event_loop(&self, event_callback: Box<dyn FnMut(u64, GuiEvent)>) {
        let event_loop = EventLoop::new().expect("failed to create winit event loop");

        // Drain pending show_window requests so the ApplicationHandler can create them.
        let pending = std::mem::take(&mut *lock(&self.pending));

        let mut state = BridgeState {
            bridge: self,
            callback: event_callback,
            pending,
            windows: HashMap::new(),
            guest_to_winit: HashMap::new(),
            surfaces: HashMap::new(),
            cursor_pos: HashMap::new(),
        };

        event_loop.run_app(&mut state).expect("winit event loop error");
    }

    fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    fn window_surface(&self, _guest_window: u64) -> Option<(*mut u8, u32, u32, u32)> {
        // Surface access is not thread-safe across the bridge boundary.
        // Drawing goes through redraw requests instead.
        None
    }
}

// ── Internal event-loop state ──────────────────────────────────────

struct BridgeState<'a> {
    bridge: &'a WinitGuiBridge,
    callback: Box<dyn FnMut(u64, GuiEvent)>,
    pending: Vec<WindowDesc>,
    // winit WindowId -> guest u64-address
    windows: HashMap<WindowId, u64>,
    // guest u64 -> winit WindowId
    guest_to_winit: HashMap<u64, WindowId>,
    // WindowId -> (Arc<Window>, softbuffer surface)
    surfaces: HashMap<WindowId, WindowEntry>,
    // WindowId -> last cursor position (logical x, y)
    cursor_pos: HashMap<WindowId, (f64, f64)>,
}

impl<'a> BridgeState<'a> {
    fn create_window(&mut self, event_loop: &ActiveEventLoop, desc: &WindowDesc) {
        let attrs = Window::default_attributes()
            .with_title(&desc.title)
            .with_inner_size(LogicalSize::new(desc.width, desc.height))
            .with_resizable(true);

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::warn!("winit_bridge: failed to create window: {e}");
                return;
            }
        };

        let context = match softbuffer::Context::new(window.clone()) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("winit_bridge: softbuffer context failed: {e}");
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("winit_bridge: softbuffer surface failed: {e}");
                return;
            }
        };

        let wid = window.id();
        log::info!(
            "winit_bridge: created window {:?} for guest {:#x} \"{}\"",
            wid, desc.guest_window, desc.title,
        );

        self.windows.insert(wid, desc.guest_window);
        self.guest_to_winit.insert(desc.guest_window, wid);
        self.surfaces.insert(wid, (window.clone(), surface));
        window.request_redraw();
    }

    fn guest_for(&self, wid: WindowId) -> u64 {
        self.windows.get(&wid).copied().unwrap_or(0)
    }

    /// Process queued operations (title changes, frame changes, hide requests, new windows).
    fn process_queued(&mut self, event_loop: &ActiveEventLoop) {
        // New windows
        let new_windows = std::mem::take(&mut *lock(&self.bridge.pending));
        for desc in &new_windows {
            self.create_window(event_loop, desc);
        }

        // Title updates
        let titles = std::mem::take(&mut *lock(&self.bridge.title_updates));
        for (guest, title) in titles {
            if let Some(&wid) = self.guest_to_winit.get(&guest) {
                if let Some((ref w, _)) = self.surfaces.get(&wid) {
                    w.set_title(&title);
                }
            }
        }

        // Frame updates
        let frames = std::mem::take(&mut *lock(&self.bridge.frame_updates));
        for (guest, _x, _y, w, h) in frames {
            if let Some(&wid) = self.guest_to_winit.get(&guest) {
                if let Some((ref win, _)) = self.surfaces.get(&wid) {
                    let _ = win.request_inner_size(LogicalSize::new(w, h));
                }
            }
        }

        // Hide requests
        let hides = std::mem::take(&mut *lock(&self.bridge.hide_requests));
        for guest in hides {
            if let Some(&wid) = self.guest_to_winit.get(&guest) {
                if let Some((ref w, _)) = self.surfaces.get(&wid) {
                    w.set_visible(false);
                }
            }
        }
    }

    /// Fill the window surface with a light gray background (placeholder drawing).
    fn redraw(&mut self, wid: WindowId) {
        let Some((ref window, ref mut surface)) = self.surfaces.get_mut(&wid) else {
            return;
        };
        let phys = window.inner_size();
        if phys.width == 0 || phys.height == 0 {
            return;
        }
        let Some(nz_w) = NonZeroU32::new(phys.width) else { return };
        let Some(nz_h) = NonZeroU32::new(phys.height) else { return };

        if let Err(e) = surface.resize(nz_w, nz_h) {
            log::warn!("winit_bridge: resize failed: {e}");
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };

        // Fill with light background (0xF0F0F0) as placeholder.
        // TODO: route guest drawRect: → CGContext → this surface.
        let bg = 0x00F0_F0F0u32;
        for pixel in buffer.iter_mut() {
            *pixel = bg;
        }
        if let Err(e) = buffer.present() {
            log::warn!("winit_bridge: present failed: {e}");
        }
    }
}

impl<'a> ApplicationHandler for BridgeState<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create windows that were queued before the event loop started.
        let pending = std::mem::take(&mut self.pending);
        for desc in &pending {
            self.create_window(event_loop, desc);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, wid: WindowId, event: WindowEvent) {
        let guest = self.guest_for(wid);

        match event {
            WindowEvent::CloseRequested => {
                (self.callback)(guest, GuiEvent::WindowCloseRequested);

                // If guest didn't cancel, remove the window.
                if let Some(gw) = self.windows.remove(&wid) {
                    self.guest_to_winit.remove(&gw);
                }
                self.surfaces.remove(&wid);
                self.cursor_pos.remove(&wid);

                // If no windows remain, exit.
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }

            WindowEvent::RedrawRequested => {
                self.redraw(wid);
            }

            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    let scale = self.surfaces.get(&wid)
                        .map(|(w, _)| w.scale_factor())
                        .unwrap_or(1.0);
                    let lw = f64::from(size.width) / scale;
                    let lh = f64::from(size.height) / scale;
                    (self.callback)(guest, GuiEvent::WindowResized { width: lw, height: lh });

                    if let Some((ref w, _)) = self.surfaces.get(&wid) {
                        w.request_redraw();
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.surfaces.get(&wid)
                    .map(|(w, _)| w.scale_factor())
                    .unwrap_or(1.0);
                let x = position.x / scale;
                let y = position.y / scale;
                self.cursor_pos.insert(wid, (x, y));
                (self.callback)(guest, GuiEvent::MouseMoved { x, y });
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let (x, y) = self.cursor_pos.get(&wid).copied().unwrap_or((0.0, 0.0));
                let btn = match button {
                    winit::event::MouseButton::Left => 0,
                    winit::event::MouseButton::Right => 1,
                    winit::event::MouseButton::Middle => 2,
                    _ => 3,
                };
                let evt = match state {
                    ElementState::Pressed => GuiEvent::MouseDown { x, y, button: btn },
                    ElementState::Released => GuiEvent::MouseUp { x, y, button: btn },
                };
                (self.callback)(guest, evt);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        (f64::from(x) * 20.0, f64::from(y) * 20.0)
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        let scale = self.surfaces.get(&wid)
                            .map(|(w, _)| w.scale_factor())
                            .unwrap_or(1.0);
                        (pos.x / scale, pos.y / scale)
                    }
                };
                (self.callback)(guest, GuiEvent::ScrollWheel { dx, dy });
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let key_code = match event.physical_key {
                    PhysicalKey::Code(code) => winit_key_to_macos(code),
                    PhysicalKey::Unidentified(_) => 0xFFFF,
                };
                let characters = match &event.logical_key {
                    winit::keyboard::Key::Character(s) => s.to_string(),
                    _ => String::new(),
                };
                let modifiers = 0u64; // TODO: track modifier state
                let evt = match event.state {
                    ElementState::Pressed => GuiEvent::KeyDown { key_code, characters, modifiers },
                    ElementState::Released => GuiEvent::KeyUp { key_code, characters, modifiers },
                };
                (self.callback)(guest, evt);
            }

            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                let mut flags = 0u64;
                if state.shift_key() { flags |= 1 << 17; }    // NSEventModifierFlagShift
                if state.control_key() { flags |= 1 << 18; }   // NSEventModifierFlagControl
                if state.alt_key() { flags |= 1 << 19; }       // NSEventModifierFlagOption
                if state.super_key() { flags |= 1 << 20; }     // NSEventModifierFlagCommand
                (self.callback)(guest, GuiEvent::FlagsChanged { modifiers: flags });
            }

            _ => {}
        }

        // Process any operations queued by the guest during event dispatch.
        self.process_queued(event_loop);

        // Check stop flag.
        if self.bridge.stop.load(Ordering::SeqCst) {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Process queued operations between events.
        self.process_queued(event_loop);

        if self.bridge.stop.load(Ordering::SeqCst) {
            event_loop.exit();
        }
    }
}

// ── Winit keycode → macOS virtual keycode ──────────────────────────

/// Map a winit `KeyCode` to a macOS virtual key code (kVK_*).
fn winit_key_to_macos(key: winit::keyboard::KeyCode) -> u16 {
    use winit::keyboard::KeyCode as WK;
    match key {
        WK::KeyA => 0x00,
        WK::KeyS => 0x01,
        WK::KeyD => 0x02,
        WK::KeyF => 0x03,
        WK::KeyH => 0x04,
        WK::KeyG => 0x05,
        WK::KeyZ => 0x06,
        WK::KeyX => 0x07,
        WK::KeyC => 0x08,
        WK::KeyV => 0x09,
        WK::KeyB => 0x0B,
        WK::KeyQ => 0x0C,
        WK::KeyW => 0x0D,
        WK::KeyE => 0x0E,
        WK::KeyR => 0x0F,
        WK::KeyY => 0x10,
        WK::KeyT => 0x11,
        WK::Digit1 => 0x12,
        WK::Digit2 => 0x13,
        WK::Digit3 => 0x14,
        WK::Digit4 => 0x15,
        WK::Digit6 => 0x16,
        WK::Digit5 => 0x17,
        WK::Equal => 0x18,
        WK::Digit9 => 0x19,
        WK::Digit7 => 0x1A,
        WK::Minus => 0x1B,
        WK::Digit8 => 0x1C,
        WK::Digit0 => 0x1D,
        WK::BracketRight => 0x1E,
        WK::KeyO => 0x1F,
        WK::KeyU => 0x20,
        WK::BracketLeft => 0x21,
        WK::KeyI => 0x22,
        WK::KeyP => 0x23,
        WK::Enter => 0x24,
        WK::KeyL => 0x25,
        WK::KeyJ => 0x26,
        WK::Quote => 0x27,
        WK::KeyK => 0x28,
        WK::Semicolon => 0x29,
        WK::Backslash => 0x2A,
        WK::Comma => 0x2B,
        WK::Slash => 0x2C,
        WK::KeyN => 0x2D,
        WK::KeyM => 0x2E,
        WK::Period => 0x2F,
        WK::Tab => 0x30,
        WK::Space => 0x31,
        WK::Backquote => 0x32,
        WK::Backspace => 0x33,
        WK::Escape => 0x35,
        WK::ArrowRight => 0x7C,
        WK::ArrowLeft => 0x7B,
        WK::ArrowDown => 0x7D,
        WK::ArrowUp => 0x7E,
        WK::F1 => 0x7A,
        WK::F2 => 0x78,
        WK::F3 => 0x63,
        WK::F4 => 0x76,
        WK::F5 => 0x60,
        WK::F6 => 0x61,
        WK::F7 => 0x62,
        WK::F8 => 0x64,
        WK::F9 => 0x65,
        WK::F10 => 0x6D,
        WK::F11 => 0x67,
        WK::F12 => 0x6F,
        WK::Delete => 0x75,
        WK::Home => 0x73,
        WK::End => 0x77,
        WK::PageUp => 0x74,
        WK::PageDown => 0x79,
        _ => 0xFFFF,
    }
}
