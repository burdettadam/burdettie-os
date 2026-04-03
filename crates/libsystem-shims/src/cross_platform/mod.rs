//! Cross-platform library wrappers.
//!
//! Thin shims that delegate to system libraries available on both
//! macOS and Linux: POSIX extras, and optionally libxml2, zlib, ICU
//! when those features are enabled.
//!
//! On macOS, many of these symbols resolve natively via the
//! StubResolver's dlsym path. These shims ensure they also resolve
//! on Linux where the host libraries differ.

mod icu;
mod posix_extras;

use crate::table::ShimTable;

/// Install all cross-platform shims into the table.
pub fn install_cross_platform_shims(table: &mut ShimTable) {
    posix_extras::PosixExtrasShims::install(table);
    icu::IcuShims::install(table);
    log::debug!("cross_platform: registered all cross-platform shims");
}
