//! Output formatters for structured log events.

use crate::collector::LogEvent;

/// Output format for log events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON Lines — one JSON object per line.
    Json,
    /// Key=value logfmt.
    Logfmt,
    /// Human-readable text (subsystem:category [level] message).
    Text,
}

/// Format a single log event according to the chosen format.
pub fn format_event(event: &LogEvent, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => format_json(event),
        OutputFormat::Logfmt => format_logfmt(event),
        OutputFormat::Text => format_text(event),
    }
}

fn format_json(event: &LogEvent) -> String {
    // Use serde for correct JSON escaping
    serde_json::to_string(event).unwrap_or_else(|_| format_text(event))
}

fn format_logfmt(event: &LogEvent) -> String {
    format!(
        "ts={} seq={} subsystem={} category={} level={} msg=\"{}\"",
        event.timestamp_ms,
        event.sequence,
        event.subsystem,
        event.category,
        event.level,
        event.message.replace('\\', "\\\\").replace('"', "\\\""),
    )
}

fn format_text(event: &LogEvent) -> String {
    format!(
        "{}:{} [{}] {}",
        event.subsystem, event.category, event.level, event.message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::{LogLevel, LogEvent};

    fn sample_event() -> LogEvent {
        LogEvent {
            subsystem: "com.apple.dt".into(),
            category: "default".into(),
            level: LogLevel::Info,
            message: "test message".into(),
            sequence: 42,
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn json_format_is_valid() {
        let output = format_event(&sample_event(), OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
        assert_eq!(parsed["subsystem"], "com.apple.dt");
        assert_eq!(parsed["level"], "info");
        assert_eq!(parsed["message"], "test message");
        assert_eq!(parsed["sequence"], 42);
    }

    #[test]
    fn logfmt_contains_all_fields() {
        let output = format_event(&sample_event(), OutputFormat::Logfmt);
        assert!(output.contains("subsystem=com.apple.dt"));
        assert!(output.contains("level=info"));
        assert!(output.contains("msg=\"test message\""));
        assert!(output.contains("seq=42"));
    }

    #[test]
    fn text_format_readable() {
        let output = format_event(&sample_event(), OutputFormat::Text);
        assert_eq!(output, "com.apple.dt:default [info] test message");
    }

    #[test]
    fn json_escapes_special_chars() {
        let mut event = sample_event();
        event.message = "quotes \"here\" and \\backslash".into();
        let output = format_event(&event, OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
        assert_eq!(parsed["message"], "quotes \"here\" and \\backslash");
    }
}
