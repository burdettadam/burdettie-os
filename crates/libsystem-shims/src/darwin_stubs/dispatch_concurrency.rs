//! Real `dispatch_group` and `dispatch_semaphore` implementations for
//! cross-platform GCD concurrency primitives.
//!
//! On macOS these resolve via dlsym; on Linux (and in our shim layer)
//! we need working implementations.  Groups track outstanding work via
//! an atomic counter and wake waiters / fire notify blocks when it
//! reaches zero.  Semaphores are standard counting semaphores.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::table::ShimTable;

// ---------------------------------------------------------------------------
// Global registry
// ---------------------------------------------------------------------------

/// Opaque handle allocator — starts high to avoid collisions with real pointers.
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0x00DD_0000_0001_0000);

fn alloc_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

enum Primitive {
    Group(Arc<Group>),
    Semaphore(Arc<Semaphore>),
}

fn registry() -> &'static Mutex<HashMap<u64, Primitive>> {
    use std::sync::OnceLock;
    static R: OnceLock<Mutex<HashMap<u64, Primitive>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

// ---------------------------------------------------------------------------
// Group internals
// ---------------------------------------------------------------------------

struct Group {
    count: AtomicI64,
    notify: Mutex<Vec<(u64, u64)>>,
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl Group {
    fn new() -> Self {
        Self {
            count: AtomicI64::new(0),
            notify: Mutex::new(Vec::new()),
            condvar: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Semaphore internals
// ---------------------------------------------------------------------------

struct Semaphore {
    value: AtomicI64,
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl Semaphore {
    fn new(initial: i64) -> Self {
        Self {
            value: AtomicI64::new(initial),
            condvar: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Block invocation helper (same pattern as dispatch.rs)
// ---------------------------------------------------------------------------

/// Invoke a GCD block.  The block layout puts the invoke function pointer
/// at offset 16: `isa(8) + flags(4) + reserved(4) + invoke(8)`.
unsafe fn invoke_block(block: u64) {
    if block == 0 {
        return;
    }
    let block_ptr = block as *const u8;
    let invoke_ptr = unsafe { *(block_ptr.add(16) as *const usize) };
    let invoke: extern "C" fn(*const u8) = unsafe { std::mem::transmute(invoke_ptr) };
    invoke(block_ptr);
}

// ---------------------------------------------------------------------------
// dispatch_group shims
// ---------------------------------------------------------------------------

extern "C" fn shim_dispatch_group_create() -> u64 {
    let handle = alloc_handle();
    let group = Arc::new(Group::new());
    registry()
        .lock()
        .unwrap()
        .insert(handle, Primitive::Group(group));
    log::trace!("dispatch_group_create -> 0x{handle:x}");
    handle
}

extern "C" fn shim_dispatch_group_enter(group: u64) {
    let reg = registry().lock().unwrap();
    if let Some(Primitive::Group(g)) = reg.get(&group) {
        g.count.fetch_add(1, Ordering::AcqRel);
    }
}

extern "C" fn shim_dispatch_group_leave(group: u64) {
    let g = {
        let reg = registry().lock().unwrap();
        match reg.get(&group) {
            Some(Primitive::Group(g)) => Arc::clone(g),
            _ => return,
        }
    };

    let prev = g.count.fetch_sub(1, Ordering::AcqRel);
    if prev == 1 {
        // Count just reached 0 — wake waiters and fire notify blocks.
        g.condvar.notify_all();

        let pending: Vec<(u64, u64)> = {
            let mut notify = g.notify.lock().unwrap();
            std::mem::take(&mut *notify)
        };
        for (queue, block) in pending {
            let block_val = block;
            super::dispatch_queue::queue_async(queue, Box::new(move || {
                // SAFETY: caller guaranteed valid block layout.
                unsafe { invoke_block(block_val) };
            }));
        }
    }
}

extern "C" fn shim_dispatch_group_wait(group: u64, timeout: u64) -> i64 {
    let g = {
        let reg = registry().lock().unwrap();
        match reg.get(&group) {
            Some(Primitive::Group(g)) => Arc::clone(g),
            _ => return -1,
        }
    };

    // Fast path — already at zero.
    if g.count.load(Ordering::Acquire) == 0 {
        return 0;
    }

    let guard = g.mutex.lock().unwrap();

    // DISPATCH_TIME_FOREVER = u64::MAX = !0
    if timeout == !0u64 {
        let mut guard = guard;
        while g.count.load(Ordering::Acquire) != 0 {
            guard = g.condvar.wait(guard).unwrap();
        }
        0
    } else {
        // timeout is in nanoseconds (dispatch_time_t).
        let dur = Duration::from_nanos(timeout);
        let mut guard = guard;
        let start = std::time::Instant::now();
        while g.count.load(Ordering::Acquire) != 0 {
            let elapsed = start.elapsed();
            if elapsed >= dur {
                return 49; // non-zero = timeout
            }
            let remaining = dur - elapsed;
            let result;
            (guard, result) = g.condvar.wait_timeout(guard, remaining).unwrap();
            if result.timed_out() && g.count.load(Ordering::Acquire) != 0 {
                return 49;
            }
        }
        0
    }
}

extern "C" fn shim_dispatch_group_notify(group: u64, queue: u64, block: u64) {
    let g = {
        let reg = registry().lock().unwrap();
        match reg.get(&group) {
            Some(Primitive::Group(g)) => Arc::clone(g),
            _ => return,
        }
    };

    if g.count.load(Ordering::Acquire) == 0 {
        // Already done — fire immediately.
        super::dispatch_queue::queue_async(queue, Box::new(move || {
            // SAFETY: caller guaranteed valid block layout.
            unsafe { invoke_block(block) };
        }));
    } else {
        g.notify.lock().unwrap().push((queue, block));
    }
}

extern "C" fn shim_dispatch_group_async(group: u64, queue: u64, block: u64) {
    // Enter before submitting so the group count is incremented synchronously.
    shim_dispatch_group_enter(group);
    super::dispatch_queue::queue_async(queue, Box::new(move || {
        // SAFETY: caller guaranteed valid block layout.
        unsafe { invoke_block(block) };
        shim_dispatch_group_leave(group);
    }));
}

// ---------------------------------------------------------------------------
// dispatch_semaphore shims
// ---------------------------------------------------------------------------

extern "C" fn shim_dispatch_semaphore_create(value: i64) -> u64 {
    let handle = alloc_handle();
    let sema = Arc::new(Semaphore::new(value));
    registry()
        .lock()
        .unwrap()
        .insert(handle, Primitive::Semaphore(sema));
    log::trace!("dispatch_semaphore_create({value}) -> 0x{handle:x}");
    handle
}

extern "C" fn shim_dispatch_semaphore_wait(sema: u64, timeout: u64) -> i64 {
    let s = {
        let reg = registry().lock().unwrap();
        match reg.get(&sema) {
            Some(Primitive::Semaphore(s)) => Arc::clone(s),
            _ => return -1,
        }
    };

    // Decrement.  If the previous value was > 0 we acquired immediately.
    let prev = s.value.fetch_sub(1, Ordering::AcqRel);
    if prev > 0 {
        return 0;
    }

    // Need to wait for a signal.
    let guard = s.mutex.lock().unwrap();

    if timeout == !0u64 {
        let mut guard = guard;
        while s.value.load(Ordering::Acquire) < 0 {
            guard = s.condvar.wait(guard).unwrap();
        }
        0
    } else {
        let dur = Duration::from_nanos(timeout);
        let mut guard = guard;
        let start = std::time::Instant::now();
        while s.value.load(Ordering::Acquire) < 0 {
            let elapsed = start.elapsed();
            if elapsed >= dur {
                // Undo the decrement on timeout.
                s.value.fetch_add(1, Ordering::AcqRel);
                return 49;
            }
            let remaining = dur - elapsed;
            let result;
            (guard, result) = s.condvar.wait_timeout(guard, remaining).unwrap();
            if result.timed_out() && s.value.load(Ordering::Acquire) < 0 {
                s.value.fetch_add(1, Ordering::AcqRel);
                return 49;
            }
        }
        0
    }
}

extern "C" fn shim_dispatch_semaphore_signal(sema: u64) -> i64 {
    let s = {
        let reg = registry().lock().unwrap();
        match reg.get(&sema) {
            Some(Primitive::Semaphore(s)) => Arc::clone(s),
            _ => return 0,
        }
    };

    let prev = s.value.fetch_add(1, Ordering::AcqRel);
    if prev < 0 {
        // At least one waiter — wake one.
        s.condvar.notify_one();
    }
    prev
}

// ---------------------------------------------------------------------------
// Installation
// ---------------------------------------------------------------------------

pub struct DispatchConcurrencyStubs;

impl DispatchConcurrencyStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_dispatch_group_create", shim_dispatch_group_create as *const () as u64, "dispatch_group_create (real)"),
            ("_dispatch_group_enter", shim_dispatch_group_enter as *const () as u64, "dispatch_group_enter (real)"),
            ("_dispatch_group_leave", shim_dispatch_group_leave as *const () as u64, "dispatch_group_leave (real)"),
            ("_dispatch_group_wait", shim_dispatch_group_wait as *const () as u64, "dispatch_group_wait (real)"),
            ("_dispatch_group_notify", shim_dispatch_group_notify as *const () as u64, "dispatch_group_notify (real)"),
            ("_dispatch_group_async", shim_dispatch_group_async as *const () as u64, "dispatch_group_async (real)"),
            ("_dispatch_semaphore_create", shim_dispatch_semaphore_create as *const () as u64, "dispatch_semaphore_create (real)"),
            ("_dispatch_semaphore_wait", shim_dispatch_semaphore_wait as *const () as u64, "dispatch_semaphore_wait (real)"),
            ("_dispatch_semaphore_signal", shim_dispatch_semaphore_signal as *const () as u64, "dispatch_semaphore_signal (real)"),
        );
        log::debug!("DispatchConcurrencyStubs: registered 9 stubs");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_create_non_null() {
        let h = shim_dispatch_group_create();
        assert_ne!(h, 0);
    }

    #[test]
    fn test_group_enter_leave_wait() {
        let g = shim_dispatch_group_create();
        shim_dispatch_group_enter(g);
        shim_dispatch_group_leave(g);
        let rc = shim_dispatch_group_wait(g, !0u64);
        assert_eq!(rc, 0);
    }

    #[test]
    fn test_group_wait_already_done() {
        let g = shim_dispatch_group_create();
        // Fresh group has count=0, wait should return immediately.
        let rc = shim_dispatch_group_wait(g, !0u64);
        assert_eq!(rc, 0);
    }

    #[test]
    fn test_group_notify_immediate() {
        // When count is already 0, notify should fire the block immediately.
        // We test the counter logic without invoking real blocks.
        let g = shim_dispatch_group_create();
        let reg = registry().lock().unwrap();
        if let Some(Primitive::Group(group)) = reg.get(&g) {
            assert_eq!(group.count.load(Ordering::Acquire), 0);
            // In real usage this would invoke the block; here we verify
            // the count condition that triggers immediate execution.
        } else {
            panic!("expected Group in registry");
        }
    }

    #[test]
    fn test_group_enter_leave_counter() {
        let g = shim_dispatch_group_create();
        shim_dispatch_group_enter(g);
        shim_dispatch_group_enter(g);

        let reg = registry().lock().unwrap();
        if let Some(Primitive::Group(group)) = reg.get(&g) {
            assert_eq!(group.count.load(Ordering::Acquire), 2);
        }
        drop(reg);

        shim_dispatch_group_leave(g);
        shim_dispatch_group_leave(g);

        let reg = registry().lock().unwrap();
        if let Some(Primitive::Group(group)) = reg.get(&g) {
            assert_eq!(group.count.load(Ordering::Acquire), 0);
        }
    }

    #[test]
    fn test_semaphore_create_non_null() {
        let h = shim_dispatch_semaphore_create(1);
        assert_ne!(h, 0);
    }

    #[test]
    fn test_semaphore_signal_wait() {
        let s = shim_dispatch_semaphore_create(0);
        // Signal first, then wait should succeed immediately.
        shim_dispatch_semaphore_signal(s);
        let rc = shim_dispatch_semaphore_wait(s, !0u64);
        assert_eq!(rc, 0);
    }

    #[test]
    fn test_semaphore_initial_value() {
        let s = shim_dispatch_semaphore_create(2);
        // Two waits should succeed (initial value = 2).
        assert_eq!(shim_dispatch_semaphore_wait(s, !0u64), 0);
        assert_eq!(shim_dispatch_semaphore_wait(s, !0u64), 0);
        // Third wait would block — verify counter went negative.
        let reg = registry().lock().unwrap();
        if let Some(Primitive::Semaphore(sema)) = reg.get(&s) {
            assert_eq!(sema.value.load(Ordering::Acquire), 0);
        }
    }

    #[test]
    fn test_semaphore_signal_returns_previous() {
        let s = shim_dispatch_semaphore_create(0);
        // Signal from 0 → prev should be 0 (no waiters).
        let prev = shim_dispatch_semaphore_signal(s);
        assert_eq!(prev, 0);
        // Signal again from 1 → prev should be 1.
        let prev = shim_dispatch_semaphore_signal(s);
        assert_eq!(prev, 1);
    }

    #[test]
    fn test_group_concurrent_enter_leave() {
        let g = shim_dispatch_group_create();
        let n = 50;
        for _ in 0..n {
            shim_dispatch_group_enter(g);
        }

        let handles: Vec<_> = (0..n)
            .map(|_| {
                let gv = g;
                std::thread::spawn(move || shim_dispatch_group_leave(gv))
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(shim_dispatch_group_wait(g, !0u64), 0);
    }

    #[test]
    fn test_semaphore_concurrent_signal_wait() {
        let s = shim_dispatch_semaphore_create(0);
        let n = 20;

        // Spawn signal threads.
        let handles: Vec<_> = (0..n)
            .map(|_| {
                let sv = s;
                std::thread::spawn(move || {
                    shim_dispatch_semaphore_signal(sv);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        // All signals done — n waits should succeed.
        for _ in 0..n {
            assert_eq!(shim_dispatch_semaphore_wait(s, !0u64), 0);
        }
    }
}
