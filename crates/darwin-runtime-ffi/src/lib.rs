//! C-ABI FFI wrapper for the burdetty-os runtime.
//!
//! Exposes the runtime's core functionality through stable `extern "C"` functions
//! that can be called from C, Swift, Python, or any language with C FFI support.
//!
//! # Usage from C
//!
//! ```c
//! #include "darwin_runtime.h"
//!
//! DarwinVersion v = darwin_runtime_version();
//! printf("darwin-runtime %d.%d.%d\n", v.major, v.minor, v.patch);
//!
//! DarwinRuntimeResult result = darwin_runtime_run("/path/to/binary");
//! printf("exit code: %d, images loaded: %zu\n", result.exit_code, result.images_loaded);
//! ```

mod error;
mod run;
mod introspection;
mod memory;

pub use error::darwin_runtime_last_error;
pub use run::*;
pub use introspection::*;
pub use memory::*;

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// Version information for the darwin-runtime library.
#[repr(C)]
pub struct DarwinVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Returns the library version.
#[no_mangle]
pub extern "C" fn darwin_runtime_version() -> DarwinVersion {
    DarwinVersion {
        major: 0,
        minor: 1,
        patch: 0,
    }
}
