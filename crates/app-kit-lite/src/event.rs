//! Event types for the responder chain.

use graphics_core::Point;
use window_backend::KeyCode;

/// An input event dispatched through the responder chain.
#[derive(Debug, Clone)]
pub enum Event {
    KeyDown {
        key: KeyCode,
    },
    KeyUp {
        key: KeyCode,
    },
    MouseDown {
        location: Point,
        button: window_backend::MouseButton,
    },
    MouseUp {
        location: Point,
        button: window_backend::MouseButton,
    },
    MouseMoved {
        location: Point,
    },
}
