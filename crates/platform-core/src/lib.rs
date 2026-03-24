//! Platform abstraction traits for burdetty-os.
//!
//! Defines the interfaces that the runtime needs from the host platform.
//! All host-specific implementations live in `platform-darwin-host`
//! (or future platform crates).  Core crates depend only on these traits,
//! making the runtime portable in theory.

pub mod clock;
pub mod fs;
pub mod hash;
pub mod memory;
pub mod process;
pub mod rng;
pub mod symbols;
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
}

/// Bundle trait that provides access to all platform capabilities.
///
/// Implementors supply all the host services the runtime needs via
/// a single trait object.
pub trait PlatformProvider: Send + Sync {
    fn memory(&self) -> Arc<dyn memory::MemoryMapper>;
    fn symbols(&self) -> &dyn symbols::HostSymbolResolver;
    fn unwind(&self) -> &dyn unwind::UnwindRegistry;
    fn clock(&self) -> &dyn clock::Clock;
    fn rng(&self) -> &dyn rng::Rng;
    fn process(&self) -> &dyn process::ProcessInfo;
    fn fs(&self) -> &dyn fs::FilesystemProvider;
    fn hash(&self) -> &dyn hash::Hasher;
}
