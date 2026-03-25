//! NSString shims: split into basic, search, and path/extension modules.

mod basic;
mod path_ext;
mod search;

pub use basic::*;
pub use path_ext::*;
pub use search::*;
