//! Minimal NSObject class and metaclass data symbols.
//!
//! When a binary inherits from NSObject, the linker emits bind opcodes
//! for `_OBJC_CLASS_$_NSObject` and `_OBJC_METACLASS_$_NSObject`. These
//! are normally exported by libobjc / Foundation on macOS.
//!
//! We provide minimal fake class structs that contain:
//! - An ISA pointer (metaclass points to itself for root)
//! - A superclass pointer (null for root)
//! - method cache (zeros)
//! - vtable (zeros)
//! - A class_ro_t pointer (null — our runtime uses register_image, not raw parsing)
//!
//! The key requirement: these addresses must be the same ones written into
//! guest class structs' superclass fields during fixup, so that class
//! hierarchy queries work.
//!
//! All class pairs are declared via the [`define_class_pair!`] macro to
//! eliminate boilerplate. To add a new Foundation class, add one macro
//! invocation — either `root` (for a new root class, unlikely) or `child`
//! (for any subclass).

use std::cell::UnsafeCell;

/// A wrapper around a byte array that can be safely shared across threads.
/// Interior mutation is synchronized via `std::sync::Once`.
#[repr(transparent)]
pub(crate) struct SyncArray(pub(crate) UnsafeCell<[u8; 40]>);

// SAFETY: Mutation is guarded by `std::sync::Once`; after init the data is read-only.
unsafe impl Sync for SyncArray {}

/// Declares a class/metaclass pair with `Once`-guarded initialization.
///
/// Two variants:
/// - **`root`**: for NSObject — metaclass ISA points to itself, no superclass.
/// - **`child`**: for subclasses — metaclass ISA → root metaclass, superclass → parent.
macro_rules! define_class_pair {
    // ── Root class (NSObject) ──────────────────────────────────────
    (root $class_static:ident, $meta_static:ident, $once:ident,
     ensure: $ensure_fn:ident,
     class_addr: $class_fn:ident,
     meta_addr: $meta_fn:ident) => {
        static $class_static: SyncArray = SyncArray(UnsafeCell::new([0u8; 40]));
        static $meta_static: SyncArray = SyncArray(UnsafeCell::new([0u8; 40]));
        static $once: std::sync::Once = std::sync::Once::new();

        fn $ensure_fn() {
            $once.call_once(|| {
                unsafe {
                    let class_ptr = $class_static.0.get() as *mut u8;
                    let meta_ptr = $meta_static.0.get() as *mut u8;
                    let meta_bytes = (meta_ptr as u64).to_le_bytes();
                    // Class ISA → metaclass
                    std::ptr::copy_nonoverlapping(meta_bytes.as_ptr(), class_ptr, 8);
                    // Metaclass ISA → self (root metaclass)
                    std::ptr::copy_nonoverlapping(meta_bytes.as_ptr(), meta_ptr, 8);
                }
            });
        }

        pub fn $class_fn() -> u64 {
            $ensure_fn();
            $class_static.0.get() as u64
        }

        pub fn $meta_fn() -> u64 {
            $ensure_fn();
            $meta_static.0.get() as u64
        }
    };

    // ── Child class (inherits from a parent) ───────────────────────
    (child $class_static:ident, $meta_static:ident, $once:ident,
     ensure: $ensure_fn:ident,
     class_addr: $class_fn:ident,
     meta_addr: $meta_fn:ident,
     parent_class: $parent_class_fn:ident,
     parent_meta: $parent_meta_fn:ident) => {
        static $class_static: SyncArray = SyncArray(UnsafeCell::new([0u8; 40]));
        static $meta_static: SyncArray = SyncArray(UnsafeCell::new([0u8; 40]));
        static $once: std::sync::Once = std::sync::Once::new();

        fn $ensure_fn() {
            let _ = $parent_class_fn();
            $once.call_once(|| {
                unsafe {
                    let class_ptr = $class_static.0.get() as *mut u8;
                    let meta_ptr = $meta_static.0.get() as *mut u8;
                    // Class ISA → own metaclass
                    std::ptr::copy_nonoverlapping(
                        (meta_ptr as u64).to_le_bytes().as_ptr(),
                        class_ptr,
                        8,
                    );
                    // Metaclass ISA → root metaclass (NSObject)
                    std::ptr::copy_nonoverlapping(
                        nsobject_metaclass_addr().to_le_bytes().as_ptr(),
                        meta_ptr,
                        8,
                    );
                    // Class superclass (offset 8) → parent class
                    std::ptr::copy_nonoverlapping(
                        $parent_class_fn().to_le_bytes().as_ptr(),
                        class_ptr.add(8),
                        8,
                    );
                    // Metaclass superclass (offset 8) → parent metaclass
                    std::ptr::copy_nonoverlapping(
                        $parent_meta_fn().to_le_bytes().as_ptr(),
                        meta_ptr.add(8),
                        8,
                    );
                }
            });
        }

        pub fn $class_fn() -> u64 {
            $ensure_fn();
            $class_static.0.get() as u64
        }

        pub fn $meta_fn() -> u64 {
            $ensure_fn();
            $meta_static.0.get() as u64
        }
    };
}

// NSObject — root class pair
define_class_pair!(root NSOBJECT_CLASS_DATA, NSOBJECT_METACLASS_DATA, NSOBJECT_INIT,
    ensure: ensure_nsobject_init,
    class_addr: nsobject_class_addr,
    meta_addr: nsobject_metaclass_addr);

// The macro must be visible to submodules (macro defined before mod declarations).
mod class_pairs_foundation;
mod class_pairs_appkit;
mod class_pairs_gap;
mod class_pairs_xcode;
mod methods;

pub use class_pairs_foundation::*;
pub use class_pairs_appkit::*;
pub use class_pairs_gap::*;
pub use class_pairs_xcode::*;
pub use methods::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nsobject_class_addr_nonzero() {
        let addr = nsobject_class_addr();
        assert_ne!(addr, 0);
    }

    #[test]
    fn nsobject_metaclass_addr_nonzero() {
        let addr = nsobject_metaclass_addr();
        assert_ne!(addr, 0);
    }

    #[test]
    fn nsobject_class_isa_points_to_metaclass() {
        let class_addr = nsobject_class_addr();
        let meta_addr = nsobject_metaclass_addr();
        let isa = unsafe { (class_addr as *const u64).read_unaligned() };
        assert_eq!(isa, meta_addr);
    }

    #[test]
    fn nsobject_metaclass_isa_points_to_self() {
        let meta_addr = nsobject_metaclass_addr();
        let isa = unsafe { (meta_addr as *const u64).read_unaligned() };
        assert_eq!(isa, meta_addr);
    }

    #[test]
    fn class_and_metaclass_are_different() {
        assert_ne!(nsobject_class_addr(), nsobject_metaclass_addr());
    }
}
