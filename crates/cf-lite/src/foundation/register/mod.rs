//! Synthetic Foundation class registration.
//!
//! Each `register_*_class()` function builds a `ClassInfo` with the
//! correct method table and registers it in the ObjC runtime.

mod advanced_collections;
mod appkit;
mod collections;
mod data_url;
mod datetime;
mod error;
mod framework_stubs;
mod gap_data;
mod gap_network;
mod gap_streams;
mod invocation;
mod measurement;
mod number;
mod object;
mod string;
mod system;
mod text;
mod threading;
mod xcode_stubs;

pub use advanced_collections::*;
pub use appkit::*;
pub use collections::*;
pub use data_url::*;
pub use datetime::*;
pub use error::*;
pub use framework_stubs::*;
pub use gap_data::*;
pub use gap_network::*;
pub use gap_streams::*;
pub use invocation::*;
pub use measurement::*;
pub use number::*;
pub use object::*;
pub use string::*;
pub use system::*;
pub use text::*;
pub use threading::*;

use objc_runtime::MethodEntry;

/// Concise constructor for `MethodEntry`.
fn me(sel: &'static str, imp: u64, enc: &'static str) -> MethodEntry {
    MethodEntry {
        selector: sel.into(),
        imp,
        type_encoding: enc.into(),
    }
}
