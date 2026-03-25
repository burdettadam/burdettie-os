//! NSCondition shims.

use super::*;

/// `-[NSCondition init]`
pub extern "C" fn shim_nscondition_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nscondition_class_addr();
        crate::pal_threading().mutex_init(mutex_ptr(obj));
        crate::pal_threading().cond_init(cond_ptr(obj));
    }
    obj
}

/// `+[NSCondition alloc]`
pub extern "C" fn shim_nscondition_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSCONDITION_SIZE) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = nsobj::nscondition_class_addr();
        }
    }
    obj
}

/// `-[NSCondition lock]`
pub extern "C" fn shim_nscondition_lock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_lock(mutex_ptr(obj));
}

/// `-[NSCondition unlock]`
pub extern "C" fn shim_nscondition_unlock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_unlock(mutex_ptr(obj));
}

/// `-[NSCondition wait]`
pub extern "C" fn shim_nscondition_wait(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().cond_wait(cond_ptr(obj), mutex_ptr(obj));
}

/// `-[NSCondition signal]`
pub extern "C" fn shim_nscondition_signal(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().cond_signal(cond_ptr(obj));
}

/// `-[NSCondition broadcast]`
pub extern "C" fn shim_nscondition_broadcast(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().cond_broadcast(cond_ptr(obj));
}

/// `-[NSCondition name]`
pub extern "C" fn shim_nscondition_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSCONDITION_NAME_OFFSET) as *const u64) }
}

/// `-[NSCondition setName:]`
pub extern "C" fn shim_nscondition_set_name(obj: u64, _cmd: u64, name: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + NSCONDITION_NAME_OFFSET) as *mut u64) = name;
    }
}

/// `-[NSCondition dealloc]`
pub extern "C" fn shim_nscondition_dealloc(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().cond_destroy(cond_ptr(obj));
    crate::pal_threading().mutex_destroy(mutex_ptr(obj));
    crate::pal_alloc().free(obj as *mut u8);
}
