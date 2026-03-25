//! NSConditionLock shims.

use super::*;

/// `-[NSConditionLock initWithCondition:]`
pub extern "C" fn shim_nsconditionlock_init_with_condition(
    obj: u64,
    _cmd: u64,
    condition: i64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsconditionlock_class_addr();
        crate::pal_threading().mutex_init(mutex_ptr(obj));
        crate::pal_threading().cond_init(cond_ptr(obj));
        *((obj as usize + CONDLOCK_VALUE_OFFSET) as *mut i64) = condition;
    }
    obj
}

/// `-[NSConditionLock init]` — initializes with condition 0.
pub extern "C" fn shim_nsconditionlock_init(obj: u64, cmd: u64) -> u64 {
    shim_nsconditionlock_init_with_condition(obj, cmd, 0)
}

/// `+[NSConditionLock alloc]`
pub extern "C" fn shim_nsconditionlock_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSCONDITIONLOCK_SIZE) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = nsobj::nsconditionlock_class_addr();
        }
    }
    obj
}

/// `-[NSConditionLock condition]`
pub extern "C" fn shim_nsconditionlock_condition(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CONDLOCK_VALUE_OFFSET) as *const i64) }
}

/// `-[NSConditionLock lock]`
pub extern "C" fn shim_nsconditionlock_lock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().mutex_lock(mutex_ptr(obj));
}

/// `-[NSConditionLock unlock]`
pub extern "C" fn shim_nsconditionlock_unlock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().cond_broadcast(cond_ptr(obj));
    crate::pal_threading().mutex_unlock(mutex_ptr(obj));
}

/// `-[NSConditionLock lockWhenCondition:]`
pub extern "C" fn shim_nsconditionlock_lock_when_condition(obj: u64, _cmd: u64, condition: i64) {
    if obj == 0 {
        return;
    }
    let val_ptr = (obj as usize + CONDLOCK_VALUE_OFFSET) as *mut i64;
    unsafe {
        crate::pal_threading().mutex_lock(mutex_ptr(obj));
        // The condition value is modified by unlockWithCondition: on another
        // thread while we wait. pthread_cond_wait releases/reacquires the mutex.
        #[allow(clippy::while_immutable_condition)]
        while std::ptr::read_volatile(val_ptr) != condition {
            crate::pal_threading().cond_wait(cond_ptr(obj), mutex_ptr(obj));
        }
    }
}

/// `-[NSConditionLock unlockWithCondition:]`
pub extern "C" fn shim_nsconditionlock_unlock_with_condition(obj: u64, _cmd: u64, condition: i64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + CONDLOCK_VALUE_OFFSET) as *mut i64) = condition;
        crate::pal_threading().cond_broadcast(cond_ptr(obj));
        crate::pal_threading().mutex_unlock(mutex_ptr(obj));
    }
}

/// `-[NSConditionLock tryLock]`
pub extern "C" fn shim_nsconditionlock_try_lock(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    if crate::pal_threading().mutex_trylock(mutex_ptr(obj)) {
        1
    } else {
        0
    }
}

/// `-[NSConditionLock name]`
pub extern "C" fn shim_nsconditionlock_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CONDLOCK_NAME_OFFSET) as *const u64) }
}

/// `-[NSConditionLock setName:]`
pub extern "C" fn shim_nsconditionlock_set_name(obj: u64, _cmd: u64, name: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + CONDLOCK_NAME_OFFSET) as *mut u64) = name;
    }
}

/// `-[NSConditionLock dealloc]`
pub extern "C" fn shim_nsconditionlock_dealloc(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    crate::pal_threading().cond_destroy(cond_ptr(obj));
    crate::pal_threading().mutex_destroy(mutex_ptr(obj));
    crate::pal_alloc().free(obj as *mut u8);
}
