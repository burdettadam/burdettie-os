//! BurdettieOS Structured Logging Pipeline.
//!
//! Replaces the simple `log::debug!("os_log: ...")` passthrough with a
//! structured pipeline that captures subsystem, category, level, and
//! message from Apple's unified logging system.
//!
//! # Architecture
//!
//! ```text
//! Guest Mach-O ──► os_log_create(subsystem, category)
//!                  _os_log_impl(dso, log, type, fmt, buf, size)
//!                       │
//!                       ▼
//!               ┌───────────────┐
//!               │  Collector    │  ← receives LogEvent structs
//!               │  (thread-safe)│
//!               └───────┬───────┘
//!                       │
//!               ┌───────▼───────┐
//!               │  Formatter    │  ← JSON / logfmt / text
//!               └───────┬───────┘
//!                       │
//!               ┌───────▼───────┐
//!               │  Sink(s)      │  ← stdout / file / callback
//!               └───────────────┘
//! ```

mod collector;
mod formatter;

pub use collector::{Collector, LogEvent, LogLevel, GLOBAL_COLLECTOR};
pub use formatter::{format_event, OutputFormat};

/// Initialize the global log pipeline.
///
/// Call this once at startup.  After initialization, `emit()` will route
/// events through the configured formatter and sink.
pub fn init(format: OutputFormat) {
    GLOBAL_COLLECTOR.init(format);
    log::debug!("burdettie-log-pipeline: initialized with format {:?}", format);
}

/// Emit a structured log event into the pipeline.
///
/// This is the primary entry point called from the os_log shims.
/// Thread-safe and lock-free for the hot path.
pub fn emit(event: LogEvent) {
    GLOBAL_COLLECTOR.emit(event);
}

/// Return the number of events collected so far.
pub fn event_count() -> u64 {
    GLOBAL_COLLECTOR.event_count()
}

/// Drain all collected events (clears the buffer).
///
/// Useful for testing or for periodic flush to an external store.
pub fn drain_events() -> Vec<LogEvent> {
    GLOBAL_COLLECTOR.drain()
}
