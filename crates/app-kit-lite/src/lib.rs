//! Application object, responder chain, and event dispatch.
//!
//! Provides `NSApplication` (singleton app lifecycle), `AppDelegate`
//! (application-level callbacks), `Responder` (event chain), and `Event`
//! (user input events).

mod app;
mod event;
pub mod menu;
mod responder;

pub use app::{AppDelegate, NSApplication};
pub use event::Event;
pub use menu::{KeyEquivalent, ModifierFlags, NSMenu, NSMenuItem};
pub use responder::Responder;
