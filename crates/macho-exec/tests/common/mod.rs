use std::sync::Arc;

use macho_exec::StubResolver;
use platform_core::memory::MemoryMapper;
use platform_core::unwind::UnwindRegistry;

#[allow(unused_imports)]
pub use macho_fixtures::{fixtures_dir, read_fixture};

pub fn default_mapper() -> Arc<dyn MemoryMapper> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(platform_darwin_host::DarwinMemoryMapper)
    }
    #[cfg(target_os = "linux")]
    {
        Arc::new(platform_linux::LinuxMemoryMapper)
    }
}

pub fn default_unwind() -> Arc<dyn UnwindRegistry> {
    Arc::new(platform_core::unwind::NoopUnwindRegistry)
}

// Each integration crate compiles this module separately; security tests do not
// resolve symbols, while the entry, mapping, and registry tests use this helper.
#[allow(dead_code)]
pub fn default_stub_resolver() -> StubResolver {
    #[cfg(target_os = "macos")]
    let host: Arc<dyn platform_core::symbols::HostSymbolResolver> =
        Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
    #[cfg(target_os = "linux")]
    let host: Arc<dyn platform_core::symbols::HostSymbolResolver> =
        Arc::new(platform_linux::LinuxSymbolResolver::new());
    StubResolver::new(host)
}
