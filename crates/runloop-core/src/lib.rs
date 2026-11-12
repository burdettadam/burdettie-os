//! Event loop and timer infrastructure.
//!
//! Provides a `RunLoop` that can schedule timers, enqueue closures, and
//! drive the application event cycle. Designed to work headlessly (no
//! window backend required) for testing.

use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ── Timer token ────────────────────────────────────────────────────

/// Opaque identifier for a scheduled timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerToken(u64);

/// Alias used by the public `Timer` API.
pub type TimerId = TimerToken;

// ── Public Timer ───────────────────────────────────────────────────

/// A standalone timer that can be fired manually or scheduled on a `RunLoop`.
pub struct Timer {
    pub interval: Duration,
    pub repeats: bool,
    pub callback: Box<dyn FnMut() + Send>,
    pub next_fire: Instant,
    pub valid: bool,
}

impl Timer {
    /// Create a new timer that will first fire after `interval`.
    pub fn new(interval: Duration, repeats: bool, callback: impl FnMut() + Send + 'static) -> Self {
        Self {
            interval,
            repeats,
            callback: Box::new(callback),
            next_fire: Instant::now() + interval,
            valid: true,
        }
    }

    /// Mark the timer as invalid; it will never fire again.
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Whether the timer is still valid.
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Fire the timer's callback immediately.
    ///
    /// For repeating timers, `next_fire` is updated.
    /// For non-repeating timers, the timer is auto-invalidated.
    pub fn fire(&mut self) {
        if !self.valid {
            return;
        }
        (self.callback)();
        if self.repeats {
            self.next_fire = Instant::now() + self.interval;
        } else {
            self.valid = false;
        }
    }
}

// ── Timer entry (internal) ─────────────────────────────────────────

struct TimerEntry {
    token: TimerToken,
    fire_at: Instant,
    interval: Duration,
    repeats: bool,
    callback: Box<dyn FnMut() + Send>,
    cancelled: bool,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.fire_at == other.fire_at
    }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering so BinaryHeap gives us the earliest fire time first
        other.fire_at.cmp(&self.fire_at)
    }
}

// ── Pending block ──────────────────────────────────────────────────

type PendingBlock = Box<dyn FnOnce() + Send>;

// ── RunLoop ────────────────────────────────────────────────────────

/// The main event loop.
///
/// Supports timers, deferred block execution, and stop/resume.
/// The run loop processes work in order:
/// 1. Fire any pending timers whose fire time has passed
/// 2. Execute any enqueued blocks
/// 3. Sleep until the next timer or a short poll interval
pub struct RunLoop {
    running: AtomicBool,
    next_token: AtomicU64,
    timers: Mutex<BinaryHeap<TimerEntry>>,
    pending: Mutex<Vec<PendingBlock>>,
    /// Cancelled timer tokens (cheaply checked during fire).
    cancelled: Mutex<Vec<TimerToken>>,
}

impl RunLoop {
    /// Create a new run loop.
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            next_token: AtomicU64::new(1),
            timers: Mutex::new(BinaryHeap::new()),
            pending: Mutex::new(Vec::new()),
            cancelled: Mutex::new(Vec::new()),
        }
    }

    /// Run the loop, blocking until `stop()` is called.
    pub fn run(&self) {
        self.running.store(true, Ordering::SeqCst);

        while self.running.load(Ordering::SeqCst) {
            self.run_once();

            // Sleep until next timer or a short poll interval
            let sleep_dur = {
                let timers = self.timers.lock().unwrap_or_else(|e| e.into_inner());
                timers
                    .peek()
                    .map(|t| {
                        let now = Instant::now();
                        if t.fire_at > now {
                            t.fire_at - now
                        } else {
                            Duration::ZERO
                        }
                    })
                    .unwrap_or(Duration::from_millis(10))
            };

            if sleep_dur > Duration::ZERO {
                std::thread::sleep(sleep_dur.min(Duration::from_millis(10)));
            }
        }
    }

    /// Process one cycle: fire timers + run pending blocks.
    /// Returns the number of items processed.
    pub fn run_once(&self) -> usize {
        let mut processed = 0;
        processed += self.fire_timers();
        processed += self.drain_pending();
        processed
    }

    /// Stop the run loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Whether the run loop is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Enqueue a block to execute on the next cycle.
    pub fn perform(&self, block: impl FnOnce() + Send + 'static) {
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Box::new(block));
    }

    /// Schedule a timer.
    pub fn schedule_timer(
        &self,
        interval: Duration,
        repeats: bool,
        callback: impl FnMut() + Send + 'static,
    ) -> TimerToken {
        let token = TimerToken(self.next_token.fetch_add(1, Ordering::SeqCst));
        let entry = TimerEntry {
            token,
            fire_at: Instant::now() + interval,
            interval,
            repeats,
            callback: Box::new(callback),
            cancelled: false,
        };
        self.timers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(entry);
        token
    }

    /// Cancel a previously scheduled timer.
    pub fn cancel_timer(&self, token: TimerToken) {
        self.cancelled
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(token);
    }

    /// Schedule a `Timer` on this run loop, returning a `TimerId`.
    pub fn add_timer(&self, timer: Timer) -> TimerId {
        let token = TimerToken(self.next_token.fetch_add(1, Ordering::SeqCst));
        let entry = TimerEntry {
            token,
            fire_at: timer.next_fire,
            interval: timer.interval,
            repeats: timer.repeats,
            callback: timer.callback,
            cancelled: !timer.valid,
        };
        self.timers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(entry);
        token
    }

    /// Cancel a timer by its `TimerId`.
    pub fn invalidate_timer(&self, id: TimerId) {
        self.cancel_timer(id);
    }

    /// Fire all expired timers. Returns the number of timers fired.
    pub fn process_timers(&self) -> usize {
        self.fire_timers()
    }

    // ── Internal ───────────────────────────────────────────────

    fn fire_timers(&self) -> usize {
        let now = Instant::now();
        let cancelled = {
            let mut c = self.cancelled.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *c)
        };

        let mut fired = 0;
        let mut requeue = Vec::new();

        let mut timers = self.timers.lock().unwrap_or_else(|e| e.into_inner());

        // Drain all timers that should fire
        while let Some(entry) = timers.peek() {
            if entry.fire_at > now {
                break;
            }
            let mut entry = timers.pop().unwrap();

            if entry.cancelled || cancelled.contains(&entry.token) {
                continue;
            }

            (entry.callback)();
            fired += 1;

            if entry.repeats {
                entry.fire_at = now + entry.interval;
                requeue.push(entry);
            }
        }

        for entry in requeue {
            timers.push(entry);
        }

        fired
    }

    fn drain_pending(&self) -> usize {
        let blocks: Vec<PendingBlock> = {
            let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *pending)
        };
        let count = blocks.len();
        for block in blocks {
            block();
        }
        count
    }
}

impl Default for RunLoop {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;
    use std::sync::Arc;

    #[test]
    fn runloop_perform_block() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));
        let c = counter.clone();
        rl.perform(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let processed = rl.run_once();
        assert_eq!(processed, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn runloop_timer_fires() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));
        let c = counter.clone();
        rl.schedule_timer(Duration::from_millis(1), false, move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        // Wait for timer to be ready
        std::thread::sleep(Duration::from_millis(5));
        rl.run_once();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Non-repeating: should not fire again
        std::thread::sleep(Duration::from_millis(5));
        rl.run_once();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn runloop_repeating_timer() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));
        let c = counter.clone();
        rl.schedule_timer(Duration::from_millis(1), true, move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        // Fire a few times
        for _ in 0..3 {
            std::thread::sleep(Duration::from_millis(5));
            rl.run_once();
        }
        assert!(counter.load(Ordering::SeqCst) >= 3);
    }

    #[test]
    fn runloop_cancel_timer() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));
        let c = counter.clone();
        let token = rl.schedule_timer(Duration::from_millis(1), true, move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        std::thread::sleep(Duration::from_millis(5));
        rl.run_once();
        assert!(counter.load(Ordering::SeqCst) >= 1);

        rl.cancel_timer(token);
        let before = counter.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(10));
        rl.run_once();
        assert_eq!(counter.load(Ordering::SeqCst), before);
    }

    #[test]
    fn runloop_stop() {
        let rl = Arc::new(RunLoop::new());
        let rl2 = rl.clone();

        // Schedule a stop after a short delay
        rl.schedule_timer(Duration::from_millis(10), false, move || {
            rl2.stop();
        });

        // This should return once stop() is called
        rl.run();
        assert!(!rl.is_running());
    }

    #[test]
    fn runloop_multiple_blocks() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));

        for _ in 0..5 {
            let c = counter.clone();
            rl.perform(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        rl.run_once();
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    // ── TimerToken tests ───────────────────────────────────────

    #[test]
    fn timer_token_equality() {
        let a = TimerToken(42);
        let b = TimerToken(42);
        assert_eq!(a, b);
    }

    #[test]
    fn timer_token_inequality() {
        let a = TimerToken(1);
        let b = TimerToken(2);
        assert_ne!(a, b);
    }

    #[test]
    fn timer_token_debug() {
        let token = TimerToken(99);
        let debug = format!("{:?}", token);
        assert!(debug.contains("99"), "Debug output was: {debug}");
    }

    #[test]
    fn timer_token_hash() {
        use std::collections::HashMap;
        let token = TimerToken(7);
        let mut map = HashMap::new();
        map.insert(token, "hello");
        assert_eq!(map.get(&TimerToken(7)), Some(&"hello"));
    }

    #[test]
    fn timer_token_copy() {
        let a = TimerToken(10);
        let b = a; // Copy
        let c = a; // still valid — not moved
        assert_eq!(b, c);
    }

    // ── RunLoop construction / state ───────────────────────────

    #[test]
    fn runloop_default_trait() {
        let rl = RunLoop::default();
        assert!(!rl.is_running());
    }

    #[test]
    fn runloop_initial_state() {
        let rl = RunLoop::new();
        assert!(!rl.is_running());
    }

    #[test]
    fn runloop_run_once_empty() {
        let rl = RunLoop::new();
        assert_eq!(rl.run_once(), 0);
    }

    #[test]
    fn runloop_is_running_during_run() {
        let rl = Arc::new(RunLoop::new());
        let was_running = Arc::new(AtomicBool::new(false));
        let wr = was_running.clone();
        let rl2 = rl.clone();
        let rl3 = rl.clone();

        rl.perform(move || {
            wr.store(rl2.is_running(), Ordering::SeqCst);
            rl3.stop();
        });

        rl.run();
        assert!(was_running.load(Ordering::SeqCst));
    }

    // ── Edge cases ─────────────────────────────────────────────

    #[test]
    fn cancel_nonexistent_timer() {
        let rl = RunLoop::new();
        // Should not panic
        rl.cancel_timer(TimerToken(999));
        rl.run_once();
    }

    #[test]
    fn perform_after_run_once() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));

        // First run_once — nothing enqueued
        rl.run_once();

        // Enqueue after run_once
        let c = counter.clone();
        rl.perform(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        // Should fire on next run_once
        rl.run_once();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn timer_ordering() {
        let rl = RunLoop::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        let o1 = order.clone();
        rl.schedule_timer(Duration::from_millis(1), false, move || {
            o1.lock().unwrap().push(1);
        });

        let o2 = order.clone();
        rl.schedule_timer(Duration::from_millis(5), false, move || {
            o2.lock().unwrap().push(2);
        });

        // Wait for both to be ready
        std::thread::sleep(Duration::from_millis(10));
        rl.run_once();

        let fired = order.lock().unwrap();
        assert_eq!(*fired, vec![1, 2]);
    }

    #[test]
    fn run_once_returns_total_count() {
        let rl = RunLoop::new();

        // Enqueue 2 blocks
        rl.perform(|| {});
        rl.perform(|| {});

        // Schedule 1 timer that fires immediately
        rl.schedule_timer(Duration::from_millis(1), false, || {});
        std::thread::sleep(Duration::from_millis(5));

        assert_eq!(rl.run_once(), 3);
    }

    #[test]
    fn many_timers() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));

        for _ in 0..10 {
            let c = counter.clone();
            rl.schedule_timer(Duration::from_millis(1), false, move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        std::thread::sleep(Duration::from_millis(5));
        rl.run_once();
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn cancel_already_fired_timer() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));

        let c = counter.clone();
        let token = rl.schedule_timer(Duration::from_millis(1), false, move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        std::thread::sleep(Duration::from_millis(5));
        rl.run_once();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Cancel after it already fired — should not panic
        rl.cancel_timer(token);
        rl.run_once();
    }

    #[test]
    fn perform_order() {
        let rl = RunLoop::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        for i in 0..5 {
            let o = order.clone();
            rl.perform(move || {
                o.lock().unwrap().push(i);
            });
        }

        rl.run_once();
        let result = order.lock().unwrap();
        assert_eq!(*result, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn cancel_before_fire_prevents_execution() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));
        let c = counter.clone();
        let token = rl.schedule_timer(Duration::from_millis(1), false, move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        // Cancel immediately, before it has a chance to fire
        rl.cancel_timer(token);

        // Wait long enough that the timer would have fired
        std::thread::sleep(Duration::from_millis(10));
        rl.run_once();
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    // ── Public Timer struct tests ──────────────────────────────

    #[test]
    fn timer_new_is_valid() {
        let t = Timer::new(Duration::from_secs(1), false, || {});
        assert!(t.is_valid());
    }

    #[test]
    fn timer_invalidate() {
        let mut t = Timer::new(Duration::from_secs(1), false, || {});
        t.invalidate();
        assert!(!t.is_valid());
    }

    #[test]
    fn timer_fires_callback() {
        let counter = Arc::new(AtomicI32::new(0));
        let c = counter.clone();
        let mut t = Timer::new(Duration::from_secs(1), false, move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        t.fire();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn timer_repeating_reschedules() {
        let mut t = Timer::new(Duration::from_millis(100), true, || {});
        let first = t.next_fire;
        std::thread::sleep(Duration::from_millis(5));
        t.fire();
        assert!(t.next_fire > first, "next_fire should advance after fire");
        assert!(t.is_valid(), "repeating timer stays valid");
    }

    #[test]
    fn timer_non_repeating_invalidates_after_fire() {
        let mut t = Timer::new(Duration::from_millis(1), false, || {});
        assert!(t.is_valid());
        t.fire();
        assert!(!t.is_valid(), "non-repeating timer auto-invalidates");
    }

    #[test]
    fn schedule_and_process_timers() {
        let rl = RunLoop::new();
        let counter = Arc::new(AtomicI32::new(0));
        let c = counter.clone();
        let timer = Timer::new(Duration::from_millis(0), false, move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        rl.add_timer(timer);
        std::thread::sleep(Duration::from_millis(5));
        let fired = rl.process_timers();
        assert!(fired >= 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
