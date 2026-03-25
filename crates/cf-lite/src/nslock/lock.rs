//! NSLock shims.

use super::*;

/// `-[NSLock init]` — initializes the mutex.
pub extern "C" fn shim_nslock_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nslock_class_addr();
        crate::pal_threading().mutex_init(mutex_ptr(obj));
    }
    obj
}

/// `+[NSLock alloc]` → allocate.
pub extern "C" fn shim_nslock_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSLOCK_SIZE) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = nsobj::nslock_class_addr();
        }
    }
    obj
}

/// `-[NSLock lock]`
pub extern "C" fn shim_nslock_lock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_lock(mutex_ptr(obj));
}

/// `-[NSLock unlock]`
pub extern "C" fn shim_nslock_unlock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_unlock(mutex_ptr(obj));
}

/// `-[NSLock tryLock]`
pub extern "C" fn shim_nslock_try_lock(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    if crate::pal_threading().mutex_trylock(mutex_ptr(obj)) {
        1
    } else {
        0
    }
}

/// `-[NSLock name]`
pub extern "C" fn shim_nslock_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSLOCK_NAME_OFFSET) as *const u64) }
}

/// `-[NSLock setName:]`
pub extern "C" fn shim_nslock_set_name(obj: u64, _cmd: u64, name: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + NSLOCK_NAME_OFFSET) as *mut u64) = name;
    }
}

/// `-[NSLock dealloc]`
pub extern "C" fn shim_nslock_dealloc(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_destroy(mutex_ptr(obj));
    crate::pal_alloc().free(obj as *mut u8);
}

/// `-[NSLock description]`
pub extern "C" fn shim_nslock_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let desc = c"<NSLock>";
    cfstring::shim_cf_string_create_with_cstring(0, desc.as_ptr(), 0)
}
