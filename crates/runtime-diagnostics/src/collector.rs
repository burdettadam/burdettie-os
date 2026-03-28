//! Thread-safe global diagnostic event collector.
//!
//! Backed by a fixed-capacity [`DiagRing`] that silently evicts the
//! oldest entries when full, preventing unbounded memory growth from
//! verbose dispatch tracing.

use std::sync::{LazyLock, Mutex};

use crate::DiagEvent;
use crate::ring::{DiagRing, StampedEvent, CategoryCounters, DiagCategory};

static RING: LazyLock<Mutex<DiagRing>> = LazyLock::new(|| Mutex::new(DiagRing::new()));

fn lock_ring() -> std::sync::MutexGuard<'static, DiagRing> {
    RING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Record a diagnostic event. Returns the event's sequence number.
pub fn emit(event: DiagEvent) -> u64 {
    lock_ring().push(event)
}

/// Drain all events in the ring buffer, returning them in chronological
/// order and clearing the buffer. Aggregate counters are preserved.
pub fn drain() -> Vec<DiagEvent> {
    lock_ring().drain().into_iter().map(|se| se.event).collect()
}

/// Drain stamped events (with sequence numbers).
pub fn drain_stamped() -> Vec<StampedEvent> {
    lock_ring().drain()
}

/// Return a snapshot of all events currently in the ring buffer.
pub fn snapshot() -> Vec<DiagEvent> {
    lock_ring().iter().map(|se| se.event.clone()).collect()
}

/// Return a stamped snapshot (with sequence numbers).
pub fn snapshot_stamped() -> Vec<StampedEvent> {
    lock_ring().snapshot()
}

/// Check whether any events are currently in the ring buffer.
pub fn has_events() -> bool {
    !lock_ring().is_empty()
}

/// Clear all events in the ring buffer. Counters are preserved.
pub fn clear() {
    lock_ring().clear();
}

/// Number of events currently in the ring buffer (not total emitted).
pub fn event_count() -> usize {
    lock_ring().len()
}

/// Total number of events ever emitted (including evicted ones).
pub fn total_emitted() -> u64 {
    lock_ring().total_emitted()
}

/// Number of events that were evicted due to ring buffer overflow.
pub fn evicted_count() -> u64 {
    lock_ring().evicted()
}

/// Aggregate counters across all categories (never evicted).
pub fn counters() -> CategoryCounters {
    lock_ring().counters.clone()
}

/// Return events matching a specific diagnostic category.
pub fn by_category(cat: DiagCategory) -> Vec<StampedEvent> {
    lock_ring().by_category(cat)
}

/// Return the most recent N events.
pub fn tail(n: usize) -> Vec<StampedEvent> {
    lock_ring().tail(n)
}

/// Resize the ring buffer (events are preserved up to the new capacity).
pub fn resize(new_capacity: usize) {
    let mut ring = lock_ring();
    let events = ring.drain();
    *ring = DiagRing::with_capacity(new_capacity);
    for se in events {
        ring.push(se.event);
    }
}
