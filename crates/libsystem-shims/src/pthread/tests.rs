use super::shims::*;
use super::PthreadShims;
use crate::table::ShimTable;

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
    assert!(table.resolve("_pthread_get_stackaddr_np").is_some());
    assert!(table.resolve("_pthread_get_stacksize_np").is_some());
    assert_eq!(table.len(), 47);
}

#[test]
fn pthread_self_returns_nonzero() {
    let val = shim_pthread_self();
    assert_ne!(val as usize, 0);
}

#[test]
fn pthread_stack_queries_return_nonzero() {
    let thread = shim_pthread_self();

    assert!(!shim_pthread_get_stackaddr_np(thread).is_null());
    assert!(shim_pthread_get_stacksize_np(thread) > 0);
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
fn pthread_create_with_virtualized_guest_attr_buffer() {
    extern "C" fn thread_func(arg: *mut libc::c_void) -> *mut libc::c_void {
        arg
    }

    let mut guest_attr = [0u8; 8];
    let attr = guest_attr.as_mut_ptr().cast::<libc::pthread_attr_t>();

    assert_eq!(shim_pthread_attr_init(attr), 0);
    assert_eq!(shim_pthread_attr_setstacksize(attr, 1 << 20), 0);

    let mut stacksize: libc::size_t = 0;
    assert_eq!(shim_pthread_attr_getstacksize(attr, &mut stacksize), 0);
    assert!(stacksize >= (1 << 20));

    unsafe {
        let mut thread: libc::pthread_t = std::mem::zeroed();
        let arg = 0x1234usize as *mut libc::c_void;
        assert_eq!(
            shim_pthread_create(&mut thread, attr, Some(thread_func), arg),
            0
        );
        let mut retval: *mut libc::c_void = std::ptr::null_mut();
        assert_eq!(shim_pthread_join(thread, &mut retval), 0);
        assert_eq!(retval, arg);
    }

    assert_eq!(shim_pthread_attr_destroy(attr), 0);
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
fn pthread_rwlock_lazy_init_roundtrip() {
    unsafe {
        let mut rwlock: libc::pthread_rwlock_t = std::mem::zeroed();
        assert_eq!(shim_pthread_rwlock_wrlock(&mut rwlock), 0);
        assert_eq!(shim_pthread_rwlock_unlock(&mut rwlock), 0);
        assert_eq!(shim_pthread_rwlock_rdlock(&mut rwlock), 0);
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

#[test]
fn pthread_cond_lazy_guest_buffer_signal_thread() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let ready = Arc::new(AtomicBool::new(false));
    let mut guest_cond = [0u8; 8];
    let cond_ptr = guest_cond.as_mut_ptr().cast::<libc::pthread_cond_t>();

    let mutex_ptr = unsafe { libc::calloc(1, std::mem::size_of::<libc::pthread_mutex_t>()) }
        as *mut libc::pthread_mutex_t;
    unsafe {
        libc::pthread_mutex_init(mutex_ptr, std::ptr::null());
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

    std::thread::sleep(std::time::Duration::from_millis(50));

    shim_pthread_mutex_lock(mutex_ptr);
    ready.store(true, Ordering::Release);
    shim_pthread_cond_signal(cond_ptr);
    shim_pthread_mutex_unlock(mutex_ptr);

    waiter.join().unwrap();

    assert_eq!(shim_pthread_cond_destroy(cond_ptr), 0);
    unsafe {
        libc::pthread_mutex_destroy(mutex_ptr);
        libc::free(mutex_ptr as *mut libc::c_void);
    }
}
