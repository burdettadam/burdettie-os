//! Synthetic Foundation class registration.
//!
//! Each `register_*_class()` function builds a `ClassInfo` with the
//! correct method table and registers it in the ObjC runtime.

mod string;
mod object;
mod collections;
mod number;
mod datetime;
mod data_url;
mod error;
mod system;
mod threading;
mod invocation;
mod text;
mod advanced_collections;
mod appkit;

pub use string::*;
pub use object::*;
pub use collections::*;
pub use number::*;
pub use datetime::*;
pub use data_url::*;
pub use error::*;
pub use system::*;
pub use threading::*;
pub use invocation::*;
pub use text::*;
pub use advanced_collections::*;
pub use appkit::*;

use objc_runtime::MethodEntry;

/// Concise constructor for `MethodEntry`.
fn me(sel: &str, imp: u64, enc: &str) -> MethodEntry {
    MethodEntry {
        selector: sel.to_string(),
        imp,
        type_encoding: enc.to_string(),
    }
}
