//! Cross-platform window backend using winit + softbuffer.
//!
//! This backend works on Linux (X11/Wayland), macOS, and Windows.
//! It uses `winit` for window creation and event handling, and
//! `softbuffer` for presenting software-rendered surfaces.
//!
//! # Apple Stack Mapping
//!
//! ```text
//! Apple:  Quartz Compositor / WindowServer
//! Us:     window-backend-linux → winit (event loop) + softbuffer (surface blit)
//! ```

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use graphics_core::Surface;
use window_backend::{
    WindowConfig, WindowEvent, WindowId, WindowBackend,
};

/// Winit-based window backend.
///
/// Each window is backed by a `winit::window::Window` + a `softbuffer::Surface`
/// for blitting our software-rendered `graphics_core::Surface` pixels.
///
/// Interior mutability via `Mutex` is required because the `WindowBackend`
/// trait methods take `&self`.
///
/// # Safety
///
/// `WinitBackend` wraps platform window handles that are not `Send`/`Sync`
/// on all platforms (e.g. macOS requires main-thread access). We implement
/// `Send + Sync` because the `WindowBackend` trait requires it and callers
/// guarantee windowing operations happen on the main thread.
pub struct WinitBackend {
    inner: Mutex<WinitInner>,
}

// Safety: All windowing operations happen on the main thread. The Mutex
// ensures exclusive access to the underlying platform handles. Platform
// windowing APIs (AppKit, X11, Wayland) enforce main-thread usage at the
// OS level; this marker lets the Rust type system accommodate that pattern.
unsafe impl Send for WinitBackend {}
unsafe impl Sync for WinitBackend {}

struct WinitInner {
    event_loop: Option<winit::event_loop::EventLoop<()>>,
    windows: HashMap<WindowId, WinitWindow>,
    next_id: u64,
    pending_events: Vec<WindowEvent>,
}

struct WinitWindow {
    window: Arc<winit::window::Window>,
    surface: softbuffer::Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>,
}

impl WinitBackend {
    /// Create a new winit backend.
    ///
    /// The event loop is created lazily on first window creation.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(WinitInner {
                event_loop: None,
                windows: HashMap::new(),
                next_id: 1,
                pending_events: Vec::new(),
            }),
        }
    }
}

impl Default for WinitBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WinitInner {
    fn ensure_event_loop(&mut self) -> &winit::event_loop::EventLoop<()> {
        if self.event_loop.is_none() {
            self.event_loop = Some(
                winit::event_loop::EventLoop::new()
                    .expect("window-backend-linux: failed to create event loop"),
            );
        }
        self.event_loop.as_ref().expect("event loop just created")
    }
}

impl WindowBackend for WinitBackend {
    fn create_window(&self, config: WindowConfig) -> WindowId {
        let mut inner = self.inner.lock().expect("lock poisoned");
        let event_loop = inner.ensure_event_loop();

        let window_attrs = winit::window::Window::default_attributes()
            .with_title(&config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height))
            .with_resizable(config.resizable);

        #[allow(deprecated)] // EventLoop::create_window is the imperative path
        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("window-backend-linux: failed to create window"),
        );

        let context = softbuffer::Context::new(window.clone())
            .expect("window-backend-linux: failed to create softbuffer context");
        let surface = softbuffer::Surface::new(&context, window.clone())
            .expect("window-backend-linux: failed to create softbuffer surface");

        let id = WindowId(inner.next_id);
        inner.next_id += 1;

        log::debug!("window-backend-linux: created window {:?} \"{}\"", id, config.title);
        inner.windows.insert(id, WinitWindow { window, surface });
        id
    }

    fn destroy_window(&self, id: WindowId) {
        let mut inner = self.inner.lock().expect("lock poisoned");
        if inner.windows.remove(&id).is_some() {
            log::debug!("window-backend-linux: destroyed window {:?}", id);
        }
    }

    fn set_title(&self, id: WindowId, title: &str) {
        let inner = self.inner.lock().expect("lock poisoned");
        if let Some(w) = inner.windows.get(&id) {
            w.window.set_title(title);
        }
    }

    fn title(&self, id: WindowId) -> Option<String> {
        let inner = self.inner.lock().expect("lock poisoned");
        inner.windows.get(&id).map(|w| w.window.title().to_string())
    }

    fn set_resizable(&self, id: WindowId, resizable: bool) {
        let inner = self.inner.lock().expect("lock poisoned");
        if let Some(w) = inner.windows.get(&id) {
            w.window.set_resizable(resizable);
        }
    }

    fn is_resizable(&self, id: WindowId) -> bool {
        let inner = self.inner.lock().expect("lock poisoned");
        inner.windows.get(&id).map(|w| w.window.is_resizable()).unwrap_or(true)
    }

    fn set_size(&self, id: WindowId, width: f64, height: f64) {
        let inner = self.inner.lock().expect("lock poisoned");
        if let Some(w) = inner.windows.get(&id) {
            let _ = w.window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
        }
    }

    fn present_surface(&self, id: WindowId, surface: &Surface) {
        let mut inner = self.inner.lock().expect("lock poisoned");
        if let Some(w) = inner.windows.get_mut(&id) {
            let width = surface.width();
            let height = surface.height();

            if width == 0 || height == 0 {
                return;
            }

            let nz_w = NonZeroU32::new(width).expect("non-zero width");
            let nz_h = NonZeroU32::new(height).expect("non-zero height");

            if let Err(e) = w.surface.resize(nz_w, nz_h) {
                log::warn!("window-backend-linux: resize failed: {e}");
                return;
            }

            if let Ok(mut buffer) = w.surface.buffer_mut() {
                let pixels = surface.pixels();
                // Surface stores ARGB32; softbuffer expects 0x00RRGGBB — strip alpha.
                for (dst, &src) in buffer.iter_mut().zip(pixels.iter()) {
                    *dst = src & 0x00FF_FFFF;
                }

                if let Err(e) = buffer.present() {
                    log::warn!("window-backend-linux: present failed: {e}");
                }
            }
        }
    }

    fn poll_events(&self) -> Vec<WindowEvent> {
        let mut inner = self.inner.lock().expect("lock poisoned");
        std::mem::take(&mut inner.pending_events)
    }

    fn request_redraw(&self, id: WindowId) {
        let inner = self.inner.lock().expect("lock poisoned");
        if let Some(w) = inner.windows.get(&id) {
            w.window.request_redraw();
        }
    }

    fn window_size(&self, id: WindowId) -> (f64, f64) {
        let inner = self.inner.lock().expect("lock poisoned");
        inner.windows.get(&id).map(|w| {
            let size = w.window.inner_size();
            let scale = w.window.scale_factor();
            (size.width as f64 / scale, size.height as f64 / scale)
        }).unwrap_or((0.0, 0.0))
    }

    fn scale_factor(&self, id: WindowId) -> f64 {
        let inner = self.inner.lock().expect("lock poisoned");
        inner.windows.get(&id).map(|w| w.window.scale_factor()).unwrap_or(1.0)
    }
}
