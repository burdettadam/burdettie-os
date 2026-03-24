//! POSIX thread (pthread) shims — real threading via host delegation.
//!
//! All pthread operations delegate to the host OS's real pthread
//! implementation. This works because lab binaries run in-process —
//! their memory (mutex structs, thread stacks, etc.) is real host
//! memory that the OS's pthread APIs operate on directly.
//!
//! # Thread Safety
//!
//! These shims enable real multi-threading. The emulated binary can
//! create threads, use real mutexes with actual contention, condition
//! variables, read-write locks, and thread-specific data — all backed
//! by the host OS kernel.

use crate::table::ShimTable;

/// Install pthread-related shims.
pub struct PthreadShims;

impl PthreadShims {
    /// Register all pthread shims in the table.
    pub fn install(table: &mut ShimTable) {
        // ── Thread lifecycle ──
        table.register(
            "_pthread_create",
            shim_pthread_create as u64,
            "pthread_create (real)",
        );
        table.register(
            "_pthread_join",
            shim_pthread_join as u64,
            "pthread_join (real)",
        );
        table.register(
            "_pthread_detach",
            shim_pthread_detach as u64,
            "pthread_detach (real)",
        );
        table.register(
            "_pthread_exit",
            shim_pthread_exit as u64,
            "pthread_exit (real)",
        );
        table.register(
            "_pthread_self",
            shim_pthread_self as u64,
            "pthread_self (real)",
        );
        table.register(
            "_pthread_equal",
            shim_pthread_equal as u64,
            "pthread_equal (real)",
        );
        table.register(
            "_pthread_cancel",
            shim_pthread_cancel as u64,
            "pthread_cancel (real)",
        );

        // ── Thread attributes ──
        table.register(
            "_pthread_attr_init",
            shim_pthread_attr_init as u64,
            "pthread_attr_init (real)",
        );
        table.register(
            "_pthread_attr_destroy",
            shim_pthread_attr_destroy as u64,
            "pthread_attr_destroy (real)",
        );
        table.register(
            "_pthread_attr_setdetachstate",
            shim_pthread_attr_setdetachstate as u64,
            "pthread_attr_setdetachstate (real)",
        );
        table.register(
            "_pthread_attr_getdetachstate",
            shim_pthread_attr_getdetachstate as u64,
            "pthread_attr_getdetachstate (real)",
        );
        table.register(
            "_pthread_attr_setstacksize",
            shim_pthread_attr_setstacksize as u64,
            "pthread_attr_setstacksize (real)",
        );
        table.register(
            "_pthread_attr_getstacksize",
            shim_pthread_attr_getstacksize as u64,
            "pthread_attr_getstacksize (real)",
        );

        // ── Mutexes ──
        table.register(
            "_pthread_mutex_init",
            shim_pthread_mutex_init as u64,
            "pthread_mutex_init (real)",
        );
        table.register(
            "_pthread_mutex_lock",
            shim_pthread_mutex_lock as u64,
            "pthread_mutex_lock (real)",
        );
        table.register(
            "_pthread_mutex_trylock",
            shim_pthread_mutex_trylock as u64,
            "pthread_mutex_trylock (real)",
        );
        table.register(
            "_pthread_mutex_unlock",
            shim_pthread_mutex_unlock as u64,
            "pthread_mutex_unlock (real)",
        );
        table.register(
            "_pthread_mutex_destroy",
            shim_pthread_mutex_destroy as u64,
            "pthread_mutex_destroy (real)",
        );

        // ── Mutex attributes ──
        table.register(
            "_pthread_mutexattr_init",
            shim_pthread_mutexattr_init as u64,
            "pthread_mutexattr_init (real)",
        );
        table.register(
            "_pthread_mutexattr_destroy",
            shim_pthread_mutexattr_destroy as u64,
            "pthread_mutexattr_destroy (real)",
        );
        table.register(
            "_pthread_mutexattr_settype",
            shim_pthread_mutexattr_settype as u64,
            "pthread_mutexattr_settype (real)",
        );
        // ── Read-write locks ──
        table.register(
            "_pthread_rwlock_init",
            shim_pthread_rwlock_init as u64,
            "pthread_rwlock_init (real)",
        );
        table.register(
            "_pthread_rwlock_rdlock",
            shim_pthread_rwlock_rdlock as u64,
            "pthread_rwlock_rdlock (real)",
        );
        table.register(
            "_pthread_rwlock_wrlock",
            shim_pthread_rwlock_wrlock as u64,
            "pthread_rwlock_wrlock (real)",
        );
        table.register(
            "_pthread_rwlock_tryrdlock",
            shim_pthread_rwlock_tryrdlock as u64,
            "pthread_rwlock_tryrdlock (real)",
        );
        table.register(
            "_pthread_rwlock_trywrlock",
            shim_pthread_rwlock_trywrlock as u64,
            "pthread_rwlock_trywrlock (real)",
        );
        table.register(
            "_pthread_rwlock_unlock",
            shim_pthread_rwlock_unlock as u64,
            "pthread_rwlock_unlock (real)",
        );
        table.register(
            "_pthread_rwlock_destroy",
            shim_pthread_rwlock_destroy as u64,
            "pthread_rwlock_destroy (real)",
        );

        // ── Condition variables ──
        table.register(
            "_pthread_cond_init",
            shim_pthread_cond_init as u64,
            "pthread_cond_init (real)",
        );
        table.register(
            "_pthread_cond_wait",
            shim_pthread_cond_wait as u64,
            "pthread_cond_wait (real)",
        );
        table.register(
            "_pthread_cond_timedwait",
            shim_pthread_cond_timedwait as u64,
            "pthread_cond_timedwait (real)",
        );
        table.register(
            "_pthread_cond_signal",
            shim_pthread_cond_signal as u64,
            "pthread_cond_signal (real)",
        );
        table.register(
            "_pthread_cond_broadcast",
            shim_pthread_cond_broadcast as u64,
            "pthread_cond_broadcast (real)",
        );
        table.register(
            "_pthread_cond_destroy",
            shim_pthread_cond_destroy as u64,
            "pthread_cond_destroy (real)",
        );

        // ── Thread-specific data (TSD / pthread keys) ──
        table.register(
            "_pthread_key_create",
            shim_pthread_key_create as u64,
            "pthread_key_create (real)",
        );
        table.register(
            "_pthread_key_delete",
            shim_pthread_key_delete as u64,
            "pthread_key_delete (real)",
        );
        table.register(
            "_pthread_getspecific",
            shim_pthread_getspecific as u64,
            "pthread_getspecific (real)",
        );
        table.register(
            "_pthread_setspecific",
            shim_pthread_setspecific as u64,
            "pthread_setspecific (real)",
        );

        // ── Once control ──
        table.register(
            "_pthread_once",
            shim_pthread_once as u64,
            "pthread_once (real)",
        );

        // ── Thread scheduling ──
        table.register(
            "_sched_yield",
            shim_sched_yield as u64,
            "sched_yield (real)",
        );
        table.register(
            "_pthread_yield_np",
            shim_sched_yield as u64,
            "pthread_yield_np (real)",
        );

        // ── Main thread query ──
        table.register(
            "_pthread_main_np",
            shim_pthread_main_np as u64,
            "pthread_main_np (real)",
        );

        log::debug!("PthreadShims: registered 42 pthread shims");
    }
}

// ── Thread lifecycle ────────────────────────────────────────────────

/// `int pthread_create(pthread_t *, const pthread_attr_t *, start_routine, void *arg)`
extern "C" fn shim_pthread_create(
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
extern "C" fn shim_pthread_join(
    thread: libc::pthread_t,
    retval: *mut *mut libc::c_void,
) -> libc::c_int {
    log::trace!("pthread_join: waiting for thread");
    unsafe { libc::pthread_join(thread, retval) }
}

/// `int pthread_detach(pthread_t thread)`
extern "C" fn shim_pthread_detach(thread: libc::pthread_t) -> libc::c_int {
    unsafe { libc::pthread_detach(thread) }
}

/// `void pthread_exit(void *retval)` — never returns.
extern "C" fn shim_pthread_exit(retval: *mut libc::c_void) {
    unsafe { libc::pthread_exit(retval) }
}

/// `pthread_t pthread_self(void)`
extern "C" fn shim_pthread_self() -> libc::pthread_t {
    unsafe { libc::pthread_self() }
}

/// `int pthread_equal(pthread_t t1, pthread_t t2)`
extern "C" fn shim_pthread_equal(t1: libc::pthread_t, t2: libc::pthread_t) -> libc::c_int {
    if t1 == t2 {
        1
    } else {
        0
    }
}

/// `int pthread_cancel(pthread_t thread)`
extern "C" fn shim_pthread_cancel(thread: libc::pthread_t) -> libc::c_int {
    unsafe { libc::pthread_cancel(thread) }
}

// ── Thread attributes ───────────────────────────────────────────────

extern "C" fn shim_pthread_attr_init(attr: *mut libc::pthread_attr_t) -> libc::c_int {
    unsafe { libc::pthread_attr_init(attr) }
}

extern "C" fn shim_pthread_attr_destroy(attr: *mut libc::pthread_attr_t) -> libc::c_int {
    unsafe { libc::pthread_attr_destroy(attr) }
}

extern "C" fn shim_pthread_attr_setdetachstate(
    attr: *mut libc::pthread_attr_t,
    detachstate: libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_attr_setdetachstate(attr, detachstate) }
}

extern "C" fn shim_pthread_attr_getdetachstate(
    attr: *const libc::pthread_attr_t,
    detachstate: *mut libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_attr_getdetachstate(attr, detachstate) }
}

extern "C" fn shim_pthread_attr_setstacksize(
    attr: *mut libc::pthread_attr_t,
    stacksize: libc::size_t,
) -> libc::c_int {
    unsafe { libc::pthread_attr_setstacksize(attr, stacksize) }
}

extern "C" fn shim_pthread_attr_getstacksize(
    attr: *const libc::pthread_attr_t,
    stacksize: *mut libc::size_t,
) -> libc::c_int {
    unsafe { libc::pthread_attr_getstacksize(attr, stacksize) }
}

// ── Mutexes ─────────────────────────────────────────────────────────

extern "C" fn shim_pthread_mutex_init(
    mutex: *mut libc::pthread_mutex_t,
    attr: *const libc::pthread_mutexattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_mutex_init(mutex, attr) }
}

extern "C" fn shim_pthread_mutex_lock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_lock(mutex) }
}

extern "C" fn shim_pthread_mutex_trylock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_trylock(mutex) }
}

extern "C" fn shim_pthread_mutex_unlock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_unlock(mutex) }
}

extern "C" fn shim_pthread_mutex_destroy(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { libc::pthread_mutex_destroy(mutex) }
}

// ── Mutex attributes ────────────────────────────────────────────────

extern "C" fn shim_pthread_mutexattr_init(attr: *mut libc::pthread_mutexattr_t) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_init(attr) }
}

extern "C" fn shim_pthread_mutexattr_destroy(attr: *mut libc::pthread_mutexattr_t) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_destroy(attr) }
}

extern "C" fn shim_pthread_mutexattr_settype(
    attr: *mut libc::pthread_mutexattr_t,
    kind: libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_settype(attr, kind) }
}

// ── Read-write locks ────────────────────────────────────────────────

extern "C" fn shim_pthread_rwlock_init(
    rwlock: *mut libc::pthread_rwlock_t,
    attr: *const libc::pthread_rwlockattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_rwlock_init(rwlock, attr) }
}

extern "C" fn shim_pthread_rwlock_rdlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_rdlock(rwlock) }
}

extern "C" fn shim_pthread_rwlock_wrlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_wrlock(rwlock) }
}

extern "C" fn shim_pthread_rwlock_tryrdlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_tryrdlock(rwlock) }
}

extern "C" fn shim_pthread_rwlock_trywrlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_trywrlock(rwlock) }
}

extern "C" fn shim_pthread_rwlock_unlock(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_unlock(rwlock) }
}

extern "C" fn shim_pthread_rwlock_destroy(rwlock: *mut libc::pthread_rwlock_t) -> libc::c_int {
    unsafe { libc::pthread_rwlock_destroy(rwlock) }
}

// ── Condition variables ─────────────────────────────────────────────

extern "C" fn shim_pthread_cond_init(
    cond: *mut libc::pthread_cond_t,
    attr: *const libc::pthread_condattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_cond_init(cond, attr) }
}

extern "C" fn shim_pthread_cond_wait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    unsafe { libc::pthread_cond_wait(cond, mutex) }
}

extern "C" fn shim_pthread_cond_timedwait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
    abstime: *const libc::timespec,
) -> libc::c_int {
    unsafe { libc::pthread_cond_timedwait(cond, mutex, abstime) }
}

extern "C" fn shim_pthread_cond_signal(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_signal(cond) }
}

extern "C" fn shim_pthread_cond_broadcast(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_broadcast(cond) }
}

extern "C" fn shim_pthread_cond_destroy(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    unsafe { libc::pthread_cond_destroy(cond) }
}

// ── Thread-specific data ────────────────────────────────────────────

extern "C" fn shim_pthread_key_create(
    key: *mut libc::pthread_key_t,
    destructor: Option<unsafe extern "C" fn(*mut libc::c_void)>,
) -> libc::c_int {
    unsafe { libc::pthread_key_create(key, destructor) }
}

extern "C" fn shim_pthread_key_delete(key: libc::pthread_key_t) -> libc::c_int {
    unsafe { libc::pthread_key_delete(key) }
}

extern "C" fn shim_pthread_getspecific(key: libc::pthread_key_t) -> *mut libc::c_void {
    unsafe { libc::pthread_getspecific(key) }
}

extern "C" fn shim_pthread_setspecific(
    key: libc::pthread_key_t,
    value: *const libc::c_void,
) -> libc::c_int {
    unsafe { libc::pthread_setspecific(key, value) }
}

// ── Once control ────────────────────────────────────────────────────

extern "C" fn shim_pthread_once(
    once_control: *mut libc::pthread_once_t,
    init_routine: Option<extern "C" fn()>,
) -> libc::c_int {
    match init_routine {
        Some(f) => unsafe { libc::pthread_once(once_control, Some(f)) },
        None => libc::EINVAL,
    }
}

// ── Scheduling ──────────────────────────────────────────────────────

extern "C" fn shim_sched_yield() -> libc::c_int {
    unsafe { libc::sched_yield() }
}

// ── macOS-specific ──────────────────────────────────────────────────

extern "C" fn shim_pthread_main_np() -> libc::c_int {
    unsafe { libc::pthread_main_np() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pthread_shims_registered() {
        let mut table = ShimTable::new();
        PthreadShims::install(&mut table);
        assert!(table.resolve("_pthread_self").is_some());
        assert!(table.resolve("_pthread_create").is_some());
        assert!(table.resolve("_pthread_join").is_some());
        assert!(table.resolve("_pthread_mutex_init").is_some());
        assert!(table.resolve("_pthread_mutex_lock").is_some());
        assert!(table.resolve("_pthread_mutex_unlock").is_some());
        assert!(table.resolve("_pthread_mutex_destroy").is_some());
        assert!(table.resolve("_pthread_mutex_trylock").is_some());
        assert!(table.resolve("_pthread_cond_init").is_some());
        assert!(table.resolve("_pthread_rwlock_init").is_some());
        assert!(table.resolve("_pthread_key_create").is_some());
        assert!(table.resolve("_pthread_once").is_some());
        assert!(table.resolve("_sched_yield").is_some());
        assert!(table.resolve("_pthread_main_np").is_some());
        assert_eq!(table.len(), 42);
    }

    #[test]
    fn pthread_self_returns_nonzero() {
        let val = shim_pthread_self();
        assert_ne!(val as usize, 0);
    }

    #[test]
    fn pthread_create_and_join() {
        extern "C" fn thread_func(arg: *mut libc::c_void) -> *mut libc::c_void {
            let val = arg as usize;
            (val * 2) as *mut libc::c_void
        }

        unsafe {
            let mut thread: libc::pthread_t = std::mem::zeroed();
            let arg = 21usize as *mut libc::c_void;
            assert_eq!(
                shim_pthread_create(&mut thread, std::ptr::null(), Some(thread_func), arg),
                0
            );
            let mut retval: *mut libc::c_void = std::ptr::null_mut();
            assert_eq!(shim_pthread_join(thread, &mut retval), 0);
            assert_eq!(retval as usize, 42);
        }
    }

    #[test]
    fn pthread_mutex_real_locking() {
        use std::sync::atomic::{AtomicI32, Ordering};
        use std::sync::Arc;

        static COUNTER: AtomicI32 = AtomicI32::new(0);

        // Allocate a mutex on the heap so it can be shared across threads
        let mutex = Arc::new(std::sync::Mutex::new(()));
        let mutex_raw: *mut libc::pthread_mutex_t =
            unsafe { libc::calloc(1, std::mem::size_of::<libc::pthread_mutex_t>()) }
                as *mut libc::pthread_mutex_t;
        unsafe { libc::pthread_mutex_init(mutex_raw, std::ptr::null()) };

        COUNTER.store(0, Ordering::SeqCst);
        let _ = mutex; // keep Arc alive

        // Spawn threads that increment a counter under the mutex
        let mut threads = Vec::new();
        for _ in 0..4 {
            let raw = mutex_raw as usize; // sendable
            let handle = std::thread::spawn(move || {
                let m = raw as *mut libc::pthread_mutex_t;
                for _ in 0..100 {
                    assert_eq!(shim_pthread_mutex_lock(m), 0);
                    let val = COUNTER.load(Ordering::Relaxed);
                    COUNTER.store(val + 1, Ordering::Relaxed);
                    assert_eq!(shim_pthread_mutex_unlock(m), 0);
                }
            });
            threads.push(handle);
        }
        for h in threads {
            h.join().unwrap();
        }

        assert_eq!(COUNTER.load(Ordering::SeqCst), 400);
        unsafe {
            libc::pthread_mutex_destroy(mutex_raw);
            libc::free(mutex_raw as *mut libc::c_void);
        }
    }

    #[test]
    fn pthread_once_thread_safe() {
        use std::sync::atomic::{AtomicI32, Ordering};

        static COUNTER: AtomicI32 = AtomicI32::new(0);

        extern "C" fn increment() {
            COUNTER.fetch_add(1, Ordering::Relaxed);
        }

        COUNTER.store(0, Ordering::Relaxed);
        let mut control: libc::pthread_once_t = libc::PTHREAD_ONCE_INIT;
        shim_pthread_once(&mut control, Some(increment));
        shim_pthread_once(&mut control, Some(increment));
        shim_pthread_once(&mut control, Some(increment));
        assert_eq!(COUNTER.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn pthread_key_roundtrip() {
        let mut key: libc::pthread_key_t = 0;
        assert_eq!(shim_pthread_key_create(&mut key, None), 0);
        let val = 42usize as *const libc::c_void;
        assert_eq!(shim_pthread_setspecific(key, val), 0);
        let retrieved = shim_pthread_getspecific(key);
        assert_eq!(retrieved as usize, 42);
        assert_eq!(shim_pthread_key_delete(key), 0);
    }

    #[test]
    fn pthread_rwlock_roundtrip() {
        unsafe {
            let mut rwlock: libc::pthread_rwlock_t = std::mem::zeroed();
            assert_eq!(shim_pthread_rwlock_init(&mut rwlock, std::ptr::null()), 0);
            assert_eq!(shim_pthread_rwlock_rdlock(&mut rwlock), 0);
            assert_eq!(shim_pthread_rwlock_unlock(&mut rwlock), 0);
            assert_eq!(shim_pthread_rwlock_wrlock(&mut rwlock), 0);
            assert_eq!(shim_pthread_rwlock_unlock(&mut rwlock), 0);
            assert_eq!(shim_pthread_rwlock_destroy(&mut rwlock), 0);
        }
    }

    #[test]
    fn pthread_cond_roundtrip() {
        unsafe {
            let mut cond: libc::pthread_cond_t = std::mem::zeroed();
            let mut mutex: libc::pthread_mutex_t = std::mem::zeroed();
            assert_eq!(shim_pthread_cond_init(&mut cond, std::ptr::null()), 0);
            assert_eq!(shim_pthread_mutex_init(&mut mutex, std::ptr::null()), 0);
            // Signal without waiters is valid
            assert_eq!(shim_pthread_cond_signal(&mut cond), 0);
            assert_eq!(shim_pthread_cond_broadcast(&mut cond), 0);
            assert_eq!(shim_pthread_cond_destroy(&mut cond), 0);
            assert_eq!(shim_pthread_mutex_destroy(&mut mutex), 0);
        }
    }

    #[test]
    fn pthread_cond_signal_thread() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let ready = Arc::new(AtomicBool::new(false));

        // Heap-allocate mutex + cond for cross-thread sharing
        let mutex_ptr = unsafe { libc::calloc(1, std::mem::size_of::<libc::pthread_mutex_t>()) }
            as *mut libc::pthread_mutex_t;
        let cond_ptr = unsafe { libc::calloc(1, std::mem::size_of::<libc::pthread_cond_t>()) }
            as *mut libc::pthread_cond_t;
        unsafe {
            libc::pthread_mutex_init(mutex_ptr, std::ptr::null());
            libc::pthread_cond_init(cond_ptr, std::ptr::null());
        }

        let m = mutex_ptr as usize;
        let c = cond_ptr as usize;
        let ready2 = ready.clone();

        let waiter = std::thread::spawn(move || {
            let mutex = m as *mut libc::pthread_mutex_t;
            let cond = c as *mut libc::pthread_cond_t;
            shim_pthread_mutex_lock(mutex);
            while !ready2.load(Ordering::Acquire) {
                shim_pthread_cond_wait(cond, mutex);
            }
            shim_pthread_mutex_unlock(mutex);
        });

        // Give the waiter time to start waiting
        std::thread::sleep(std::time::Duration::from_millis(50));

        shim_pthread_mutex_lock(mutex_ptr);
        ready.store(true, Ordering::Release);
        shim_pthread_cond_signal(cond_ptr);
        shim_pthread_mutex_unlock(mutex_ptr);

        waiter.join().unwrap();

        unsafe {
            libc::pthread_cond_destroy(cond_ptr);
            libc::pthread_mutex_destroy(mutex_ptr);
            libc::free(cond_ptr as *mut libc::c_void);
            libc::free(mutex_ptr as *mut libc::c_void);
        }
    }
}
