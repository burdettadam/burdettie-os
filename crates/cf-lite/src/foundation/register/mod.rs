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
mod gap_data;
mod gap_network;
mod gap_streams;
mod xcode_stubs;
mod measurement;
mod framework_stubs;

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
pub use gap_data::*;
pub use gap_network::*;
pub use gap_streams::*;
pub use xcode_stubs::*;
pub use measurement::*;
pub use framework_stubs::*;

use objc_runtime::MethodEntry;

/// Concise constructor for `MethodEntry`.
fn me(sel: &'static str, imp: u64, enc: &'static str) -> MethodEntry {
    MethodEntry {
        selector: sel.into(),
        imp,
        type_encoding: enc.into(),
    }
}
