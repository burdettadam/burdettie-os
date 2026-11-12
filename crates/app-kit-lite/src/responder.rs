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

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::ModifierFlags;
    use crate::Event;
    use std::cell::Cell;
    use std::rc::Rc;
    use window_backend::KeyCode;

    // ── Minimal responder (all defaults) ───────────────────────────

    struct MinimalResponder;

    impl Responder for MinimalResponder {}

    #[test]
    fn responder_default_next_is_none() {
        let r = MinimalResponder;
        assert!(r.next_responder().is_none());
    }

    #[test]
    fn responder_default_does_not_accept_first() {
        let r = MinimalResponder;
        assert!(!r.accepts_first_responder());
    }

    // ── Chain-aware test responder ─────────────────────────────────

    struct TestResponder {
        next: Option<Box<dyn Responder>>,
        received_key_down: Rc<Cell<bool>>,
        received_key_up: Rc<Cell<bool>>,
        received_mouse_down: Rc<Cell<bool>>,
        received_mouse_up: Rc<Cell<bool>>,
        handle_key_down: bool,
    }

    impl TestResponder {
        fn new(
            next: Option<Box<dyn Responder>>,
            received_key_down: Rc<Cell<bool>>,
            received_key_up: Rc<Cell<bool>>,
            received_mouse_down: Rc<Cell<bool>>,
            received_mouse_up: Rc<Cell<bool>>,
            handle_key_down: bool,
        ) -> Self {
            Self {
                next,
                received_key_down,
                received_key_up,
                received_mouse_down,
                received_mouse_up,
                handle_key_down,
            }
        }
    }

    impl Responder for TestResponder {
        fn next_responder(&self) -> Option<&dyn Responder> {
            self.next.as_ref().map(|b| b.as_ref() as &dyn Responder)
        }

        fn key_down(&self, event: &Event) {
            self.received_key_down.set(true);
            if !self.handle_key_down {
                if let Some(next) = self.next_responder() {
                    next.key_down(event);
                }
            }
        }

        fn key_up(&self, event: &Event) {
            self.received_key_up.set(true);
            if let Some(next) = self.next_responder() {
                next.key_up(event);
            }
        }

        fn mouse_down(&self, event: &Event) {
            self.received_mouse_down.set(true);
            if let Some(next) = self.next_responder() {
                next.mouse_down(event);
            }
        }

        fn mouse_up(&self, event: &Event) {
            self.received_mouse_up.set(true);
            if let Some(next) = self.next_responder() {
                next.mouse_up(event);
            }
        }
    }

    fn make_event_key_down() -> Event {
        Event::KeyDown {
            key: KeyCode::A,
            modifiers: ModifierFlags::NONE,
        }
    }

    fn make_event_key_up() -> Event {
        Event::KeyUp {
            key: KeyCode::A,
            modifiers: ModifierFlags::NONE,
        }
    }

    fn make_event_mouse_down() -> Event {
        Event::MouseDown {
            location: graphics_core::Point::new(10.0, 20.0),
            button: window_backend::MouseButton::Left,
        }
    }

    fn make_event_mouse_up() -> Event {
        Event::MouseUp {
            location: graphics_core::Point::new(10.0, 20.0),
            button: window_backend::MouseButton::Left,
        }
    }

    type EventFlags = [Rc<Cell<bool>>; 4];

    /// Build a chain A → B → C. Returns (A, flags_a, flags_b, flags_c) where
    /// each flags tuple is (key_down, key_up, mouse_down, mouse_up).
    fn build_chain(
        b_handles_key_down: bool,
    ) -> (TestResponder, EventFlags, EventFlags, EventFlags) {
        let c_kd = Rc::new(Cell::new(false));
        let c_ku = Rc::new(Cell::new(false));
        let c_md = Rc::new(Cell::new(false));
        let c_mu = Rc::new(Cell::new(false));
        let c = TestResponder::new(
            None,
            c_kd.clone(),
            c_ku.clone(),
            c_md.clone(),
            c_mu.clone(),
            false,
        );

        let b_kd = Rc::new(Cell::new(false));
        let b_ku = Rc::new(Cell::new(false));
        let b_md = Rc::new(Cell::new(false));
        let b_mu = Rc::new(Cell::new(false));
        let b = TestResponder::new(
            Some(Box::new(c)),
            b_kd.clone(),
            b_ku.clone(),
            b_md.clone(),
            b_mu.clone(),
            b_handles_key_down,
        );

        let a_kd = Rc::new(Cell::new(false));
        let a_ku = Rc::new(Cell::new(false));
        let a_md = Rc::new(Cell::new(false));
        let a_mu = Rc::new(Cell::new(false));
        let a = TestResponder::new(
            Some(Box::new(b)),
            a_kd.clone(),
            a_ku.clone(),
            a_md.clone(),
            a_mu.clone(),
            false,
        );

        (
            a,
            [a_kd, a_ku, a_md, a_mu],
            [b_kd, b_ku, b_md, b_mu],
            [c_kd, c_ku, c_md, c_mu],
        )
    }

    #[test]
    fn responder_chain_forwarding() {
        let (a, flags_a, flags_b, flags_c) = build_chain(false);
        a.key_down(&make_event_key_down());

        assert!(flags_a[0].get(), "A should receive key_down");
        assert!(flags_b[0].get(), "B should receive key_down");
        assert!(flags_c[0].get(), "C should receive key_down");
    }

    #[test]
    fn responder_chain_stops_at_handler() {
        let (a, flags_a, flags_b, flags_c) = build_chain(true);
        a.key_down(&make_event_key_down());

        assert!(flags_a[0].get(), "A should receive key_down");
        assert!(flags_b[0].get(), "B should receive key_down (handles it)");
        assert!(!flags_c[0].get(), "C should NOT receive key_down");
    }

    #[test]
    fn responder_mouse_down_forwarding() {
        let (a, flags_a, flags_b, flags_c) = build_chain(false);
        a.mouse_down(&make_event_mouse_down());

        assert!(flags_a[2].get(), "A received mouse_down");
        assert!(flags_b[2].get(), "B received mouse_down");
        assert!(flags_c[2].get(), "C received mouse_down");
    }

    #[test]
    fn responder_mouse_up_forwarding() {
        let (a, flags_a, flags_b, flags_c) = build_chain(false);
        a.mouse_up(&make_event_mouse_up());

        assert!(flags_a[3].get(), "A received mouse_up");
        assert!(flags_b[3].get(), "B received mouse_up");
        assert!(flags_c[3].get(), "C received mouse_up");
    }

    #[test]
    fn responder_key_up_forwarding() {
        let (a, flags_a, flags_b, flags_c) = build_chain(false);
        a.key_up(&make_event_key_up());

        assert!(flags_a[1].get(), "A received key_up");
        assert!(flags_b[1].get(), "B received key_up");
        assert!(flags_c[1].get(), "C received key_up");
    }
}
