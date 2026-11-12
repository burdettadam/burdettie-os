//! Grand Central Dispatch (GCD) shims.
//!
//! Thread-safe implementations of `dispatch_once`, `dispatch_once_f`,
//! `dispatch_async`, and `dispatch_sync`.  The once-primitives use
//! atomic CAS for correctness under concurrent access.
//!
//! `dispatch_async` and `dispatch_sync` route through the real queue
//! model in `dispatch_queue` which provides:
//! - Serial queues with FIFO ordering (dedicated worker thread)
//! - Concurrent queues with a bounded thread pool
//! - Per-queue identity and label storage

use std::sync::atomic::{AtomicI64, Ordering};

use crate::darwin_stubs::blocks_runtime::{invoke_block0, OwnedBlock};
use crate::darwin_stubs::dispatch_queue;
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
/// The sentinel address is backed by a real queue object in dispatch_queue.
static DISPATCH_MAIN_Q: [u8; 8] = [0; 8];

/// Fake dispatch_queue_t for global queues.
///
/// `dispatch_get_global_queue(identifier, flags)` returns a global concurrent
/// queue.  The sentinel is backed by a real concurrent queue + thread pool.
static DISPATCH_GLOBAL_Q: [u8; 8] = [0; 8];

impl DispatchShims {
    /// Register dispatch shims in the table.
    pub fn install(table: &mut ShimTable) {
        table.register(
            "_dispatch_once",
            shim_dispatch_once as *const () as u64,
            "dispatch_once (thread-safe)",
        );
        table.register(
            "_dispatch_once_f",
            shim_dispatch_once_f as *const () as u64,
            "dispatch_once_f (thread-safe)",
        );
        table.register(
            "_dispatch_async",
            shim_dispatch_async as *const () as u64,
            "dispatch_async (threaded)",
        );
        table.register(
            "_dispatch_sync",
            shim_dispatch_sync as *const () as u64,
            "dispatch_sync (inline)",
        );
        table.register(
            "__dispatch_main_q",
            DISPATCH_MAIN_Q.as_ptr() as u64,
            "_dispatch_main_q data symbol",
        );
        table.register(
            "_dispatch_get_global_queue",
            shim_dispatch_get_global_queue as *const () as u64,
            "dispatch_get_global_queue (inline)",
        );

        log::debug!("DispatchShims: registered 6 dispatch shims");
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
/// Routes the block through the queue model:
/// - Serial queues: enqueued for FIFO execution on a dedicated worker thread
/// - Concurrent queues: submitted to the bounded thread pool
/// SAFETY: The block must remain valid for the duration of execution.
extern "C" fn shim_dispatch_async(queue: *const u8, block: *const u8) {
    let Some(block) = OwnedBlock::from_ptr(block) else {
        log::warn!("dispatch_async called with null block");
        return;
    };

    let queue_handle = queue as u64;

    dispatch_queue::queue_async(
        queue_handle,
        Box::new(move || {
            unsafe { invoke_block0(block.addr()) };
        }),
    );
}

/// `void dispatch_sync(dispatch_queue_t queue, dispatch_block_t block)`
///
/// Routes the block through the queue model synchronously (blocks until done).
extern "C" fn shim_dispatch_sync(queue: *const u8, block: *const u8) {
    let Some(block) = OwnedBlock::from_ptr(block) else {
        log::warn!("dispatch_sync called with null block");
        return;
    };

    let queue_handle = queue as u64;

    dispatch_queue::queue_sync(
        queue_handle,
        Box::new(move || {
            unsafe { invoke_block0(block.addr()) };
        }),
    );
}

/// `dispatch_queue_t dispatch_get_global_queue(long identifier, unsigned long flags)`
///
/// Returns the global concurrent queue, backed by a real bounded thread pool.
extern "C" fn shim_dispatch_get_global_queue(_identifier: i64, _flags: u64) -> *const u8 {
    DISPATCH_GLOBAL_Q.as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize};

    const BLOCK_HAS_COPY_DISPOSE_FLAG: i32 = 1 << 25;

    #[repr(C)]
    struct TestBlockDescriptor {
        reserved: u64,
        size: u64,
        copy: extern "C" fn(u64, u64),
        dispose: extern "C" fn(u64),
    }

    static TEST_BLOCK_INVOKED: AtomicBool = AtomicBool::new(false);
    static TEST_BLOCK_COPIED: AtomicUsize = AtomicUsize::new(0);
    static TEST_BLOCK_DISPOSED: AtomicUsize = AtomicUsize::new(0);

    extern "C" fn test_block_invoke(_block: *const u8) {
        TEST_BLOCK_INVOKED.store(true, Ordering::SeqCst);
    }

    extern "C" fn test_block_copy(_dst: u64, _src: u64) {
        TEST_BLOCK_COPIED.fetch_add(1, Ordering::SeqCst);
    }

    extern "C" fn test_block_dispose(_block: u64) {
        TEST_BLOCK_DISPOSED.fetch_add(1, Ordering::SeqCst);
    }

    static TEST_BLOCK_DESCRIPTOR: TestBlockDescriptor = TestBlockDescriptor {
        reserved: 0,
        size: 32,
        copy: test_block_copy,
        dispose: test_block_dispose,
    };

    fn make_copyable_block(buf: &mut [u8; 32]) {
        buf.fill(0);
        unsafe {
            std::ptr::write_unaligned(
                buf.as_mut_ptr().add(8) as *mut i32,
                BLOCK_HAS_COPY_DISPOSE_FLAG,
            );
            std::ptr::write_unaligned(
                buf.as_mut_ptr().add(16) as *mut extern "C" fn(*const u8),
                test_block_invoke,
            );
            std::ptr::write_unaligned(
                buf.as_mut_ptr().add(24) as *mut u64,
                &TEST_BLOCK_DESCRIPTOR as *const TestBlockDescriptor as u64,
            );
        }
    }

    #[test]
    fn dispatch_shims_registered() {
        let mut table = ShimTable::new();
        DispatchShims::install(&mut table);
        assert!(table.resolve("_dispatch_once").is_some());
        assert!(table.resolve("_dispatch_once_f").is_some());
        assert!(table.resolve("_dispatch_async").is_some());
        assert!(table.resolve("_dispatch_sync").is_some());
        assert!(table.resolve("__dispatch_main_q").is_some());
        assert!(table.resolve("_dispatch_get_global_queue").is_some());
        assert_eq!(table.len(), 6);
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
    fn dispatch_async_copies_and_releases_block() {
        TEST_BLOCK_INVOKED.store(false, Ordering::SeqCst);
        TEST_BLOCK_COPIED.store(0, Ordering::SeqCst);
        TEST_BLOCK_DISPOSED.store(0, Ordering::SeqCst);

        let queue = dispatch_queue::queue_create("test.dispatch.async.block", false);
        let mut block = [0u8; 32];
        make_copyable_block(&mut block);

        shim_dispatch_async(queue as *const u8, block.as_ptr());
        dispatch_queue::queue_sync(queue, Box::new(|| {}));

        assert!(TEST_BLOCK_INVOKED.load(Ordering::SeqCst));
        assert_eq!(TEST_BLOCK_COPIED.load(Ordering::SeqCst), 1);
        assert_eq!(TEST_BLOCK_DISPOSED.load(Ordering::SeqCst), 1);
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

    #[test]
    fn dispatch_get_global_queue_returns_non_null() {
        let ptr = shim_dispatch_get_global_queue(0, 0);
        assert!(!ptr.is_null());
    }

    #[test]
    fn dispatch_get_global_queue_stable_across_calls() {
        let p1 = shim_dispatch_get_global_queue(0, 0);
        let p2 = shim_dispatch_get_global_queue(0, 0);
        assert_eq!(p1, p2);
    }

    #[test]
    fn dispatch_global_queue_differs_from_main_queue() {
        let global = shim_dispatch_get_global_queue(0, 0);
        let main = DISPATCH_MAIN_Q.as_ptr();
        assert_ne!(global, main);
    }

    /// Helper: build a fake block struct on the heap with a given invoke function.
    ///
    /// Block layout: [isa: u64(8)] [flags: u32(4)] [reserved: u32(4)]
    /// [invoke: fn ptr(8)] [descriptor: u64(8)]
    fn make_fake_block(invoke: extern "C" fn(*const u8)) -> *const u8 {
        let block = vec![0u8; 32].into_boxed_slice();
        let ptr = Box::into_raw(block) as *mut u8;
        unsafe {
            // Write invoke pointer at offset 16
            *(ptr.add(16) as *mut usize) = invoke as usize;
        }
        ptr
    }

    #[test]
    fn dispatch_async_executes_block() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        static FLAG: AtomicBool = AtomicBool::new(false);
        FLAG.store(false, Ordering::SeqCst);

        extern "C" fn set_flag(_block: *const u8) {
            FLAG.store(true, Ordering::SeqCst);
        }

        let block = make_fake_block(set_flag);
        shim_dispatch_async(std::ptr::null(), block);

        // Wait for the spawned thread to finish (up to 1 second)
        for _ in 0..100 {
            if FLAG.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(
            FLAG.load(Ordering::SeqCst),
            "dispatch_async block should have run"
        );

        // Leak the block — it's a test and reclaiming is tricky with threads
    }

    #[test]
    fn dispatch_sync_executes_inline() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static FLAG: AtomicBool = AtomicBool::new(false);
        FLAG.store(false, Ordering::SeqCst);

        extern "C" fn set_flag(_block: *const u8) {
            FLAG.store(true, Ordering::SeqCst);
        }

        let block = make_fake_block(set_flag);
        shim_dispatch_sync(std::ptr::null(), block);

        // Should have run synchronously — flag is set immediately
        assert!(
            FLAG.load(Ordering::SeqCst),
            "dispatch_sync block should have run inline"
        );
    }
}
