//! Linux host implementation of `platform-core` traits.
//!
//! Provides a full `LinuxHostPlatform` implementing all 10 PAL traits,
//! enabling the runtime to compile and execute on Linux ARM64.

mod alloc;
mod clock;
mod fs;
mod hash;
mod memory;
mod process;
mod rng;
mod symbols;
mod threading;
mod unwind;

pub use alloc::LinuxAllocator;
pub use clock::LinuxClock;
pub use fs::LinuxFilesystem;
pub use hash::LinuxHasher;
pub use memory::LinuxMemoryMapper;
pub use process::LinuxProcessInfo;
pub use rng::LinuxRng;
pub use symbols::LinuxSymbolResolver;
pub use threading::LinuxThreading;
pub use unwind::LinuxUnwindRegistry;

use std::sync::Arc;

/// Bundle of all Linux host platform services.
pub struct LinuxHostPlatform {
    alloc: LinuxAllocator,
    symbols: LinuxSymbolResolver,
    unwind: LinuxUnwindRegistry,
    clock: LinuxClock,
    rng: LinuxRng,
    process: LinuxProcessInfo,
    fs: LinuxFilesystem,
    hash: LinuxHasher,
    threading: LinuxThreading,
}

impl LinuxHostPlatform {
    pub fn new() -> Self {
        LinuxProcessInfo::mark_main_thread();
        Self {
            alloc: LinuxAllocator,
            symbols: LinuxSymbolResolver::new(),
            unwind: LinuxUnwindRegistry::new(),
            clock: LinuxClock,
            rng: LinuxRng,
            process: LinuxProcessInfo,
            fs: LinuxFilesystem,
            hash: LinuxHasher,
            threading: LinuxThreading,
        }
    }
}

impl Default for LinuxHostPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl platform_core::PlatformProvider for LinuxHostPlatform {
    fn alloc(&self) -> &dyn platform_core::alloc::Allocator {
        &self.alloc
    }
    fn memory(&self) -> Arc<dyn platform_core::memory::MemoryMapper> {
        Arc::new(LinuxMemoryMapper)
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
    fn threading(&self) -> &dyn platform_core::threading::Threading {
        &self.threading
    }
}

