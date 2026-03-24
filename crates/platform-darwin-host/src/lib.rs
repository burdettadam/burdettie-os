//! macOS host implementation of `platform-core` traits.
//!
//! All host-specific `libc::` calls are centralised here.  The runtime
//! crates depend only on `platform-core` traits and receive a
//! `DarwinHostPlatform` via dependency injection.

mod clock;
mod fs;
mod hash;
mod memory;
mod process;
mod rng;
mod symbols;
mod unwind;

pub use clock::DarwinClock;
pub use fs::DarwinFilesystem;
pub use hash::DarwinHasher;
pub use memory::DarwinMemoryMapper;
pub use process::DarwinProcessInfo;
pub use rng::DarwinRng;
pub use symbols::DarwinSymbolResolver;
pub use unwind::DarwinUnwindRegistry;

use std::sync::Arc;

/// Bundle of all macOS host platform services.
pub struct DarwinHostPlatform {
    symbols: DarwinSymbolResolver,
    unwind: DarwinUnwindRegistry,
    clock: DarwinClock,
    rng: DarwinRng,
    process: DarwinProcessInfo,
    fs: DarwinFilesystem,
    hash: DarwinHasher,
}

impl DarwinHostPlatform {
    pub fn new() -> Self {
        Self {
            symbols: DarwinSymbolResolver::new(),
            unwind: DarwinUnwindRegistry,
            clock: DarwinClock,
            rng: DarwinRng,
            process: DarwinProcessInfo,
            fs: DarwinFilesystem,
            hash: DarwinHasher,
        }
    }
}

impl Default for DarwinHostPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl platform_core::PlatformProvider for DarwinHostPlatform {
    fn memory(&self) -> Arc<dyn platform_core::memory::MemoryMapper> {
        Arc::new(DarwinMemoryMapper)
    }
    fn symbols(&self) -> &dyn platform_core::symbols::HostSymbolResolver {
        &self.symbols
    }
    fn unwind(&self) -> &dyn platform_core::unwind::UnwindRegistry {
        &self.unwind
    }
    fn clock(&self) -> &dyn platform_core::clock::Clock {
        &self.clock
    }
    fn rng(&self) -> &dyn platform_core::rng::Rng {
        &self.rng
    }
    fn process(&self) -> &dyn platform_core::process::ProcessInfo {
        &self.process
    }
    fn fs(&self) -> &dyn platform_core::fs::FilesystemProvider {
        &self.fs
    }
    fn hash(&self) -> &dyn platform_core::hash::Hasher {
        &self.hash
    }
}
