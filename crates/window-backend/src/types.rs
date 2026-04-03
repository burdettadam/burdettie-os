// ── Window identity ────────────────────────────────────────────────

/// Opaque identifier for a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

// ── Window configuration ───────────────────────────────────────────

/// Configuration for creating a new window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "burdettie-os".to_string(),
            width: 800.0,
            height: 600.0,
            resizable: true,
        }
    }
}

// ── Input types ────────────────────────────────────────────────────

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Keyboard key codes (subset covering common keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Space,
    Return,
    Escape,
    Tab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // Navigation
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    // Punctuation / symbols
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Grave,
    // Modifier keys
    LeftShift,
    RightShift,
    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
    LeftSuper,
    RightSuper,
    // Lock / media keys
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
    Unknown(u32),
}

/// Modifier key state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub command: bool,
    pub shift: bool,
    pub option: bool,
    pub control: bool,
}

// ── Cursor / window-mode / theme ───────────────────────────────────

/// Cursor icon for the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorIcon {
    Default,
    Pointer,
    Text,
    Crosshair,
    Move,
    NotAllowed,
    ResizeNS,
    ResizeEW,
}

/// Window display mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowMode {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

/// UI theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

// ── Window events ──────────────────────────────────────────────────

/// Events produced by the windowing system.
#[derive(Debug, Clone)]
pub enum WindowEvent {
    CloseRequested(WindowId),
    Resized(WindowId, f64, f64),
    KeyDown(WindowId, KeyCode),
    KeyUp(WindowId, KeyCode),
    /// Typed text (logical characters after modifier/input method processing).
    CharacterInput(WindowId, String),
    /// Modifier key state changed.
    ModifiersChanged(WindowId, Modifiers),
    MouseDown(WindowId, f64, f64, MouseButton),
    MouseUp(WindowId, f64, f64, MouseButton),
    MouseMoved(WindowId, f64, f64),
    /// Scroll/wheel input (dx, dy in logical pixels).
    ScrollWheel(WindowId, f64, f64),
    RedrawRequested(WindowId),
    /// The OS theme changed.
    ThemeChanged(WindowId, Theme),
    /// Window gained or lost focus.
    Focused(WindowId, bool),
    /// A file was dropped onto the window.
    DroppedFile(WindowId, String),
}
