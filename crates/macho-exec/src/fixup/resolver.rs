//! StubResolver: maps known-safe symbols to host process addresses.

use platform_core::symbols::HostSymbolResolver;
use std::sync::Arc;

/// Trait for resolving symbol names to in-process addresses.
pub trait SymbolResolver {
    fn resolve(&self, symbol_name: &str) -> Option<u64>;
}

/// Resolves symbol stubs to real in-process function addresses.
///
/// Only resolves symbols on a curated safe-list to prevent dangerous
/// side effects during lab execution.
pub struct StubResolver {
    host: Arc<dyn HostSymbolResolver>,
    handle: *mut (),
}

// StubResolver's handle is an opaque RTLD_DEFAULT / dlopen handle.
// It is read-only and valid for the process lifetime.
unsafe impl Send for StubResolver {}
unsafe impl Sync for StubResolver {}

impl StubResolver {
    pub fn new(host: Arc<dyn HostSymbolResolver>) -> Self {
        let handle = host.default_handle();
        Self { host, handle }
    }

    pub fn resolve(&self, name: &str) -> Option<u64> {
        let c_name = name.strip_prefix('_').unwrap_or(name);
        if !Self::is_safe_symbol(c_name) {
            log::debug!("StubResolver: skipping unsafe symbol: {name}");
            return None;
        }

        let addr = self.host.resolve_in(self.handle, c_name);
        match addr {
            Some(ptr) => Some(ptr as u64),
            None => {
                log::debug!("StubResolver: host returned null for {c_name}");
                None
            }
        }
    }

    fn is_safe_symbol(name: &str) -> bool {
        matches!(
            name,
            // I/O
            "printf" | "puts" | "putchar" | "fprintf" | "snprintf" | "sprintf"
            | "write" | "fputs" | "fwrite" | "fread" | "fflush"
            | "fopen" | "fclose" | "__stderrp" | "__stdoutp" | "__stdinp"
            // Memory
            | "malloc" | "calloc" | "realloc" | "free"
            | "memcpy" | "memset" | "memmove" | "memcmp" | "memchr"
            // Strings
            | "strlen" | "strnlen" | "strcmp" | "strncmp"
            | "strcpy" | "strncpy" | "strchr" | "strrchr" | "strstr"
            | "strerror" | "strdup" | "strlcpy" | "strlcat"
            // Process
            | "exit" | "_exit" | "abort" | "getenv" | "getpid" | "isatty"
            // Conversion
            | "atoi" | "atol" | "strtol" | "strtoul" | "strtod"
            // Stack protection
            | "__stack_chk_fail" | "__stack_chk_guard"
            // Assertion
            | "__assert_rtn"
            // pthread
            | "pthread_create" | "pthread_join" | "pthread_self"
            | "pthread_mutex_init" | "pthread_mutex_lock"
            | "pthread_mutex_unlock" | "pthread_mutex_destroy"
            | "pthread_once" | "pthread_key_create"
            | "pthread_getspecific" | "pthread_setspecific"
            // ObjC runtime
            | "objc_msgSend" | "objc_retain" | "objc_release"
            | "objc_autoreleasePoolPush" | "objc_autoreleasePoolPop"
            | "objc_getClass" | "objc_alloc" | "objc_alloc_init"
            | "objc_opt_self" | "objc_storeStrong"
            // Blocks runtime
            | "_Block_copy" | "_Block_release"
            | "_Block_object_assign" | "_Block_object_dispose"
            // GCD
            | "dispatch_once" | "dispatch_once_f"
            // Math
            | "floor" | "ceil" | "round" | "sqrt" | "pow"
            | "log" | "sin" | "cos" | "fabs"
            // Mach
            | "mach_task_self" | "mach_absolute_time"
            // Exception handling / unwinding
            | "_Unwind_Resume" | "__objc_personality_v0"
        )
    }
}

impl SymbolResolver for StubResolver {
    fn resolve(&self, symbol_name: &str) -> Option<u64> {
        self.resolve(symbol_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_resolver() -> StubResolver {
        let host: Arc<dyn HostSymbolResolver> =
            Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        StubResolver::new(host)
    }

    #[test]
    fn safe_symbols_allowed() {
        assert!(StubResolver::is_safe_symbol("printf"));
        assert!(StubResolver::is_safe_symbol("puts"));
        assert!(StubResolver::is_safe_symbol("malloc"));
        assert!(StubResolver::is_safe_symbol("free"));
        assert!(StubResolver::is_safe_symbol("exit"));
    }

    #[test]
    fn unsafe_symbols_blocked() {
        assert!(!StubResolver::is_safe_symbol("system"));
        assert!(!StubResolver::is_safe_symbol("exec"));
        assert!(!StubResolver::is_safe_symbol("popen"));
        assert!(!StubResolver::is_safe_symbol("fork"));
        assert!(!StubResolver::is_safe_symbol("dlopen"));
    }

    #[test]
    fn stub_resolver_finds_printf() {
        let resolver = test_resolver();
        let addr = resolver.resolve("_printf");
        assert!(addr.is_some(), "should resolve printf via dlsym");
        assert!(addr.unwrap() != 0, "printf address should be non-zero");
    }

    #[test]
    fn stub_resolver_strips_underscore() {
        let resolver = test_resolver();
        let addr = resolver.resolve("_puts");
        assert!(addr.is_some(), "should resolve _puts → puts");
    }

    #[test]
    fn stub_resolver_rejects_unsafe() {
        let resolver = test_resolver();
        let addr = resolver.resolve("_system");
        assert!(addr.is_none(), "should not resolve _system");
    }
}
