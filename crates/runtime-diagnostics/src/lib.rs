//! Structured diagnostic events for the Darwin loader runtime.
//!
//! This crate provides a zero-dependency event system for recording runtime
//! failures (missing selectors, classes, symbols) and host fallback paths.
//! Events are collected in a thread-safe global collector and can be drained
//! by CLI tools like `macho-inspect --why-failed`.

mod events;
mod collector;

pub use events::DiagEvent;
pub use collector::{emit, drain, snapshot, has_events, clear, event_count};

#[cfg(test)]
mod tests;
