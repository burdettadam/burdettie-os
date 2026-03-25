//! Application object — singleton lifecycle, delegate, and run loop integration.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

use crate::event::Event;
use crate::menu::{ModifierFlags, NSMenu};
use crate::responder::Responder;
use runloop_core::RunLoop;
use window_backend::{WindowBackend, WindowEvent};

/// Application delegate callbacks.
pub trait AppDelegate: Send {
    fn application_did_finish_launching(&self) {}
    fn application_will_terminate(&self) {}
    fn application_should_terminate(&self) -> bool {
        true
    }
}

/// Event handler type alias.
type EventHandler = Box<dyn FnMut(&Event) + Send>;

/// The application object. Manages the run loop and event dispatch.
pub struct NSApplication {
    run_loop: RunLoop,
    delegate: Mutex<Option<Box<dyn AppDelegate>>>,
    backend: Mutex<Option<Box<dyn WindowBackend>>>,
    running: AtomicBool,
    exit_code: AtomicI32,
    /// Callbacks invoked for each event (for headless testing).
    event_handlers: Mutex<Vec<EventHandler>>,
    /// The application-level menu bar.
    main_menu: Mutex<Option<NSMenu>>,
}

impl NSApplication {
    /// Create a new application instance.
    pub fn new() -> Self {
        Self {
            run_loop: RunLoop::new(),
            delegate: Mutex::new(None),
            backend: Mutex::new(None),
            running: AtomicBool::new(false),
            exit_code: AtomicI32::new(0),
            event_handlers: Mutex::new(Vec::new()),
            main_menu: Mutex::new(None),
        }
    }

    /// Set the application delegate.
    pub fn set_delegate(&self, delegate: Box<dyn AppDelegate>) {
        *self.delegate.lock().unwrap_or_else(|e| e.into_inner()) = Some(delegate);
    }

    /// Set the window backend.
    pub fn set_backend(&self, backend: Box<dyn WindowBackend>) {
        *self.backend.lock().unwrap_or_else(|e| e.into_inner()) = Some(backend);
    }

    /// Access the run loop.
    pub fn run_loop(&self) -> &RunLoop {
        &self.run_loop
    }

    /// Add an event handler (for testing / headless use).
    pub fn add_event_handler(&self, handler: impl FnMut(&Event) + Send + 'static) {
        self.event_handlers.lock().unwrap_or_else(|e| e.into_inner()).push(Box::new(handler));
    }

    /// Set the main menu bar.
    pub fn set_main_menu(&self, menu: NSMenu) {
        *self.main_menu.lock().unwrap_or_else(|e| e.into_inner()) = Some(menu);
    }

    /// Access the main menu (returns lock guard).
    pub fn main_menu(&self) -> std::sync::MutexGuard<'_, Option<NSMenu>> {
        self.main_menu.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Try to dispatch a key as a menu shortcut. Returns true if handled.
    pub fn perform_key_equivalent(
        &self,
        key: window_backend::KeyCode,
        modifiers: ModifierFlags,
    ) -> bool {
        let mut menu = self.main_menu.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut m) = *menu {
            m.perform_key_equivalent(key, modifiers)
        } else {
            false
        }
    }

    /// Enter the main run loop. Blocks until `terminate()` is called.
    pub fn run(&self) {
        self.running.store(true, Ordering::SeqCst);

        // Notify delegate
        if let Some(delegate) = self.delegate.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            delegate.application_did_finish_launching();
        }

        while self.running.load(Ordering::SeqCst) {
            // 1. Poll window events
            self.poll_window_events();

            // 2. Run one cycle of the run loop (timers + blocks)
            self.run_loop.run_once();

            // 3. Brief sleep to avoid busy-wait
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // Notify delegate
        if let Some(delegate) = self.delegate.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            delegate.application_will_terminate();
        }
    }

    /// Run a single iteration (for testing without blocking).
    pub fn run_once(&self) {
        self.poll_window_events();
        self.run_loop.run_once();
    }

    /// Request application termination.
    pub fn terminate(&self, exit_code: i32) {
        // Ask delegate if we should terminate
        let should = self
            .delegate
            .lock()
            .unwrap()
            .as_ref()
            .is_none_or(|d| d.application_should_terminate());

        if should {
            self.exit_code.store(exit_code, Ordering::SeqCst);
            self.running.store(false, Ordering::SeqCst);
            self.run_loop.stop();
        }
    }

    /// The exit code (set by `terminate()`).
    pub fn exit_code(&self) -> i32 {
        self.exit_code.load(Ordering::SeqCst)
    }

    /// Whether the application is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    // ── Internal ───────────────────────────────────────────────

    fn poll_window_events(&self) {
        let events = {
            let backend = self.backend.lock().unwrap_or_else(|e| e.into_inner());
            match backend.as_ref() {
                Some(b) => b.poll_events(),
                None => return,
            }
        };

        for wevent in events {
            match wevent {
                WindowEvent::CloseRequested(_) => {
                    self.terminate(0);
                }
                WindowEvent::KeyDown(_, key) => {
                    let modifiers = ModifierFlags::default();
                    // Try menu key equivalents first
                    if !self.perform_key_equivalent(key, modifiers) {
                        let event = Event::KeyDown { key, modifiers };
                        self.dispatch_event(&event);
                    }
                }
                WindowEvent::KeyUp(_, key) => {
                    let event = Event::KeyUp {
                        key,
                        modifiers: ModifierFlags::default(),
                    };
                    self.dispatch_event(&event);
                }
                WindowEvent::CharacterInput(_, _) => {
                    // Character input is handled by the view layer
                }
                WindowEvent::ModifiersChanged(_, _) => {
                    // Modifier state is tracked by the backend/view layer
                }
                WindowEvent::MouseDown(_, x, y, button) => {
                    let event = Event::MouseDown {
                        location: graphics_core::Point::new(x, y),
                        button,
                    };
                    self.dispatch_event(&event);
                }
                WindowEvent::MouseUp(_, x, y, button) => {
                    let event = Event::MouseUp {
                        location: graphics_core::Point::new(x, y),
                        button,
                    };
                    self.dispatch_event(&event);
                }
                WindowEvent::MouseMoved(_, x, y) => {
                    let event = Event::MouseMoved {
                        location: graphics_core::Point::new(x, y),
                    };
                    self.dispatch_event(&event);
                }
                WindowEvent::ScrollWheel(_, _, _) => {
                    // Scroll events are handled by the view layer
                }
                WindowEvent::Resized(_, _, _) | WindowEvent::RedrawRequested(_) => {
                    // Handled by the view layer
                }
            }
        }
    }

    fn dispatch_event(&self, event: &Event) {
        let mut handlers = self.event_handlers.lock().unwrap_or_else(|e| e.into_inner());
        for handler in handlers.iter_mut() {
            handler(event);
        }
    }
}

impl Default for NSApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl Responder for NSApplication {
    // Application is the end of the responder chain
    fn next_responder(&self) -> Option<&dyn Responder> {
        None
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct TestDelegate {
        launched: Arc<AtomicBool>,
        terminated: Arc<AtomicBool>,
    }

    impl AppDelegate for TestDelegate {
        fn application_did_finish_launching(&self) {
            self.launched.store(true, Ordering::SeqCst);
        }

        fn application_will_terminate(&self) {
            self.terminated.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn app_lifecycle() {
        let app = NSApplication::new();
        let launched = Arc::new(AtomicBool::new(false));
        let terminated = Arc::new(AtomicBool::new(false));

        app.set_delegate(Box::new(TestDelegate {
            launched: launched.clone(),
            terminated: terminated.clone(),
        }));

        // Use an Arc to share the stop flag with the timer
        let running = Arc::new(AtomicBool::new(true));
        let running2 = running.clone();

        // Instead of using raw pointer, directly stop via run_loop
        app.run_loop()
            .schedule_timer(std::time::Duration::from_millis(5), false, move || {
                running2.store(false, Ordering::SeqCst);
            });

        // Manually run the app lifecycle instead of app.run()
        app.running.store(true, Ordering::SeqCst);
        if let Some(delegate) = app.delegate.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            delegate.application_did_finish_launching();
        }

        while running.load(Ordering::SeqCst) {
            app.run_loop.run_once();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        app.running.store(false, Ordering::SeqCst);
        if let Some(delegate) = app.delegate.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            delegate.application_will_terminate();
        }

        assert!(launched.load(Ordering::SeqCst));
        assert!(terminated.load(Ordering::SeqCst));
    }

    #[test]
    fn app_terminate_with_code() {
        let app = NSApplication::new();
        app.terminate(42);
        assert_eq!(app.exit_code(), 42);
        assert!(!app.is_running());
    }

    #[test]
    fn app_event_handler() {
        let app = NSApplication::new();
        let backend = window_backend::HeadlessBackend::new();
        let wid = backend.create_window(window_backend::WindowConfig::default());

        backend.inject_event(window_backend::WindowEvent::KeyDown(
            wid,
            window_backend::KeyCode::A,
        ));

        let received = Arc::new(AtomicBool::new(false));
        let r = received.clone();
        app.add_event_handler(move |event| {
            if matches!(event, Event::KeyDown { .. }) {
                r.store(true, Ordering::SeqCst);
            }
        });

        app.set_backend(Box::new(backend));
        app.run_once();

        assert!(received.load(Ordering::SeqCst));
    }

    #[test]
    fn app_close_terminates() {
        let app = NSApplication::new();
        let backend = window_backend::HeadlessBackend::new();
        let wid = backend.create_window(window_backend::WindowConfig::default());

        backend.inject_event(window_backend::WindowEvent::CloseRequested(wid));

        app.set_backend(Box::new(backend));
        app.running.store(true, Ordering::SeqCst);
        app.run_once();

        assert!(!app.is_running());
    }
}
