//! Thread-local storage (TLS) shims.
//!
//! macOS Mach-O binaries use `__thread` / `_Thread_local` variables that
//! the dynamic linker manages via `_tlv_bootstrap` and `tlv_get_addr`.
//! Each thread gets its own copy of TLV data. We use `pthread_key_create`
//! to back per-thread storage, with a global descriptor-to-key mapping
//! protected by a mutex.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::table::ShimTable;

/// Manages TLS descriptor → allocation mapping.
pub struct TlsShims;

/// TLV descriptor layout matching `struct TLVDescriptor` in dyld/libpthread.
#[repr(C)]
pub struct TlvDescriptor {
    pub thunk: extern "C" fn(*mut TlvDescriptor) -> *mut u8,
    pub key: usize,
    pub offset: usize,
}

/// Registry mapping descriptor keys to pthread keys for per-thread storage.
static TLV_REGISTRY: Mutex<Option<TlvRegistry>> = Mutex::new(None);

struct TlvRegistry {
    /// Maps TLV descriptor key → (pthread_key, allocation size).
    keys: HashMap<usize, (libc::pthread_key_t, usize)>,
    next_key: usize,
}

impl TlvRegistry {
    fn new() -> Self {
        Self {
            keys: HashMap::new(),
            next_key: 1,
        }
    }

    fn alloc_key(&mut self) -> usize {
        let key = self.next_key;
        self.next_key += 1;
        key
    }
}

/// Destructor called when a thread exits — frees the TLV allocation.
unsafe extern "C" fn tlv_destructor(ptr: *mut libc::c_void) {
    if !ptr.is_null() {
        unsafe { libc::free(ptr) };
    }
}

impl TlsShims {
    /// Register TLS-related shims in the table.
    pub fn install(table: &mut ShimTable) {
        {
            let mut guard = TLV_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
            if guard.is_none() {
                *guard = Some(TlvRegistry::new());
            }
        }

        table.register(
            "__tlv_bootstrap",
            shim_tlv_bootstrap as u64,
            "TLV bootstrap thunk",
        );
        table.register("__tlv_atexit", shim_tlv_atexit as u64, "TLV atexit (no-op)");

        log::debug!("TlsShims: registered 2 TLS shims");
    }

    /// Reset all TLS state (useful between test runs).
    pub fn reset() {
        let mut guard = TLV_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(registry) = guard.as_mut() {
            // Delete all pthread keys
            for &(pkey, _) in registry.keys.values() {
                unsafe { libc::pthread_key_delete(pkey) };
            }
        }
        *guard = Some(TlvRegistry::new());
    }
}

/// `void *_tlv_bootstrap(TLVDescriptor *desc)` — returns a pointer to
/// the current thread's copy of the thread-local variable's storage.
///
/// On first call for a given descriptor on each thread, allocates zeroed
/// storage of `desc->offset` bytes. Subsequent calls on the same thread
/// return the same allocation.
extern "C" fn shim_tlv_bootstrap(desc: *mut TlvDescriptor) -> *mut u8 {
    if desc.is_null() {
        log::error!("_tlv_bootstrap called with null descriptor");
        return std::ptr::null_mut();
    }

    // SAFETY: desc is non-null, we assume valid layout from the Mach-O binary.
    let (key, size) = unsafe { ((*desc).key, (*desc).offset) };

    let mut guard = TLV_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    let registry = guard.as_mut().expect("TLV registry not initialised");

    // If key == 0, this descriptor hasn't been assigned a key yet.
    let actual_key = if key == 0 {
        let new_key = registry.alloc_key();
        // SAFETY: desc is non-null, writing the key back.
        unsafe { (*desc).key = new_key };
        new_key
    } else {
        key
    };

    let alloc_size = if size == 0 { 8 } else { size }; // minimum 8 bytes

    // Ensure we have a pthread key for this TLV descriptor
    let (pkey, _) = registry.keys.entry(actual_key).or_insert_with(|| {
        let mut pkey: libc::pthread_key_t = 0;
        unsafe {
            libc::pthread_key_create(&mut pkey, Some(tlv_destructor));
        }
        (pkey, alloc_size)
    });
    let pkey = *pkey;

    // Drop the registry lock before accessing per-thread storage
    drop(guard);

    // Check if this thread already has storage for this key
    let existing = unsafe { libc::pthread_getspecific(pkey) };
    if !existing.is_null() {
        return existing as *mut u8;
    }

    // Allocate per-thread storage
    let ptr = unsafe { libc::calloc(1, alloc_size) };
    if ptr.is_null() {
        log::error!("_tlv_bootstrap: calloc failed for size {}", alloc_size);
        return std::ptr::null_mut();
    }
    unsafe {
        libc::pthread_setspecific(pkey, ptr);
    }
    ptr as *mut u8
}

/// `void _tlv_atexit(void (*dtor)(void *), void *arg)` — register a
/// destructor for a TLV. Thread cleanup is handled by the pthread key
/// destructor, so this is a no-op.
extern "C" fn shim_tlv_atexit(_dtor: Option<extern "C" fn(*mut u8)>, _arg: *mut u8) {
    log::trace!("_tlv_atexit called (cleanup via pthread key destructor)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_shims_registered() {
        let mut table = ShimTable::new();
        TlsShims::install(&mut table);
        assert!(table.resolve("__tlv_bootstrap").is_some());
        assert!(table.resolve("__tlv_atexit").is_some());
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn tlv_bootstrap_allocates() {
        TlsShims::reset();
        let mut desc = TlvDescriptor {
            thunk: shim_tlv_bootstrap,
            key: 0,
            offset: 16,
        };
        let ptr = shim_tlv_bootstrap(&mut desc);
        assert!(!ptr.is_null());
        // Key should have been assigned
        assert_ne!(desc.key, 0);
    }

    #[test]
    fn tlv_bootstrap_returns_same_ptr_same_thread() {
        TlsShims::reset();
        let mut desc = TlvDescriptor {
            thunk: shim_tlv_bootstrap,
            key: 0,
            offset: 32,
        };
        let ptr1 = shim_tlv_bootstrap(&mut desc);
        let ptr2 = shim_tlv_bootstrap(&mut desc);
        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn tlv_bootstrap_different_threads_get_different_storage() {
        TlsShims::reset();
        // We need a shared descriptor — use a static-like approach
        // Allocate descriptor on heap so both threads can access it
        let desc = Box::into_raw(Box::new(TlvDescriptor {
            thunk: shim_tlv_bootstrap,
            key: 0,
            offset: 8,
        }));
        let desc_addr = desc as usize;

        // Get main thread's pointer
        let main_ptr = shim_tlv_bootstrap(desc) as usize;

        // Get child thread's pointer
        let child_ptr = std::thread::spawn(move || {
            let d = desc_addr as *mut TlvDescriptor;
            shim_tlv_bootstrap(d) as usize
        })
        .join()
        .unwrap();

        // Each thread should have its own storage
        assert_ne!(main_ptr, child_ptr);
        assert_ne!(main_ptr, 0);
        assert_ne!(child_ptr, 0);

        // Clean up
        unsafe { drop(Box::from_raw(desc)) };
    }

    #[test]
    fn tlv_bootstrap_null_desc() {
        let ptr = shim_tlv_bootstrap(std::ptr::null_mut());
        assert!(ptr.is_null());
    }
}
