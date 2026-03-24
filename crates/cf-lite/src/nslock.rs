//! NSLock, NSRecursiveLock, NSCondition, NSConditionLock shims.
//!
//! These Foundation classes wrap POSIX threading primitives.
//! Since our pthread shims delegate to the host OS, these classes
//! provide real locking with full thread safety.
//!
//! ## Object Layouts
//!
//! **NSLock** (80 bytes):
//!   - [0..8)    ISA pointer
//!   - [8..72)   pthread_mutex_t (64 bytes on arm64 macOS)
//!   - [72..80)  name pointer (NSString * or 0)
//!
//! **NSRecursiveLock** (80 bytes): same layout, recursive mutex type.
//!
//! **NSCondition** (128 bytes):
//!   - [0..8)     ISA pointer
//!   - [8..72)    pthread_mutex_t (64 bytes)
//!   - [72..120)  pthread_cond_t (48 bytes)
//!   - [120..128) name pointer
//!
//! **NSConditionLock** (136 bytes):
//!   - [0..8)     ISA pointer
//!   - [8..72)    pthread_mutex_t (64 bytes)
//!   - [72..120)  pthread_cond_t (48 bytes)
//!   - [120..128) condition value (i64)
//!   - [128..136) name pointer

use crate::cfstring;
use crate::nsobj;

const NSLOCK_SIZE: usize = 80;
const NSRECURSIVELOCK_SIZE: usize = 80;
const NSCONDITION_SIZE: usize = 128;
const NSCONDITIONLOCK_SIZE: usize = 136;

// Offsets
const MUTEX_OFFSET: usize = 8;
const NSLOCK_NAME_OFFSET: usize = 72;
const COND_OFFSET: usize = 72;
const NSCONDITION_NAME_OFFSET: usize = 120;
const CONDLOCK_VALUE_OFFSET: usize = 120;
const CONDLOCK_NAME_OFFSET: usize = 128;

fn mutex_ptr(obj: u64) -> *mut libc::pthread_mutex_t {
    (obj as usize + MUTEX_OFFSET) as *mut libc::pthread_mutex_t
}

fn cond_ptr(obj: u64) -> *mut libc::pthread_cond_t {
    (obj as usize + COND_OFFSET) as *mut libc::pthread_cond_t
}

// ════════════════════════════════════════════════════════════════════
// NSLock
// ════════════════════════════════════════════════════════════════════

/// `-[NSLock init]` — initializes the mutex.
pub extern "C" fn shim_nslock_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nslock_class_addr();
        libc::pthread_mutex_init(mutex_ptr(obj), std::ptr::null());
    }
    obj
}

/// `+[NSLock alloc]` → allocate.
pub extern "C" fn shim_nslock_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, NSLOCK_SIZE) } as u64;
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
    unsafe {
        libc::pthread_mutex_lock(mutex_ptr(obj));
    }
}

/// `-[NSLock unlock]`
pub extern "C" fn shim_nslock_unlock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        libc::pthread_mutex_unlock(mutex_ptr(obj));
    }
}

/// `-[NSLock tryLock]`
pub extern "C" fn shim_nslock_try_lock(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let result = unsafe { libc::pthread_mutex_trylock(mutex_ptr(obj)) };
    if result == 0 {
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
    unsafe {
        libc::pthread_mutex_destroy(mutex_ptr(obj));
        libc::free(obj as *mut libc::c_void);
    }
}

/// `-[NSLock description]`
pub extern "C" fn shim_nslock_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let desc = c"<NSLock>";
    cfstring::shim_cf_string_create_with_cstring(0, desc.as_ptr(), 0)
}

// ════════════════════════════════════════════════════════════════════
// NSRecursiveLock
// ════════════════════════════════════════════════════════════════════

/// `-[NSRecursiveLock init]` — initializes a recursive mutex.
pub extern "C" fn shim_nsrecursivelock_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsrecursivelock_class_addr();
        let mut attr: libc::pthread_mutexattr_t = std::mem::zeroed();
        libc::pthread_mutexattr_init(&mut attr);
        libc::pthread_mutexattr_settype(&mut attr, libc::PTHREAD_MUTEX_RECURSIVE);
        libc::pthread_mutex_init(mutex_ptr(obj), &attr);
        libc::pthread_mutexattr_destroy(&mut attr);
    }
    obj
}

/// `+[NSRecursiveLock alloc]`
pub extern "C" fn shim_nsrecursivelock_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, NSRECURSIVELOCK_SIZE) } as u64;
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
    unsafe {
        libc::pthread_mutex_lock(mutex_ptr(obj));
    }
}

/// `-[NSRecursiveLock unlock]`
pub extern "C" fn shim_nsrecursivelock_unlock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        libc::pthread_mutex_unlock(mutex_ptr(obj));
    }
}

/// `-[NSRecursiveLock tryLock]`
pub extern "C" fn shim_nsrecursivelock_try_lock(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let result = unsafe { libc::pthread_mutex_trylock(mutex_ptr(obj)) };
    if result == 0 {
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
    unsafe {
        libc::pthread_mutex_destroy(mutex_ptr(obj));
        libc::free(obj as *mut libc::c_void);
    }
}

// ════════════════════════════════════════════════════════════════════
// NSCondition
// ════════════════════════════════════════════════════════════════════

/// `-[NSCondition init]`
pub extern "C" fn shim_nscondition_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nscondition_class_addr();
        libc::pthread_mutex_init(mutex_ptr(obj), std::ptr::null());
        libc::pthread_cond_init(cond_ptr(obj), std::ptr::null());
    }
    obj
}

/// `+[NSCondition alloc]`
pub extern "C" fn shim_nscondition_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, NSCONDITION_SIZE) } as u64;
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
    unsafe {
        libc::pthread_mutex_lock(mutex_ptr(obj));
    }
}

/// `-[NSCondition unlock]`
pub extern "C" fn shim_nscondition_unlock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        libc::pthread_mutex_unlock(mutex_ptr(obj));
    }
}

/// `-[NSCondition wait]`
pub extern "C" fn shim_nscondition_wait(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        libc::pthread_cond_wait(cond_ptr(obj), mutex_ptr(obj));
    }
}

/// `-[NSCondition signal]`
pub extern "C" fn shim_nscondition_signal(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        libc::pthread_cond_signal(cond_ptr(obj));
    }
}

/// `-[NSCondition broadcast]`
pub extern "C" fn shim_nscondition_broadcast(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        libc::pthread_cond_broadcast(cond_ptr(obj));
    }
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
    unsafe {
        libc::pthread_cond_destroy(cond_ptr(obj));
        libc::pthread_mutex_destroy(mutex_ptr(obj));
        libc::free(obj as *mut libc::c_void);
    }
}

// ════════════════════════════════════════════════════════════════════
// NSConditionLock
// ════════════════════════════════════════════════════════════════════

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
        libc::pthread_mutex_init(mutex_ptr(obj), std::ptr::null());
        libc::pthread_cond_init(cond_ptr(obj), std::ptr::null());
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
    let obj = unsafe { libc::calloc(1, NSCONDITIONLOCK_SIZE) } as u64;
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
    unsafe {
        libc::pthread_mutex_lock(mutex_ptr(obj));
    }
}

/// `-[NSConditionLock unlock]`
pub extern "C" fn shim_nsconditionlock_unlock(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        libc::pthread_cond_broadcast(cond_ptr(obj));
        libc::pthread_mutex_unlock(mutex_ptr(obj));
    }
}

/// `-[NSConditionLock lockWhenCondition:]`
pub extern "C" fn shim_nsconditionlock_lock_when_condition(obj: u64, _cmd: u64, condition: i64) {
    if obj == 0 {
        return;
    }
    let val_ptr = (obj as usize + CONDLOCK_VALUE_OFFSET) as *mut i64;
    unsafe {
        libc::pthread_mutex_lock(mutex_ptr(obj));
        // The condition value is modified by unlockWithCondition: on another
        // thread while we wait. pthread_cond_wait releases/reacquires the mutex.
        #[allow(clippy::while_immutable_condition)]
        while std::ptr::read_volatile(val_ptr) != condition {
            libc::pthread_cond_wait(cond_ptr(obj), mutex_ptr(obj));
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
        libc::pthread_cond_broadcast(cond_ptr(obj));
        libc::pthread_mutex_unlock(mutex_ptr(obj));
    }
}

/// `-[NSConditionLock tryLock]`
pub extern "C" fn shim_nsconditionlock_try_lock(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let result = unsafe { libc::pthread_mutex_trylock(mutex_ptr(obj)) };
    if result == 0 {
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
    unsafe {
        libc::pthread_cond_destroy(cond_ptr(obj));
        libc::pthread_mutex_destroy(mutex_ptr(obj));
        libc::free(obj as *mut libc::c_void);
    }
}
