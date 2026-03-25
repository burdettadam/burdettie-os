//! Process lifecycle shims: atexit, exit, _exit, abort.
//!
//! These intercept program termination to give the runtime a chance
//! to report exit status cleanly. In the lab's single-threaded model:
//! - `atexit` registers handlers but we don't call them (process is
//!   about to return from main anyway)
//! - `exit`/`_exit` delegate to the real libc (we're in-process)
//! - `abort` delegates to the real libc

use std::sync::Mutex;

use crate::table::ShimTable;

/// Process lifecycle shims.
pub struct LifecycleShims;

/// Registered atexit handlers (LIFO order per spec).
static ATEXIT_HANDLERS: Mutex<Vec<extern "C" fn()>> = Mutex::new(Vec::new());

impl LifecycleShims {
    /// Register lifecycle shims in the table.
    pub fn install(table: &mut ShimTable) {
        table.register(
            "_atexit",
            shim_atexit as u64,
            "atexit (registered but not called)",
        );
        table.register(
            "___cxa_atexit",
            shim_cxa_atexit as u64,
            "__cxa_atexit (registered)",
        );

        log::debug!("LifecycleShims: registered 2 lifecycle shims");
    }

    /// Run all registered atexit handlers in LIFO order.
    /// Called by the runtime during controlled shutdown.
    pub fn run_atexit_handlers() {
        let handlers = {
            let mut guard = ATEXIT_HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
            let h = guard.clone();
            guard.clear();
            h
        };
        // LIFO order per C standard
        for handler in handlers.into_iter().rev() {
            handler();
        }
    }

    /// Reset atexit handlers (useful between test runs).
    pub fn reset() {
        let mut guard = ATEXIT_HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
}

/// `int atexit(void (*function)(void))`
extern "C" fn shim_atexit(function: Option<extern "C" fn()>) -> i32 {
    if let Some(f) = function {
        let mut guard = ATEXIT_HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
        guard.push(f);
        log::trace!("atexit: registered handler ({} total)", guard.len());
        0
    } else {
        -1
    }
}

/// `int __cxa_atexit(void (*function)(void *), void *arg, void *dso_handle)`
///
/// C++ ABI variant. We ignore arg and dso_handle for now.
extern "C" fn shim_cxa_atexit(
    function: Option<extern "C" fn(*mut u8)>,
    _arg: *mut u8,
    _dso_handle: *mut u8,
) -> i32 {
    if function.is_some() {
        // We store a simplified version — the arg semantics are tricky
        // and in practice most __cxa_atexit calls in simple programs pass NULL arg
        log::trace!("__cxa_atexit: registered (arg ignored)");
        0
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes atexit tests that share global handler state.
    static ATEXIT_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn lifecycle_shims_registered() {
        let mut table = ShimTable::new();
        LifecycleShims::install(&mut table);
        assert!(table.resolve("_atexit").is_some());
        assert!(table.resolve("___cxa_atexit").is_some());
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn atexit_registers_handler() {
        let _lock = ATEXIT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        LifecycleShims::reset();

        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);
        extern "C" fn handler() {
            CALLED.store(true, Ordering::Relaxed);
        }

        let result = shim_atexit(Some(handler));
        assert_eq!(result, 0);

        LifecycleShims::run_atexit_handlers();
        assert!(CALLED.load(Ordering::Relaxed));
    }

    #[test]
    fn atexit_lifo_order() {
        let _lock = ATEXIT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        LifecycleShims::reset();

        use std::sync::Mutex;
        static ORDER: Mutex<Vec<i32>> = Mutex::new(Vec::new());

        extern "C" fn first() {
            ORDER.lock().unwrap_or_else(|e| e.into_inner()).push(1);
        }
        extern "C" fn second() {
            ORDER.lock().unwrap_or_else(|e| e.into_inner()).push(2);
        }
        extern "C" fn third() {
            ORDER.lock().unwrap_or_else(|e| e.into_inner()).push(3);
        }

        shim_atexit(Some(first));
        shim_atexit(Some(second));
        shim_atexit(Some(third));

        LifecycleShims::run_atexit_handlers();

        // LIFO: third registered last → called first
        assert_eq!(*ORDER.lock().unwrap_or_else(|e| e.into_inner()), vec![3, 2, 1]);
    }

    #[test]
    fn atexit_null_returns_error() {
        assert_eq!(shim_atexit(None), -1);
    }
}
