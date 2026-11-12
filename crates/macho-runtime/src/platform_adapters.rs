use std::sync::Arc;

use platform_core::PlatformProvider;

pub(crate) struct ProviderHostSymbols {
    platform: Arc<dyn PlatformProvider>,
}

impl ProviderHostSymbols {
    pub(crate) fn new(platform: Arc<dyn PlatformProvider>) -> Self {
        Self { platform }
    }
}

impl platform_core::symbols::HostSymbolResolver for ProviderHostSymbols {
    fn resolve(&self, name: &str) -> Option<*const ()> {
        self.platform.symbols().resolve(name)
    }

    fn resolve_in(&self, handle: *mut (), name: &str) -> Option<*const ()> {
        self.platform.symbols().resolve_in(handle, name)
    }

    fn open_library(&self, path: &str) -> Option<*mut ()> {
        self.platform.symbols().open_library(path)
    }

    fn default_handle(&self) -> *mut () {
        self.platform.symbols().default_handle()
    }
}

pub(crate) struct ProviderAllocator {
    platform: Arc<dyn PlatformProvider>,
}

impl ProviderAllocator {
    pub(crate) fn new(platform: Arc<dyn PlatformProvider>) -> Self {
        Self { platform }
    }
}

impl platform_core::alloc::Allocator for ProviderAllocator {
    fn calloc(&self, count: usize, size: usize) -> *mut u8 {
        self.platform.alloc().calloc(count, size)
    }

    fn malloc(&self, size: usize) -> *mut u8 {
        self.platform.alloc().malloc(size)
    }

    fn realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        self.platform.alloc().realloc(ptr, new_size)
    }

    fn free(&self, ptr: *mut u8) {
        self.platform.alloc().free(ptr);
    }
}

pub(crate) struct ProviderThreading {
    platform: Arc<dyn PlatformProvider>,
}

impl ProviderThreading {
    pub(crate) fn new(platform: Arc<dyn PlatformProvider>) -> Self {
        Self { platform }
    }
}

impl platform_core::threading::Threading for ProviderThreading {
    fn mutex_size(&self) -> usize {
        self.platform.threading().mutex_size()
    }

    fn cond_size(&self) -> usize {
        self.platform.threading().cond_size()
    }

    fn mutex_init(&self, ptr: platform_core::threading::MutexPtr) {
        self.platform.threading().mutex_init(ptr);
    }

    fn mutex_init_recursive(&self, ptr: platform_core::threading::MutexPtr) {
        self.platform.threading().mutex_init_recursive(ptr);
    }

    fn mutex_lock(&self, ptr: platform_core::threading::MutexPtr) {
        self.platform.threading().mutex_lock(ptr);
    }

    fn mutex_unlock(&self, ptr: platform_core::threading::MutexPtr) {
        self.platform.threading().mutex_unlock(ptr);
    }

    fn mutex_trylock(&self, ptr: platform_core::threading::MutexPtr) -> bool {
        self.platform.threading().mutex_trylock(ptr)
    }

    fn mutex_destroy(&self, ptr: platform_core::threading::MutexPtr) {
        self.platform.threading().mutex_destroy(ptr);
    }

    fn cond_init(&self, ptr: platform_core::threading::CondPtr) {
        self.platform.threading().cond_init(ptr);
    }

    fn cond_wait(
        &self,
        cond: platform_core::threading::CondPtr,
        mutex: platform_core::threading::MutexPtr,
    ) {
        self.platform.threading().cond_wait(cond, mutex);
    }

    fn cond_signal(&self, ptr: platform_core::threading::CondPtr) {
        self.platform.threading().cond_signal(ptr);
    }

    fn cond_broadcast(&self, ptr: platform_core::threading::CondPtr) {
        self.platform.threading().cond_broadcast(ptr);
    }

    fn cond_destroy(&self, ptr: platform_core::threading::CondPtr) {
        self.platform.threading().cond_destroy(ptr);
    }
}

pub(crate) struct ProviderUnwindRegistry {
    platform: Arc<dyn PlatformProvider>,
}

impl ProviderUnwindRegistry {
    pub(crate) fn new(platform: Arc<dyn PlatformProvider>) -> Self {
        Self { platform }
    }
}

impl platform_core::unwind::UnwindRegistry for ProviderUnwindRegistry {
    fn register(
        &self,
        text_start: usize,
        text_end: usize,
        compact_unwind: Option<(usize, usize)>,
        eh_frame: Option<(usize, usize)>,
    ) -> Result<(), platform_core::PalError> {
        self.platform
            .unwind()
            .register(text_start, text_end, compact_unwind, eh_frame)
    }

    fn unregister(&self, text_start: usize) -> Result<(), platform_core::PalError> {
        self.platform.unwind().unregister(text_start)
    }
}

pub(crate) struct ProviderHasher {
    platform: Arc<dyn PlatformProvider>,
}

impl ProviderHasher {
    pub(crate) fn new(platform: Arc<dyn PlatformProvider>) -> Self {
        Self { platform }
    }
}

impl platform_core::hash::Hasher for ProviderHasher {
    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        self.platform.hash().sha256(data)
    }
}
