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

use crate::register_shims;
use crate::table::ShimTable;

mod shims;
use shims::*;

/// Install pthread-related shims.
pub struct PthreadShims;

impl PthreadShims {
    /// Register all pthread shims in the table.
    pub fn install(table: &mut ShimTable) {
        // ── Thread lifecycle ──
        register_shims!(table,
            ("_pthread_create", shim_pthread_create as *const () as u64, "pthread_create (real)"),
            ("_pthread_join", shim_pthread_join as *const () as u64, "pthread_join (real)"),
            ("_pthread_detach", shim_pthread_detach as *const () as u64, "pthread_detach (real)"),
            ("_pthread_exit", shim_pthread_exit as *const () as u64, "pthread_exit (real)"),
            ("_pthread_self", shim_pthread_self as *const () as u64, "pthread_self (real)"),
            ("_pthread_equal", shim_pthread_equal as *const () as u64, "pthread_equal (real)"),
            ("_pthread_cancel", shim_pthread_cancel as *const () as u64, "pthread_cancel (real)"),
        );

        // ── Thread attributes ──
        register_shims!(table,
            ("_pthread_attr_init", shim_pthread_attr_init as *const () as u64, "pthread_attr_init (real)"),
            ("_pthread_attr_destroy", shim_pthread_attr_destroy as *const () as u64, "pthread_attr_destroy (real)"),
            ("_pthread_attr_setdetachstate", shim_pthread_attr_setdetachstate as *const () as u64, "pthread_attr_setdetachstate (real)"),
            ("_pthread_attr_getdetachstate", shim_pthread_attr_getdetachstate as *const () as u64, "pthread_attr_getdetachstate (real)"),
            ("_pthread_attr_setstacksize", shim_pthread_attr_setstacksize as *const () as u64, "pthread_attr_setstacksize (real)"),
            ("_pthread_attr_getstacksize", shim_pthread_attr_getstacksize as *const () as u64, "pthread_attr_getstacksize (real)"),
        );

        // ── Mutexes ──
        register_shims!(table,
            ("_pthread_mutex_init", shim_pthread_mutex_init as *const () as u64, "pthread_mutex_init (real)"),
            ("_pthread_mutex_lock", shim_pthread_mutex_lock as *const () as u64, "pthread_mutex_lock (real)"),
            ("_pthread_mutex_trylock", shim_pthread_mutex_trylock as *const () as u64, "pthread_mutex_trylock (real)"),
            ("_pthread_mutex_unlock", shim_pthread_mutex_unlock as *const () as u64, "pthread_mutex_unlock (real)"),
            ("_pthread_mutex_destroy", shim_pthread_mutex_destroy as *const () as u64, "pthread_mutex_destroy (real)"),
        );

        // ── Mutex attributes ──
        register_shims!(table,
            ("_pthread_mutexattr_init", shim_pthread_mutexattr_init as *const () as u64, "pthread_mutexattr_init (real)"),
            ("_pthread_mutexattr_destroy", shim_pthread_mutexattr_destroy as *const () as u64, "pthread_mutexattr_destroy (real)"),
            ("_pthread_mutexattr_settype", shim_pthread_mutexattr_settype as *const () as u64, "pthread_mutexattr_settype (real)"),
        );

        // ── Read-write locks ──
        register_shims!(table,
            ("_pthread_rwlock_init", shim_pthread_rwlock_init as *const () as u64, "pthread_rwlock_init (real)"),
            ("_pthread_rwlock_rdlock", shim_pthread_rwlock_rdlock as *const () as u64, "pthread_rwlock_rdlock (real)"),
            ("_pthread_rwlock_wrlock", shim_pthread_rwlock_wrlock as *const () as u64, "pthread_rwlock_wrlock (real)"),
            ("_pthread_rwlock_tryrdlock", shim_pthread_rwlock_tryrdlock as *const () as u64, "pthread_rwlock_tryrdlock (real)"),
            ("_pthread_rwlock_trywrlock", shim_pthread_rwlock_trywrlock as *const () as u64, "pthread_rwlock_trywrlock (real)"),
            ("_pthread_rwlock_unlock", shim_pthread_rwlock_unlock as *const () as u64, "pthread_rwlock_unlock (real)"),
            ("_pthread_rwlock_destroy", shim_pthread_rwlock_destroy as *const () as u64, "pthread_rwlock_destroy (real)"),
        );

        // ── Condition variables ──
        register_shims!(table,
            ("_pthread_cond_init", shim_pthread_cond_init as *const () as u64, "pthread_cond_init (real)"),
            ("_pthread_cond_wait", shim_pthread_cond_wait as *const () as u64, "pthread_cond_wait (real)"),
            ("_pthread_cond_timedwait", shim_pthread_cond_timedwait as *const () as u64, "pthread_cond_timedwait (real)"),
            ("_pthread_cond_signal", shim_pthread_cond_signal as *const () as u64, "pthread_cond_signal (real)"),
            ("_pthread_cond_broadcast", shim_pthread_cond_broadcast as *const () as u64, "pthread_cond_broadcast (real)"),
            ("_pthread_cond_destroy", shim_pthread_cond_destroy as *const () as u64, "pthread_cond_destroy (real)"),
        );

        // ── Thread-specific data (TSD / pthread keys) ──
        register_shims!(table,
            ("_pthread_key_create", shim_pthread_key_create as *const () as u64, "pthread_key_create (real)"),
            ("_pthread_key_delete", shim_pthread_key_delete as *const () as u64, "pthread_key_delete (real)"),
            ("_pthread_getspecific", shim_pthread_getspecific as *const () as u64, "pthread_getspecific (real)"),
            ("_pthread_setspecific", shim_pthread_setspecific as *const () as u64, "pthread_setspecific (real)"),
        );

        // ── Once control ──
        register_shims!(table,
            ("_pthread_once", shim_pthread_once as *const () as u64, "pthread_once (real)"),
        );

        // ── Thread scheduling ──
        register_shims!(table,
            ("_sched_yield", shim_sched_yield as *const () as u64, "sched_yield (real)"),
            ("_pthread_yield_np", shim_sched_yield as *const () as u64, "pthread_yield_np (real)"),
        );

        // ── Main thread query ──
        register_shims!(table,
            ("_pthread_main_np", shim_pthread_main_np as *const () as u64, "pthread_main_np (real)"),
        );

        log::debug!("PthreadShims: registered 42 pthread shims");
    }
}

#[cfg(test)]
mod tests;
