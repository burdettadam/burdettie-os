//! Keyboard layout definitions — maps physical key positions to characters.
//!
//! Supports QWERTY (default), Colemak, and Dvorak layouts. Modeled after
//! macOS Input Sources: the layout remaps characters produced by physical
//! key positions, independent of the host OS keyboard setting.

use crate::KeyCode;

/// Available keyboard layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardLayout {
    /// Standard US QWERTY (default — passthrough).
    Qwerty,
    /// Colemak layout.
    Colemak,
    /// Dvorak Simplified layout.
    Dvorak,
}

impl KeyboardLayout {
    /// All available layouts, in order.
    pub const ALL: &'static [KeyboardLayout] = &[
        KeyboardLayout::Qwerty,
        KeyboardLayout::Colemak,
        KeyboardLayout::Dvorak,
    ];

    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Qwerty => "QWERTY",
            Self::Colemak => "Colemak",
            Self::Dvorak => "Dvorak",
        }
    }

    /// Cycle to the next layout.
    pub fn next(self) -> Self {
        match self {
            Self::Qwerty => Self::Colemak,
            Self::Colemak => Self::Dvorak,
            Self::Dvorak => Self::Qwerty,
        }
    }

    /// Remap a character produced by QWERTY physical key to the character
    /// this layout produces at the same physical position.
    ///
    /// Non-alphabetic and unmapped characters pass through unchanged.
    pub fn remap_char(self, ch: char) -> char {
        match self {
            Self::Qwerty => ch,
            Self::Colemak => remap_colemak(ch),
            Self::Dvorak => remap_dvorak(ch),
        }
    }

    /// Remap a string through this layout (each character individually).
    pub fn remap_str(self, s: &str) -> String {
        if matches!(self, Self::Qwerty) {
            return s.to_string();
        }
        s.chars().map(|c| self.remap_char(c)).collect()
    }

    /// Remap a physical `KeyCode` through this layout. Only letter keys
    /// are remapped; all other keys (numbers, punctuation, modifiers,
    /// navigation) pass through unchanged.
    pub fn remap_keycode(self, key: KeyCode) -> KeyCode {
        match self {
            Self::Qwerty => key,
            Self::Colemak => remap_keycode_colemak(key),
            Self::Dvorak => remap_keycode_dvorak(key),
        }
    }
}

impl Default for KeyboardLayout {
    fn default() -> Self {
        Self::Qwerty
    }
}

// ── Colemak ────────────────────────────────────────────────────────
//
// QWERTY → Colemak mapping (physical position → produced character):
//   q→q  w→w  e→f  r→p  t→g  y→j  u→l  i→u  o→y  p→;
//   a→a  s→r  d→s  f→t  g→d  h→h  j→n  k→e  l→i  ;→o
//   z→z  x→x  c→c  v→v  b→b  n→k  m→m

fn remap_colemak(ch: char) -> char {
    match ch {
        'e' => 'f', 'r' => 'p', 't' => 'g', 'y' => 'j', 'u' => 'l',
        'i' => 'u', 'o' => 'y', 'p' => ';',
        's' => 'r', 'd' => 's', 'f' => 't', 'g' => 'd',
        'j' => 'n', 'k' => 'e', 'l' => 'i', ';' => 'o',
        'n' => 'k',
        // Uppercase
        'E' => 'F', 'R' => 'P', 'T' => 'G', 'Y' => 'J', 'U' => 'L',
        'I' => 'U', 'O' => 'Y', 'P' => ':',
        'S' => 'R', 'D' => 'S', 'F' => 'T', 'G' => 'D',
        'J' => 'N', 'K' => 'E', 'L' => 'I',
        'N' => 'K',
        _ => ch,
    }
}

fn remap_keycode_colemak(key: KeyCode) -> KeyCode {
    match key {
        KeyCode::E => KeyCode::F, KeyCode::R => KeyCode::P, KeyCode::T => KeyCode::G,
        KeyCode::Y => KeyCode::J, KeyCode::U => KeyCode::L, KeyCode::I => KeyCode::U,
        KeyCode::O => KeyCode::Y, KeyCode::P => KeyCode::Semicolon,
        KeyCode::S => KeyCode::R, KeyCode::D => KeyCode::S, KeyCode::F => KeyCode::T,
        KeyCode::G => KeyCode::D,
        KeyCode::J => KeyCode::N, KeyCode::K => KeyCode::E, KeyCode::L => KeyCode::I,
        KeyCode::Semicolon => KeyCode::O,
        KeyCode::N => KeyCode::K,
        _ => key,
    }
}

// ── Dvorak ─────────────────────────────────────────────────────────
//
// QWERTY → Dvorak mapping:
//   q→'  w→,  e→.  r→p  t→y  y→f  u→g  i→c  o→r  p→l
//   a→a  s→o  d→e  f→u  g→i  h→d  j→h  k→t  l→n  ;→s
//   z→;  x→q  c→j  v→k  b→x  n→b  m→m

fn remap_dvorak(ch: char) -> char {
    match ch {
        'q' => '\'', 'w' => ',', 'e' => '.', 'r' => 'p', 't' => 'y',
        'y' => 'f', 'u' => 'g', 'i' => 'c', 'o' => 'r', 'p' => 'l',
        'a' => 'a', 's' => 'o', 'd' => 'e', 'f' => 'u', 'g' => 'i',
        'h' => 'd', 'j' => 'h', 'k' => 't', 'l' => 'n', ';' => 's',
        'z' => ';', 'x' => 'q', 'c' => 'j', 'v' => 'k', 'b' => 'x',
        'n' => 'b', 'm' => 'm',
        '-' => '[', '=' => ']', '[' => '/', ']' => '=',
        // Uppercase
        'Q' => '"', 'W' => '<', 'E' => '>', 'R' => 'P', 'T' => 'Y',
        'Y' => 'F', 'U' => 'G', 'I' => 'C', 'O' => 'R', 'P' => 'L',
        'A' => 'A', 'S' => 'O', 'D' => 'E', 'F' => 'U', 'G' => 'I',
        'H' => 'D', 'J' => 'H', 'K' => 'T', 'L' => 'N',
        'Z' => ':', 'X' => 'Q', 'C' => 'J', 'V' => 'K', 'B' => 'X',
        'N' => 'B', 'M' => 'M',
        _ => ch,
    }
}

fn remap_keycode_dvorak(key: KeyCode) -> KeyCode {
    match key {
        KeyCode::Q => KeyCode::Quote, KeyCode::W => KeyCode::Comma,
        KeyCode::E => KeyCode::Period, KeyCode::R => KeyCode::P, KeyCode::T => KeyCode::Y,
        KeyCode::Y => KeyCode::F, KeyCode::U => KeyCode::G, KeyCode::I => KeyCode::C,
        KeyCode::O => KeyCode::R, KeyCode::P => KeyCode::L,
        KeyCode::S => KeyCode::O, KeyCode::D => KeyCode::E, KeyCode::F => KeyCode::U,
        KeyCode::G => KeyCode::I, KeyCode::H => KeyCode::D, KeyCode::J => KeyCode::H,
        KeyCode::K => KeyCode::T, KeyCode::L => KeyCode::N, KeyCode::Semicolon => KeyCode::S,
        KeyCode::Z => KeyCode::Semicolon, KeyCode::X => KeyCode::Q, KeyCode::C => KeyCode::J,
        KeyCode::V => KeyCode::K, KeyCode::B => KeyCode::X, KeyCode::N => KeyCode::B,
        KeyCode::Minus => KeyCode::LeftBracket, KeyCode::Equal => KeyCode::RightBracket,
        KeyCode::LeftBracket => KeyCode::Slash, KeyCode::RightBracket => KeyCode::Equal,
        _ => key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwerty_is_passthrough() {
        let layout = KeyboardLayout::Qwerty;
        assert_eq!(layout.remap_char('a'), 'a');
        assert_eq!(layout.remap_char('z'), 'z');
        assert_eq!(layout.remap_str("hello"), "hello");
        assert_eq!(layout.remap_keycode(KeyCode::A), KeyCode::A);
    }

    #[test]
    fn colemak_remaps_correctly() {
        let layout = KeyboardLayout::Colemak;
        // s→r, d→s, f→t in Colemak
        assert_eq!(layout.remap_char('s'), 'r');
        assert_eq!(layout.remap_char('d'), 's');
        assert_eq!(layout.remap_char('f'), 't');
        assert_eq!(layout.remap_char('j'), 'n');
        assert_eq!(layout.remap_char('k'), 'e');
        // Unmapped keys pass through
        assert_eq!(layout.remap_char('a'), 'a');
        assert_eq!(layout.remap_char('q'), 'q');
        assert_eq!(layout.remap_char('1'), '1');
    }

    #[test]
    fn colemak_uppercase() {
        let layout = KeyboardLayout::Colemak;
        assert_eq!(layout.remap_char('S'), 'R');
        assert_eq!(layout.remap_char('D'), 'S');
    }

    #[test]
    fn dvorak_remaps_correctly() {
        let layout = KeyboardLayout::Dvorak;
        assert_eq!(layout.remap_char('s'), 'o');
        assert_eq!(layout.remap_char('d'), 'e');
        assert_eq!(layout.remap_char('f'), 'u');
        assert_eq!(layout.remap_char('k'), 't');
        assert_eq!(layout.remap_char(';'), 's');
    }

    #[test]
    fn remap_str_colemak() {
        let layout = KeyboardLayout::Colemak;
        assert_eq!(layout.remap_str("the"), "ghf");
    }

    #[test]
    fn cycle_layouts() {
        assert_eq!(KeyboardLayout::Qwerty.next(), KeyboardLayout::Colemak);
        assert_eq!(KeyboardLayout::Colemak.next(), KeyboardLayout::Dvorak);
        assert_eq!(KeyboardLayout::Dvorak.next(), KeyboardLayout::Qwerty);
    }

    #[test]
    fn layout_names() {
        assert_eq!(KeyboardLayout::Qwerty.name(), "QWERTY");
        assert_eq!(KeyboardLayout::Colemak.name(), "Colemak");
        assert_eq!(KeyboardLayout::Dvorak.name(), "Dvorak");
    }

    #[test]
    fn keycode_remap_colemak() {
        let layout = KeyboardLayout::Colemak;
        assert_eq!(layout.remap_keycode(KeyCode::S), KeyCode::R);
        assert_eq!(layout.remap_keycode(KeyCode::D), KeyCode::S);
        assert_eq!(layout.remap_keycode(KeyCode::F), KeyCode::T);
        // Non-letter keys pass through
        assert_eq!(layout.remap_keycode(KeyCode::Space), KeyCode::Space);
        assert_eq!(layout.remap_keycode(KeyCode::Return), KeyCode::Return);
    }

    #[test]
    fn keycode_remap_dvorak() {
        let layout = KeyboardLayout::Dvorak;
        assert_eq!(layout.remap_keycode(KeyCode::S), KeyCode::O);
        assert_eq!(layout.remap_keycode(KeyCode::D), KeyCode::E);
        assert_eq!(layout.remap_keycode(KeyCode::K), KeyCode::T);
    }

    #[test]
    fn default_is_qwerty() {
        assert_eq!(KeyboardLayout::default(), KeyboardLayout::Qwerty);
    }

    #[test]
    fn all_layouts_list() {
        assert_eq!(KeyboardLayout::ALL.len(), 3);
        assert_eq!(KeyboardLayout::ALL[0], KeyboardLayout::Qwerty);
        assert_eq!(KeyboardLayout::ALL[1], KeyboardLayout::Colemak);
        assert_eq!(KeyboardLayout::ALL[2], KeyboardLayout::Dvorak);
    }
}
