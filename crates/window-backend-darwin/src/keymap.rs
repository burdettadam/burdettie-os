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
        // Function keys
        WK::F1 => KeyCode::F1,
        WK::F2 => KeyCode::F2,
        WK::F3 => KeyCode::F3,
        WK::F4 => KeyCode::F4,
        WK::F5 => KeyCode::F5,
        WK::F6 => KeyCode::F6,
        WK::F7 => KeyCode::F7,
        WK::F8 => KeyCode::F8,
        WK::F9 => KeyCode::F9,
        WK::F10 => KeyCode::F10,
        WK::F11 => KeyCode::F11,
        WK::F12 => KeyCode::F12,
        // Navigation
        WK::Home => KeyCode::Home,
        WK::End => KeyCode::End,
        WK::PageUp => KeyCode::PageUp,
        WK::PageDown => KeyCode::PageDown,
        WK::Insert => KeyCode::Insert,
        // Punctuation / symbols
        WK::Minus => KeyCode::Minus,
        WK::Equal => KeyCode::Equal,
        WK::BracketLeft => KeyCode::LeftBracket,
        WK::BracketRight => KeyCode::RightBracket,
        WK::Backslash => KeyCode::Backslash,
        WK::Semicolon => KeyCode::Semicolon,
        WK::Quote => KeyCode::Quote,
        WK::Comma => KeyCode::Comma,
        WK::Period => KeyCode::Period,
        WK::Slash => KeyCode::Slash,
        WK::Backquote => KeyCode::Grave,
        // Modifier keys
        WK::ShiftLeft => KeyCode::LeftShift,
        WK::ShiftRight => KeyCode::RightShift,
        WK::ControlLeft => KeyCode::LeftControl,
        WK::ControlRight => KeyCode::RightControl,
        WK::AltLeft => KeyCode::LeftAlt,
        WK::AltRight => KeyCode::RightAlt,
        WK::SuperLeft => KeyCode::LeftSuper,
        WK::SuperRight => KeyCode::RightSuper,
        // Lock / media keys
        WK::CapsLock => KeyCode::CapsLock,
        WK::NumLock => KeyCode::NumLock,
        WK::ScrollLock => KeyCode::ScrollLock,
        WK::PrintScreen => KeyCode::PrintScreen,
        WK::Pause => KeyCode::Pause,
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

    #[test]
    fn map_key_function_keys() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::F1), KeyCode::F1);
        assert_eq!(map_winit_key(WK::F2), KeyCode::F2);
        assert_eq!(map_winit_key(WK::F3), KeyCode::F3);
        assert_eq!(map_winit_key(WK::F4), KeyCode::F4);
        assert_eq!(map_winit_key(WK::F5), KeyCode::F5);
        assert_eq!(map_winit_key(WK::F6), KeyCode::F6);
        assert_eq!(map_winit_key(WK::F7), KeyCode::F7);
        assert_eq!(map_winit_key(WK::F8), KeyCode::F8);
        assert_eq!(map_winit_key(WK::F9), KeyCode::F9);
        assert_eq!(map_winit_key(WK::F10), KeyCode::F10);
        assert_eq!(map_winit_key(WK::F11), KeyCode::F11);
        assert_eq!(map_winit_key(WK::F12), KeyCode::F12);
    }

    #[test]
    fn map_key_navigation() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::Home), KeyCode::Home);
        assert_eq!(map_winit_key(WK::End), KeyCode::End);
        assert_eq!(map_winit_key(WK::PageUp), KeyCode::PageUp);
        assert_eq!(map_winit_key(WK::PageDown), KeyCode::PageDown);
        assert_eq!(map_winit_key(WK::Insert), KeyCode::Insert);
    }

    #[test]
    fn map_key_punctuation() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::Minus), KeyCode::Minus);
        assert_eq!(map_winit_key(WK::Equal), KeyCode::Equal);
        assert_eq!(map_winit_key(WK::BracketLeft), KeyCode::LeftBracket);
        assert_eq!(map_winit_key(WK::BracketRight), KeyCode::RightBracket);
        assert_eq!(map_winit_key(WK::Backslash), KeyCode::Backslash);
        assert_eq!(map_winit_key(WK::Semicolon), KeyCode::Semicolon);
        assert_eq!(map_winit_key(WK::Quote), KeyCode::Quote);
        assert_eq!(map_winit_key(WK::Comma), KeyCode::Comma);
        assert_eq!(map_winit_key(WK::Period), KeyCode::Period);
        assert_eq!(map_winit_key(WK::Slash), KeyCode::Slash);
        assert_eq!(map_winit_key(WK::Backquote), KeyCode::Grave);
    }

    #[test]
    fn map_key_modifiers() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::ShiftLeft), KeyCode::LeftShift);
        assert_eq!(map_winit_key(WK::ShiftRight), KeyCode::RightShift);
        assert_eq!(map_winit_key(WK::ControlLeft), KeyCode::LeftControl);
        assert_eq!(map_winit_key(WK::ControlRight), KeyCode::RightControl);
        assert_eq!(map_winit_key(WK::AltLeft), KeyCode::LeftAlt);
        assert_eq!(map_winit_key(WK::AltRight), KeyCode::RightAlt);
        assert_eq!(map_winit_key(WK::SuperLeft), KeyCode::LeftSuper);
        assert_eq!(map_winit_key(WK::SuperRight), KeyCode::RightSuper);
    }

    #[test]
    fn map_key_lock_keys() {
        use winit::keyboard::KeyCode as WK;
        assert_eq!(map_winit_key(WK::CapsLock), KeyCode::CapsLock);
        assert_eq!(map_winit_key(WK::NumLock), KeyCode::NumLock);
        assert_eq!(map_winit_key(WK::ScrollLock), KeyCode::ScrollLock);
    }
}
