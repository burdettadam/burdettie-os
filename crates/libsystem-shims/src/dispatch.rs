//! Grand Central Dispatch (GCD) shims.
//!
//! Thread-safe implementations of `dispatch_once`, `dispatch_once_f`,
//! `dispatch_async`, and `dispatch_sync`. The once-primitives use
//! atomic CAS for correctness under concurrent access. Dispatch
//! async/sync currently execute blocks inline (suitable for programs
//! that use GCD purely for initialization patterns). Full concurrent
//! queue dispatch can be added in a future phase.

use std::sync::atomic::{AtomicI64, Ordering};

use crate::table::ShimTable;

/// Installs dispatch-related shims.
pub struct DispatchShims;

/// The type that `dispatch_once` uses as the predicate.
/// On macOS this is a `long` (i64 on 64-bit).
pub type DispatchOnceT = i64;

/// The sentinel value indicating the once block has completed.
const DISPATCH_ONCE_DONE: i64 = !0; // -1, same as macOS

/// Intermediate gate value while the block is executing.
const DISPATCH_ONCE_GATE: i64 = 1;

/// Fake dispatch_queue_t for `_dispatch_main_q`.
///
/// `dispatch_get_main_queue()` is a macro expanding to `&_dispatch_main_q`.
/// We only need a stable non-null address — queue identity is ignored by our
/// inline dispatch_sync/dispatch_async shims.
static DISPATCH_MAIN_Q: [u8; 8] = [0; 8];

impl DispatchShims {
    /// Register dispatch shims in the table.
    pub fn install(table: &mut ShimTable) {
        table.register(
            "_dispatch_once",
            shim_dispatch_once as u64,
            "dispatch_once (thread-safe)",
        );
        table.register(
            "_dispatch_once_f",
            shim_dispatch_once_f as u64,
            "dispatch_once_f (thread-safe)",
        );
        table.register(
            "_dispatch_async",
            shim_dispatch_async as u64,
            "dispatch_async (inline)",
        );
        table.register(
            "_dispatch_sync",
            shim_dispatch_sync as u64,
            "dispatch_sync (inline)",
        );
        table.register(
            "__dispatch_main_q",
            DISPATCH_MAIN_Q.as_ptr() as u64,
            "_dispatch_main_q data symbol",
        );

        log::debug!("DispatchShims: registered 5 dispatch shims");
    }
}

/// `void dispatch_once(dispatch_once_t *predicate, dispatch_block_t block)`
///
/// Thread-safe implementation using atomic CAS:
/// - Fast path: if predicate == DONE, return immediately
/// - Slow path: CAS from 0 → GATE, execute block, set DONE
/// - Contention: spin-wait until DONE
extern "C" fn shim_dispatch_once(predicate: *mut DispatchOnceT, block: *const u8) {
    if predicate.is_null() {
        log::error!("dispatch_once called with null predicate");
        return;
    }

    // SAFETY: predicate is non-null, we treat it as an atomic i64.
    // This is safe because dispatch_once_t is naturally aligned.
    let pred = unsafe { &*(predicate as *const AtomicI64) };

    // Fast path — already done
    if pred.load(Ordering::Acquire) == DISPATCH_ONCE_DONE {
        return;
    }

    // Slow path — try to become the executor
    match pred.compare_exchange(0, DISPATCH_ONCE_GATE, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {
            // We won the race — execute the block
            if !block.is_null() {
                // The block's invoke pointer is at offset 16 in Block_literal:
                // Layout: isa(8) + flags(4) + reserved(4) + invoke(8)
                // SAFETY: We trust the Mach-O binary provides a valid block layout.
                unsafe {
                    let invoke_ptr = *(block.add(16) as *const extern "C" fn(*const u8));
                    invoke_ptr(block);
                }
            }
            // Mark as done — release so other threads see the block's side effects
            pred.store(DISPATCH_ONCE_DONE, Ordering::Release);
        }
        Err(current) => {
            if current == DISPATCH_ONCE_DONE {
                return; // Another thread finished between our first check and CAS
            }
            // Another thread is executing — spin-wait until done
            while pred.load(Ordering::Acquire) != DISPATCH_ONCE_DONE {
                std::hint::spin_loop();
            }
        }
    }
}

/// `void dispatch_once_f(dispatch_once_t *predicate, void *context, void (*function)(void *))`
///
/// Function-pointer variant of dispatch_once. Thread-safe via atomic CAS.
extern "C" fn shim_dispatch_once_f(
    predicate: *mut DispatchOnceT,
    context: *mut u8,
    function: Option<extern "C" fn(*mut u8)>,
) {
    if predicate.is_null() {
        log::error!("dispatch_once_f called with null predicate");
        return;
    }

    let pred = unsafe { &*(predicate as *const AtomicI64) };

    if pred.load(Ordering::Acquire) == DISPATCH_ONCE_DONE {
        return;
    }

    match pred.compare_exchange(0, DISPATCH_ONCE_GATE, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {
            if let Some(f) = function {
                f(context);
            }
            pred.store(DISPATCH_ONCE_DONE, Ordering::Release);
        }
        Err(current) => {
            if current == DISPATCH_ONCE_DONE {
                return;
            }
            while pred.load(Ordering::Acquire) != DISPATCH_ONCE_DONE {
                std::hint::spin_loop();
            }
        }
    }
}

/// `void dispatch_async(dispatch_queue_t queue, dispatch_block_t block)`
///
/// Executes the block inline. For programs that dispatch work for
/// initialization or simple sequencing, this is semantically correct.
extern "C" fn shim_dispatch_async(_queue: *const u8, block: *const u8) {
    if block.is_null() {
        log::warn!("dispatch_async called with null block");
        return;
    }

    // Execute block inline (same invoke convention as dispatch_once).
    // SAFETY: block is non-null, we trust Mach-O binary block layout.
    unsafe {
        let invoke_ptr = *(block.add(16) as *const extern "C" fn(*const u8));
        invoke_ptr(block);
    }
}

/// `void dispatch_sync(dispatch_queue_t queue, dispatch_block_t block)`
///
/// Executes the block inline (identical to dispatch_async for now).
extern "C" fn shim_dispatch_sync(_queue: *const u8, block: *const u8) {
    if block.is_null() {
        log::warn!("dispatch_sync called with null block");
        return;
    }

    unsafe {
        let invoke_ptr = *(block.add(16) as *const extern "C" fn(*const u8));
        invoke_ptr(block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_shims_registered() {
        let mut table = ShimTable::new();
        DispatchShims::install(&mut table);
        assert!(table.resolve("_dispatch_once").is_some());
        assert!(table.resolve("_dispatch_once_f").is_some());
        assert!(table.resolve("_dispatch_async").is_some());
        assert!(table.resolve("_dispatch_sync").is_some());
        assert!(table.resolve("__dispatch_main_q").is_some());
        assert_eq!(table.len(), 5);
    }

    #[test]
    fn dispatch_once_f_runs_once() {
        let mut predicate: DispatchOnceT = 0;
        let mut counter: u64 = 0;

        extern "C" fn increment(ctx: *mut u8) {
            // SAFETY: ctx points to our counter.
            unsafe { *(ctx as *mut u64) += 1 };
        }

        // First call — should run
        shim_dispatch_once_f(
            &mut predicate,
            &mut counter as *mut u64 as *mut u8,
            Some(increment),
        );
        assert_eq!(counter, 1);
        assert_eq!(predicate, DISPATCH_ONCE_DONE);

        // Second call — should not run
        shim_dispatch_once_f(
            &mut predicate,
            &mut counter as *mut u64 as *mut u8,
            Some(increment),
        );
        assert_eq!(counter, 1);
    }

    #[test]
    fn dispatch_once_f_thread_safe() {
        use std::sync::atomic::{AtomicI32, Ordering};
        use std::sync::Arc;

        // Heap-allocate the predicate for cross-thread sharing
        let predicate = Arc::new(AtomicI64::new(0));
        static COUNTER: AtomicI32 = AtomicI32::new(0);
        COUNTER.store(0, Ordering::SeqCst);

        extern "C" fn increment(_ctx: *mut u8) {
            COUNTER.fetch_add(1, Ordering::Relaxed);
        }

        let mut threads = Vec::new();
        for _ in 0..8 {
            let pred = predicate.clone();
            threads.push(std::thread::spawn(move || {
                let ptr = pred.as_ptr() as *mut DispatchOnceT;
                shim_dispatch_once_f(ptr, std::ptr::null_mut(), Some(increment));
            }));
        }
        for t in threads {
            t.join().unwrap();
        }

        // The function should have been called exactly once
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn dispatch_once_f_null_predicate() {
        // Should not panic
        extern "C" fn noop(_: *mut u8) {}
        shim_dispatch_once_f(std::ptr::null_mut(), std::ptr::null_mut(), Some(noop));
    }

    #[test]
    fn dispatch_once_f_null_function() {
        let mut predicate: DispatchOnceT = 0;
        shim_dispatch_once_f(&mut predicate, std::ptr::null_mut(), None);
        assert_eq!(predicate, DISPATCH_ONCE_DONE);
    }
}
