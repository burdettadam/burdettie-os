//! Log event collector — thread-safe storage and routing of structured events.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use crate::formatter::{self, OutputFormat};

/// Apple os_log level mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// OS_LOG_TYPE_DEFAULT (0x00)
    Default,
    /// OS_LOG_TYPE_INFO (0x01)
    Info,
    /// OS_LOG_TYPE_DEBUG (0x02)
    Debug,
    /// OS_LOG_TYPE_ERROR (0x10)
    Error,
    /// OS_LOG_TYPE_FAULT (0x11)
    Fault,
}

impl LogLevel {
    /// Convert from Apple's os_log_type_t value.
    pub fn from_apple(type_val: u8) -> Self {
        match type_val {
            0x01 => Self::Info,
            0x02 => Self::Debug,
            0x10 => Self::Error,
            0x11 => Self::Fault,
            _ => Self::Default,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Error => write!(f, "error"),
            Self::Fault => write!(f, "fault"),
        }
    }
}

/// A structured log event from Apple's unified logging system.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEvent {
    /// The subsystem identifier (e.g. "com.apple.dt.instruments").
    pub subsystem: String,
    /// The category within the subsystem (e.g. "default").
    pub category: String,
    /// The log level.
    pub level: LogLevel,
    /// The format string / message.
    pub message: String,
    /// Monotonic event sequence number.
    pub sequence: u64,
    /// Milliseconds since pipeline initialization.
    pub timestamp_ms: u64,
}

/// Global log event collector.
///
/// Thread-safe.  Events are buffered in memory and simultaneously
/// formatted+emitted to the configured sink.
pub struct Collector {
    initialized: AtomicBool,
    counter: AtomicU64,
    start_time: Mutex<Option<std::time::Instant>>,
    format: Mutex<OutputFormat>,
    events: Mutex<Vec<LogEvent>>,
}

impl Collector {
    const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            counter: AtomicU64::new(0),
            start_time: Mutex::new(None),
            format: Mutex::new(OutputFormat::Text),
            events: Mutex::new(Vec::new()),
        }
    }

    /// Initialize the collector with a format.  Safe to call multiple times
    /// (only the first call takes effect).
    pub fn init(&self, format: OutputFormat) {
        if self.initialized.swap(true, Ordering::AcqRel) {
            return; // already initialized
        }
        *self.start_time.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());
        *self.format.lock().unwrap_or_else(|e| e.into_inner()) = format;
    }

    /// Emit a structured event.
    pub fn emit(&self, mut event: LogEvent) {
        let seq = self.counter.fetch_add(1, Ordering::Relaxed);
        event.sequence = seq;

        // Compute relative timestamp
        let elapsed = self
            .start_time
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        event.timestamp_ms = elapsed;

        // Format and emit to sink
        let fmt = *self.format.lock().unwrap_or_else(|e| e.into_inner());
        let formatted = formatter::format_event(&event, fmt);

        // Route to the `log` crate at the appropriate level
        match event.level {
            LogLevel::Error | LogLevel::Fault => log::error!("{formatted}"),
            LogLevel::Debug => log::debug!("{formatted}"),
            _ => log::info!("{formatted}"),
        }

        // Buffer the event
        if let Ok(mut events) = self.events.lock() {
            // Cap buffer at 10_000 events to prevent unbounded growth
            if events.len() < 10_000 {
                events.push(event);
            }
        }
    }

    /// Total events emitted.
    pub fn event_count(&self) -> u64 {
        self.counter.load(Ordering::Relaxed)
    }

    /// Drain all buffered events.
    pub fn drain(&self) -> Vec<LogEvent> {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *events)
    }
}

/// Global singleton collector.
pub static GLOBAL_COLLECTOR: Collector = Collector::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_and_count() {
        // Note: uses global state — test may see events from other tests.
        // We check that count increases by at least 1.
        let before = GLOBAL_COLLECTOR.event_count();
        GLOBAL_COLLECTOR.init(OutputFormat::Json);
        GLOBAL_COLLECTOR.emit(LogEvent {
            subsystem: "com.apple.test".into(),
            category: "unit".into(),
            level: LogLevel::Info,
            message: "hello from test".into(),
            sequence: 0,
            timestamp_ms: 0,
        });
        let after = GLOBAL_COLLECTOR.event_count();
        assert!(after > before);
    }

    #[test]
    fn level_from_apple() {
        assert_eq!(LogLevel::from_apple(0x00), LogLevel::Default);
        assert_eq!(LogLevel::from_apple(0x01), LogLevel::Info);
        assert_eq!(LogLevel::from_apple(0x02), LogLevel::Debug);
        assert_eq!(LogLevel::from_apple(0x10), LogLevel::Error);
        assert_eq!(LogLevel::from_apple(0x11), LogLevel::Fault);
        assert_eq!(LogLevel::from_apple(0xFF), LogLevel::Default);
    }

    #[test]
    fn drain_clears_buffer() {
        GLOBAL_COLLECTOR.init(OutputFormat::Text);
        GLOBAL_COLLECTOR.emit(LogEvent {
            subsystem: "com.apple.drain-test".into(),
            category: "test".into(),
            level: LogLevel::Default,
            message: "drain test".into(),
            sequence: 0,
            timestamp_ms: 0,
        });
        let events = GLOBAL_COLLECTOR.drain();
        // At least our event should be present (may include others from parallel tests)
        assert!(!events.is_empty());
    }
}
