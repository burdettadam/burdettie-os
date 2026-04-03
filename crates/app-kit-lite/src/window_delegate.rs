//! Window delegate trait for receiving window lifecycle events.

/// Delegate callbacks for window events.
///
/// All methods have default no-op implementations so conforming types
/// only need to override the events they care about.
pub trait WindowDelegate {
    fn window_will_close(&mut self, _window_id: u64) {}
    fn window_did_resize(&mut self, _window_id: u64, _width: f64, _height: f64) {}
    fn window_did_move(&mut self, _window_id: u64, _x: f64, _y: f64) {}
    fn window_did_become_key(&mut self, _window_id: u64) {}
    fn window_did_resign_key(&mut self, _window_id: u64) {}
    fn window_should_close(&mut self, _window_id: u64) -> bool {
        true
    }
}

/// A no-op window delegate for testing.
pub struct NoOpWindowDelegate;

impl WindowDelegate for NoOpWindowDelegate {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_delegate_compiles() {
        let mut d = NoOpWindowDelegate;
        d.window_will_close(1);
        d.window_did_resize(1, 800.0, 600.0);
        d.window_did_move(1, 0.0, 0.0);
        d.window_did_become_key(1);
        d.window_did_resign_key(1);
        // just confirming it compiles and runs without panic
    }

    #[test]
    fn window_should_close_default_true() {
        let mut d = NoOpWindowDelegate;
        assert!(d.window_should_close(1));
    }

    #[test]
    fn custom_delegate_will_close() {
        struct Tracker {
            closed_id: Option<u64>,
        }
        impl WindowDelegate for Tracker {
            fn window_will_close(&mut self, window_id: u64) {
                self.closed_id = Some(window_id);
            }
        }

        let mut t = Tracker { closed_id: None };
        t.window_will_close(42);
        assert_eq!(t.closed_id, Some(42));
    }
}
