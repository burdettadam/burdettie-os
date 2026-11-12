//! C-ABI FFI wrapper for the burdettie-os runtime.
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
mod introspection;
mod memory;
mod run;

pub use error::darwin_runtime_last_error;
pub use introspection::*;
pub use memory::*;
pub use run::*;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_returns_expected() {
        let v = darwin_runtime_version();
        assert_eq!(v.major, 0);
        assert!(v.minor >= 1);
    }

    #[test]
    fn version_struct_fields() {
        let v = darwin_runtime_version();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }
}
