//! Structured diagnostic events for the Darwin loader runtime.
//!
//! This crate provides a zero-dependency event system for recording runtime
//! failures (missing selectors, classes, symbols), host fallback paths,
//! and resource-request graph edges (dependency loading, symbol resolution).
//!
//! Events are stored in a fixed-capacity ring buffer that silently evicts
//! the oldest entries when full, preventing unbounded memory growth.
//! Per-category aggregate counters survive eviction.

mod events;
mod collector;
pub mod ring;

pub use events::DiagEvent;
pub use ring::{DiagRing, StampedEvent, CategoryCounters, DiagCategory};
pub use collector::{
    emit, drain, drain_stamped, snapshot, snapshot_stamped,
    has_events, clear, event_count,
    total_emitted, evicted_count, counters, by_category, tail, resize,
};

#[cfg(test)]
mod tests;
