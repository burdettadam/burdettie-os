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
struct SyncArray(UnsafeCell<[u8; 40]>);

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
                // SAFETY: Mutation is guarded by Once; the data is write-once.
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
            // Ensure parent is initialized first (calls its own ensure).
            let _ = $parent_class_fn();
            $once.call_once(|| {
                // SAFETY: Mutation is guarded by Once; parent addrs are valid
                // because the parent's ensure ran above.
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

// ── Class pair declarations ────────────────────────────────────────

define_class_pair!(root NSOBJECT_CLASS_DATA, NSOBJECT_METACLASS_DATA, NSOBJECT_INIT,
    ensure: ensure_nsobject_init,
    class_addr: nsobject_class_addr,
    meta_addr: nsobject_metaclass_addr);

define_class_pair!(child NSSTRING_CLASS_DATA, NSSTRING_METACLASS_DATA, NSSTRING_INIT,
    ensure: ensure_nsstring_init,
    class_addr: nsstring_class_addr,
    meta_addr: nsstring_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSARRAY_CLASS_DATA, NSARRAY_METACLASS_DATA, NSARRAY_INIT,
    ensure: ensure_nsarray_init,
    class_addr: nsarray_class_addr,
    meta_addr: nsarray_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNUMBER_CLASS_DATA, NSNUMBER_METACLASS_DATA, NSNUMBER_INIT,
    ensure: ensure_nsnumber_init,
    class_addr: nsnumber_class_addr,
    meta_addr: nsnumber_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSDICTIONARY_CLASS_DATA, NSDICTIONARY_METACLASS_DATA, NSDICTIONARY_INIT,
    ensure: ensure_nsdictionary_init,
    class_addr: nsdictionary_class_addr,
    meta_addr: nsdictionary_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLEARRAY_CLASS_DATA, NSMUTABLEARRAY_METACLASS_DATA, NSMUTABLEARRAY_INIT,
    ensure: ensure_nsmutablearray_init,
    class_addr: nsmutablearray_class_addr,
    meta_addr: nsmutablearray_metaclass_addr,
    parent_class: nsarray_class_addr,
    parent_meta: nsarray_metaclass_addr);

define_class_pair!(child NSMUTABLEDICTIONARY_CLASS_DATA, NSMUTABLEDICTIONARY_METACLASS_DATA, NSMUTABLEDICTIONARY_INIT,
    ensure: ensure_nsmutabledictionary_init,
    class_addr: nsmutabledictionary_class_addr,
    meta_addr: nsmutabledictionary_metaclass_addr,
    parent_class: nsdictionary_class_addr,
    parent_meta: nsdictionary_metaclass_addr);

define_class_pair!(child NSMUTABLESTRING_CLASS_DATA, NSMUTABLESTRING_METACLASS_DATA, NSMUTABLESTRING_INIT,
    ensure: ensure_nsmutablestring_init,
    class_addr: nsmutablestring_class_addr,
    meta_addr: nsmutablestring_metaclass_addr,
    parent_class: nsstring_class_addr,
    parent_meta: nsstring_metaclass_addr);

define_class_pair!(child NSDATA_CLASS_DATA, NSDATA_METACLASS_DATA, NSDATA_INIT,
    ensure: ensure_nsdata_init,
    class_addr: nsdata_class_addr,
    meta_addr: nsdata_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPROCESSINFO_CLASS_DATA, NSPROCESSINFO_METACLASS_DATA, NSPROCESSINFO_INIT,
    ensure: ensure_nsprocessinfo_init,
    class_addr: nsprocessinfo_class_addr,
    meta_addr: nsprocessinfo_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSERROR_CLASS_DATA, NSERROR_METACLASS_DATA, NSERROR_INIT,
    ensure: ensure_nserror_init,
    class_addr: nserror_class_addr,
    meta_addr: nserror_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSBUNDLE_CLASS_DATA, NSBUNDLE_METACLASS_DATA, NSBUNDLE_INIT,
    ensure: ensure_nsbundle_init,
    class_addr: nsbundle_class_addr,
    meta_addr: nsbundle_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSFILEMANAGER_CLASS_DATA, NSFILEMANAGER_METACLASS_DATA, NSFILEMANAGER_INIT,
    ensure: ensure_nsfilemanager_init,
    class_addr: nsfilemanager_class_addr,
    meta_addr: nsfilemanager_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSEXCEPTION_CLASS_DATA, NSEXCEPTION_METACLASS_DATA, NSEXCEPTION_INIT,
    ensure: ensure_nsexception_init,
    class_addr: nsexception_class_addr,
    meta_addr: nsexception_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSENUMERATOR_CLASS_DATA, NSENUMERATOR_METACLASS_DATA, NSENUMERATOR_INIT,
    ensure: ensure_nsenumerator_init,
    class_addr: nsenumerator_class_addr,
    meta_addr: nsenumerator_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSDATE_CLASS_DATA, NSDATE_METACLASS_DATA, NSDATE_INIT,
    ensure: ensure_nsdate_init,
    class_addr: nsdate_class_addr,
    meta_addr: nsdate_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSUSERDEFAULTS_CLASS_DATA, NSUSERDEFAULTS_METACLASS_DATA, NSUSERDEFAULTS_INIT,
    ensure: ensure_nsuserdefaults_init,
    class_addr: nsuserdefaults_class_addr,
    meta_addr: nsuserdefaults_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSSET_CLASS_DATA, NSSET_METACLASS_DATA, NSSET_INIT,
    ensure: ensure_nsset_init,
    class_addr: nsset_class_addr,
    meta_addr: nsset_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLESET_CLASS_DATA, NSMUTABLESET_METACLASS_DATA, NSMUTABLESET_INIT,
    ensure: ensure_nsmutableset_init,
    class_addr: nsmutableset_class_addr,
    meta_addr: nsmutableset_metaclass_addr,
    parent_class: nsset_class_addr,
    parent_meta: nsset_metaclass_addr);

define_class_pair!(child NSURL_CLASS_DATA, NSURL_METACLASS_DATA, NSURL_INIT,
    ensure: ensure_nsurl_init,
    class_addr: nsurl_class_addr,
    meta_addr: nsurl_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNULL_CLASS_DATA, NSNULL_METACLASS_DATA, NSNULL_INIT,
    ensure: ensure_nsnull_init,
    class_addr: nsnull_class_addr,
    meta_addr: nsnull_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSDATEFORMATTER_CLASS_DATA, NSDATEFORMATTER_METACLASS_DATA, NSDATEFORMATTER_INIT,
    ensure: ensure_nsdateformatter_init,
    class_addr: nsdateformatter_class_addr,
    meta_addr: nsdateformatter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCHARACTERSET_CLASS_DATA, NSCHARACTERSET_METACLASS_DATA, NSCHARACTERSET_INIT,
    ensure: ensure_nscharacterset_init,
    class_addr: nscharacterset_class_addr,
    meta_addr: nscharacterset_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNOTIFICATIONCENTER_CLASS_DATA, NSNOTIFICATIONCENTER_METACLASS_DATA, NSNOTIFICATIONCENTER_INIT,
    ensure: ensure_nsnotificationcenter_init,
    class_addr: nsnotificationcenter_class_addr,
    meta_addr: nsnotificationcenter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSVALUE_CLASS_DATA, NSVALUE_METACLASS_DATA, NSVALUE_INIT,
    ensure: ensure_nsvalue_init,
    class_addr: nsvalue_class_addr,
    meta_addr: nsvalue_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSUUID_CLASS_DATA, NSUUID_METACLASS_DATA, NSUUID_INIT,
    ensure: ensure_nsuuid_init,
    class_addr: nsuuid_class_addr,
    meta_addr: nsuuid_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSLOCALE_CLASS_DATA, NSLOCALE_METACLASS_DATA, NSLOCALE_INIT,
    ensure: ensure_nslocale_init,
    class_addr: nslocale_class_addr,
    meta_addr: nslocale_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSTIMEZONE_CLASS_DATA, NSTIMEZONE_METACLASS_DATA, NSTIMEZONE_INIT,
    ensure: ensure_nstimezone_init,
    class_addr: nstimezone_class_addr,
    meta_addr: nstimezone_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCALENDAR_CLASS_DATA, NSCALENDAR_METACLASS_DATA, NSCALENDAR_INIT,
    ensure: ensure_nscalendar_init,
    class_addr: nscalendar_class_addr,
    meta_addr: nscalendar_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSNUMBERFORMATTER_CLASS_DATA, NSNUMBERFORMATTER_METACLASS_DATA, NSNUMBERFORMATTER_INIT,
    ensure: ensure_nsnumberformatter_init,
    class_addr: nsnumberformatter_class_addr,
    meta_addr: nsnumberformatter_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSINDEXSET_CLASS_DATA, NSINDEXSET_METACLASS_DATA, NSINDEXSET_INIT,
    ensure: ensure_nsindexset_init,
    class_addr: nsindexset_class_addr,
    meta_addr: nsindexset_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLEINDEXSET_CLASS_DATA, NSMUTABLEINDEXSET_METACLASS_DATA, NSMUTABLEINDEXSET_INIT,
    ensure: ensure_nsmutableindexset_init,
    class_addr: nsmutableindexset_class_addr,
    meta_addr: nsmutableindexset_metaclass_addr,
    parent_class: nsindexset_class_addr,
    parent_meta: nsindexset_metaclass_addr);

define_class_pair!(child NSSORTDESCRIPTOR_CLASS_DATA, NSSORTDESCRIPTOR_METACLASS_DATA, NSSORTDESCRIPTOR_INIT,
    ensure: ensure_nssortdescriptor_init,
    class_addr: nssortdescriptor_class_addr,
    meta_addr: nssortdescriptor_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSTIMER_CLASS_DATA, NSTIMER_METACLASS_DATA, NSTIMER_INIT,
    ensure: ensure_nstimer_init,
    class_addr: nstimer_class_addr,
    meta_addr: nstimer_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSTHREAD_CLASS_DATA, NSTHREAD_METACLASS_DATA, NSTHREAD_INIT,
    ensure: ensure_nsthread_init,
    class_addr: nsthread_class_addr,
    meta_addr: nsthread_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSLOCK_CLASS_DATA, NSLOCK_METACLASS_DATA, NSLOCK_INIT,
    ensure: ensure_nslock_init,
    class_addr: nslock_class_addr,
    meta_addr: nslock_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSRECURSIVELOCK_CLASS_DATA, NSRECURSIVELOCK_METACLASS_DATA, NSRECURSIVELOCK_INIT,
    ensure: ensure_nsrecursivelock_init,
    class_addr: nsrecursivelock_class_addr,
    meta_addr: nsrecursivelock_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCONDITION_CLASS_DATA, NSCONDITION_METACLASS_DATA, NSCONDITION_INIT,
    ensure: ensure_nscondition_init,
    class_addr: nscondition_class_addr,
    meta_addr: nscondition_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSCONDITIONLOCK_CLASS_DATA, NSCONDITIONLOCK_METACLASS_DATA, NSCONDITIONLOCK_INIT,
    ensure: ensure_nsconditionlock_init,
    class_addr: nsconditionlock_class_addr,
    meta_addr: nsconditionlock_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMETHODSIGNATURE_CLASS_DATA, NSMETHODSIGNATURE_METACLASS_DATA, NSMETHODSIGNATURE_INIT,
    ensure: ensure_nsmethodsignature_init,
    class_addr: nsmethodsignature_class_addr,
    meta_addr: nsmethodsignature_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSINVOCATION_CLASS_DATA, NSINVOCATION_METACLASS_DATA, NSINVOCATION_INIT,
    ensure: ensure_nsinvocation_init,
    class_addr: nsinvocation_class_addr,
    meta_addr: nsinvocation_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── Phase B6 Foundation classes ────────────────────────────────────

define_class_pair!(child NSREGULAREXPRESSION_CLASS_DATA, NSREGULAREXPRESSION_METACLASS_DATA, NSREGULAREXPRESSION_INIT,
    ensure: ensure_nsregularexpression_init,
    class_addr: nsregularexpression_class_addr,
    meta_addr: nsregularexpression_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPREDICATE_CLASS_DATA, NSPREDICATE_METACLASS_DATA, NSPREDICATE_INIT,
    ensure: ensure_nspredicate_init,
    class_addr: nspredicate_class_addr,
    meta_addr: nspredicate_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSATTRIBUTEDSTRING_CLASS_DATA, NSATTRIBUTEDSTRING_METACLASS_DATA, NSATTRIBUTEDSTRING_INIT,
    ensure: ensure_nsattributedstring_init,
    class_addr: nsattributedstring_class_addr,
    meta_addr: nsattributedstring_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMUTABLEATTRIBUTEDSTRING_CLASS_DATA, NSMUTABLEATTRIBUTEDSTRING_METACLASS_DATA, NSMUTABLEATTRIBUTEDSTRING_INIT,
    ensure: ensure_nsmutableattributedstring_init,
    class_addr: nsmutableattributedstring_class_addr,
    meta_addr: nsmutableattributedstring_metaclass_addr,
    parent_class: nsattributedstring_class_addr,
    parent_meta: nsattributedstring_metaclass_addr);

define_class_pair!(child NSCACHE_CLASS_DATA, NSCACHE_METACLASS_DATA, NSCACHE_INIT,
    ensure: ensure_nscache_init,
    class_addr: nscache_class_addr,
    meta_addr: nscache_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSMAPTABLE_CLASS_DATA, NSMAPTABLE_METACLASS_DATA, NSMAPTABLE_INIT,
    ensure: ensure_nsmaptable_init,
    class_addr: nsmaptable_class_addr,
    meta_addr: nsmaptable_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSHASHTABLE_CLASS_DATA, NSHASHTABLE_METACLASS_DATA, NSHASHTABLE_INIT,
    ensure: ensure_nshashtable_init,
    class_addr: nshashtable_class_addr,
    meta_addr: nshashtable_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSOPERATIONQUEUE_CLASS_DATA, NSOPERATIONQUEUE_METACLASS_DATA, NSOPERATIONQUEUE_INIT,
    ensure: ensure_nsoperationqueue_init,
    class_addr: nsoperationqueue_class_addr,
    meta_addr: nsoperationqueue_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSOPERATION_CLASS_DATA, NSOPERATION_METACLASS_DATA, NSOPERATION_INIT,
    ensure: ensure_nsoperation_init,
    class_addr: nsoperation_class_addr,
    meta_addr: nsoperation_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSBLOCKOPERATION_CLASS_DATA, NSBLOCKOPERATION_METACLASS_DATA, NSBLOCKOPERATION_INIT,
    ensure: ensure_nsblockoperation_init,
    class_addr: nsblockoperation_class_addr,
    meta_addr: nsblockoperation_metaclass_addr,
    parent_class: nsoperation_class_addr,
    parent_meta: nsoperation_metaclass_addr);

define_class_pair!(child NSJSONSERIALIZATION_CLASS_DATA, NSJSONSERIALIZATION_METACLASS_DATA, NSJSONSERIALIZATION_INIT,
    ensure: ensure_nsjsonserialization_init,
    class_addr: nsjsonserialization_class_addr,
    meta_addr: nsjsonserialization_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child NSPROPERTYLISTSERIALIZATION_CLASS_DATA, NSPROPERTYLISTSERIALIZATION_METACLASS_DATA, NSPROPERTYLISTSERIALIZATION_INIT,
    ensure: ensure_nspropertylistserialization_init,
    class_addr: nspropertylistserialization_class_addr,
    meta_addr: nspropertylistserialization_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// ── NSObject instance method shims ─────────────────────────────────
//
// These are `extern "C"` functions matching the ObjC calling convention:
// first arg is self (object pointer), second is _cmd (SEL).

/// `-[NSObject init]` — returns self unchanged.
pub extern "C" fn shim_nsobject_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSObject self]` — returns self.
pub extern "C" fn shim_nsobject_self(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSObject class]` — returns the ISA pointer (the class).
pub extern "C" fn shim_nsobject_class(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *(obj as *const u64) }
}

/// `-[NSObject respondsToSelector:]` — checks if the object's class
/// (or superclass chain) has the given selector.
pub extern "C" fn shim_nsobject_responds_to_selector(obj: u64, _cmd: u64, sel: u64) -> u8 {
    if obj == 0 || sel == 0 {
        return 0;
    }

    let isa = unsafe { *(obj as *const u64) };
    let sel_cstr = unsafe { std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char) };
    let sel_str = sel_cstr.to_str().unwrap_or("");

    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    if rt.msg_lookup(isa, sel_str).is_some() {
        1
    } else {
        0
    }
}

/// `-[NSObject isKindOfClass:]` — walks the isa/superclass chain.
pub extern "C" fn shim_nsobject_is_kind_of_class(obj: u64, _cmd: u64, cls: u64) -> u8 {
    if obj == 0 || cls == 0 {
        return 0;
    }

    // Walk the class hierarchy from isa upward
    let isa = unsafe { *(obj as *const u64) };

    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    let mut current = Some(isa);
    let mut depth = 0;
    while let Some(addr) = current {
        if depth > 64 {
            break;
        }
        if addr == cls {
            return 1;
        }
        if let Some(class_info) = rt.get_class_by_addr(addr) {
            current = class_info.superclass_addr;
        } else {
            break;
        }
        depth += 1;
    }
    0
}

// ── NSObject class method shims ────────────────────────────────────

/// `+[NSObject alloc]` — allocates a new instance.
pub extern "C" fn shim_nsobject_class_alloc(cls: u64, _cmd: u64) -> u64 {
    if cls == 0 {
        return 0;
    }

    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    let Some(class_info) = rt.get_class_by_addr(cls) else {
        return 0;
    };

    let size = std::cmp::max(class_info.instance_size as usize, 8);
    let ptr = unsafe { libc::calloc(1, size) } as u64;
    if ptr == 0 {
        return 0;
    }

    // Set isa = class pointer
    unsafe {
        *(ptr as *mut u64) = cls;
    }
    ptr
}

/// `+[NSObject new]` — alloc + init.
pub extern "C" fn shim_nsobject_class_new(cls: u64, cmd: u64) -> u64 {
    shim_nsobject_class_alloc(cls, cmd)
}

// ── NSObject forwarding methods ────────────────────────────────────

/// `-[NSObject methodSignatureForSelector:]`
///
/// Looks up the type encoding for the given selector in the runtime
/// and returns an NSMethodSignature, or 0 (nil) if not found.
pub extern "C" fn shim_nsobject_method_signature_for_selector(
    obj: u64,
    _cmd: u64,
    sel: u64,
) -> u64 {
    if obj == 0 || sel == 0 {
        return 0;
    }

    let isa = unsafe { *(obj as *const u64) };
    let sel_str = unsafe {
        std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };

    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    if let Some(entry) = rt.lookup_method_entry(isa, sel_str) {
        let encoding = entry.type_encoding.clone();
        drop(guard);
        crate::nsmethodsignature::create_method_signature(&encoding)
    } else {
        0
    }
}

/// `+[NSObject methodSignatureForSelector:]` (class method variant)
pub extern "C" fn shim_nsobject_class_method_signature_for_selector(
    cls: u64,
    _cmd: u64,
    sel: u64,
) -> u64 {
    // For a class object, look up in its metaclass's class_methods
    // Reuse the instance method version — ISA of the class is the metaclass
    shim_nsobject_method_signature_for_selector(cls, _cmd, sel)
}

/// `-[NSObject forwardInvocation:]`
///
/// Default implementation: logs and does nothing.
/// Subclasses override this to implement forwarding.
pub extern "C" fn shim_nsobject_forward_invocation(obj: u64, _cmd: u64, invocation: u64) {
    if obj == 0 || invocation == 0 {
        return;
    }
    // Get the selector from the invocation for the error message
    let sel_ptr = unsafe { *((invocation + 24) as *const u64) };
    let sel_str = if sel_ptr != 0 {
        unsafe {
            std::ffi::CStr::from_ptr(sel_ptr as *const std::ffi::c_char)
                .to_str()
                .unwrap_or("<unknown>")
        }
    } else {
        "<unknown>"
    };
    eprintln!(
        "-[NSObject forwardInvocation:]: unrecognized selector '{}' sent to instance {:#x}",
        sel_str, obj
    );
}

/// `-[NSObject doesNotRecognizeSelector:]`
pub extern "C" fn shim_nsobject_does_not_recognize_selector(obj: u64, _cmd: u64, sel: u64) {
    let sel_str = if sel != 0 {
        unsafe {
            std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
                .to_str()
                .unwrap_or("<unknown>")
        }
    } else {
        "<unknown>"
    };
    eprintln!(
        "-[NSObject doesNotRecognizeSelector:]: '{}' sent to instance {:#x}",
        sel_str, obj
    );
}

/// `+[NSObject resolveInstanceMethod:]` — default returns NO.
pub extern "C" fn shim_nsobject_resolve_instance_method(
    _cls: u64,
    _cmd: u64,
    _sel: u64,
) -> u8 {
    0 // NO
}

/// `+[NSObject resolveClassMethod:]` — default returns NO.
pub extern "C" fn shim_nsobject_resolve_class_method(
    _cls: u64,
    _cmd: u64,
    _sel: u64,
) -> u8 {
    0 // NO
}

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
        // Read ISA from class
        let isa = unsafe { *(class_addr as *const u64) };
        assert_eq!(isa, meta_addr);
    }

    #[test]
    fn nsobject_metaclass_isa_points_to_self() {
        let meta_addr = nsobject_metaclass_addr();
        // Root metaclass ISA points to itself
        let isa = unsafe { *(meta_addr as *const u64) };
        assert_eq!(isa, meta_addr);
    }

    #[test]
    fn class_and_metaclass_are_different() {
        assert_ne!(nsobject_class_addr(), nsobject_metaclass_addr());
    }
}
