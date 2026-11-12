//! Controlled system library shims for the burdettie-os runtime.
//!
//! Each shim is a Rust function with `extern "C"` ABI that can be
//! registered in the `ShimTable` and resolved by the runtime's symbol
//! resolver chain. Shims fall into three strategies:
// Function pointers are cast to u64 addresses for the emulated Mach-O address space.
#![allow(
    dead_code,                      // FFI shims referenced by pointer, not by Rust call
    clippy::fn_to_numeric_cast,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
)]
//!
//! 1. **Implement** — pure Rust implementation (e.g. `strlen`, `getprogname`)
//! 2. **Delegate** — forward to host via `libc` (e.g. `malloc`, `printf`)
//! 3. **Deny** — abort or return error (e.g. `fork`, `exec`)
//!
//! # Safety
//!
//! Shim functions are called from mapped Mach-O binaries via function
//! pointers. They must maintain C ABI compatibility and handle all
//! inputs defensively.

#[cfg(not(unix))]
mod non_unix;

#[cfg(unix)]
pub(crate) mod darwin_compat;
#[cfg(not(unix))]
pub(crate) mod darwin_compat {
    pub use crate::non_unix::DarwinCompatShims;
}
#[cfg(unix)]
pub(crate) mod dispatch;
#[cfg(not(unix))]
pub(crate) mod dispatch {
    pub use crate::non_unix::DispatchShims;
}
#[cfg(unix)]
pub(crate) mod lifecycle;
#[cfg(not(unix))]
pub(crate) mod lifecycle {
    pub use crate::non_unix::LifecycleShims;
}
#[cfg(unix)]
pub(crate) mod mach;
#[cfg(not(unix))]
pub(crate) mod mach {
    pub use crate::non_unix::MachShims;
}
pub mod process;
#[cfg(unix)]
pub(crate) mod pthread;
pub(crate) mod ptrauth;
#[cfg(not(unix))]
pub(crate) mod pthread {
    pub use crate::non_unix::PthreadShims;
}
pub(crate) mod random;
#[cfg(unix)]
pub(crate) mod socket;
#[cfg(not(unix))]
pub(crate) mod socket {
    pub use crate::non_unix::SocketShims;
}
pub mod table;
#[cfg(unix)]
pub(crate) mod tls;
#[cfg(not(unix))]
pub(crate) mod tls {
    pub use crate::non_unix::TlsShims;
}
#[cfg(unix)]
pub mod cross_platform;
#[cfg(not(unix))]
pub mod cross_platform {
    use crate::table::ShimTable;

    pub fn install_cross_platform_shims(_table: &mut ShimTable) {
        log::debug!("cross_platform: Unix-backed shims unavailable on this host");
    }
}
#[cfg(unix)]
pub mod darwin_stubs;
#[cfg(not(unix))]
#[path = "darwin_stubs_non_unix.rs"]
pub mod darwin_stubs;

pub use darwin_compat::DarwinCompatShims;
pub use darwin_stubs::install_darwin_stubs;
pub use dispatch::DispatchShims;
pub use lifecycle::LifecycleShims;
pub use mach::MachShims;
pub use process::ProcessShims;
pub use pthread::PthreadShims;
pub use ptrauth::PtrauthShims;
pub use random::RandomShims;
pub use socket::SocketShims;
pub use table::{ShimEntry, ShimTable};
pub use tls::TlsShims;

/// Install all default shims into a table.
#[doc(hidden)]
pub fn install_default_shims(table: &mut ShimTable, config: &ShimConfig) {
    ProcessShims::install(table, config);
    TlsShims::install(table);
    DispatchShims::install(table);
    PthreadShims::install(table);
    MachShims::install(table);
    PtrauthShims::install(table);
    LifecycleShims::install(table);
    RandomShims::install(table);
    SocketShims::install(table);
    DarwinCompatShims::install(table);
    cross_platform::install_cross_platform_shims(table);
    darwin_stubs::install_darwin_stubs(table);
}

/// Reset process-global shim state so the runtime can be reused in-process.
pub fn reset_runtime_state_for_reuse() {
    process::clear_env_overrides();
    TlsShims::reset();
    LifecycleShims::reset();
}

/// Configuration for shim behavior.
#[derive(Debug, Clone)]
pub struct ShimConfig {
    /// The process name returned by `getprogname`.
    pub process_name: String,
    /// The executable path returned by `_NSGetExecutablePath`.
    pub executable_path: String,
    /// argc for `_NSGetArgc`.
    pub argc: i32,
    /// argv strings for `_NSGetArgv`.
    pub argv: Vec<String>,
    /// Environment variables for `getenv`.
    pub env: Vec<(String, String)>,
}

impl Default for ShimConfig {
    fn default() -> Self {
        Self {
            process_name: "burdettie-os".to_string(),
            executable_path: "/usr/local/bin/app".to_string(),
            argc: 1,
            argv: vec!["app".to_string()],
            env: Vec::new(),
        }
    }
}
