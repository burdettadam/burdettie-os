//! Typestate loader pipeline.
//!
//! Encodes the Mach-O loader lifecycle as a compile-time state machine.
//! Each phase of loading is a distinct type; transitions are enforced by
//! the type system — you cannot skip or reorder phases.
//!
//! # Phases
//!
//! ```text
//! Parsed → Checked → Mapped → FixedUp → Registered → Protected → Initialized
//!                                                                      ↓
//!                                                                RuntimeResult
//! ```
//!
//! # Example
//!
//! ```no_run
//! use macho_runtime::typestate::Loader;
//! use macho_runtime::RuntimeConfig;
//! use std::path::Path;
//!
//! let path = Path::new("fixtures/return42/app");
//! let config = RuntimeConfig::permissive(path);
//! let result = Loader::parse(path, config).unwrap()
//!     .check_security().unwrap()
//!     .map_images().unwrap()
//!     .apply_fixups().unwrap()
//!     .register_objc().unwrap()
//!     .protect().unwrap()
//!     .initialize().unwrap()
//!     .run().unwrap();
//! assert_eq!(result.exit_code, 42);
//! ```

use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use macho_exec::{ImageRegistry, StubResolver};
use macho_parse::ParsedMachO;
use trust_policy::SecurityReport;

use crate::config::RuntimeConfig;
use crate::context::RuntimeContext;
use crate::tracer::RuntimeTracer;

// ── Phase markers (sealed) ──────────────────────────────────────────

mod sealed {
    pub trait Phase {}
}

/// Binary parsed, ready for security check.
pub struct Parsed;
/// Security evaluated, ready to map images.
pub struct Checked;
/// All images mapped into memory, ready for fixups.
pub struct Mapped;
/// Fixups applied (rebase + bind), ready for ObjC registration.
pub struct FixedUp;
/// ObjC metadata registered, ready to protect segments.
pub struct Registered;
/// Segment permissions finalized, ready for initialization.
pub struct Protected;
/// +load methods and constructors executed, ready to call entry.
pub struct Initialized;

impl sealed::Phase for Parsed {}
impl sealed::Phase for Checked {}
impl sealed::Phase for Mapped {}
impl sealed::Phase for FixedUp {}
impl sealed::Phase for Registered {}
impl sealed::Phase for Protected {}
impl sealed::Phase for Initialized {}

// ── Loader core ─────────────────────────────────────────────────────

/// Typestate Mach-O loader pipeline.
///
/// Each transition method consumes `self` and returns the next phase.
/// The compiler prevents calling methods in the wrong order.
pub struct Loader<S: sealed::Phase> {
    inner: LoaderInner,
    _phase: PhantomData<S>,
}

struct LoaderInner {
    binary_path: PathBuf,
    config: RuntimeConfig,
    tracer: RuntimeTracer,
    parsed: Arc<ParsedMachO>,
    main_data: Arc<[u8]>,
    // Populated after check_security
    security_report: Option<SecurityReport>,
    // Populated after map_images
    ctx: Option<RuntimeContext>,
    #[allow(dead_code)]
    host_symbols: Option<Arc<dyn platform_core::symbols::HostSymbolResolver>>,
    stub_resolver: Option<StubResolver>,
    registry: Option<ImageRegistry>,
    app_idx: usize,
    initializers_run: usize,
}

// ── Common accessors (all phases) ───────────────────────────────────

impl<S: sealed::Phase> Loader<S> {
    /// The parsed Mach-O binary.
    pub fn parsed(&self) -> &ParsedMachO {
        self.inner.parsed.as_ref()
    }

    /// Path to the original binary.
    pub fn binary_path(&self) -> &Path {
        &self.inner.binary_path
    }

    /// Security report (populated after `check_security`).
    pub fn security_report(&self) -> Option<&SecurityReport> {
        self.inner.security_report.as_ref()
    }

    /// Number of images loaded (populated after `map_images`).
    pub fn images_loaded(&self) -> usize {
        self.inner
            .registry
            .as_ref()
            .map(|r| r.images().len())
            .unwrap_or(0)
    }
}

mod phases_1_3;
mod phases_4_7;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn typestate_return42() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/return42/return42");
        if !path.exists() {
            eprintln!("Skipping: fixture not found");
            return;
        }
        let config = RuntimeConfig::permissive(&path);
        let result = Loader::parse(&path, config)
            .expect("parse")
            .check_security()
            .expect("security")
            .map_images()
            .expect("map")
            .apply_fixups()
            .expect("fixups")
            .register_objc()
            .expect("objc")
            .protect()
            .expect("protect")
            .initialize()
            .expect("init")
            .run()
            .expect("run");
        assert_eq!(result.exit_code, 42);
    }
}
