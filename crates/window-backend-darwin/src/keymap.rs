use window_backend::KeyCode;

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
}
