//! Platform abstraction traits for burdettie-os.
//!
//! Defines the interfaces that the runtime needs from the host platform.
//! All host-specific implementations live in `platform-darwin-host`
//! (or future platform crates).  Core crates depend only on these traits,
//! making the runtime portable in theory.

pub mod alloc;
pub mod clock;
pub mod fs;
pub mod hash;
pub mod memory;
pub mod process;
pub mod rng;
pub mod symbols;
pub mod threading;
pub mod unwind;

use std::fmt;
use std::sync::Arc;

/// Errors returned by PAL operations.
#[derive(Debug)]
pub enum PalError {
    /// A memory-mapping operation failed.
    Memory(String),
    /// An I/O operation failed.
    Io(std::io::Error),
    /// A generic platform error.
    Other(String),
}

impl fmt::Display for PalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(msg) => write!(f, "PAL memory error: {msg}"),
            Self::Io(e) => write!(f, "PAL I/O error: {e}"),
            Self::Other(msg) => write!(f, "PAL error: {msg}"),
        }
    }
}

impl std::error::Error for PalError {}

impl From<std::io::Error> for PalError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Memory protection flags (platform-independent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protection {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Protection {
    pub const READ_WRITE: Self = Self {
        read: true,
        write: true,
        execute: false,
    };
    pub const READ_ONLY: Self = Self {
        read: true,
        write: false,
        execute: false,
    };
    pub const READ_EXECUTE: Self = Self {
        read: true,
        write: false,
        execute: true,
    };
    pub const NONE: Self = Self {
        read: false,
        write: false,
        execute: false,
    };

    /// Combine two protection sets, granting any permission present in either.
    pub const fn union(self, other: Self) -> Self {
        Self {
            read: self.read || other.read,
            write: self.write || other.write,
            execute: self.execute || other.execute,
        }
    }
}

/// Bundle trait that provides access to all platform capabilities.
///
/// Implementors supply all the host services the runtime needs via
/// a single trait object.
pub trait PlatformProvider: Send + Sync {
    fn alloc(&self) -> &dyn alloc::Allocator;
    fn memory(&self) -> Arc<dyn memory::MemoryMapper>;
    fn symbols(&self) -> &dyn symbols::HostSymbolResolver;
    fn unwind(&self) -> &dyn unwind::UnwindRegistry;
    fn clock(&self) -> &dyn clock::Clock;
    fn rng(&self) -> &dyn rng::Rng;
    fn process(&self) -> &dyn process::ProcessInfo;
    fn fs(&self) -> &dyn fs::FilesystemProvider;
    fn hash(&self) -> &dyn hash::Hasher;
    fn threading(&self) -> &dyn threading::Threading;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PalError Display ──────────────────────────────────────────

    #[test]
    fn pal_error_memory_display() {
        let e = PalError::Memory("out of pages".into());
        assert_eq!(e.to_string(), "PAL memory error: out of pages");
    }

    #[test]
    fn pal_error_io_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let e = PalError::Io(io_err);
        assert_eq!(e.to_string(), "PAL I/O error: gone");
    }

    #[test]
    fn pal_error_other_display() {
        let e = PalError::Other("something broke".into());
        assert_eq!(e.to_string(), "PAL error: something broke");
    }

    #[test]
    fn pal_error_memory_empty_message() {
        let e = PalError::Memory(String::new());
        assert_eq!(e.to_string(), "PAL memory error: ");
    }

    // ── PalError implements std::error::Error ─────────────────────

    #[test]
    fn pal_error_implements_error_trait() {
        use std::error::Error;
        let e = PalError::Other("test".into());
        let _err_ref: &dyn Error = &e;
        assert!(e.source().is_none());
    }

    // ── PalError From<io::Error> ──────────────────────────────────

    #[test]
    fn pal_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");
        let pal: PalError = io_err.into();
        match &pal {
            PalError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied),
            other => panic!("expected Io variant, got: {other:?}"),
        }
    }

    // ── PalError Debug ────────────────────────────────────────────

    #[test]
    fn pal_error_debug() {
        let e = PalError::Memory("boom".into());
        let dbg = format!("{e:?}");
        assert!(dbg.contains("Memory"));
        assert!(dbg.contains("boom"));
    }

    #[test]
    fn pal_error_io_debug() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "disk");
        let e = PalError::Io(io_err);
        let dbg = format!("{e:?}");
        assert!(dbg.contains("Io"));
    }

    // ── Protection constants ──────────────────────────────────────

    #[test]
    fn protection_read_write() {
        assert_eq!(
            Protection::READ_WRITE,
            Protection { read: true, write: true, execute: false }
        );
    }

    #[test]
    fn protection_read_only() {
        assert_eq!(
            Protection::READ_ONLY,
            Protection { read: true, write: false, execute: false }
        );
    }

    #[test]
    fn protection_read_execute() {
        assert_eq!(
            Protection::READ_EXECUTE,
            Protection { read: true, write: false, execute: true }
        );
    }

    #[test]
    fn protection_none() {
        assert_eq!(
            Protection::NONE,
            Protection { read: false, write: false, execute: false }
        );
    }

    // ── Protection derives ────────────────────────────────────────

    #[test]
    fn protection_clone_and_copy() {
        let p = Protection::READ_WRITE;
        let cloned = p.clone();
        let copied = p; // Copy
        assert_eq!(p, cloned);
        assert_eq!(p, copied);
    }

    #[test]
    fn protection_eq_and_ne() {
        assert_eq!(Protection::READ_ONLY, Protection::READ_ONLY);
        assert_ne!(Protection::READ_ONLY, Protection::NONE);
    }

    #[test]
    fn protection_debug() {
        let dbg = format!("{:?}", Protection::NONE);
        assert!(dbg.contains("Protection"));
        assert!(dbg.contains("read"));
    }

    #[test]
    fn protection_custom() {
        let p = Protection { read: false, write: true, execute: true };
        assert!(!p.read);
        assert!(p.write);
        assert!(p.execute);
    }
}
