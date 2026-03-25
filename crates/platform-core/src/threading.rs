//! Threading primitives abstraction.
//!
//! Provides platform-independent mutex and condition variable operations.
//! These operate on opaque memory regions that callers pre-allocate at
//! known offsets within their object layouts (e.g., NSLock embeds a mutex
//! inline at offset 8).

/// Opaque mutex handle.  On POSIX this is `*mut pthread_mutex_t`.
pub type MutexPtr = *mut u8;

/// Opaque condition variable handle.  On POSIX this is `*mut pthread_cond_t`.
pub type CondPtr = *mut u8;

/// Platform mutex and condition variable operations.
///
/// All methods take raw pointers to pre-allocated memory. The caller
/// is responsible for ensuring the memory is large enough (see
/// `mutex_size` / `cond_size`) and properly aligned.
pub trait Threading: Send + Sync {
    /// Size in bytes of a mutex object.
    fn mutex_size(&self) -> usize;

    /// Size in bytes of a condition variable object.
    fn cond_size(&self) -> usize;

    /// Initialize a default (non-recursive) mutex at `ptr`.
    fn mutex_init(&self, ptr: MutexPtr);

    /// Initialize a recursive mutex at `ptr`.
    fn mutex_init_recursive(&self, ptr: MutexPtr);

    /// Lock the mutex. Blocks until acquired.
    fn mutex_lock(&self, ptr: MutexPtr);

    /// Unlock the mutex.
    fn mutex_unlock(&self, ptr: MutexPtr);

    /// Try to lock without blocking. Returns `true` if acquired.
    fn mutex_trylock(&self, ptr: MutexPtr) -> bool;

    /// Destroy the mutex.
    fn mutex_destroy(&self, ptr: MutexPtr);

    /// Initialize a condition variable at `ptr`.
    fn cond_init(&self, ptr: CondPtr);

    /// Wait on the condition variable, releasing `mutex` while waiting.
    fn cond_wait(&self, cond: CondPtr, mutex: MutexPtr);

    /// Wake one thread waiting on this condition variable.
    fn cond_signal(&self, ptr: CondPtr);

    /// Wake all threads waiting on this condition variable.
    fn cond_broadcast(&self, ptr: CondPtr);

    /// Destroy the condition variable.
    fn cond_destroy(&self, ptr: CondPtr);
}
