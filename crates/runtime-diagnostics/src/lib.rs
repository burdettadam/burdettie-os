//! Structured diagnostic events for the Darwin loader runtime.
//!
//! This crate provides a zero-dependency event system for recording runtime
//! failures (missing selectors, classes, symbols), host fallback paths,
//! and resource-request graph edges (dependency loading, symbol resolution).
//!
//! Events are stored in a fixed-capacity ring buffer that silently evicts
//! the oldest entries when full, preventing unbounded memory growth.
//! Per-category aggregate counters survive eviction.

mod collector;
mod events;
pub mod ring;

pub use collector::{
    by_category, clear, counters, drain, drain_stamped, emit, event_count, evicted_count,
    has_events, resize, snapshot, snapshot_stamped, tail, total_emitted,
};
pub use events::DiagEvent;
pub use ring::{CategoryCounters, DiagCategory, DiagRing, StampedEvent};

#[cfg(test)]
mod tests;
