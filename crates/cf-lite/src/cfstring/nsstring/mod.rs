//! NSString shims: split into basic, search, path/extension, and URL encoding modules.

mod basic;
mod path_ext;
mod search;
mod url_encoding;

pub use basic::*;
pub use path_ext::*;
pub use search::*;
pub use url_encoding::*;
