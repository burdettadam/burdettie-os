//! POSIX threading primitives for macOS.

use platform_core::threading::{CondPtr, MutexPtr, Threading};

pub struct PosixThreading;

impl Threading for PosixThreading {
    fn mutex_size(&self) -> usize {
        std::mem::size_of::<libc::pthread_mutex_t>()
    }

    fn cond_size(&self) -> usize {
        std::mem::size_of::<libc::pthread_cond_t>()
    }

    fn mutex_init(&self, ptr: MutexPtr) {
        unsafe {
            libc::pthread_mutex_init(ptr as *mut libc::pthread_mutex_t, std::ptr::null());
        }
    }

    fn mutex_init_recursive(&self, ptr: MutexPtr) {
        unsafe {
            let mut attr: libc::pthread_mutexattr_t = std::mem::zeroed();
            libc::pthread_mutexattr_init(&mut attr);
            libc::pthread_mutexattr_settype(&mut attr, libc::PTHREAD_MUTEX_RECURSIVE);
            libc::pthread_mutex_init(ptr as *mut libc::pthread_mutex_t, &attr);
            libc::pthread_mutexattr_destroy(&mut attr);
        }
    }

    fn mutex_lock(&self, ptr: MutexPtr) {
        unsafe {
            libc::pthread_mutex_lock(ptr as *mut libc::pthread_mutex_t);
        }
    }

    fn mutex_unlock(&self, ptr: MutexPtr) {
        unsafe {
            libc::pthread_mutex_unlock(ptr as *mut libc::pthread_mutex_t);
        }
    }

    fn mutex_trylock(&self, ptr: MutexPtr) -> bool {
        unsafe { libc::pthread_mutex_trylock(ptr as *mut libc::pthread_mutex_t) == 0 }
    }

    fn mutex_destroy(&self, ptr: MutexPtr) {
        unsafe {
            libc::pthread_mutex_destroy(ptr as *mut libc::pthread_mutex_t);
        }
    }

    fn cond_init(&self, ptr: CondPtr) {
        unsafe {
            libc::pthread_cond_init(ptr as *mut libc::pthread_cond_t, std::ptr::null());
        }
    }

    fn cond_wait(&self, cond: CondPtr, mutex: MutexPtr) {
        unsafe {
            libc::pthread_cond_wait(
                cond as *mut libc::pthread_cond_t,
                mutex as *mut libc::pthread_mutex_t,
            );
        }
    }

    fn cond_signal(&self, ptr: CondPtr) {
        unsafe {
            libc::pthread_cond_signal(ptr as *mut libc::pthread_cond_t);
        }
    }

    fn cond_broadcast(&self, ptr: CondPtr) {
        unsafe {
            libc::pthread_cond_broadcast(ptr as *mut libc::pthread_cond_t);
        }
    }

    fn cond_destroy(&self, ptr: CondPtr) {
        unsafe {
            libc::pthread_cond_destroy(ptr as *mut libc::pthread_cond_t);
        }
    }
}
