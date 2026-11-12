//! Runtime orchestration for burdettie-os.
//!
//! `macho-runtime` provides a single entry point — `Runtime::load_and_run` —
//! that takes a Mach-O binary path and a configuration, then drives the
//! full pipeline:
//!
//! 1. Parse the binary
//! 2. Evaluate security policy
//! 3. Resolve and load dependencies (BFS)
//! 4. Map all images via `ImageRegistry`
//! 5. Install shims into the resolution chain
//! 6. Apply fixups (rebases + binds) across all images
//! 7. Protect all images (mprotect)
//! 8. Run constructors for all images in load order
//! 9. Transfer to entry point
//!
//! The runtime **never** re-implements parsing, mapping, or fixup logic.
//! It orchestrates the loader core crates and provides the execution
//! environment (shims, TLS, process identity) that real programs expect.

mod app_bundle;
pub mod config;
pub mod context;
pub mod error;
pub mod interp_bridge;
mod loader;
pub mod objc_setup;
pub(crate) mod parse_cache;
mod pipeline_interp;
mod pipeline_native;
mod platform_adapters;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod platform_fallback;
pub mod resolver;
mod security;
pub mod shared_cache;
pub mod tracer;
pub mod typestate;

pub use config::{RuntimeConfig, ShimSet};
pub use context::RuntimeContext;
pub use error::RuntimeError;
pub use resolver::{CompositeResolver, EagerCacheResolver};
pub use tracer::{EventKind, RuntimeEvent, RuntimeTracer, TRACER_SCHEMA_VERSION};

use std::path::Path;

use trust_policy::SecurityReport;

/// Reset process-global runtime state so the loader can be reused safely for
/// multiple guest executions inside the same host process.
pub fn reset_runtime_state_for_reuse() {
    cf_lite::reset_runtime_state_for_reuse();
    libsystem_shims::reset_runtime_state_for_reuse();
}

/// Result of a complete runtime execution.
#[derive(Debug)]
pub struct RuntimeResult {
    /// Process exit code from entry point.
    pub exit_code: i32,
    /// Security evaluation report (if policy was applied).
    pub security_report: Option<SecurityReport>,
    /// Number of images loaded (including main binary).
    pub images_loaded: usize,
    /// Number of shims installed.
    pub shims_installed: usize,
    /// Number of constructors/initializers run.
    pub initializers_run: usize,
    /// Runtime event tracer with collected events (if tracing was enabled).
    pub tracer: Option<RuntimeTracer>,
}

/// Top-level runtime facade.
pub struct Runtime;

impl Runtime {
    /// Load a Mach-O binary and run it to completion.
    ///
    /// This is the primary entry point for the runtime layer.
    /// It orchestrates parsing, security, dependency resolution,
    /// mapping (via `ImageRegistry`), shim installation, fixups,
    /// protection, initializers, and entry.
    pub fn load_and_run(
        binary_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<RuntimeResult, RuntimeError> {
        pipeline_native::run_native(binary_path, config)
    }

    /// Load a Mach-O binary and run it via the ARM64 interpreter.
    ///
    /// Same pipeline as [`load_and_run`] through phase 7.5 (parse, map, shim,
    /// fixup, ObjC registration), then executes through the interpreter instead
    /// of native execution. This enables running ObjC/Foundation fixtures on
    /// any architecture.
    pub fn load_and_run_interpreted(
        binary_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<RuntimeResult, RuntimeError> {
        pipeline_interp::run_interpreted(binary_path, config)
    }

    /// Resolve an `.app` bundle path to the executable inside.
    ///
    /// Given a path like `Foo.app`, reads `Contents/Info.plist` for
    /// `CFBundleExecutable` and returns `Foo.app/Contents/MacOS/<exe>`.
    /// Returns `None` if the path is not an `.app` bundle.
    pub fn resolve_app_bundle(bundle_path: &Path) -> Option<std::path::PathBuf> {
        app_bundle::resolve_bundle(bundle_path)
    }
}
