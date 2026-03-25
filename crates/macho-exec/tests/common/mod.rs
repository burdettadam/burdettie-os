use std::sync::Arc;

use macho_exec::StubResolver;
use platform_core::memory::MemoryMapper;
use platform_core::unwind::UnwindRegistry;

#[allow(unused_imports)]
pub use macho_fixtures::{fixtures_dir, read_fixture};

pub fn default_mapper() -> Arc<dyn MemoryMapper> {
    Arc::new(platform_darwin_host::DarwinMemoryMapper)
}

pub fn default_unwind() -> Arc<dyn UnwindRegistry> {
    Arc::new(platform_core::unwind::NoopUnwindRegistry)
}

pub fn default_stub_resolver() -> StubResolver {
    let host: Arc<dyn platform_core::symbols::HostSymbolResolver> =
        Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
    StubResolver::new(host)
}
