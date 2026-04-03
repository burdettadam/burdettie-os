//! Event types for the responder chain.

use graphics_core::Point;
use window_backend::KeyCode;

use crate::menu::ModifierFlags;

/// An input event dispatched through the responder chain.
#[derive(Debug, Clone)]
pub enum Event {
    KeyDown {
        key: KeyCode,
        modifiers: ModifierFlags,
    },
    KeyUp {
        key: KeyCode,
        modifiers: ModifierFlags,
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
    ScrollWheel {
        delta_x: f64,
        delta_y: f64,
    },
    CharacterInput {
        text: String,
    },
}
