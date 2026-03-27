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
            runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::SymbolBlocked {
                name: name.into(),
            });
            return None;
        }

        let addr = self.host.resolve_in(self.handle, c_name);
        match addr {
            Some(ptr) => Some(ptr as u64),
            None => {
                log::debug!("StubResolver: host returned null for {c_name}");
                runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::SymbolMissing {
                    name: name.into(),
                });
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
            | "atexit"
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
            | "pthread_mutex_trylock"
            | "pthread_once" | "pthread_key_create"
            | "pthread_getspecific" | "pthread_setspecific"
            | "pthread_cond_init" | "pthread_cond_wait"
            | "pthread_cond_signal" | "pthread_cond_broadcast"
            | "pthread_cond_destroy"
            | "pthread_rwlock_init" | "pthread_rwlock_rdlock"
            | "pthread_rwlock_wrlock" | "pthread_rwlock_unlock"
            | "pthread_rwlock_destroy"
            // ObjC runtime
            | "objc_msgSend" | "objc_retain" | "objc_release"
            | "objc_autoreleasePoolPush" | "objc_autoreleasePoolPop"
            | "objc_getClass" | "objc_alloc" | "objc_alloc_init"
            | "objc_opt_self" | "objc_storeStrong"
            | "objc_opt_class" | "objc_opt_respondsToSelector"
            | "objc_msgSendSuper2"
            | "objc_setProperty_nonatomic" | "objc_setProperty_nonatomic_copy"
            | "objc_begin_catch" | "objc_end_catch"
            | "objc_exception_throw" | "objc_exception_rethrow"
            | "objc_terminate"
            | "object_getClass"
            // ObjC runtime introspection
            | "class_getName" | "class_getSuperclass" | "class_isMetaClass"
            | "class_getInstanceSize" | "class_getInstanceVariable"
            | "class_getInstanceMethod" | "class_getMethodImplementation"
            | "class_getProperty" | "class_respondsToSelector"
            | "class_addMethod" | "class_replaceMethod"
            | "class_copyMethodList" | "class_copyIvarList"
            | "class_copyPropertyList" | "class_createInstance"
            | "sel_getName" | "sel_registerName" | "sel_isEqual"
            | "method_getName" | "method_getTypeEncoding"
            | "method_getImplementation" | "method_setImplementation"
            | "method_getNumberOfArguments" | "method_exchangeImplementations"
            | "ivar_getName" | "ivar_getOffset" | "ivar_getTypeEncoding"
            | "property_getName" | "property_getAttributes"
            | "protocol_getName" | "protocol_isEqual"
            | "protocol_conformsToProtocol" | "protocol_copyProtocolList"
            | "protocol_copyMethodDescriptionList" | "protocol_getMethodDescription"
            | "protocol_copyPropertyList" | "protocol_getProperty"
            | "objc_getProtocol" | "objc_allocateProtocol" | "objc_registerProtocol"
            | "class_conformsToProtocol" | "class_copyProtocolList"
            | "class_addProtocol"
            | "objc_allocateClassPair" | "objc_registerClassPair"
            | "objc_disposeClassPair" | "objc_duplicateClass"
            | "class_addIvar" | "class_addProperty"
            // Blocks runtime
            | "_Block_copy" | "_Block_release"
            | "_Block_object_assign" | "_Block_object_dispose"
            | "_NSConcreteStackBlock" | "_NSConcreteGlobalBlock"
            // GCD
            | "dispatch_once" | "dispatch_once_f"
            | "dispatch_sync" | "dispatch_async"
            | "dispatch_queue_create" | "dispatch_get_main_queue"
            | "dispatch_get_global_queue"
            | "_dispatch_main_q"
            // Math
            | "floor" | "ceil" | "round" | "sqrt" | "pow"
            | "log" | "log2" | "log10" | "exp" | "exp2"
            | "sin" | "cos" | "tan" | "fabs" | "fmod"
            // Random — now shimmed via RandomShims, but keep safe-list
            // entries for binaries that link directly.
            | "arc4random" | "arc4random_buf" | "arc4random_uniform"
            // Mach
            | "mach_task_self" | "mach_absolute_time"
            // System info
            | "sysctl" | "sysctlbyname" | "getrusage"
            // Atomics / locks
            | "os_unfair_lock_lock" | "os_unfair_lock_unlock"
            | "OSAtomicAdd32" | "OSAtomicIncrement32"
            // Exception handling / unwinding
            | "_Unwind_Resume" | "__objc_personality_v0"
            // TLS
            | "_tlv_bootstrap"
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
        #[cfg(target_os = "macos")]
        let host: Arc<dyn HostSymbolResolver> =
            Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        #[cfg(target_os = "linux")]
        let host: Arc<dyn HostSymbolResolver> =
            Arc::new(platform_linux::LinuxSymbolResolver::new());
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
