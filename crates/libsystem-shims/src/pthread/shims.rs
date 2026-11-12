//! Thin pthread shim functions that delegate to the host OS.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

/// macOS `_PTHREAD_MUTEX_SIG_init` — the 4-byte signature Darwin writes
/// at the start of every `pthread_mutex_t` to mark it as initialised.
/// Linux interprets those same bytes as lock-state / owner, causing
/// deadlocks when we hand a macOS-format mutex to `pthread_mutex_lock`.
const DARWIN_PTHREAD_MUTEX_SIG_INIT: u32 = 0x32AA_ABA7;

/// If `mutex` starts with the macOS signature, zero it and call
/// `pthread_mutex_init` so Linux gets a valid mutex.
#[cfg(not(target_os = "macos"))]
unsafe fn maybe_reinit_darwin_mutex(mutex: *mut libc::pthread_mutex_t) {
    // Guard: mutex pointers below the page boundary are always invalid.
    // We've seen guest code pass bogus values (e.g. 0x1) when TLS or
    // heap state is corrupted.  Don't crash the host trying to read them.
    if (mutex as usize) < 0x10000 {
        log::warn!(
            "pthread_mutex: refusing to reinit mutex at bogus address {mutex:p} — possible TLS/heap corruption"
        );
        return;
    }
    let sig = (mutex as *const u32).read_unaligned();
    if sig == DARWIN_PTHREAD_MUTEX_SIG_INIT {
        log::debug!("pthread_mutex: detected macOS sig {sig:#x}, reinitializing for Linux");
        std::ptr::write_bytes(
            mutex as *mut u8,
            0,
            std::mem::size_of::<libc::pthread_mutex_t>(),
        );
        libc::pthread_mutex_init(mutex, std::ptr::null());
    }
}

#[cfg(target_os = "macos")]
unsafe fn maybe_reinit_darwin_mutex(_mutex: *mut libc::pthread_mutex_t) {
    // On macOS the signature is expected — nothing to do.
}

// ── Thread lifecycle ────────────────────────────────────────────────

struct GuestThreadStart {
    start_routine: extern "C" fn(*mut libc::c_void) -> *mut libc::c_void,
    arg: *mut libc::c_void,
}

#[cfg(not(target_os = "macos"))]
fn pthread_attr_registry() -> &'static Mutex<HashMap<usize, Box<libc::pthread_attr_t>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Box<libc::pthread_attr_t>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(not(target_os = "macos"))]
fn guest_attr_key(attr: *const libc::pthread_attr_t) -> Result<usize, libc::c_int> {
    if attr.is_null() {
        Err(libc::EINVAL)
    } else {
        Ok(attr as usize)
    }
}

extern "C" fn guest_thread_start(raw: *mut libc::c_void) -> *mut libc::c_void {
    let start = unsafe { Box::from_raw(raw.cast::<GuestThreadStart>()) };
    let start_routine = start.start_routine;
    let arg = start.arg;
    drop(start);

    log::trace!(
        "pthread_create: entering guest thread start routine {:p} with arg {:p}",
        start_routine as *const (),
        arg,
    );

    start_routine(arg)
}

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
    let start = Box::new(GuestThreadStart {
        start_routine: f,
        arg,
    });
    let raw = Box::into_raw(start).cast::<libc::c_void>();

    log::trace!(
        "pthread_create: spawning new thread for guest start routine {:p}",
        f as *const (),
    );

    #[cfg(not(target_os = "macos"))]
    let host_attr_storage = if attr.is_null() {
        log::trace!("pthread_create: guest attr is null; using default host thread attrs");
        None
    } else {
        let key = attr as usize;
        let mut registry = pthread_attr_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match registry.remove(&key) {
            Some(host_attr) => {
                log::trace!(
                    "pthread_create: using virtualized host attr for guest attr {:p}",
                    attr,
                );
                Some(host_attr)
            }
            None => {
                log::trace!(
                    "pthread_create: guest attr {:p} not initialized via shim; using default host thread attrs",
                    attr,
                );
                None
            }
        }
    };
    #[cfg(target_os = "macos")]
    let host_attr = attr;

    #[cfg(not(target_os = "macos"))]
    let host_attr = host_attr_storage
        .as_deref()
        .map_or(std::ptr::null(), |host| host as *const libc::pthread_attr_t);

    let rc = unsafe { libc::pthread_create(thread, host_attr, guest_thread_start, raw) };
    if rc != 0 {
        unsafe {
            drop(Box::from_raw(raw.cast::<GuestThreadStart>()));
        }
    }
    #[cfg(not(target_os = "macos"))]
    if let Some(mut host_attr) = host_attr_storage {
        let destroy_rc = unsafe { libc::pthread_attr_destroy(&mut *host_attr) };
        if destroy_rc != 0 {
            log::warn!(
                "pthread_create: host attr destroy failed with {destroy_rc} for guest attr {:p}",
                attr,
            );
        }
    }
    rc
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
pub(super) extern "C" fn shim_pthread_equal(
    t1: libc::pthread_t,
    t2: libc::pthread_t,
) -> libc::c_int {
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
    #[cfg(not(target_os = "macos"))]
    {
        let Ok(key) = guest_attr_key(attr) else {
            return libc::EINVAL;
        };

        let mut host_attr = Box::new(unsafe { std::mem::zeroed::<libc::pthread_attr_t>() });
        let rc = unsafe { libc::pthread_attr_init(&mut *host_attr) };
        if rc != 0 {
            return rc;
        }

        let mut registry = pthread_attr_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(mut old_attr) = registry.insert(key, host_attr) {
            let _ = unsafe { libc::pthread_attr_destroy(&mut *old_attr) };
        }
        return 0;
    }

    #[cfg(target_os = "macos")]
    unsafe {
        libc::pthread_attr_init(attr)
    }
}

pub(super) extern "C" fn shim_pthread_attr_destroy(attr: *mut libc::pthread_attr_t) -> libc::c_int {
    #[cfg(not(target_os = "macos"))]
    {
        let Ok(key) = guest_attr_key(attr) else {
            return libc::EINVAL;
        };

        let mut registry = pthread_attr_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(mut host_attr) = registry.remove(&key) else {
            return 0;
        };
        return unsafe { libc::pthread_attr_destroy(&mut *host_attr) };
    }

    #[cfg(target_os = "macos")]
    unsafe {
        libc::pthread_attr_destroy(attr)
    }
}

pub(super) extern "C" fn shim_pthread_attr_setdetachstate(
    attr: *mut libc::pthread_attr_t,
    detachstate: libc::c_int,
) -> libc::c_int {
    #[cfg(not(target_os = "macos"))]
    {
        let Ok(key) = guest_attr_key(attr) else {
            return libc::EINVAL;
        };

        let mut registry = pthread_attr_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(host_attr) = registry.get_mut(&key) else {
            return libc::EINVAL;
        };
        return unsafe { libc::pthread_attr_setdetachstate(&mut **host_attr, detachstate) };
    }

    #[cfg(target_os = "macos")]
    unsafe {
        libc::pthread_attr_setdetachstate(attr, detachstate)
    }
}

pub(super) extern "C" fn shim_pthread_attr_getdetachstate(
    attr: *const libc::pthread_attr_t,
    detachstate: *mut libc::c_int,
) -> libc::c_int {
    // libc crate doesn't expose pthread_attr_getdetachstate on all targets,
    // so we link it directly via POSIX.
    extern "C" {
        fn pthread_attr_getdetachstate(
            attr: *const libc::pthread_attr_t,
            detachstate: *mut libc::c_int,
        ) -> libc::c_int;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let Ok(key) = guest_attr_key(attr) else {
            return libc::EINVAL;
        };

        let registry = pthread_attr_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(host_attr) = registry.get(&key) else {
            return libc::EINVAL;
        };
        return unsafe { pthread_attr_getdetachstate(&**host_attr, detachstate) };
    }

    #[cfg(target_os = "macos")]
    unsafe {
        pthread_attr_getdetachstate(attr, detachstate)
    }
}

pub(super) extern "C" fn shim_pthread_attr_setstacksize(
    attr: *mut libc::pthread_attr_t,
    stacksize: libc::size_t,
) -> libc::c_int {
    #[cfg(not(target_os = "macos"))]
    {
        let Ok(key) = guest_attr_key(attr) else {
            return libc::EINVAL;
        };

        let mut registry = pthread_attr_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(host_attr) = registry.get_mut(&key) else {
            return libc::EINVAL;
        };
        return unsafe { libc::pthread_attr_setstacksize(&mut **host_attr, stacksize) };
    }

    #[cfg(target_os = "macos")]
    unsafe {
        libc::pthread_attr_setstacksize(attr, stacksize)
    }
}

pub(super) extern "C" fn shim_pthread_attr_getstacksize(
    attr: *const libc::pthread_attr_t,
    stacksize: *mut libc::size_t,
) -> libc::c_int {
    #[cfg(not(target_os = "macos"))]
    {
        let Ok(key) = guest_attr_key(attr) else {
            return libc::EINVAL;
        };

        let registry = pthread_attr_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(host_attr) = registry.get(&key) else {
            return libc::EINVAL;
        };
        return unsafe { libc::pthread_attr_getstacksize(&**host_attr, stacksize) };
    }

    #[cfg(target_os = "macos")]
    unsafe {
        libc::pthread_attr_getstacksize(attr, stacksize)
    }
}

pub(super) extern "C" fn shim_pthread_get_stackaddr_np(
    thread: libc::pthread_t,
) -> *mut libc::c_void {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn pthread_get_stackaddr_np(thread: libc::pthread_t) -> *mut libc::c_void;
        }

        unsafe { pthread_get_stackaddr_np(thread) }
    }

    #[cfg(not(target_os = "macos"))]
    {
        extern "C" {
            fn pthread_getattr_np(
                thread: libc::pthread_t,
                attr: *mut libc::pthread_attr_t,
            ) -> libc::c_int;
            fn pthread_attr_getstack(
                attr: *const libc::pthread_attr_t,
                stackaddr: *mut *mut libc::c_void,
                stacksize: *mut libc::size_t,
            ) -> libc::c_int;
        }

        unsafe {
            let mut attr = std::mem::zeroed::<libc::pthread_attr_t>();
            if pthread_getattr_np(thread, &mut attr) != 0 {
                return std::ptr::null_mut();
            }

            let mut stackaddr = std::ptr::null_mut();
            let mut stacksize = 0;
            let ret = pthread_attr_getstack(&attr, &mut stackaddr, &mut stacksize);
            let _ = libc::pthread_attr_destroy(&mut attr);
            if ret != 0 || stackaddr.is_null() {
                return std::ptr::null_mut();
            }

            (stackaddr as *mut u8).add(stacksize) as *mut libc::c_void
        }
    }
}

pub(super) extern "C" fn shim_pthread_get_stacksize_np(thread: libc::pthread_t) -> libc::size_t {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn pthread_get_stacksize_np(thread: libc::pthread_t) -> libc::size_t;
        }

        unsafe { pthread_get_stacksize_np(thread) }
    }

    #[cfg(not(target_os = "macos"))]
    {
        extern "C" {
            fn pthread_getattr_np(
                thread: libc::pthread_t,
                attr: *mut libc::pthread_attr_t,
            ) -> libc::c_int;
            fn pthread_attr_getstack(
                attr: *const libc::pthread_attr_t,
                stackaddr: *mut *mut libc::c_void,
                stacksize: *mut libc::size_t,
            ) -> libc::c_int;
        }

        unsafe {
            let mut attr = std::mem::zeroed::<libc::pthread_attr_t>();
            if pthread_getattr_np(thread, &mut attr) != 0 {
                return 0;
            }

            let mut stackaddr = std::ptr::null_mut();
            let mut stacksize = 0;
            let ret = pthread_attr_getstack(&attr, &mut stackaddr, &mut stacksize);
            let _ = libc::pthread_attr_destroy(&mut attr);
            if ret != 0 {
                return 0;
            }

            stacksize
        }
    }
}

// ── Mutexes ─────────────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_mutex_init(
    mutex: *mut libc::pthread_mutex_t,
    attr: *const libc::pthread_mutexattr_t,
) -> libc::c_int {
    // Zero out first to clear any macOS signature bytes.
    unsafe {
        std::ptr::write_bytes(
            mutex as *mut u8,
            0,
            std::mem::size_of::<libc::pthread_mutex_t>(),
        );
        libc::pthread_mutex_init(mutex, attr)
    }
}

pub(super) extern "C" fn shim_pthread_mutex_lock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    unsafe { maybe_reinit_darwin_mutex(mutex) };
    if (mutex as usize) < 0x10000 {
        return libc::EINVAL;
    }
    unsafe { libc::pthread_mutex_lock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_trylock(
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    unsafe { maybe_reinit_darwin_mutex(mutex) };
    if (mutex as usize) < 0x10000 {
        return libc::EINVAL;
    }
    unsafe { libc::pthread_mutex_trylock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_unlock(
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    if (mutex as usize) < 0x10000 {
        return libc::EINVAL;
    }
    unsafe { libc::pthread_mutex_unlock(mutex) }
}

pub(super) extern "C" fn shim_pthread_mutex_destroy(
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    unsafe { libc::pthread_mutex_destroy(mutex) }
}

// ── Mutex attributes ────────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_mutexattr_init(
    attr: *mut libc::pthread_mutexattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_init(attr) }
}

pub(super) extern "C" fn shim_pthread_mutexattr_destroy(
    attr: *mut libc::pthread_mutexattr_t,
) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_destroy(attr) }
}

pub(super) extern "C" fn shim_pthread_mutexattr_settype(
    attr: *mut libc::pthread_mutexattr_t,
    kind: libc::c_int,
) -> libc::c_int {
    unsafe { libc::pthread_mutexattr_settype(attr, kind) }
}

// ── Read-write locks ────────────────────────────────────────────────

#[derive(Default)]
struct DarwinRwLockState {
    readers: HashMap<u64, usize>,
    writer: Option<u64>,
    waiting_writers: usize,
}

struct DarwinRwLock {
    state: Mutex<DarwinRwLockState>,
    condvar: Condvar,
}

impl DarwinRwLock {
    fn new() -> Self {
        Self {
            state: Mutex::new(DarwinRwLockState::default()),
            condvar: Condvar::new(),
        }
    }
}

static DARWIN_RWLOCKS: OnceLock<Mutex<HashMap<usize, Arc<DarwinRwLock>>>> = OnceLock::new();

fn darwin_rwlocks() -> &'static Mutex<HashMap<usize, Arc<DarwinRwLock>>> {
    DARWIN_RWLOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn current_thread_token() -> u64 {
    unsafe { libc::pthread_self() as usize as u64 }
}

fn get_or_create_darwin_rwlock(
    rwlock: *mut libc::pthread_rwlock_t,
) -> Result<Arc<DarwinRwLock>, libc::c_int> {
    if rwlock.is_null() {
        return Err(libc::EINVAL);
    }

    let key = rwlock as usize;
    let mut registry = darwin_rwlocks().lock().unwrap_or_else(|e| e.into_inner());
    Ok(Arc::clone(
        registry
            .entry(key)
            .or_insert_with(|| Arc::new(DarwinRwLock::new())),
    ))
}

fn remove_darwin_rwlock(rwlock: *mut libc::pthread_rwlock_t) {
    let key = rwlock as usize;
    let mut registry = darwin_rwlocks().lock().unwrap_or_else(|e| e.into_inner());
    registry.remove(&key);
}

struct DarwinCondVar {
    inner: std::cell::UnsafeCell<libc::pthread_cond_t>,
}

unsafe impl Send for DarwinCondVar {}
unsafe impl Sync for DarwinCondVar {}

impl DarwinCondVar {
    fn new(attr: *const libc::pthread_condattr_t) -> Result<Self, libc::c_int> {
        let mut inner = unsafe { std::mem::zeroed::<libc::pthread_cond_t>() };
        let rc = unsafe { libc::pthread_cond_init(&mut inner, attr) };
        if rc != 0 {
            return Err(rc);
        }
        Ok(Self {
            inner: std::cell::UnsafeCell::new(inner),
        })
    }

    fn as_ptr(&self) -> *mut libc::pthread_cond_t {
        self.inner.get()
    }
}

static DARWIN_CONDS: OnceLock<Mutex<HashMap<usize, Arc<DarwinCondVar>>>> = OnceLock::new();

fn darwin_conds() -> &'static Mutex<HashMap<usize, Arc<DarwinCondVar>>> {
    DARWIN_CONDS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn destroy_darwin_condvar(mut cond: Arc<DarwinCondVar>) -> libc::c_int {
    let Some(host_cond) = Arc::get_mut(&mut cond) else {
        return libc::EBUSY;
    };
    unsafe { libc::pthread_cond_destroy(host_cond.as_ptr()) }
}

fn get_or_create_darwin_cond(
    cond: *mut libc::pthread_cond_t,
    attr: *const libc::pthread_condattr_t,
) -> Result<Arc<DarwinCondVar>, libc::c_int> {
    if cond.is_null() {
        return Err(libc::EINVAL);
    }

    let key = cond as usize;
    let mut registry = darwin_conds().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(existing) = registry.get(&key) {
        return Ok(Arc::clone(existing));
    }

    #[cfg(not(target_os = "macos"))]
    let host_attr = {
        if !attr.is_null() {
            log::trace!(
                "pthread_cond_init: guest attr {:p} ignored on Linux; using default host cond attrs",
                attr,
            );
        }
        std::ptr::null()
    };
    #[cfg(target_os = "macos")]
    let host_attr = attr;

    let host_cond = Arc::new(DarwinCondVar::new(host_attr)?);
    registry.insert(key, Arc::clone(&host_cond));
    Ok(host_cond)
}

fn remove_darwin_cond(cond: *mut libc::pthread_cond_t) -> Option<Arc<DarwinCondVar>> {
    let key = cond as usize;
    let mut registry = darwin_conds().lock().unwrap_or_else(|e| e.into_inner());
    registry.remove(&key)
}

pub(super) extern "C" fn shim_pthread_rwlock_init(
    rwlock: *mut libc::pthread_rwlock_t,
    _attr: *const libc::pthread_rwlockattr_t,
) -> libc::c_int {
    let Ok(lock) = get_or_create_darwin_rwlock(rwlock) else {
        return libc::EINVAL;
    };
    let mut state = lock.state.lock().unwrap_or_else(|e| e.into_inner());
    *state = DarwinRwLockState::default();
    0
}

pub(super) extern "C" fn shim_pthread_rwlock_rdlock(
    rwlock: *mut libc::pthread_rwlock_t,
) -> libc::c_int {
    let Ok(lock) = get_or_create_darwin_rwlock(rwlock) else {
        return libc::EINVAL;
    };
    let thread = current_thread_token();
    let mut state = lock.state.lock().unwrap_or_else(|e| e.into_inner());
    while state.writer.is_some() || state.waiting_writers > 0 {
        state = lock.condvar.wait(state).unwrap_or_else(|e| e.into_inner());
    }
    *state.readers.entry(thread).or_insert(0) += 1;
    0
}

pub(super) extern "C" fn shim_pthread_rwlock_wrlock(
    rwlock: *mut libc::pthread_rwlock_t,
) -> libc::c_int {
    let Ok(lock) = get_or_create_darwin_rwlock(rwlock) else {
        return libc::EINVAL;
    };
    let thread = current_thread_token();
    let mut state = lock.state.lock().unwrap_or_else(|e| e.into_inner());
    if state.writer == Some(thread) {
        return libc::EDEADLK;
    }
    state.waiting_writers += 1;
    while state.writer.is_some() || !state.readers.is_empty() {
        state = lock.condvar.wait(state).unwrap_or_else(|e| e.into_inner());
    }
    state.waiting_writers -= 1;
    state.writer = Some(thread);
    0
}

pub(super) extern "C" fn shim_pthread_rwlock_tryrdlock(
    rwlock: *mut libc::pthread_rwlock_t,
) -> libc::c_int {
    let Ok(lock) = get_or_create_darwin_rwlock(rwlock) else {
        return libc::EINVAL;
    };
    let thread = current_thread_token();
    let mut state = lock.state.lock().unwrap_or_else(|e| e.into_inner());
    if state.writer.is_some() || state.waiting_writers > 0 {
        return libc::EBUSY;
    }
    *state.readers.entry(thread).or_insert(0) += 1;
    0
}

pub(super) extern "C" fn shim_pthread_rwlock_trywrlock(
    rwlock: *mut libc::pthread_rwlock_t,
) -> libc::c_int {
    let Ok(lock) = get_or_create_darwin_rwlock(rwlock) else {
        return libc::EINVAL;
    };
    let thread = current_thread_token();
    let mut state = lock.state.lock().unwrap_or_else(|e| e.into_inner());
    if state.writer.is_some() || !state.readers.is_empty() {
        return libc::EBUSY;
    }
    state.writer = Some(thread);
    0
}

pub(super) extern "C" fn shim_pthread_rwlock_unlock(
    rwlock: *mut libc::pthread_rwlock_t,
) -> libc::c_int {
    let Ok(lock) = get_or_create_darwin_rwlock(rwlock) else {
        return libc::EINVAL;
    };
    let thread = current_thread_token();
    let mut state = lock.state.lock().unwrap_or_else(|e| e.into_inner());

    if state.writer == Some(thread) {
        state.writer = None;
        lock.condvar.notify_all();
        return 0;
    }

    if let Some(count) = state.readers.get_mut(&thread) {
        *count -= 1;
        if *count == 0 {
            state.readers.remove(&thread);
        }
        if state.readers.is_empty() {
            lock.condvar.notify_all();
        }
        return 0;
    }

    libc::EPERM
}

pub(super) extern "C" fn shim_pthread_rwlock_destroy(
    rwlock: *mut libc::pthread_rwlock_t,
) -> libc::c_int {
    let Ok(lock) = get_or_create_darwin_rwlock(rwlock) else {
        return libc::EINVAL;
    };
    let state = lock.state.lock().unwrap_or_else(|e| e.into_inner());
    if state.writer.is_some() || !state.readers.is_empty() {
        return libc::EBUSY;
    }
    drop(state);
    remove_darwin_rwlock(rwlock);
    0
}

// ── Condition variables ─────────────────────────────────────────────

pub(super) extern "C" fn shim_pthread_cond_init(
    cond: *mut libc::pthread_cond_t,
    attr: *const libc::pthread_condattr_t,
) -> libc::c_int {
    let host_cond = match get_or_create_darwin_cond(cond, attr) {
        Ok(condvar) => condvar,
        Err(rc) => return rc,
    };

    let _ = host_cond;
    0
}

pub(super) extern "C" fn shim_pthread_cond_wait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    let host_cond = match get_or_create_darwin_cond(cond, std::ptr::null()) {
        Ok(condvar) => condvar,
        Err(rc) => return rc,
    };
    log::debug!("pthread_cond_wait(cond={cond:?}, mutex={mutex:?})");
    let ret = unsafe { libc::pthread_cond_wait(host_cond.as_ptr(), mutex) };
    log::debug!("pthread_cond_wait returned {ret}");
    ret
}

pub(super) extern "C" fn shim_pthread_cond_timedwait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
    abstime: *const libc::timespec,
) -> libc::c_int {
    let host_cond = match get_or_create_darwin_cond(cond, std::ptr::null()) {
        Ok(condvar) => condvar,
        Err(rc) => return rc,
    };
    let ret = unsafe { libc::pthread_cond_timedwait(host_cond.as_ptr(), mutex, abstime) };
    // Translate Linux ETIMEDOUT (110) → macOS ETIMEDOUT (60).
    // Guest code (e.g. BIND9) checks the return value against the macOS
    // constant and asserts on unexpected values.
    #[cfg(target_os = "linux")]
    if ret == libc::ETIMEDOUT {
        return DARWIN_ETIMEDOUT;
    }
    ret
}

// macOS ETIMEDOUT = 60 (Operation timed out).
#[cfg(target_os = "linux")]
const DARWIN_ETIMEDOUT: i32 = 60;

pub(super) extern "C" fn shim_pthread_cond_signal(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    let host_cond = match get_or_create_darwin_cond(cond, std::ptr::null()) {
        Ok(condvar) => condvar,
        Err(rc) => return rc,
    };
    unsafe { libc::pthread_cond_signal(host_cond.as_ptr()) }
}

pub(super) extern "C" fn shim_pthread_cond_broadcast(
    cond: *mut libc::pthread_cond_t,
) -> libc::c_int {
    let host_cond = match get_or_create_darwin_cond(cond, std::ptr::null()) {
        Ok(condvar) => condvar,
        Err(rc) => return rc,
    };
    unsafe { libc::pthread_cond_broadcast(host_cond.as_ptr()) }
}

pub(super) extern "C" fn shim_pthread_cond_destroy(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    let Some(host_cond) = remove_darwin_cond(cond) else {
        return 0;
    };
    destroy_darwin_condvar(host_cond)
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

// macOS pthread_once_t layout (16 bytes):
//   __sig: i64    = 0x30B1BCBA (PTHREAD_ONCE_INIT magic)
//   __opaque: i64 = 0
// Linux pthread_once_t layout (4 bytes):
//   __once: i32   = 0 (PTHREAD_ONCE_INIT)
//
// If the macOS once-control has the DARWIN magic and hasn't run (__opaque == 0),
// we must zero the first 4 bytes before passing to Linux's pthread_once.
const DARWIN_PTHREAD_ONCE_SIG: i64 = 0x30B1_BCBA;

pub(super) extern "C" fn shim_pthread_once(
    once_control: *mut libc::pthread_once_t,
    init_routine: Option<extern "C" fn()>,
) -> libc::c_int {
    match init_routine {
        Some(f) => {
            // macOS libc wants Option<extern "C" fn()>, Linux wants bare fn ptr.
            #[cfg(target_os = "macos")]
            {
                unsafe { libc::pthread_once(once_control, Some(f)) }
            }
            #[cfg(not(target_os = "macos"))]
            {
                // Translate macOS pthread_once_t → Linux pthread_once_t.
                // The guest binary has macOS PTHREAD_ONCE_INIT: first 4 bytes = 0x30B1BCBA.
                // Linux PTHREAD_ONCE_INIT: first 4 bytes = 0.
                // If we see the macOS magic, zero it so Linux treats it as "not yet run".
                unsafe {
                    let raw = once_control as *mut i32;
                    let first_word = raw.read_unaligned();
                    // macOS magic signature in little-endian:
                    if first_word == DARWIN_PTHREAD_ONCE_SIG as i32 {
                        raw.write_unaligned(0);
                    }
                    libc::pthread_once(once_control, f)
                }
            }
        }
        None => libc::EINVAL,
    }
}

// ── Scheduling ──────────────────────────────────────────────────────

pub(super) extern "C" fn shim_sched_yield() -> libc::c_int {
    unsafe { libc::sched_yield() }
}

// ── macOS-specific (with portable fallbacks) ────────────────────────

/// `int pthread_main_np(void)` — returns 1 if current thread is main.
///
/// On macOS: delegates to `libc::pthread_main_np()`.
/// On Linux: compares `gettid() == getpid()`.
pub(super) extern "C" fn shim_pthread_main_np() -> libc::c_int {
    #[cfg(target_os = "macos")]
    {
        unsafe { libc::pthread_main_np() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        // On Linux, the main thread has tid == pid.
        unsafe {
            let tid = libc::syscall(libc::SYS_gettid) as libc::pid_t;
            let pid = libc::getpid();
            if tid == pid {
                1
            } else {
                0
            }
        }
    }
}

// ── C++ std::mutex shims (macOS ABI → Linux pthread) ────────────────
//
// Guest binaries compiled for macOS inline-initialise `pthread_mutex_t`
// with the Darwin signature `0x32AAABA7` instead of calling
// `pthread_mutex_init`.  The host's `std::mutex::lock()` (from the
// system libc++) doesn't know about this, so we shim the mangled C++
// symbols to intercept the call at the guest import level.

/// `std::__1::mutex::lock()` — _ZNSt3__15mutex4lockEv
///
/// `this` points to the `std::mutex` object whose first member is
/// a `pthread_mutex_t`.
pub(super) extern "C" fn shim_std_mutex_lock(this: *mut libc::pthread_mutex_t) {
    unsafe { maybe_reinit_darwin_mutex(this) };
    let ret = unsafe { libc::pthread_mutex_lock(this) };
    if ret != 0 {
        log::warn!("std::mutex::lock failed with {ret}");
    }
}

/// `std::__1::mutex::unlock()` — _ZNSt3__15mutex6unlockEv
pub(super) extern "C" fn shim_std_mutex_unlock(this: *mut libc::pthread_mutex_t) {
    let ret = unsafe { libc::pthread_mutex_unlock(this) };
    if ret != 0 {
        log::warn!("std::mutex::unlock failed with {ret}");
    }
}

/// `std::__1::mutex::try_lock()` — _ZNSt3__15mutex8try_lockEv
pub(super) extern "C" fn shim_std_mutex_try_lock(this: *mut libc::pthread_mutex_t) -> bool {
    unsafe { maybe_reinit_darwin_mutex(this) };
    unsafe { libc::pthread_mutex_trylock(this) == 0 }
}
