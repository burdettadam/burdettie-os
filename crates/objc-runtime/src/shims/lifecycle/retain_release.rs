//! ARC memory management: retain, release, autorelease, store-strong.

/// `void *objc_autoreleasePoolPush(void)`
pub extern "C" fn shim_objc_autorelease_pool_push() -> u64 {
    crate::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow_mut().push() as u64)
}

/// `void objc_autoreleasePoolPop(void *token)`
pub extern "C" fn shim_objc_autorelease_pool_pop(token: u64) {
    let drained =
        crate::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow_mut().pop(token as usize));
    for obj in drained {
        if obj != 0 {
            let remaining = crate::refcount::REFCOUNTS.release(obj);
            if remaining == 0 {
                crate::refcount::deallocate_tracked_object(obj, "autorelease pool");
            }
        }
    }
}

/// `id objc_retain(id obj)`
pub extern "C" fn shim_objc_retain(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::refcount::REFCOUNTS.retain(obj);
    obj
}

/// `void objc_release(id obj)`
pub extern "C" fn shim_objc_release(obj: u64) {
    if obj == 0 {
        return;
    }
    let remaining = crate::refcount::REFCOUNTS.release(obj);
    if remaining == 0 {
        crate::refcount::deallocate_tracked_object(obj, "objc_release");
    }
}

/// `void objc_setProperty_nonatomic(id self, SEL _cmd, id newValue, ptrdiff_t offset)`
pub extern "C" fn shim_objc_set_property_nonatomic(
    obj: u64,
    _sel: u64,
    new_value: u64,
    offset: isize,
) {
    if obj == 0 {
        return;
    }
    unsafe {
        let slot = (obj as *mut u8).offset(offset) as *mut u64;
        *slot = new_value;
    }
}

/// `void objc_setProperty_nonatomic_copy(id self, SEL _cmd, id newValue, ptrdiff_t offset)`
///
/// In a full runtime this would call `[newValue copy]`. We store directly
/// since our guest runtime doesn't yet support sending `copy` through the
/// host ObjC runtime (the host can't resolve synthetic ISA pointers).
pub extern "C" fn shim_objc_set_property_nonatomic_copy(
    obj: u64,
    _sel: u64,
    new_value: u64,
    offset: isize,
) {
    if obj == 0 {
        return;
    }
    unsafe {
        let slot = (obj as *mut u8).offset(offset) as *mut u64;
        *slot = new_value;
    }
}

/// `id objc_getProperty(id self, SEL _cmd, ptrdiff_t offset, BOOL atomic)`
///
/// Returns the object stored at `offset` bytes into `self`.
/// Atomic: retain+autorelease to ensure the caller gets a stable reference.
/// Non-atomic: direct load.
pub extern "C" fn shim_objc_get_property(obj: u64, _sel: u64, offset: isize, atomic: u8) -> u64 {
    if obj == 0 {
        return 0;
    }
    let val = unsafe { *((obj as *const u8).offset(offset) as *const u64) };
    if atomic != 0 && val != 0 {
        shim_objc_retain(val);
        shim_objc_autorelease(val)
    } else {
        val
    }
}

/// `void objc_setProperty_atomic(id self, SEL _cmd, id newValue, ptrdiff_t offset)`
///
/// Atomic property setter — retain new, release old (eliding host spinlock).
pub extern "C" fn shim_objc_set_property_atomic(
    obj: u64,
    _sel: u64,
    new_value: u64,
    offset: isize,
) {
    if obj == 0 {
        return;
    }
    let slot = unsafe { (obj as *mut u8).offset(offset) as *mut u64 };
    let old = unsafe { *slot };
    if old == new_value {
        return;
    }
    if new_value != 0 {
        shim_objc_retain(new_value);
    }
    unsafe {
        *slot = new_value;
    }
    if old != 0 {
        shim_objc_release(old);
    }
}

/// `void objc_setProperty_atomic_copy(id self, SEL _cmd, id newValue, ptrdiff_t offset)`
///
/// Atomic property setter with copy — elides copy (same as nonatomic_copy),
/// retains new, releases old.
pub extern "C" fn shim_objc_set_property_atomic_copy(
    obj: u64,
    _sel: u64,
    new_value: u64,
    offset: isize,
) {
    shim_objc_set_property_atomic(obj, _sel, new_value, offset)
}

/// `void objc_copyStruct(void *dest, const void *src, ptrdiff_t size, BOOL atomic, BOOL hasStrong)`
///
/// Copies a struct between ivars. Used by synthesized struct-type property accessors.
pub extern "C" fn shim_objc_copy_struct(
    dest: u64,
    src: u64,
    size: isize,
    _atomic: u8,
    _has_strong: u8,
) {
    if dest == 0 || src == 0 || size <= 0 {
        return;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, size as usize);
    }
}

/// `void objc_storeStrong(id *location, id obj)`
pub extern "C" fn shim_objc_store_strong(location: *mut u64, obj: u64) {
    if location.is_null() {
        return;
    }
    let old = unsafe { *location };
    if old == obj {
        return;
    }
    if obj != 0 {
        shim_objc_retain(obj);
    }
    unsafe {
        *location = obj;
    }
    if old != 0 {
        shim_objc_release(old);
    }
}

/// `id objc_autorelease(id obj)`
pub extern "C" fn shim_objc_autorelease(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::refcount::AUTORELEASE_POOLS.with(|pools| {
        pools.borrow_mut().autorelease(obj);
    });
    obj
}

// ── ARC return-value optimization shims ────────────────────────────

/// `id objc_autoreleaseReturnValue(id obj)`
pub extern "C" fn shim_objc_autorelease_return_value(obj: u64) -> u64 {
    shim_objc_autorelease(obj)
}

/// `id objc_retainAutoreleasedReturnValue(id obj)`
pub extern "C" fn shim_objc_retain_autoreleased_return_value(obj: u64) -> u64 {
    shim_objc_retain(obj)
}

/// `id objc_unsafeClaimAutoreleasedReturnValue(id obj)`
pub extern "C" fn shim_objc_unsafe_claim_autoreleased_return_value(obj: u64) -> u64 {
    obj
}

/// `id objc_retainAutorelease(id obj)`
pub extern "C" fn shim_objc_retain_autorelease(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    shim_objc_retain(obj);
    shim_objc_autorelease(obj)
}

// ── NSObject memory-management instance method shims ───────────────

/// `-[NSObject retain]`
pub extern "C" fn shim_nsobject_retain(obj: u64, _cmd: u64) -> u64 {
    shim_objc_retain(obj)
}

/// `-[NSObject release]`
pub extern "C" fn shim_nsobject_release(obj: u64, _cmd: u64) {
    shim_objc_release(obj);
}

/// `-[NSObject autorelease]`
pub extern "C" fn shim_nsobject_autorelease(obj: u64, _cmd: u64) -> u64 {
    shim_objc_autorelease(obj)
}

/// `-[NSObject retainCount]`
pub extern "C" fn shim_nsobject_retain_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::refcount::REFCOUNTS.count(obj) as u64
}

/// `-[NSObject dealloc]` — no-op; actual free happens in objc_release.
pub extern "C" fn shim_nsobject_dealloc(_obj: u64, _cmd: u64) {}

/// `void objc_enumerationMutation(id obj)`
///
/// Called by the compiler when a collection is mutated during fast enumeration.
/// Apple's runtime throws an `NSGenericException`; we log and abort.
pub extern "C" fn shim_objc_enumeration_mutation(obj: u64) {
    log::error!(
        "objc_enumerationMutation: collection {:#x} mutated during enumeration",
        obj
    );
    std::process::abort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retain_null_is_noop() {
        assert_eq!(shim_objc_retain(0), 0);
    }

    #[test]
    fn release_null_is_noop() {
        shim_objc_release(0); // must not panic
    }

    #[test]
    fn autorelease_null_returns_zero() {
        assert_eq!(shim_objc_autorelease(0), 0);
    }

    #[test]
    fn store_strong_null_location() {
        shim_objc_store_strong(std::ptr::null_mut(), 42); // must not panic
    }

    #[test]
    fn autorelease_return_value_passthrough() {
        assert_eq!(shim_objc_autorelease_return_value(0), 0);
        // Non-zero goes through autorelease path; just verify it returns the value
        assert_eq!(
            shim_objc_unsafe_claim_autoreleased_return_value(0xABC),
            0xABC
        );
    }

    #[test]
    fn retain_autoreleased_null() {
        assert_eq!(shim_objc_retain_autoreleased_return_value(0), 0);
    }

    #[test]
    fn unsafe_claim_passthrough() {
        assert_eq!(shim_objc_unsafe_claim_autoreleased_return_value(0), 0);
        assert_eq!(
            shim_objc_unsafe_claim_autoreleased_return_value(0xDEAD),
            0xDEAD
        );
    }

    #[test]
    fn retain_autorelease_null() {
        assert_eq!(shim_objc_retain_autorelease(0), 0);
    }

    #[test]
    fn pool_push_pop_cycle() {
        let token = shim_objc_autorelease_pool_push();
        shim_objc_autorelease_pool_pop(token); // must not panic
    }

    #[test]
    fn nsobject_dealloc_null() {
        shim_nsobject_dealloc(0, 0); // must not panic
    }

    #[test]
    fn autorelease_return_value_nonzero() {
        // Push a pool so autorelease has somewhere to record the object.
        let token = shim_objc_autorelease_pool_push();
        let val = 0x4242_0000u64;
        let result = shim_objc_autorelease_return_value(val);
        assert_eq!(result, val);
        // Don't pop — val isn't a real allocation, so release would be UB.
        let _ = token;
    }

    #[test]
    fn retain_autoreleased_return_value_nonzero() {
        let val = 0x1234_0000u64;
        let result = shim_objc_retain_autoreleased_return_value(val);
        assert_eq!(result, val);
    }

    #[test]
    fn set_property_nonatomic_basic() {
        let buf = crate::pal_alloc().calloc(1, 24) as u64;
        assert_ne!(buf, 0);
        shim_objc_set_property_nonatomic(buf, 0, 0xDEAD_BEEF, 8);
        let stored = unsafe { *((buf as *const u8).add(8) as *const u64) };
        assert_eq!(stored, 0xDEAD_BEEF);
        crate::pal_alloc().free(buf as *mut u8);
    }

    #[test]
    fn set_property_nonatomic_copy_basic() {
        let buf = crate::pal_alloc().calloc(1, 24) as u64;
        assert_ne!(buf, 0);
        shim_objc_set_property_nonatomic_copy(buf, 0, 0xCAFE_BABE, 8);
        let stored = unsafe { *((buf as *const u8).add(8) as *const u64) };
        assert_eq!(stored, 0xCAFE_BABE);
        crate::pal_alloc().free(buf as *mut u8);
    }

    #[test]
    fn get_property_null_obj_returns_zero() {
        assert_eq!(shim_objc_get_property(0, 0, 8, 0), 0);
    }

    #[test]
    fn get_property_nonatomic_reads_slot() {
        let buf = crate::pal_alloc().calloc(1, 24) as u64;
        assert_ne!(buf, 0);
        unsafe {
            *((buf as *mut u8).add(8) as *mut u64) = 0xBEEF_FACE;
        }
        let val = shim_objc_get_property(buf, 0, 8, 0);
        assert_eq!(val, 0xBEEF_FACE);
        crate::pal_alloc().free(buf as *mut u8);
    }

    #[test]
    fn get_property_null_value_returns_zero() {
        let buf = crate::pal_alloc().calloc(1, 24) as u64;
        assert_ne!(buf, 0);
        // slot is zero from calloc
        let val = shim_objc_get_property(buf, 0, 8, 1); // atomic=true
        assert_eq!(val, 0);
        crate::pal_alloc().free(buf as *mut u8);
    }

    #[test]
    fn set_property_atomic_null_obj() {
        shim_objc_set_property_atomic(0, 0, 42, 8); // must not crash
    }

    #[test]
    fn set_property_atomic_stores_value() {
        let buf = crate::pal_alloc().calloc(1, 24) as u64;
        assert_ne!(buf, 0);
        shim_objc_set_property_atomic(buf, 0, 0xABCD_1234, 8);
        let stored = unsafe { *((buf as *const u8).add(8) as *const u64) };
        assert_eq!(stored, 0xABCD_1234);
        crate::pal_alloc().free(buf as *mut u8);
    }

    #[test]
    fn set_property_atomic_copy_stores_value() {
        let buf = crate::pal_alloc().calloc(1, 24) as u64;
        assert_ne!(buf, 0);
        shim_objc_set_property_atomic_copy(buf, 0, 0xFEED_FACE, 8);
        let stored = unsafe { *((buf as *const u8).add(8) as *const u64) };
        assert_eq!(stored, 0xFEED_FACE);
        crate::pal_alloc().free(buf as *mut u8);
    }

    #[test]
    fn copy_struct_null_is_noop() {
        shim_objc_copy_struct(0, 0, 8, 0, 0); // must not crash
    }

    #[test]
    fn copy_struct_copies_bytes() {
        let src = crate::pal_alloc().calloc(1, 16) as u64;
        let dst = crate::pal_alloc().calloc(1, 16) as u64;
        assert_ne!(src, 0);
        assert_ne!(dst, 0);
        unsafe {
            *(src as *mut u64) = 0xDEAD_BEEF;
            *((src as *mut u8).add(8) as *mut u64) = 0xCAFE_BABE;
        }
        shim_objc_copy_struct(dst, src, 16, 1, 0);
        let a = unsafe { *(dst as *const u64) };
        let b = unsafe { *((dst as *const u8).add(8) as *const u64) };
        assert_eq!(a, 0xDEAD_BEEF);
        assert_eq!(b, 0xCAFE_BABE);
        crate::pal_alloc().free(src as *mut u8);
        crate::pal_alloc().free(dst as *mut u8);
    }

    #[test]
    fn copy_struct_zero_size_is_noop() {
        let src = crate::pal_alloc().calloc(1, 8) as u64;
        let dst = crate::pal_alloc().calloc(1, 8) as u64;
        shim_objc_copy_struct(dst, src, 0, 0, 0); // must not crash
        shim_objc_copy_struct(dst, src, -1, 0, 0); // negative size is noop
        crate::pal_alloc().free(src as *mut u8);
        crate::pal_alloc().free(dst as *mut u8);
    }
}
