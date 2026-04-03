//! Application object, responder chain, and event dispatch.
//!
//! Provides `NSApplication` (singleton app lifecycle), `AppDelegate`
//! (application-level callbacks), `Responder` (event chain), and `Event`
//! (user input events).

pub mod alert;
mod app;
mod event;
pub mod menu;
pub mod panel;
pub mod pasteboard;
mod responder;
pub mod window_delegate;

pub use alert::{AlertReturn, AlertStyle, NSAlert};
pub use app::{AppDelegate, Appearance, NSApplication};
pub use event::Event;
pub use menu::{KeyEquivalent, ModifierFlags, NSMenu, NSMenuItem};
pub use panel::{NSOpenPanel, NSSavePanel, PanelReturn};
pub use pasteboard::{
    NSPasteboard, NS_PASTEBOARD_TYPE_PDF, NS_PASTEBOARD_TYPE_PNG, NS_PASTEBOARD_TYPE_STRING,
};
pub use responder::Responder;
pub use window_delegate::{NoOpWindowDelegate, WindowDelegate};
