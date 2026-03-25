//! ARC memory management: retain, release, autorelease, store-strong.

/// `void *objc_autoreleasePoolPush(void)`
pub extern "C" fn shim_objc_autorelease_pool_push() -> u64 {
    crate::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow_mut().push() as u64)
}

/// `void objc_autoreleasePoolPop(void *token)`
pub extern "C" fn shim_objc_autorelease_pool_pop(token: u64) {
    let drained =
        crate::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow_mut().pop(token as usize));
    let mut refcounts = crate::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for obj in drained {
        if obj != 0 {
            let remaining = refcounts.release(obj);
            if remaining == 0 {
                log::debug!("autorelease pool: deallocating {:#x}", obj);
                crate::pal_alloc().free(obj as *mut u8);
            }
        }
    }
}

/// `id objc_retain(id obj)`
pub extern "C" fn shim_objc_retain(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(obj);
    obj
}

/// `void objc_release(id obj)`
pub extern "C" fn shim_objc_release(obj: u64) {
    if obj == 0 {
        return;
    }
    let remaining = crate::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .release(obj);
    if remaining == 0 {
        log::debug!("objc_release: deallocating {:#x}", obj);
        crate::pal_alloc().free(obj as *mut u8);
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
    crate::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .count(obj) as u64
}

/// `-[NSObject dealloc]` — no-op; actual free happens in objc_release.
pub extern "C" fn shim_nsobject_dealloc(_obj: u64, _cmd: u64) {}
