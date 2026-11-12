//! NSRecursiveLock shims.

use super::*;

/// `-[NSRecursiveLock init]` — initializes a recursive mutex.
pub extern "C" fn shim_nsrecursivelock_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsrecursivelock_class_addr();
    }
    crate::pal_threading().mutex_init_recursive(mutex_ptr(obj));
    obj
}

/// `+[NSRecursiveLock alloc]`
pub extern "C" fn shim_nsrecursivelock_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSRECURSIVELOCK_SIZE) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = nsobj::nsrecursivelock_class_addr();
        }
    }
    obj
}

/// `-[NSRecursiveLock lock]`
pub extern "C" fn shim_nsrecursivelock_lock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_lock(mutex_ptr(obj));
}

/// `-[NSRecursiveLock unlock]`
pub extern "C" fn shim_nsrecursivelock_unlock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_unlock(mutex_ptr(obj));
}

/// `-[NSRecursiveLock tryLock]`
pub extern "C" fn shim_nsrecursivelock_try_lock(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    if crate::pal_threading().mutex_trylock(mutex_ptr(obj)) {
        1
    } else {
        0
    }
}

/// `-[NSRecursiveLock dealloc]`
pub extern "C" fn shim_nsrecursivelock_dealloc(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_destroy(mutex_ptr(obj));
    crate::pal_alloc().free(obj as *mut u8);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn alloc_returns_nonzero_with_correct_isa() {
        let obj = shim_nsrecursivelock_alloc(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(
            read_field(obj, 0),
            crate::nsobj::nsrecursivelock_class_addr()
        );
        // Free manually — mutex was never initialized so dealloc would be unsound.
        crate::pal_alloc().free(obj as *mut u8);
    }

    #[test]
    fn lock_nil_is_safe() {
        shim_nsrecursivelock_lock(0, 0);
    }

    #[test]
    fn unlock_nil_is_safe() {
        shim_nsrecursivelock_unlock(0, 0);
    }

    #[test]
    fn try_lock_nil_returns_zero() {
        assert_eq!(shim_nsrecursivelock_try_lock(0, 0), 0);
    }

    #[test]
    fn init_lock_unlock_dealloc_cycle() {
        let obj = shim_nsrecursivelock_alloc(0, 0);
        assert_ne!(obj, 0);
        let obj2 = shim_nsrecursivelock_init(obj, 0);
        assert_eq!(obj2, obj);
        // Lock and unlock on same thread — recursive mutex, so no deadlock.
        shim_nsrecursivelock_lock(obj, 0);
        shim_nsrecursivelock_unlock(obj, 0);
        shim_nsrecursivelock_dealloc(obj, 0);
    }

    #[test]
    fn try_lock_succeeds_on_unlocked_mutex() {
        let obj = shim_nsrecursivelock_alloc(0, 0);
        shim_nsrecursivelock_init(obj, 0);
        let result = shim_nsrecursivelock_try_lock(obj, 0);
        assert_eq!(result, 1);
        shim_nsrecursivelock_unlock(obj, 0);
        shim_nsrecursivelock_dealloc(obj, 0);
    }
}
