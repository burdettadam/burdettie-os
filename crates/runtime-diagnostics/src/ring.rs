//! Fixed-capacity ring buffer for diagnostic events.
//!
//! Replaces the unbounded `Vec` collector with a fixed-size circular buffer.
//! When the buffer is full, the oldest entry is silently overwritten.
//! Each entry carries a monotonic sequence number so consumers can detect
//! gaps (overwrites) and reconstruct chronological order.
//!
//! The ring buffer also tracks per-category counters so total counts are
//! never lost, even when individual events are evicted.

use crate::DiagEvent;

/// Default ring buffer capacity (entries).
const DEFAULT_CAPACITY: usize = 4096;

/// A stamped diagnostic event with a monotonic sequence number.
#[derive(Debug, Clone)]
pub struct StampedEvent {
    /// Monotonic sequence number (0-based, never wraps).
    pub seq: u64,
    /// The diagnostic event.
    pub event: DiagEvent,
}

/// Category buckets for aggregate counting.
/// Maps to the top-level groupings in `DiagEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagCategory {
    SelectorDispatch,
    ClassLookup,
    SymbolResolution,
    HostFallback,
    MethodTrace,
    /// Resource-request graph edges (dependency loading, image mapping).
    ResourceGraph,
}

impl DiagCategory {
    /// Classify a `DiagEvent` into its category.
    pub fn of(event: &DiagEvent) -> Self {
        match event {
            DiagEvent::SelectorMissing { .. }
            | DiagEvent::SuperSelectorMissing { .. }
            | DiagEvent::ForwardingEntered { .. } => Self::SelectorDispatch,

            DiagEvent::ClassMissing { .. }
            | DiagEvent::ClassAddrUnknown { .. }
            | DiagEvent::Super2UnknownClass { .. }
            | DiagEvent::CategoryOrphan { .. } => Self::ClassLookup,

            DiagEvent::SymbolBlocked { .. }
            | DiagEvent::SymbolMissing { .. }
            | DiagEvent::BindFailed { .. } => Self::SymbolResolution,

            DiagEvent::HostFallback { .. }
            | DiagEvent::UnknownIsa { .. } => Self::HostFallback,

            DiagEvent::MethodDispatch { .. } => Self::MethodTrace,

            DiagEvent::ResourceRequest { .. }
            | DiagEvent::ResourceResolved { .. }
            | DiagEvent::ResourceFailed { .. } => Self::ResourceGraph,
        }
    }
}

/// Per-category aggregate counters.
#[derive(Debug, Clone, Default)]
pub struct CategoryCounters {
    pub selector_dispatch: u64,
    pub class_lookup: u64,
    pub symbol_resolution: u64,
    pub host_fallback: u64,
    pub method_trace: u64,
    pub resource_graph: u64,
}

impl CategoryCounters {
    fn increment(&mut self, cat: DiagCategory) {
        match cat {
            DiagCategory::SelectorDispatch => self.selector_dispatch += 1,
            DiagCategory::ClassLookup => self.class_lookup += 1,
            DiagCategory::SymbolResolution => self.symbol_resolution += 1,
            DiagCategory::HostFallback => self.host_fallback += 1,
            DiagCategory::MethodTrace => self.method_trace += 1,
            DiagCategory::ResourceGraph => self.resource_graph += 1,
        }
    }

    /// Total events emitted (across all categories).
    pub fn total(&self) -> u64 {
        self.selector_dispatch
            + self.class_lookup
            + self.symbol_resolution
            + self.host_fallback
            + self.method_trace
            + self.resource_graph
    }
}

/// Fixed-capacity circular buffer of diagnostic events.
///
/// Thread-safety is handled by the caller (the global `Mutex` in `collector.rs`).
pub struct DiagRing {
    buf: Vec<Option<StampedEvent>>,
    /// Write cursor (next slot to fill).
    head: usize,
    /// Number of valid entries (≤ capacity).
    len: usize,
    /// Monotonic sequence counter.
    next_seq: u64,
    /// Aggregate counters (never evicted).
    pub counters: CategoryCounters,
}

impl DiagRing {
    /// Create a ring buffer with the default capacity (4096).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a ring buffer with a specific capacity.
    pub fn with_capacity(cap: usize) -> Self {
        let cap = cap.max(1);
        Self {
            buf: (0..cap).map(|_| None).collect(),
            head: 0,
            len: 0,
            next_seq: 0,
            counters: CategoryCounters::default(),
        }
    }

    /// Current capacity.
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Number of events currently in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total number of events ever emitted (including evicted).
    pub fn total_emitted(&self) -> u64 {
        self.next_seq
    }

    /// Number of events that were evicted (overwritten).
    pub fn evicted(&self) -> u64 {
        self.next_seq.saturating_sub(self.len as u64)
    }

    /// Push an event into the ring buffer. Returns its sequence number.
    pub fn push(&mut self, event: DiagEvent) -> u64 {
        let seq = self.next_seq;
        self.counters.increment(DiagCategory::of(&event));
        self.buf[self.head] = Some(StampedEvent { seq, event });
        self.head = (self.head + 1) % self.buf.len();
        if self.len < self.buf.len() {
            self.len += 1;
        }
        self.next_seq += 1;
        seq
    }

    /// Iterate over all events in chronological order (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &StampedEvent> {
        let cap = self.buf.len();
        // Start at the oldest entry.
        let start = if self.len < cap {
            0
        } else {
            self.head // head points past the newest, which wraps to the oldest
        };
        RingIter {
            buf: &self.buf,
            pos: start,
            remaining: self.len,
            cap,
        }
    }

    /// Return all events as a Vec in chronological order.
    pub fn snapshot(&self) -> Vec<StampedEvent> {
        self.iter().cloned().collect()
    }

    /// Drain all events, returning them in chronological order and clearing the buffer.
    /// Counters are preserved.
    pub fn drain(&mut self) -> Vec<StampedEvent> {
        let result = self.snapshot();
        for slot in &mut self.buf {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
        result
    }

    /// Clear all events. Counters are preserved.
    pub fn clear(&mut self) {
        for slot in &mut self.buf {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
    }

    /// Return events matching a predicate, in chronological order.
    pub fn filter<F: Fn(&DiagEvent) -> bool>(&self, pred: F) -> Vec<StampedEvent> {
        self.iter()
            .filter(|se| pred(&se.event))
            .cloned()
            .collect()
    }

    /// Return events of a specific category, in chronological order.
    pub fn by_category(&self, cat: DiagCategory) -> Vec<StampedEvent> {
        self.filter(|e| DiagCategory::of(e) == cat)
    }

    /// Return the last N events (most recent first).
    pub fn tail(&self, n: usize) -> Vec<StampedEvent> {
        let all = self.snapshot();
        let start = all.len().saturating_sub(n);
        all[start..].to_vec()
    }
}

impl Default for DiagRing {
    fn default() -> Self {
        Self::new()
    }
}

// ── Iterator ────────────────────────────────────────────────────────

struct RingIter<'a> {
    buf: &'a [Option<StampedEvent>],
    pos: usize,
    remaining: usize,
    cap: usize,
}

impl<'a> Iterator for RingIter<'a> {
    type Item = &'a StampedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let item = self.buf[self.pos].as_ref();
        self.pos = (self.pos + 1) % self.cap;
        self.remaining -= 1;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
