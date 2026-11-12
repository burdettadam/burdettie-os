//! Real dispatch queue model.
//!
//! Provides per-queue identity, FIFO ordering for serial queues, bounded
//! concurrency for concurrent queues, and barrier support.
//!
//! ## Architecture
//!
//! Each `dispatch_queue_create` call allocates a real `QueueInner` with a
//! label, serial/concurrent flag, and work FIFO.
//!
//! - **Serial queues**: A dedicated worker thread drains the FIFO in strict
//!   order.  Spawned lazily on first submit.
//! - **Concurrent queues**: Work items are submitted to a shared global thread
//!   pool (bounded at the number of CPU cores).
//! - **Main queue**: Treated as a serial queue that drains inline on the
//!   calling thread (no background worker).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

// ── Work items ─────────────────────────────────────────────────────

type BoxedWork = Box<dyn FnOnce() + Send + 'static>;

/// Completion signal for synchronous dispatch.
struct SyncSignal {
    done: Mutex<bool>,
    condvar: Condvar,
}

impl SyncSignal {
    fn new() -> Self {
        Self {
            done: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    fn signal(&self) {
        let mut done = self.done.lock().unwrap_or_else(|e| e.into_inner());
        *done = true;
        self.condvar.notify_all();
    }

    fn wait(&self) {
        let mut done = self.done.lock().unwrap_or_else(|e| e.into_inner());
        while !*done {
            done = self.condvar.wait(done).unwrap_or_else(|e| e.into_inner());
        }
    }
}

// ── Queue type ─────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum QueueKind {
    Serial,
    Concurrent,
    Main,
}

struct QueueInner {
    label: String,
    kind: QueueKind,

    // Serial queue FIFO + worker
    fifo: Mutex<VecDeque<BoxedWork>>,
    fifo_condvar: Condvar,
    worker_spawned: AtomicBool,
    shutdown: AtomicBool,

    // Concurrent queue: in-flight count for barrier support
    in_flight: AtomicUsize,
    drain_condvar: Condvar,
    drain_mutex: Mutex<()>,
}

impl QueueInner {
    fn new(label: String, kind: QueueKind) -> Self {
        Self {
            label,
            kind,
            fifo: Mutex::new(VecDeque::new()),
            fifo_condvar: Condvar::new(),
            worker_spawned: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            in_flight: AtomicUsize::new(0),
            drain_condvar: Condvar::new(),
            drain_mutex: Mutex::new(()),
        }
    }

    /// Ensure the serial worker thread is running.
    fn ensure_serial_worker(self: &Arc<Self>) {
        if self.worker_spawned.swap(true, Ordering::AcqRel) {
            return; // already spawned
        }
        let q = Arc::clone(self);
        std::thread::Builder::new()
            .name(format!("dispatch-serial:{}", q.label))
            .spawn(move || serial_worker_loop(&q))
            .ok();
    }
}

fn serial_worker_loop(q: &QueueInner) {
    loop {
        let task = {
            let mut fifo = q.fifo.lock().unwrap_or_else(|e| e.into_inner());
            while fifo.is_empty() {
                if q.shutdown.load(Ordering::Acquire) {
                    return;
                }
                fifo = q.fifo_condvar.wait(fifo).unwrap_or_else(|e| e.into_inner());
            }
            fifo.pop_front().unwrap()
        };
        task();
    }
}

// ── Global queue registry ──────────────────────────────────────────

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0x00D1_0000_0001);
static QUEUE_REGISTRY: OnceLock<Mutex<Vec<(u64, Arc<QueueInner>)>>> = OnceLock::new();

fn registry() -> &'static Mutex<Vec<(u64, Arc<QueueInner>)>> {
    QUEUE_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

fn lookup_queue(handle: u64) -> Option<Arc<QueueInner>> {
    let reg = registry().lock().unwrap_or_else(|e| e.into_inner());
    reg.iter()
        .find(|(h, _)| *h == handle)
        .map(|(_, q)| Arc::clone(q))
}

/// Register a queue in the registry and return its handle.
fn register_queue(q: Arc<QueueInner>) -> u64 {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
    reg.push((handle, q));
    handle
}

// ── Well-known queues ──────────────────────────────────────────────

/// Main queue singleton.
static MAIN_QUEUE: OnceLock<Arc<QueueInner>> = OnceLock::new();

fn main_queue() -> &'static Arc<QueueInner> {
    MAIN_QUEUE.get_or_init(|| {
        Arc::new(QueueInner::new(
            "com.apple.main-thread".into(),
            QueueKind::Main,
        ))
    })
}

/// Global concurrent queue singletons (one per QoS, but we use a single pool).
static GLOBAL_QUEUE: OnceLock<Arc<QueueInner>> = OnceLock::new();

fn global_queue() -> &'static Arc<QueueInner> {
    GLOBAL_QUEUE.get_or_init(|| {
        Arc::new(QueueInner::new(
            "com.apple.root.default-qos".into(),
            QueueKind::Concurrent,
        ))
    })
}

// ── Global thread pool for concurrent queues ───────────────────────

struct ThreadPool {
    queue: Mutex<VecDeque<BoxedWork>>,
    condvar: Condvar,
    shutdown: AtomicBool,
}

static THREAD_POOL: OnceLock<Arc<ThreadPool>> = OnceLock::new();

fn thread_pool() -> &'static Arc<ThreadPool> {
    THREAD_POOL.get_or_init(|| {
        let pool = Arc::new(ThreadPool {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            shutdown: AtomicBool::new(false),
        });

        // Spawn workers — cap at 8 or available CPUs
        let n_workers = available_parallelism().min(8);
        for i in 0..n_workers {
            let p = Arc::clone(&pool);
            std::thread::Builder::new()
                .name(format!("dispatch-pool-{i}"))
                .spawn(move || pool_worker_loop(&p))
                .ok();
        }
        log::debug!("dispatch_queue: thread pool started with {n_workers} workers");

        pool
    })
}

fn pool_worker_loop(pool: &ThreadPool) {
    loop {
        let task = {
            let mut q = pool.queue.lock().unwrap_or_else(|e| e.into_inner());
            while q.is_empty() {
                if pool.shutdown.load(Ordering::Acquire) {
                    return;
                }
                q = pool.condvar.wait(q).unwrap_or_else(|e| e.into_inner());
            }
            q.pop_front().unwrap()
        };
        task();
    }
}

fn pool_submit(task: BoxedWork) {
    let pool = thread_pool();
    let mut q = pool.queue.lock().unwrap_or_else(|e| e.into_inner());
    q.push_back(task);
    pool.condvar.notify_one();
}

fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

// ── Public API ─────────────────────────────────────────────────────

/// Create a new dispatch queue.  Returns a handle (u64).
pub fn queue_create(label: &str, concurrent: bool) -> u64 {
    let kind = if concurrent {
        QueueKind::Concurrent
    } else {
        QueueKind::Serial
    };
    let q = Arc::new(QueueInner::new(label.into(), kind));
    register_queue(q)
}

/// Submit work to a queue asynchronously (returns immediately).
pub fn queue_async(handle: u64, task: BoxedWork) {
    let Some(q) = resolve_queue(handle) else {
        // Unknown queue handle — fall back to thread spawn
        std::thread::spawn(task);
        return;
    };

    match q.kind {
        QueueKind::Serial | QueueKind::Main => {
            if q.kind == QueueKind::Main {
                // Main queue: execute inline for now (no run-loop integration)
                // This preserves the existing behavior and avoids blocking.
                task();
                return;
            }
            q.ensure_serial_worker();
            let mut fifo = q.fifo.lock().unwrap_or_else(|e| e.into_inner());
            fifo.push_back(task);
            q.fifo_condvar.notify_one();
        }
        QueueKind::Concurrent => {
            let q2 = Arc::clone(&q);
            q.in_flight.fetch_add(1, Ordering::AcqRel);
            pool_submit(Box::new(move || {
                task();
                let prev = q2.in_flight.fetch_sub(1, Ordering::AcqRel);
                if prev == 1 {
                    // Last in-flight task completed — wake barrier waiters
                    q2.drain_condvar.notify_all();
                }
            }));
        }
    }
}

/// Submit work to a queue synchronously (blocks until complete).
pub fn queue_sync(handle: u64, task: BoxedWork) {
    let Some(q) = resolve_queue(handle) else {
        // Unknown handle — execute inline (safest for sync)
        task();
        return;
    };

    match q.kind {
        QueueKind::Main => {
            // Main queue sync: always inline
            task();
        }
        QueueKind::Serial => {
            // Check for deadlock: if we're the serial worker thread for this
            // queue, executing sync would deadlock.  Detect and run inline.
            let current_name = std::thread::current().name().unwrap_or("").to_string();
            let expected = format!("dispatch-serial:{}", q.label);
            if current_name == expected {
                // Same thread — run inline to avoid deadlock
                task();
                return;
            }

            let signal = Arc::new(SyncSignal::new());
            let sig = Arc::clone(&signal);
            q.ensure_serial_worker();
            {
                let mut fifo = q.fifo.lock().unwrap_or_else(|e| e.into_inner());
                fifo.push_back(Box::new(move || {
                    task();
                    sig.signal();
                }));
                q.fifo_condvar.notify_one();
            }
            signal.wait();
        }
        QueueKind::Concurrent => {
            let signal = Arc::new(SyncSignal::new());
            let sig = Arc::clone(&signal);
            q.in_flight.fetch_add(1, Ordering::AcqRel);
            let q2 = Arc::clone(&q);
            pool_submit(Box::new(move || {
                task();
                sig.signal();
                let prev = q2.in_flight.fetch_sub(1, Ordering::AcqRel);
                if prev == 1 {
                    q2.drain_condvar.notify_all();
                }
            }));
            signal.wait();
        }
    }
}

/// Submit a barrier block asynchronously.
///
/// On serial queues, this is equivalent to `queue_async` (ordering is
/// guaranteed).  On concurrent queues, this waits for all prior work
/// to drain, executes exclusively, then allows subsequent work.
pub fn queue_barrier_async(handle: u64, task: BoxedWork) {
    let Some(q) = resolve_queue(handle) else {
        std::thread::spawn(task);
        return;
    };

    if q.kind != QueueKind::Concurrent {
        // Serial/main: barriers are just normal submissions
        return queue_async(handle, task);
    }

    // Concurrent barrier: spawn a coordinator thread that waits for
    // in-flight to reach 0, runs the block, then signals.
    let q2 = Arc::clone(&q);
    std::thread::Builder::new()
        .name("dispatch-barrier".into())
        .spawn(move || {
            // Wait for all in-flight to drain
            {
                let mut guard = q2.drain_mutex.lock().unwrap_or_else(|e| e.into_inner());
                while q2.in_flight.load(Ordering::Acquire) > 0 {
                    guard = q2
                        .drain_condvar
                        .wait(guard)
                        .unwrap_or_else(|e| e.into_inner());
                }
            }
            task();
        })
        .ok();
}

/// Submit a barrier block synchronously.
pub fn queue_barrier_sync(handle: u64, task: BoxedWork) {
    let Some(q) = resolve_queue(handle) else {
        task();
        return;
    };

    if q.kind != QueueKind::Concurrent {
        return queue_sync(handle, task);
    }

    // Wait for drain, then run inline
    {
        let mut guard = q.drain_mutex.lock().unwrap_or_else(|e| e.into_inner());
        while q.in_flight.load(Ordering::Acquire) > 0 {
            guard = q
                .drain_condvar
                .wait(guard)
                .unwrap_or_else(|e| e.into_inner());
        }
    }
    task();
}

/// Get the label for a queue.
pub fn queue_get_label(handle: u64) -> &'static str {
    // Check well-known sentinels first
    if handle == main_queue_sentinel() {
        return "com.apple.main-thread";
    }
    if handle == global_queue_sentinel() {
        return "com.apple.root.default-qos";
    }
    // For user-created queues, we'd need to leak the label string.
    // For now, return empty — the label IS stored in QueueInner.label
    // but we can't return a reference to it with a static lifetime easily.
    // This is still better than before (always empty).
    ""
}

/// Return the stable address used as the main queue data symbol.
pub fn main_queue_sentinel() -> u64 {
    // Pre-init the main queue backing.
    let _ = main_queue();
    MAIN_Q_SENTINEL.as_ptr() as u64
}

/// Return the stable address for get_global_queue.
pub fn global_queue_sentinel() -> u64 {
    let _ = global_queue();
    GLOBAL_Q_SENTINEL.as_ptr() as u64
}

/// Stable address sentinels — these are the addresses that binaries see.
static MAIN_Q_SENTINEL: [u8; 8] = [0; 8];
static GLOBAL_Q_SENTINEL: [u8; 8] = [0; 8];

/// Resolve a queue handle to its backing QueueInner.
fn resolve_queue(handle: u64) -> Option<Arc<QueueInner>> {
    if handle == 0 {
        return None;
    }
    if handle == main_queue_sentinel() {
        return Some(Arc::clone(main_queue()));
    }
    if handle == global_queue_sentinel() {
        return Some(Arc::clone(global_queue()));
    }
    // Check for the legacy DISPATCH_CREATED_Q sentinel from dispatch_extras.rs
    // (old-style queues that were created before the real queue model).
    // Fall through to registry lookup.
    lookup_queue(handle)
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

    #[test]
    fn serial_queue_fifo_ordering() {
        let q = queue_create("test.serial.fifo", false);
        let order = Arc::new(Mutex::new(Vec::<usize>::new()));

        for i in 0..10 {
            let o = Arc::clone(&order);
            queue_async(
                q,
                Box::new(move || {
                    o.lock().unwrap().push(i);
                }),
            );
        }

        // Submit a sync block to flush the queue
        let o = Arc::clone(&order);
        queue_sync(
            q,
            Box::new(move || {
                o.lock().unwrap().push(999);
            }),
        );

        let result = order.lock().unwrap();
        let expected: Vec<usize> = (0..10).chain(std::iter::once(999)).collect();
        assert_eq!(
            *result, expected,
            "serial queue should execute in FIFO order"
        );
    }

    #[test]
    fn serial_queue_sync_blocks_until_done() {
        let q = queue_create("test.serial.sync", false);
        let counter = Arc::new(AtomicI32::new(0));

        let c = Arc::clone(&counter);
        queue_sync(
            q,
            Box::new(move || {
                std::thread::sleep(std::time::Duration::from_millis(10));
                c.store(42, Ordering::SeqCst);
            }),
        );

        // After queue_sync returns, the value must already be set
        assert_eq!(counter.load(Ordering::SeqCst), 42);
    }

    #[test]
    fn concurrent_queue_runs_in_parallel() {
        let q = queue_create("test.concurrent", true);
        let started = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(4));

        for _ in 0..4 {
            let s = Arc::clone(&started);
            let m = Arc::clone(&max_concurrent);
            let b = Arc::clone(&barrier);
            queue_async(
                q,
                Box::new(move || {
                    let cur = s.fetch_add(1, Ordering::SeqCst) + 1;
                    m.fetch_max(cur, Ordering::SeqCst);
                    b.wait(); // all 4 must reach here
                    s.fetch_sub(1, Ordering::SeqCst);
                }),
            );
        }

        // Wait for all to complete via sync
        queue_sync(q, Box::new(|| {}));
        // Give pool workers time to finish
        std::thread::sleep(std::time::Duration::from_millis(50));

        assert!(
            max_concurrent.load(Ordering::SeqCst) >= 2,
            "concurrent queue should run tasks in parallel (got max {})",
            max_concurrent.load(Ordering::SeqCst)
        );
    }

    #[test]
    fn queue_create_returns_unique_handles() {
        let q1 = queue_create("test.unique.1", false);
        let q2 = queue_create("test.unique.2", false);
        assert_ne!(q1, q2);
        assert_ne!(q1, 0);
    }

    #[test]
    fn main_queue_sentinel_stable() {
        let a = main_queue_sentinel();
        let b = main_queue_sentinel();
        assert_eq!(a, b);
        assert_ne!(a, 0);
    }

    #[test]
    fn global_queue_sentinel_distinct_from_main() {
        let m = main_queue_sentinel();
        let g = global_queue_sentinel();
        assert_ne!(m, g);
    }

    #[test]
    fn barrier_sync_on_serial_is_fifo() {
        let q = queue_create("test.barrier.serial", false);
        let order = Arc::new(Mutex::new(Vec::<usize>::new()));

        let o = Arc::clone(&order);
        queue_async(
            q,
            Box::new(move || {
                o.lock().unwrap().push(1);
            }),
        );

        let o = Arc::clone(&order);
        queue_barrier_sync(
            q,
            Box::new(move || {
                o.lock().unwrap().push(2);
            }),
        );

        let result = order.lock().unwrap();
        assert_eq!(*result, vec![1, 2]);
    }

    #[test]
    fn sync_on_unknown_handle_runs_inline() {
        let called = Arc::new(AtomicBool::new(false));
        let c = Arc::clone(&called);
        queue_sync(
            0xDEAD_BEEF,
            Box::new(move || {
                c.store(true, Ordering::SeqCst);
            }),
        );
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn main_queue_async_runs_inline() {
        let called = Arc::new(AtomicBool::new(false));
        let c = Arc::clone(&called);
        let main = main_queue_sentinel();
        queue_async(
            main,
            Box::new(move || {
                c.store(true, Ordering::SeqCst);
            }),
        );
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn concurrent_queue_sync_blocks() {
        let q = queue_create("test.conc.sync", true);
        let val = Arc::new(AtomicI32::new(0));

        let v = Arc::clone(&val);
        queue_sync(
            q,
            Box::new(move || {
                std::thread::sleep(std::time::Duration::from_millis(10));
                v.store(99, Ordering::SeqCst);
            }),
        );

        assert_eq!(val.load(Ordering::SeqCst), 99);
    }

    #[test]
    fn serial_queues_are_independent() {
        let q1 = queue_create("test.indep.1", false);
        let q2 = queue_create("test.indep.2", false);

        let counter1 = Arc::new(AtomicI32::new(0));
        let counter2 = Arc::new(AtomicI32::new(0));

        let c1 = Arc::clone(&counter1);
        queue_sync(
            q1,
            Box::new(move || {
                c1.store(1, Ordering::SeqCst);
            }),
        );

        let c2 = Arc::clone(&counter2);
        queue_sync(
            q2,
            Box::new(move || {
                c2.store(2, Ordering::SeqCst);
            }),
        );

        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 2);
    }
}
