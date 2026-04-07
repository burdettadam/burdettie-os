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
mod kqueue;
mod posix_extras;
mod stat;
mod dirent;
mod fts;
pub mod variadic_shims;

use crate::table::ShimTable;

/// Install all cross-platform shims into the table.
pub fn install_cross_platform_shims(table: &mut ShimTable) {
    log::info!("cross_platform: starting installation");
    posix_extras::PosixExtrasShims::install(table);
    icu::IcuShims::install(table);
    stat::StatShims::install(table);
    dirent::DirentShims::install(table);
    variadic_shims::VariadicShims::install(table);
    kqueue::KqueueShims::install(table);
    fts::FtsShims::install(table);
    log::info!("cross_platform: registered all cross-platform shims");
}
