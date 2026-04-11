//! GUI application bridge: connects AppKit shims to a real windowing backend.
//!
//! The bridge is set once by the runtime (via `set_gui_bridge`) before the
//! guest `[NSApp run]` is called. Without a bridge, GUI operations silently
//! no-op — which is the correct behavior for headless / CLI-tool invocations.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐    trait GuiBridge    ┌──────────────────┐
//! │ cf-lite shims│───────────────────────│ winit+softbuffer │
//! │ (NSApp run,  │    fn show_window()  │ (or headless)    │
//! │  NSWindow    │    fn run_event_loop()│                  │
//! │  makeKey...) │    fn hide_window()  │                  │
//! └─────────────┘                       └──────────────────┘
//! ```

use std::sync::OnceLock;

/// A GUI event delivered from the host windowing system to the guest.
#[derive(Debug, Clone)]
pub enum GuiEvent {
    /// Mouse button pressed at (x, y) in window coordinates.
    MouseDown { x: f64, y: f64, button: u8 },
    /// Mouse button released.
    MouseUp { x: f64, y: f64, button: u8 },
    /// Mouse moved to (x, y).
    MouseMoved { x: f64, y: f64 },
    /// Mouse scroll with delta.
    ScrollWheel { dx: f64, dy: f64 },
    /// Key pressed with macOS virtual key code + characters.
    KeyDown { key_code: u16, characters: String, modifiers: u64 },
    /// Key released.
    KeyUp { key_code: u16, characters: String, modifiers: u64 },
    /// Modifier flags changed.
    FlagsChanged { modifiers: u64 },
    /// Window was resized to new dimensions.
    WindowResized { width: f64, height: f64 },
    /// Window close button pressed.
    WindowCloseRequested,
    /// Application should terminate.
    AppTerminate,
    /// Window needs to redraw its content.
    RedrawRequested,
}

/// Window descriptor passed to the bridge when showing a window.
#[derive(Debug, Clone)]
pub struct WindowDesc {
    /// Guest NSWindow address (for mapping events back).
    pub guest_window: u64,
    /// Title string.
    pub title: String,
    /// Frame: x, y, width, height.
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Trait that a host windowing backend implements.
///
/// The bridge runs on the main thread and owns the OS event loop. It must
/// call the provided event callback for each input event so cf-lite can
/// convert them into NSEvent objects and dispatch through the responder chain.
pub trait GuiBridge: Send + Sync {
    /// Create and show a real OS window for the given guest window descriptor.
    fn show_window(&self, desc: &WindowDesc);

    /// Hide / destroy the OS window associated with the guest window address.
    fn hide_window(&self, guest_window: u64);

    /// Update the title of an existing window.
    fn set_window_title(&self, guest_window: u64, title: &str);

    /// Resize an existing window.
    fn set_window_frame(&self, guest_window: u64, x: f64, y: f64, w: f64, h: f64);

    /// Enter the main event loop. This blocks until the application terminates.
    ///
    /// The `event_callback` is called for each host event that should be
    /// dispatched to the guest. The callback converts GuiEvents to NSEvent
    /// objects and routes them through `[NSApp sendEvent:]`.
    ///
    /// Returns when the application should exit.
    fn run_event_loop(&self, event_callback: Box<dyn FnMut(u64, GuiEvent)>);

    /// Request the event loop to stop (called from `[NSApp terminate:]`).
    fn request_stop(&self);

    /// Get the framebuffer data pointer and dimensions for a guest window,
    /// so `drawRect:` can route CGContext output to the window surface.
    /// Returns (data_ptr, width, height, stride) or None if not available.
    fn window_surface(&self, guest_window: u64) -> Option<(*mut u8, u32, u32, u32)>;

    /// Request the window to redraw on the next frame (called from `setNeedsDisplay:`).
    fn request_redraw(&self, guest_window: u64);

    /// Set the OS cursor to match the NSCursor type tag.
    /// The `cursor_type` values are defined in `nscursor.rs` (0=Arrow, 1=IBeam, etc.).
    fn set_cursor(&self, cursor_type: u64) { let _ = cursor_type; }

    /// Push an accessibility tree update for a guest window's content view.
    /// Called after view hierarchy changes or on redraw.
    fn update_accessibility(&self, _guest_window: u64, _content_view: u64) {}
}

static GUI_BRIDGE: OnceLock<Box<dyn GuiBridge>> = OnceLock::new();

/// Set the GUI bridge implementation. Call once before `[NSApp run]`.
pub fn set_gui_bridge(bridge: Box<dyn GuiBridge>) {
    let _ = GUI_BRIDGE.set(bridge);
}

/// Access the GUI bridge (if set).
pub fn gui_bridge() -> Option<&'static dyn GuiBridge> {
    GUI_BRIDGE.get().map(AsRef::as_ref)
}

/// Returns true if a GUI bridge has been registered.
pub fn has_gui_bridge() -> bool {
    GUI_BRIDGE.get().is_some()
}

/// Convert a host `GuiEvent` into a guest `NSEvent` object address and dispatch it.
///
/// Called by the bridge's event callback. Creates the appropriate NSEvent,
/// stamps it with the guest window, and sends it through `[NSApp sendEvent:]`.
pub fn dispatch_gui_event(guest_window: u64, event: &GuiEvent) {
    let event_addr = match event {
        GuiEvent::MouseDown { x, y, button } => {
            let event_type = match button {
                0 => 1,  // NSEventTypeLeftMouseDown
                1 => 3,  // NSEventTypeRightMouseDown
                _ => 25, // NSEventTypeOtherMouseDown
            };
            crate::nsevent::alloc_mouse_event(event_type, *x, *y, 0, guest_window)
        }
        GuiEvent::MouseUp { x, y, button } => {
            let event_type = match button {
                0 => 2,  // NSEventTypeLeftMouseUp
                1 => 4,  // NSEventTypeRightMouseUp
                _ => 26, // NSEventTypeOtherMouseUp
            };
            crate::nsevent::alloc_mouse_event(event_type, *x, *y, 0, guest_window)
        }
        GuiEvent::MouseMoved { x, y } => {
            crate::nsevent::alloc_mouse_event(5, *x, *y, 0, guest_window) // NSEventTypeMouseMoved
        }
        GuiEvent::ScrollWheel { dx, dy } => {
            crate::nsevent::alloc_scroll_event(*dx, *dy, 0, guest_window)
        }
        GuiEvent::KeyDown { key_code, ref characters, modifiers } => {
            let chars = crate::cfstring::alloc_nsstring_from_cstr(
                std::ffi::CString::new(characters.as_str()).unwrap_or_default().as_ptr(),
            );
            crate::nsevent::alloc_key_event(10, u64::from(*key_code), chars, *modifiers, guest_window) // NSEventTypeKeyDown
        }
        GuiEvent::KeyUp { key_code, ref characters, modifiers } => {
            let chars = crate::cfstring::alloc_nsstring_from_cstr(
                std::ffi::CString::new(characters.as_str()).unwrap_or_default().as_ptr(),
            );
            crate::nsevent::alloc_key_event(11, u64::from(*key_code), chars, *modifiers, guest_window) // NSEventTypeKeyUp
        }
        GuiEvent::FlagsChanged { modifiers } => {
            crate::nsevent::alloc_key_event(12, 0, 0, *modifiers, guest_window) // NSEventTypeFlagsChanged
        }
        GuiEvent::WindowResized { .. } | GuiEvent::WindowCloseRequested | GuiEvent::AppTerminate => {
            return; // handled separately
        }
        GuiEvent::RedrawRequested => {
            // Trigger display cycle on the guest window's content view.
            crate::nswindow::display_window_content(guest_window);
            // Push accessibility tree update.
            if let Some(bridge) = gui_bridge() {
                let content_view = crate::nswindow::window_content_view(guest_window);
                bridge.update_accessibility(guest_window, content_view);
            }
            return;
        }
    };

    if event_addr != 0 {
        // Enqueue for nextEventMatchingMask: to pick up
        crate::nsapplication::enqueue_event(event_addr);

        crate::nsapplication::shim_nsapplication_send_event(
            crate::nsapplication::shim_nsapplication_shared_application(0, 0),
            0,
            event_addr,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_bridge_by_default() {
        assert!(!has_gui_bridge());
        assert!(gui_bridge().is_none());
    }

    #[test]
    fn window_desc_clone() {
        let desc = WindowDesc {
            guest_window: 0x1000,
            title: "Test".to_string(),
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let cloned = desc.clone();
        assert_eq!(cloned.guest_window, 0x1000);
        assert_eq!(cloned.title, "Test");
    }

    #[test]
    fn gui_event_debug() {
        let evt = GuiEvent::MouseDown { x: 10.0, y: 20.0, button: 0 };
        let dbg = format!("{:?}", evt);
        assert!(dbg.contains("MouseDown"));
    }

    #[test]
    fn dispatch_gui_event_nil_window_no_crash() {
        // Without a bridge set, dispatch should still create NSEvents
        dispatch_gui_event(0, &GuiEvent::MouseDown { x: 0.0, y: 0.0, button: 0 });
        dispatch_gui_event(0, &GuiEvent::KeyDown {
            key_code: 0, characters: "a".to_string(), modifiers: 0,
        });
    }
}
