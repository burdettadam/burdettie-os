//! Log output sinks — stdout, file, and HTTP POST.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Configuration for where log events are written.
#[derive(Debug, Clone)]
pub enum SinkConfig {
    /// Write to stdout (via the `log` crate — default).
    Stdout,
    /// Append to a file at the given path.
    File(String),
    /// POST events as JSON to an HTTP endpoint.
    Http(String),
}

/// Global sink state — initialized once at startup.
pub(crate) struct GlobalSink {
    initialized: AtomicBool,
    file_writer: Mutex<Option<std::io::BufWriter<std::fs::File>>>,
    http_url: Mutex<Option<String>>,
}

impl GlobalSink {
    pub(crate) const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            file_writer: Mutex::new(None),
            http_url: Mutex::new(None),
        }
    }

    /// Initialize the sink.  First call wins.
    pub(crate) fn init(&self, config: SinkConfig) {
        if self.initialized.swap(true, Ordering::AcqRel) {
            return;
        }
        match config {
            SinkConfig::Stdout => {} // default — routed via `log` crate
            SinkConfig::File(path) => {
                match std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                {
                    Ok(file) => {
                        *self.file_writer.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(std::io::BufWriter::new(file));
                        log::info!("burdettie-log-pipeline: file sink → {path}");
                    }
                    Err(e) => {
                        log::warn!("burdettie-log-pipeline: failed to open {path}: {e}");
                    }
                }
            }
            SinkConfig::Http(url) => {
                *self.http_url.lock().unwrap_or_else(|e| e.into_inner()) = Some(url.clone());
                log::info!("burdettie-log-pipeline: http sink → {url}");
            }
        }
    }

    /// Write a formatted line to the configured sink.
    ///
    /// For `Stdout`, this is a no-op (caller routes through `log` crate).
    /// For `File`, appends a line.
    /// For `Http`, batches and sends periodically or on flush.
    pub(crate) fn write_line(&self, line: &str) {
        // File sink
        if let Ok(mut guard) = self.file_writer.lock() {
            if let Some(ref mut writer) = *guard {
                use std::io::Write;
                let _ = writeln!(writer, "{line}");
                let _ = writer.flush();
                return;
            }
        }

        // HTTP sink — fire-and-forget POST for each event
        if let Ok(guard) = self.http_url.lock() {
            if let Some(ref url) = *guard {
                // Best-effort — don't block the pipeline on network errors
                let _ = ureq::post(url)
                    .header("Content-Type", "application/x-ndjson")
                    .send(line.as_bytes());
            }
        }
    }
}

pub(crate) static GLOBAL_SINK: GlobalSink = GlobalSink::new();
