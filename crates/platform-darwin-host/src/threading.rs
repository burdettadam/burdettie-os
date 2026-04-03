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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutex_size_positive() {
        let t = PosixThreading;
        assert!(t.mutex_size() > 0);
    }

    #[test]
    fn cond_size_positive() {
        let t = PosixThreading;
        assert!(t.cond_size() > 0);
    }

    #[test]
    fn mutex_lifecycle() {
        let t = PosixThreading;
        let mut buf = vec![0u8; t.mutex_size()];
        let ptr = buf.as_mut_ptr();
        t.mutex_init(ptr);
        t.mutex_lock(ptr);
        t.mutex_unlock(ptr);
        t.mutex_destroy(ptr);
    }

    #[test]
    fn recursive_mutex_double_lock() {
        let t = PosixThreading;
        let mut buf = vec![0u8; t.mutex_size()];
        let ptr = buf.as_mut_ptr();
        t.mutex_init_recursive(ptr);
        t.mutex_lock(ptr);
        t.mutex_lock(ptr); // second lock from same thread should succeed
        t.mutex_unlock(ptr);
        t.mutex_unlock(ptr);
        t.mutex_destroy(ptr);
    }

    #[test]
    fn mutex_trylock_unlocked() {
        let t = PosixThreading;
        let mut buf = vec![0u8; t.mutex_size()];
        let ptr = buf.as_mut_ptr();
        t.mutex_init(ptr);
        assert!(t.mutex_trylock(ptr));
        t.mutex_unlock(ptr);
        t.mutex_destroy(ptr);
    }

    #[test]
    fn cond_lifecycle() {
        let t = PosixThreading;
        let mut buf = vec![0u8; t.cond_size()];
        let ptr = buf.as_mut_ptr();
        t.cond_init(ptr);
        t.cond_destroy(ptr);
    }
}
