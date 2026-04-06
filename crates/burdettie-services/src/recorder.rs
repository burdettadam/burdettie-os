//! Request recorder — stores all intercepted requests in SQLite for analysis.

use rusqlite::Connection;
use std::sync::Mutex;

/// Request source type.
#[derive(Debug, Clone, Copy)]
pub enum RequestSource {
    Http,
    Xpc,
    Dns,
    OsLog,
}

impl RequestSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Xpc => "xpc",
            Self::Dns => "dns",
            Self::OsLog => "os_log",
        }
    }
}

/// A recorded HTTP request/response pair.
#[derive(Debug, Clone)]
pub struct HttpRecord {
    pub original_url: String,
    pub rewritten_url: String,
    pub method: String,
    pub request_headers: String,
    pub request_body: Vec<u8>,
    pub response_status: i32,
    pub response_headers: String,
    pub response_body: Vec<u8>,
    pub guest_binary: String,
    pub duration_ms: i64,
}

/// A recorded XPC message.
#[derive(Debug, Clone)]
pub struct XpcRecord {
    pub service: String,
    pub request_json: String,
    pub reply_json: String,
    pub guest_binary: String,
}

/// A recorded DNS query.
#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub query: String,
    pub resolved_json: String,
    pub guest_binary: String,
}

/// A recorded os_log event.
#[derive(Debug, Clone)]
pub struct OsLogRecord {
    pub subsystem: String,
    pub category: String,
    pub level: String,
    pub message: String,
    pub guest_binary: String,
}

/// Thread-safe request recorder backed by SQLite.
pub struct RequestRecorder {
    conn: Mutex<Connection>,
}

impl RequestRecorder {
    /// Open (or create) the SQLite database at the given path.
    pub fn open(db_path: &str) -> Result<Self, String> {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                source TEXT NOT NULL,
                original_url TEXT,
                rewritten_url TEXT,
                method TEXT,
                request_headers TEXT,
                request_body BLOB,
                response_status INTEGER,
                response_headers TEXT,
                response_body BLOB,
                xpc_service TEXT,
                xpc_request TEXT,
                xpc_reply TEXT,
                dns_query TEXT,
                dns_resolved TEXT,
                log_subsystem TEXT,
                log_category TEXT,
                log_level TEXT,
                log_message TEXT,
                guest_binary TEXT,
                duration_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_requests_source ON requests(source);
            CREATE INDEX IF NOT EXISTS idx_requests_url ON requests(original_url);",
        )
        .map_err(|e| e.to_string())?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Record an HTTP request/response.
    pub fn record_http(&self, record: &HttpRecord) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(e) => { log::warn!("recorder lock poisoned: {e}"); return; }
        };
        if let Err(e) = conn.execute(
            "INSERT INTO requests (source, original_url, rewritten_url, method,
             request_headers, request_body, response_status, response_headers,
             response_body, guest_binary, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                RequestSource::Http.as_str(),
                record.original_url,
                record.rewritten_url,
                record.method,
                record.request_headers,
                record.request_body,
                record.response_status,
                record.response_headers,
                record.response_body,
                record.guest_binary,
                record.duration_ms,
            ],
        ) {
            log::warn!("recorder: failed to record HTTP: {e}");
        }
    }

    /// Record an XPC message exchange.
    pub fn record_xpc(&self, record: &XpcRecord) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(e) => { log::warn!("recorder lock poisoned: {e}"); return; }
        };
        if let Err(e) = conn.execute(
            "INSERT INTO requests (source, xpc_service, xpc_request, xpc_reply, guest_binary)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                RequestSource::Xpc.as_str(),
                record.service,
                record.request_json,
                record.reply_json,
                record.guest_binary,
            ],
        ) {
            log::warn!("recorder: failed to record XPC: {e}");
        }
    }

    /// Record a DNS query.
    pub fn record_dns(&self, record: &DnsRecord) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(e) => { log::warn!("recorder lock poisoned: {e}"); return; }
        };
        if let Err(e) = conn.execute(
            "INSERT INTO requests (source, dns_query, dns_resolved, guest_binary)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                RequestSource::Dns.as_str(),
                record.query,
                record.resolved_json,
                record.guest_binary,
            ],
        ) {
            log::warn!("recorder: failed to record DNS: {e}");
        }
    }

    /// Record an os_log event.
    pub fn record_os_log(&self, record: &OsLogRecord) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(e) => { log::warn!("recorder lock poisoned: {e}"); return; }
        };
        if let Err(e) = conn.execute(
            "INSERT INTO requests (source, log_subsystem, log_category, log_level,
             log_message, guest_binary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                RequestSource::OsLog.as_str(),
                record.subsystem,
                record.category,
                record.level,
                record.message,
                record.guest_binary,
            ],
        ) {
            log::warn!("recorder: failed to record os_log: {e}");
        }
    }

    /// Return the total number of recorded requests.
    pub fn count(&self) -> i64 {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return -1,
        };
        conn.query_row("SELECT COUNT(*) FROM requests", [], |row| row.get(0))
            .unwrap_or(-1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_count() {
        let recorder = RequestRecorder::open(":memory:").expect("open in-memory db");
        assert_eq!(recorder.count(), 0);

        recorder.record_http(&HttpRecord {
            original_url: "https://xp.apple.com/report".into(),
            rewritten_url: "http://127.0.0.1:9999/api/v1/telemetry/xp/report".into(),
            method: "POST".into(),
            request_headers: "{}".into(),
            request_body: b"test body".to_vec(),
            response_status: 200,
            response_headers: "{}".into(),
            response_body: b"ok".to_vec(),
            guest_binary: "xcrun".into(),
            duration_ms: 5,
        });
        assert_eq!(recorder.count(), 1);

        recorder.record_dns(&DnsRecord {
            query: "xp.apple.com".into(),
            resolved_json: "[\"127.0.0.1\"]".into(),
            guest_binary: "nslookup".into(),
        });
        assert_eq!(recorder.count(), 2);

        recorder.record_xpc(&XpcRecord {
            service: "com.apple.analyticsd".into(),
            request_json: "{\"type\":\"event\"}".into(),
            reply_json: "{\"status\":\"ok\"}".into(),
            guest_binary: "xcodebuild".into(),
        });
        assert_eq!(recorder.count(), 3);

        recorder.record_os_log(&OsLogRecord {
            subsystem: "com.apple.dt.instruments".into(),
            category: "default".into(),
            level: "info".into(),
            message: "test log message".into(),
            guest_binary: "instruments".into(),
        });
        assert_eq!(recorder.count(), 4);
    }
}
