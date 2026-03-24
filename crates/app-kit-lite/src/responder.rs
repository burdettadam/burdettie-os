//! Responder chain protocol.

use crate::event::Event;

/// The responder chain protocol. Views, windows, and the application
/// object all implement this to handle or forward events.
pub trait Responder {
    /// The next responder in the chain (e.g., superview, window, app).
    fn next_responder(&self) -> Option<&dyn Responder> {
        None
    }

    /// Whether this responder wants to become first responder.
    fn accepts_first_responder(&self) -> bool {
        false
    }

    /// Handle a key-down event. Default: forward to next responder.
    fn key_down(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.key_down(event);
        }
    }

    /// Handle a key-up event. Default: forward to next responder.
    fn key_up(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.key_up(event);
        }
    }

    /// Handle a mouse-down event. Default: forward to next responder.
    fn mouse_down(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.mouse_down(event);
        }
    }

    /// Handle a mouse-up event. Default: forward to next responder.
    fn mouse_up(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.mouse_up(event);
        }
    }
}
