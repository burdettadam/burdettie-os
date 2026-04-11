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

/// GNUstep libobjc2 backend — delegates ObjC runtime operations to the
/// real `libobjc.so` shared library on Linux.
///
/// When enabled, this:
/// 1. Links against `libobjc.so` from GNUstep (via `-lobjc`)
/// 2. Delegates class/selector/method operations to the C runtime
/// 3. Returns libobjc2's `objc_msgSend` address for message dispatch
///
/// ## Build requirements
/// ```text
/// apt-get install gnustep-libobjc2-dev   # Debian/Ubuntu
/// cargo build --features libobjc2        # Enable the backend
/// ```
#[cfg(feature = "libobjc2")]
pub struct Libobjc2Backend {
    _private: (), // prevent construction outside this module
}

#[cfg(feature = "libobjc2")]
mod ffi {
    //! Raw FFI bindings to libobjc2's public C API (objc/runtime.h).
    use std::os::raw::{c_char, c_uint};

    // Opaque types — libobjc2 handles the internal layout.
    pub type Class = *mut u8;
    pub type Sel = *const u8;
    pub type Id = *mut u8;
    pub type Imp = unsafe extern "C" fn();
    pub type Bool = c_uint; // GNUstep BOOL is unsigned int on non-Apple

    extern "C" {
        // ── Class operations ────────────────────────────────────
        pub fn objc_getClass(name: *const c_char) -> Class;
        pub fn objc_allocateClassPair(
            superclass: Class,
            name: *const c_char,
            extra_bytes: usize,
        ) -> Class;
        pub fn objc_registerClassPair(cls: Class);

        // ── Selector operations ─────────────────────────────────
        pub fn sel_registerName(name: *const c_char) -> Sel;
        pub fn sel_getName(sel: Sel) -> *const c_char;

        // ── Method operations ───────────────────────────────────
        pub fn class_addMethod(
            cls: Class,
            name: Sel,
            imp: Imp,
            types: *const c_char,
        ) -> Bool;
        pub fn class_replaceMethod(
            cls: Class,
            name: Sel,
            imp: Imp,
            types: *const c_char,
        ) -> Imp;
        pub fn class_getMethodImplementation(cls: Class, name: Sel) -> Imp;

        // ── Object lifecycle ────────────────────────────────────
        pub fn class_createInstance(cls: Class, extra_bytes: usize) -> Id;
        pub fn object_dispose(obj: Id) -> Id;

        // ── Message dispatch ────────────────────────────────────
        pub fn objc_msgSend();
        #[cfg(not(target_arch = "aarch64"))]
        pub fn objc_msgSend_stret();
    }
}

#[cfg(feature = "libobjc2")]
impl Libobjc2Backend {
    /// Create a new libobjc2 backend.
    ///
    /// # Safety
    /// Requires libobjc2 to be loaded and initialized (linked via `-lobjc`).
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[cfg(feature = "libobjc2")]
impl RuntimeBackend for Libobjc2Backend {
    fn objc_get_class(&self, name: &str) -> u64 {
        let c_name = std::ffi::CString::new(name).unwrap_or_default();
        let cls = unsafe { ffi::objc_getClass(c_name.as_ptr()) };
        cls as u64
    }

    fn objc_allocate_class_pair(&self, superclass: u64, name: &str) -> u64 {
        let c_name = std::ffi::CString::new(name).unwrap_or_default();
        let cls = unsafe {
            ffi::objc_allocateClassPair(superclass as ffi::Class, c_name.as_ptr(), 0)
        };
        cls as u64
    }

    fn objc_register_class_pair(&self, cls: u64) {
        unsafe { ffi::objc_registerClassPair(cls as ffi::Class) };
    }

    fn sel_register_name(&self, name: &str) -> u64 {
        let c_name = std::ffi::CString::new(name).unwrap_or_default();
        let sel = unsafe { ffi::sel_registerName(c_name.as_ptr()) };
        sel as u64
    }

    fn sel_get_name(&self, sel: u64) -> Option<&str> {
        if sel == 0 {
            return None;
        }
        let ptr = unsafe { ffi::sel_getName(sel as ffi::Sel) };
        if ptr.is_null() {
            return None;
        }
        // Safety: sel_getName returns a pointer to runtime-owned static storage.
        // The lifetime is tied to &self which is shorter than the runtime's lifetime.
        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
        c_str.to_str().ok()
    }

    fn class_add_method(&self, cls: u64, sel: u64, imp: u64, types: &str) -> bool {
        let c_types = std::ffi::CString::new(types).unwrap_or_default();
        let result = unsafe {
            ffi::class_addMethod(
                cls as ffi::Class,
                sel as ffi::Sel,
                std::mem::transmute::<u64, ffi::Imp>(imp),
                c_types.as_ptr(),
            )
        };
        result != 0
    }

    fn class_replace_method(&self, cls: u64, sel: u64, imp: u64, types: &str) -> u64 {
        let c_types = std::ffi::CString::new(types).unwrap_or_default();
        let prev = unsafe {
            ffi::class_replaceMethod(
                cls as ffi::Class,
                sel as ffi::Sel,
                std::mem::transmute::<u64, ffi::Imp>(imp),
                c_types.as_ptr(),
            )
        };
        prev as u64
    }

    fn class_get_method_implementation(&self, cls: u64, sel: u64) -> u64 {
        let imp = unsafe {
            ffi::class_getMethodImplementation(cls as ffi::Class, sel as ffi::Sel)
        };
        imp as u64
    }

    fn class_create_instance(&self, cls: u64, extra_bytes: usize) -> u64 {
        let obj = unsafe { ffi::class_createInstance(cls as ffi::Class, extra_bytes) };
        obj as u64
    }

    fn object_dispose(&self, obj: u64) {
        if obj != 0 {
            unsafe { ffi::object_dispose(obj as ffi::Id) };
        }
    }

    fn msg_send_addr(&self) -> u64 {
        ffi::objc_msgSend as *const () as u64
    }

    fn msg_send_stret_addr(&self) -> u64 {
        // On ARM64, objc_msgSend handles both normal and struct returns.
        #[cfg(target_arch = "aarch64")]
        {
            ffi::objc_msgSend as *const () as u64
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            ffi::objc_msgSend_stret as *const () as u64
        }
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

    // ── libobjc2 backend tests ──────────────────────────────────

    #[cfg(feature = "libobjc2")]
    mod libobjc2_tests {
        use super::*;

        fn backend() -> Libobjc2Backend {
            Libobjc2Backend::new()
        }

        #[test]
        fn msg_send_addr_is_non_zero() {
            let b = backend();
            assert_ne!(b.msg_send_addr(), 0, "libobjc2 objc_msgSend should be non-zero");
        }

        #[test]
        fn msg_send_stret_addr_is_non_zero() {
            let b = backend();
            assert_ne!(b.msg_send_stret_addr(), 0, "libobjc2 objc_msgSend_stret should be non-zero");
        }

        #[test]
        fn sel_register_and_get_name() {
            let b = backend();
            let sel = b.sel_register_name("init");
            assert_ne!(sel, 0);
            let name = b.sel_get_name(sel);
            assert_eq!(name, Some("init"));
        }

        #[test]
        fn sel_get_name_nil_returns_none() {
            let b = backend();
            assert_eq!(b.sel_get_name(0), None);
        }

        #[test]
        fn same_selector_returns_same_address() {
            let b = backend();
            let s1 = b.sel_register_name("viewDidLoad");
            let s2 = b.sel_register_name("viewDidLoad");
            assert_eq!(s1, s2);
        }

        #[test]
        fn get_class_nsobject() {
            let b = backend();
            // NSObject should exist in libobjc2 — it's the root class.
            // Note: libobjc2 might not have NSObject unless Foundation is loaded.
            // In that case this is expected to return 0.
            let _cls = b.objc_get_class("NSObject");
            // We don't assert non-zero because NSObject requires Foundation.
        }

        #[test]
        fn get_class_nonexistent_returns_zero() {
            let b = backend();
            assert_eq!(b.objc_get_class("BogusClassThatDoesNotExist99"), 0);
        }

        #[test]
        fn allocate_register_class_pair() {
            let b = backend();
            // Allocate with nil superclass — libobjc2 may or may not support this.
            // With a real superclass (NSObject) it would work if Foundation is loaded.
            let cls = b.objc_allocate_class_pair(0, "TestDynClass_libobjc2");
            if cls != 0 {
                // If allocation succeeded, register it.
                b.objc_register_class_pair(cls);
                // Should now be findable by name.
                let found = b.objc_get_class("TestDynClass_libobjc2");
                assert_eq!(found, cls);
            }
            // If cls == 0, nil superclass isn't supported without Foundation — fine.
        }

        #[test]
        fn object_dispose_nil_is_safe() {
            let b = backend();
            // Disposing nil should be a no-op, not crash.
            b.object_dispose(0);
        }
    }
}
