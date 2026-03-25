//! Thin pthread shim functions that delegate to the host OS.

// ── Thread lifecycle ────────────────────────────────────────────────

/// `int pthread_create(pthread_t *, const pthread_attr_t *, start_routine, void *arg)`
pub(super) extern "C" fn shim_pthread_create(
    thread: *mut libc::pthread_t,
    attr: *const libc::pthread_attr_t,
    start_routine: Option<extern "C" fn(*mut libc::c_void) -> *mut libc::c_void>,
    arg: *mut libc::c_void,
) -> libc::c_int {
    let Some(f) = start_routine else {
        return libc::EINVAL;
    };
    log::trace!("pthread_create: spawning new thread");
    unsafe { libc::pthread_create(thread, attr, f, arg) }
}

/// `int pthread_join(pthread_t thread, void **retval)`
pub(super) extern "C" fn shim_pthread_join(
    thread: libc::pthread_t,
    retval: *mut *mut libc::c_void,
) -> libc::c_int {
    log::trace!("pthread_join: waiting for thread");
    unsafe { libc::pthread_join(thread, retval) }
}

/// `int pthread_detach(pthread_t thread)`
pub(super) extern "C" fn shim_pthread_detach(thread: libc::pthread_t) -> libc::c_int {
    unsafe { libc::pthread_detach(thread) }
}

/// `void pthread_exit(void *retval)` — never returns.
pub(super) extern "C" fn shim_pthread_exit(retval: *mut libc::c_void) {
    unsafe { libc::pthread_exit(retval) }
}

/// `pthread_t pthread_self(void)`
pub(super) extern "C" fn shim_pthread_self() -> libc::pthread_t {
    unsafe { libc::pthread_self() }
}

/// `int pthread_equal(pthread_t t1, pthread_t t2)`
pub(super) extern "C" fn shim_pthread_equal(t1: libc::pthread_t, t2: libc::pthread_t) -> libc::c_int {
    if t1 == t2 {
        1
    } else {
        0
    }
}

/// `int pthread_cancel(pthread_t thread)`
pub(super) extern "C" fn shim_pthread_cancel(thread: libc::pthread_t) -> libc::c_int {
    unsafe { libc::pthread_cancel(thread) }
}

// ── Thread attributes ───────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_attr_init(attr: *mut libc::pthread_attr_t) -> libc::c_int {
    unsafe { libc::pthread_attr_init(attr) }
}

pub(super) extern "C" fn shim_pthread_attr_destroy(attr: *mut libc::pthread_attr_t) -> libc::c_int {
    unsafe { libc::pthread_attr_destroy(attr) }
}

pub(super) extern "C" fn shim_pthread_attr_setdetachstate(
    attr: *mut libc::pthread_attr_t,
    detachstate: libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_attr_setdetachstate(attr, detachstate) }
}

pub(super) extern "C" fn shim_pthread_attr_getdetachstate(
    attr: *const libc::pthread_attr_t,
    detachstate: *mut libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_attr_getdetachstate(attr, detachstate) }
}

pub(super) extern "C" fn shim_pthread_attr_setstacksize(
    attr: *mut libc::pthread_attr_t,
    stacksize: libc::size_t,
) -> libc::c_int {
    unsafe { libc::pthread_attr_setstacksize(attr, stacksize) }
}

pub(super) extern "C" fn shim_pthread_attr_getstacksize(
    attr: *const libc::pthread_attr_t,
    stacksize: *mut libc::size_t,
) -> libc::c_int {
    unsafe { libc::pthread_attr_getstacksize(attr, stacksize) }
}

// ── Mutexes ─────────────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_mutex_init(
    mutex: *mut libc::pthread_mutex_t,
    attr: *const libc::pthread_mutexattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_mutex_init(mutex, attr) }
}

pub(super) extern "C" fn shim_pthread_mutex_lock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_lock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_trylock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_trylock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_unlock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_unlock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_destroy(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_destroy(mutex) }
}

// ── Mutex attributes ────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_mutexattr_init(attr: *mut libc::pthread_mutexattr_t) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_init(attr) }
}

pub(super) extern "C" fn shim_pthread_mutexattr_destroy(attr: *mut libc::pthread_mutexattr_t) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_destroy(attr) }
}

pub(super) extern "C" fn shim_pthread_mutexattr_settype(
    attr: *mut libc::pthread_mutexattr_t,
    kind: libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_settype(attr, kind) }
}

// ── Read-write locks ────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_rwlock_init(
    rwlock: *mut libc::pthread_rwlock_t,
    attr: *const libc::pthread_rwlockattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_rwlock_init(rwlock, attr) }
}

pub(super) extern "C" fn shim_pthread_rwlock_rdlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_rdlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_wrlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_wrlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_tryrdlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_tryrdlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_trywrlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_trywrlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_unlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_unlock(rwlock) }
}

pub(super) extern "C" fn shim_pthread_rwlock_destroy(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_destroy(rwlock) }
}

// ── Condition variables ─────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_cond_init(
    cond: *mut libc::pthread_cond_t,
    attr: *const libc::pthread_condattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_cond_init(cond, attr) }
}

pub(super) extern "C" fn shim_pthread_cond_wait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    unsafe { libc::pthread_cond_wait(cond, mutex) }
}

pub(super) extern "C" fn shim_pthread_cond_timedwait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
    abstime: *const libc::timespec,
) -> libc::c_int {
    unsafe { libc::pthread_cond_timedwait(cond, mutex, abstime) }
}

pub(super) extern "C" fn shim_pthread_cond_signal(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_signal(cond) }
}

pub(super) extern "C" fn shim_pthread_cond_broadcast(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_broadcast(cond) }
}

pub(super) extern "C" fn shim_pthread_cond_destroy(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_destroy(cond) }
}

// ── Thread-specific data ────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_key_create(
    key: *mut libc::pthread_key_t,
    destructor: Option<unsafe extern "C" fn(*mut libc::c_void)>,
) -> libc::c_int {
    unsafe { libc::pthread_key_create(key, destructor) }
}

pub(super) extern "C" fn shim_pthread_key_delete(key: libc::pthread_key_t) -> libc::c_int {
    unsafe { libc::pthread_key_delete(key) }
}

pub(super) extern "C" fn shim_pthread_getspecific(key: libc::pthread_key_t) -> *mut libc::c_void {
    unsafe { libc::pthread_getspecific(key) }
}

pub(super) extern "C" fn shim_pthread_setspecific(
    key: libc::pthread_key_t,
    value: *const libc::c_void,
) -> libc::c_int {
    unsafe { libc::pthread_setspecific(key, value) }
}

// ── Once control ────────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_once(
    once_control: *mut libc::pthread_once_t,
    init_routine: Option<extern "C" fn()>,
) -> libc::c_int {
    match init_routine {
        Some(f) => unsafe { libc::pthread_once(once_control, Some(f)) },
        None => libc::EINVAL,
    }
}

// ── Scheduling ──────────────────────────────────────────────────────

pub(super) extern "C" fn shim_sched_yield() -> libc::c_int {
    unsafe { libc::sched_yield() }
}

// ── macOS-specific ──────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_main_np() -> libc::c_int {
    unsafe { libc::pthread_main_np() }
}
