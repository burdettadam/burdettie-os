//! NSAutoreleasePool explicit class shim.
//!
//! Legacy code uses `[[NSAutoreleasePool alloc] init]` / `[pool drain]`
//! instead of `@autoreleasepool {}`. This shim delegates to the per-thread
//! autorelease pool stack in `objc_runtime::refcount`.
//!
//! Object layout (16 bytes):
//!   [0..8):   ISA pointer
//!   [8..16):  pool_token (u64 — autorelease pool stack index)

use crate::nsobj;

const TOKEN_OFF: usize = 8;

/// `-[NSAutoreleasePool init]`
///
/// Pushes a new autorelease pool scope and stores the token.
pub extern "C" fn shim_nsautoreleasepool_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let token =
        objc_runtime::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow_mut().push() as u64);
    unsafe {
        *((obj as usize + TOKEN_OFF) as *mut u64) = token;
    }
    obj
}

/// `-[NSAutoreleasePool drain]`
///
/// Pops the autorelease pool back to the stored token, releasing all
/// autoreleased objects in the drained pools.
pub extern "C" fn shim_nsautoreleasepool_drain(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let token = unsafe { *((obj as usize + TOKEN_OFF) as *const u64) };
    let drained = objc_runtime::refcount::AUTORELEASE_POOLS
        .with(|pools| pools.borrow_mut().pop(token as usize));
    for o in drained {
        if o != 0 {
            let remaining = objc_runtime::refcount::REFCOUNTS.release(o);
            if remaining == 0 {
                crate::pal_alloc().free(o as *mut u8);
            }
        }
    }
}

/// `-[NSAutoreleasePool release]` — same as drain for legacy pools.
pub extern "C" fn shim_nsautoreleasepool_release(obj: u64, cmd: u64) {
    shim_nsautoreleasepool_drain(obj, cmd);
}

/// `-[NSAutoreleasePool dealloc]` — no-op; cleanup happens in drain.
pub extern "C" fn shim_nsautoreleasepool_dealloc(_obj: u64, _cmd: u64) {}

/// `+[NSAutoreleasePool alloc]`
pub extern "C" fn shim_nsautoreleasepool_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 16) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = nsobj::nsautoreleasepool_class_addr();
        }
    }
    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn autorelease_pool_init_and_drain() {
        // Alloc+init a pool
        let pool = alloc_obj(16, nsobj::nsautoreleasepool_class_addr());
        let pool = shim_nsautoreleasepool_init(pool, 0);
        assert_ne!(pool, 0);

        // Create an object and autorelease it
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        assert_ne!(obj, 0);
        // Give it a retain count of 1 (implicit), then autorelease it
        objc_runtime::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow_mut().autorelease(obj));

        // Drain — object should be released (count goes 1→0, freed)
        shim_nsautoreleasepool_drain(pool, 0);

        // After drain, the object's refcount entry is removed (count returns
        // implicit 1).  We verify drain didn't panic and the pool is drained.
        let depth = objc_runtime::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow().depth());
        assert_eq!(depth, 0);
        free_obj(pool);
    }

    #[test]
    fn autorelease_pool_nested() {
        // Outer pool
        let pool1 = alloc_obj(16, nsobj::nsautoreleasepool_class_addr());
        let pool1 = shim_nsautoreleasepool_init(pool1, 0);

        let obj_a = crate::pal_alloc().calloc(1, 8) as u64;
        objc_runtime::refcount::AUTORELEASE_POOLS
            .with(|pools| pools.borrow_mut().autorelease(obj_a));

        // Inner pool
        let pool2 = alloc_obj(16, nsobj::nsautoreleasepool_class_addr());
        let pool2 = shim_nsautoreleasepool_init(pool2, 0);

        let obj_b = crate::pal_alloc().calloc(1, 8) as u64;
        objc_runtime::refcount::AUTORELEASE_POOLS
            .with(|pools| pools.borrow_mut().autorelease(obj_b));

        // Drain inner — obj_b released, outer pool intact
        shim_nsautoreleasepool_drain(pool2, 0);
        let depth = objc_runtime::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow().depth());
        assert_eq!(depth, 1, "outer pool should still be active");

        // Drain outer — obj_a released
        shim_nsautoreleasepool_drain(pool1, 0);
        let depth = objc_runtime::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow().depth());
        assert_eq!(depth, 0, "all pools should be drained");

        free_obj(pool2);
        free_obj(pool1);
    }

    #[test]
    fn autorelease_pool_alloc_sets_isa() {
        let pool = shim_nsautoreleasepool_alloc(0, 0);
        assert_ne!(pool, 0);
        assert_eq!(read_field(pool, 0), nsobj::nsautoreleasepool_class_addr());
        free_obj(pool);
    }

    #[test]
    fn autorelease_pool_init_nil_returns_zero() {
        assert_eq!(shim_nsautoreleasepool_init(0, 0), 0);
    }

    #[test]
    fn autorelease_pool_drain_nil_no_crash() {
        shim_nsautoreleasepool_drain(0, 0);
    }

    #[test]
    fn autorelease_pool_release_is_drain() {
        let pool = alloc_obj(16, nsobj::nsautoreleasepool_class_addr());
        let pool = shim_nsautoreleasepool_init(pool, 0);
        // Release should behave identically to drain
        shim_nsautoreleasepool_release(pool, 0);
        let depth = objc_runtime::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow().depth());
        assert_eq!(depth, 0);
        free_obj(pool);
    }
}
