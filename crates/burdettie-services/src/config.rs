//! Service configuration — read from env vars or constructed programmatically.

/// Configuration for the BurdettieOS service server.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Bind address for the HTTP server (e.g. `127.0.0.1:0` for random port).
    pub bind_addr: String,
    /// Whether to record all requests to SQLite.
    pub record_requests: bool,
    /// Path to the SQLite database for request recording.
    pub record_db_path: String,
    /// Log format for the unified logging pipeline.
    pub log_format: LogFormat,
    /// Log output sink.
    pub log_sink: LogSink,
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// JSON Lines (one JSON object per line).
    Json,
    /// Key=value logfmt.
    Logfmt,
    /// Human-readable text.
    Text,
}

/// Log output sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogSink {
    /// Write to stdout.
    Stdout,
    /// Write to a file.
    File(String),
    /// POST to an HTTP endpoint.
    Http(String),
}

impl ServiceConfig {
    /// Build configuration from environment variables.
    ///
    /// | Variable | Default |
    /// |---|---|
    /// | `BURDETTIE_SERVICES_BIND` | `127.0.0.1:0` |
    /// | `BURDETTIE_RECORD_REQUESTS` | `true` |
    /// | `BURDETTIE_RECORD_DB` | `/tmp/burdettie-requests.db` |
    /// | `BURDETTIE_LOG_FORMAT` | `text` |
    /// | `BURDETTIE_LOG_SINK` | `stdout` |
    pub fn from_env() -> Self {
        let bind_addr = std::env::var("BURDETTIE_SERVICES_BIND")
            .unwrap_or_else(|_| "127.0.0.1:0".into());

        let record_requests = std::env::var("BURDETTIE_RECORD_REQUESTS")
            .map(|v| !matches!(v.as_str(), "0" | "false" | "no" | "off"))
            .unwrap_or(true);

        let record_db_path = std::env::var("BURDETTIE_RECORD_DB")
            .unwrap_or_else(|_| "/tmp/burdettie-requests.db".into());

        let log_format = match std::env::var("BURDETTIE_LOG_FORMAT")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "json" => LogFormat::Json,
            "logfmt" => LogFormat::Logfmt,
            _ => LogFormat::Text,
        };

        let log_sink = match std::env::var("BURDETTIE_LOG_SINK")
            .unwrap_or_default()
            .as_str()
        {
            s if s.starts_with("file:") => LogSink::File(s[5..].to_string()),
            s if s.starts_with("http://") || s.starts_with("https://") => {
                LogSink::Http(s.to_string())
            }
            _ => LogSink::Stdout,
        };

        Self {
            bind_addr,
            record_requests,
            record_db_path,
            log_format,
            log_sink,
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
