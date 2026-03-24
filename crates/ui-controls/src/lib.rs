//! Concrete UI controls: Label, Button, TextField.
//!
//! Each control wraps a `View` and adds specialized behavior.
//!
//! Also provides [`HeadlessApp`] — a testable GUI driver that mirrors the
//! `GuiApp` API from `window-backend-darwin` without opening a window.

mod button;
mod headless;
mod label;
mod textfield;

pub use button::Button;
pub use headless::HeadlessApp;
pub use label::Label;
pub use textfield::TextField;
