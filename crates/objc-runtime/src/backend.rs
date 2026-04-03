//! ObjC runtime backend abstraction.
//!
//! Defines the `RuntimeBackend` trait that abstracts the core ObjC runtime
//! operations. The default implementation uses our built-in runtime
//! (`class::ObjcRuntime`). When the `libobjc2` feature is enabled, this
//! can be swapped to delegate to GNUstep's libobjc2 on Linux.
//!
//! # Feature: `libobjc2`
//!
//! GNUstep's libobjc2 provides a full Objective-C 2.0 runtime compatible
//! with Apple's ABI. Enabling this feature would:
//!
//! - Use libobjc2's `objc_msgSend` instead of our trampoline
//! - Delegate class/protocol registration to libobjc2
//! - Enable real `@try`/`@catch` exception handling via DWARF unwinding
//! - Allow loading GNUstep's Foundation as a real shared library
//!
//! ## Status: Groundwork only
//!
//! The trait and built-in backend are defined. The libobjc2 backend is
//! feature-gated but not yet implemented — it serves as a compilation
//! boundary and documents the integration surface.

/// Core ObjC runtime operations that can be backed by different implementations.
pub trait RuntimeBackend: Send + Sync {
    // ── Class operations ────────────────────────────────────────

    /// Look up a class by name. Returns the class pointer or 0.
    fn objc_get_class(&self, name: &str) -> u64;

    /// Allocate a new class pair (metaclass + class).
    /// Returns 0 on failure.
    fn objc_allocate_class_pair(&self, superclass: u64, name: &str) -> u64;

    /// Register a class pair previously allocated with `objc_allocate_class_pair`.
    fn objc_register_class_pair(&self, cls: u64);

    // ── Selector operations ─────────────────────────────────────

    /// Intern a selector name, returning its stable address.
    fn sel_register_name(&self, name: &str) -> u64;

    /// Get the name of a selector from its address.
    fn sel_get_name(&self, sel: u64) -> Option<&str>;

    // ── Method operations ───────────────────────────────────────

    /// Add a method to a class. Returns true if added, false if it already existed.
    fn class_add_method(&self, cls: u64, sel: u64, imp: u64, types: &str) -> bool;

    /// Replace an existing method implementation. Returns the previous IMP or 0.
    fn class_replace_method(&self, cls: u64, sel: u64, imp: u64, types: &str) -> u64;

    /// Look up the implementation for a selector on a class.
    fn class_get_method_implementation(&self, cls: u64, sel: u64) -> u64;

    // ── Object lifecycle ────────────────────────────────────────

    /// Allocate an instance of a class with extra bytes.
    fn class_create_instance(&self, cls: u64, extra_bytes: usize) -> u64;

    /// Deallocate an object.
    fn object_dispose(&self, obj: u64);

    // ── Message dispatch ────────────────────────────────────────

    /// Get the address of the `objc_msgSend` trampoline/function.
    fn msg_send_addr(&self) -> u64;

    /// Get the address of `objc_msgSend_stret` (struct-return variant).
    fn msg_send_stret_addr(&self) -> u64;
}

/// The built-in runtime backend using our `class::ObjcRuntime`.
///
/// This is the default backend used when `libobjc2` feature is not enabled.
pub struct BuiltinBackend;

impl RuntimeBackend for BuiltinBackend {
    fn objc_get_class(&self, name: &str) -> u64 {
        let guard = crate::OBJC_RUNTIME.read().ok();
        guard
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|rt| rt.get_class(name))
            .map(|c| c.mapped_addr)
            .unwrap_or(0)
    }

    fn objc_allocate_class_pair(&self, _superclass: u64, _name: &str) -> u64 {
        // Built-in runtime: class pairs are registered through Mach-O metadata,
        // not dynamically allocated. Return 0 to indicate no dynamic support.
        log::debug!("BuiltinBackend::objc_allocate_class_pair: dynamic class creation not supported");
        0
    }

    fn objc_register_class_pair(&self, _cls: u64) {
        log::debug!("BuiltinBackend::objc_register_class_pair: no-op in built-in backend");
    }

    fn sel_register_name(&self, name: &str) -> u64 {
        let guard = crate::OBJC_RUNTIME.read().ok();
        guard
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|rt| rt.selector_ptr(name))
            .map(|p| p as u64)
            .unwrap_or(0)
    }

    fn sel_get_name(&self, _sel: u64) -> Option<&str> {
        // Would require reverse lookup from address → name.
        // Not yet implemented in built-in runtime.
        None
    }

    fn class_add_method(&self, _cls: u64, _sel: u64, _imp: u64, _types: &str) -> bool {
        log::debug!("BuiltinBackend::class_add_method: not yet supported");
        false
    }

    fn class_replace_method(&self, _cls: u64, _sel: u64, _imp: u64, _types: &str) -> u64 {
        log::debug!("BuiltinBackend::class_replace_method: not yet supported");
        0
    }

    fn class_get_method_implementation(&self, _cls: u64, _sel: u64) -> u64 {
        // Method dispatch goes through the trampoline, not this path.
        0
    }

    fn class_create_instance(&self, _cls: u64, _extra_bytes: usize) -> u64 {
        // Object allocation is handled by shim_objc_alloc in shims/lifecycle.
        0
    }

    fn object_dispose(&self, _obj: u64) {
        // Deallocation is handled by shim_nsobject_dealloc.
    }

    fn msg_send_addr(&self) -> u64 {
        crate::shims::objc_msg_send_addr()
    }

    fn msg_send_stret_addr(&self) -> u64 {
        // Built-in runtime: stret uses the same trampoline on ARM64
        crate::shims::objc_msg_send_addr()
    }
}

// ── libobjc2 backend (feature-gated) ───────────────────────────

/// Placeholder for the GNUstep libobjc2 backend.
///
/// When implemented, this will:
/// 1. Link against `libobjc.so` from GNUstep
/// 2. Delegate all operations to the real C runtime
/// 3. Support dynamic class creation, real exception handling, and
///    full protocol conformance checking
///
/// Integration steps (documented for future implementation):
/// ```text
/// 1. apt-get install gnustep-libobjc2-dev  (or build from source)
/// 2. Enable feature: cargo build --features libobjc2
/// 3. build.rs probes for libobjc via pkg-config
/// 4. extern "C" blocks bind to objc/runtime.h functions
/// 5. ObjcRuntime metadata is registered into libobjc2 at load time
/// ```
#[cfg(feature = "libobjc2")]
pub struct Libobjc2Backend {
    _private: (), // prevent construction outside this module
}

#[cfg(feature = "libobjc2")]
impl Libobjc2Backend {
    /// Create a new libobjc2 backend.
    ///
    /// # Safety
    /// Requires libobjc2 to be loaded and initialized.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[cfg(feature = "libobjc2")]
impl RuntimeBackend for Libobjc2Backend {
    fn objc_get_class(&self, _name: &str) -> u64 {
        // TODO: extern "C" { fn objc_getClass(name: *const c_char) -> *mut objc_class; }
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn objc_allocate_class_pair(&self, _superclass: u64, _name: &str) -> u64 {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn objc_register_class_pair(&self, _cls: u64) {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn sel_register_name(&self, _name: &str) -> u64 {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn sel_get_name(&self, _sel: u64) -> Option<&str> {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn class_add_method(&self, _cls: u64, _sel: u64, _imp: u64, _types: &str) -> bool {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn class_replace_method(&self, _cls: u64, _sel: u64, _imp: u64, _types: &str) -> u64 {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn class_get_method_implementation(&self, _cls: u64, _sel: u64) -> u64 {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn class_create_instance(&self, _cls: u64, _extra_bytes: usize) -> u64 {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn object_dispose(&self, _obj: u64) {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn msg_send_addr(&self) -> u64 {
        unimplemented!("libobjc2 backend not yet implemented")
    }

    fn msg_send_stret_addr(&self) -> u64 {
        unimplemented!("libobjc2 backend not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_backend_msg_send_is_non_zero() {
        let backend = BuiltinBackend;
        // msg_send_addr returns the trampoline address
        let addr = backend.msg_send_addr();
        assert_ne!(addr, 0, "objc_msgSend trampoline should have non-zero address");
    }

    #[test]
    fn builtin_backend_unknown_class_returns_zero() {
        let backend = BuiltinBackend;
        assert_eq!(backend.objc_get_class("NonExistentClass12345"), 0);
    }

    #[test]
    fn builtin_backend_dynamic_alloc_returns_zero() {
        let backend = BuiltinBackend;
        assert_eq!(backend.objc_allocate_class_pair(0, "DynClass"), 0);
    }
}
