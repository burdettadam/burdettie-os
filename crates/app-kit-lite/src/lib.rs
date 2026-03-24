//! Application object, responder chain, and event dispatch.
//!
//! Provides `NSApplication` (singleton app lifecycle), `AppDelegate`
//! (application-level callbacks), `Responder` (event chain), and `Event`
//! (user input events).

mod app;
mod event;
mod responder;

pub use app::{AppDelegate, NSApplication};
pub use event::Event;
pub use responder::Responder;
