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
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).push(Box::new(block));
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
        self.timers.lock().unwrap_or_else(|e| e.into_inner()).push(entry);
        token
    }

    /// Cancel a previously scheduled timer.
    pub fn cancel_timer(&self, token: TimerToken) {
        self.cancelled.lock().unwrap_or_else(|e| e.into_inner()).push(token);
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
}
