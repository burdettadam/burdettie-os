//! AppKit class method registrations (Phase C5).

use super::me;

/// Register `NSResponder` — base class for responder chain.

mod phase_c5;
mod phase_c7;
mod phase_c8;
mod phase_c9_appkit;
mod phase_c10_appkit;

pub use phase_c5::*;
pub use phase_c7::*;
pub use phase_c8::*;
pub use phase_c9_appkit::*;
pub use phase_c10_appkit::*;
