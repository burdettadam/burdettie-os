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
