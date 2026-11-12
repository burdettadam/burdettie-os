//! Request replay handler — serve previously-recorded responses from SQLite.
//!
//! When `BURDETTIE_REPLAY_DB` is set, this handler checks the recording
//! database for a matching request and returns the stored response instead
//! of generating a fresh one.  This enables:
//! - Deterministic testing without running live handlers
//! - Offline protocol analysis (record once, replay forever)
//! - Regression testing of guest tool interactions
//!
//! The replay handler is registered as the *first* handler in the router
//! chain when replay mode is active.

use super::{Handler, HandlerResponse};
use rusqlite::Connection;
use std::sync::Mutex;
use tiny_http::Request;

/// A handler that replays recorded HTTP responses from SQLite.
pub struct ReplayHandler {
    conn: Mutex<Connection>,
}

impl ReplayHandler {
    /// Open a replay database at `db_path`.
    ///
    /// Returns `None` if the database cannot be opened or has no `requests` table.
    pub fn open(db_path: &str) -> Option<Self> {
        let conn = Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .ok()?;

        // Verify the table exists
        let has_table: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='requests'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_table {
            log::warn!("replay: {db_path} has no 'requests' table — replay disabled");
            return None;
        }

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests WHERE source='http'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        log::info!("burdettie-services: replay loaded {count} HTTP records from {db_path}");

        Some(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Look up a recorded response for the given URL and method.
    fn lookup(&self, url: &str, method: &str) -> Option<ReplayRecord> {
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT response_status, response_headers, response_body
             FROM requests
             WHERE source = 'http'
               AND rewritten_url = ?1
               AND method = ?2
             ORDER BY id DESC
             LIMIT 1",
            rusqlite::params![url, method],
            |row| {
                Ok(ReplayRecord {
                    status: row.get::<_, i32>(0)? as u16,
                    headers: row.get::<_, String>(1)?,
                    body: row.get::<_, Vec<u8>>(2)?,
                })
            },
        )
        .ok()
    }

    /// Count the number of HTTP records available for replay.
    pub fn http_record_count(&self) -> i64 {
        let Ok(conn) = self.conn.lock() else {
            return -1;
        };
        conn.query_row(
            "SELECT COUNT(*) FROM requests WHERE source='http'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(-1)
    }
}

struct ReplayRecord {
    status: u16,
    headers: String,
    body: Vec<u8>,
}

impl Handler for ReplayHandler {
    fn handle(
        &self,
        path: &str,
        method: &str,
        _body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        // Try to find a recorded response matching this request path
        let record = self.lookup(path, method)?;

        super::status::inc_replay_hit();

        log::info!(
            "burdettie-services: replay hit {method} {path} → {} ({} bytes)",
            record.status,
            record.body.len(),
        );

        // Extract content-type from stored headers if available
        let content_type = extract_content_type(&record.headers)
            .unwrap_or_else(|| "application/octet-stream".to_string());

        Some(HandlerResponse::with_status(
            record.status,
            content_type,
            record.body,
        ))
    }
}

/// Extract `content-type` from a JSON-encoded headers string.
fn extract_content_type(headers_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(headers_json).ok()?;
    // Try both casing conventions
    v.get("content-type")
        .or_else(|| v.get("Content-Type"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE requests (
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
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn replay_finds_matching_record() {
        let conn = create_test_db();
        conn.execute(
            "INSERT INTO requests (source, rewritten_url, method, response_status,
             response_headers, response_body)
             VALUES ('http', '/api/v1/telemetry/xp/report', 'POST', 200,
             '{\"content-type\":\"application/json\"}', X'7B7D')",
            [],
        )
        .unwrap();

        let handler = ReplayHandler {
            conn: Mutex::new(conn),
        };
        let record = handler.lookup("/api/v1/telemetry/xp/report", "POST");
        assert!(record.is_some());
        let r = record.unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.body, b"{}");
    }

    #[test]
    fn replay_returns_none_for_missing_record() {
        let conn = create_test_db();
        let handler = ReplayHandler {
            conn: Mutex::new(conn),
        };
        assert!(handler.lookup("/nonexistent", "GET").is_none());
    }

    #[test]
    fn replay_returns_latest_record() {
        let conn = create_test_db();
        conn.execute(
            "INSERT INTO requests (source, rewritten_url, method, response_status,
             response_headers, response_body)
             VALUES ('http', '/api/v1/dns/resolve', 'GET', 200,
             '{}', X'6F6C64')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO requests (source, rewritten_url, method, response_status,
             response_headers, response_body)
             VALUES ('http', '/api/v1/dns/resolve', 'GET', 200,
             '{}', X'6E6577')",
            [],
        )
        .unwrap();

        let handler = ReplayHandler {
            conn: Mutex::new(conn),
        };
        let record = handler.lookup("/api/v1/dns/resolve", "GET").unwrap();
        assert_eq!(record.body, b"new");
    }

    #[test]
    fn extract_content_type_works() {
        assert_eq!(
            extract_content_type(r#"{"content-type":"application/json"}"#),
            Some("application/json".to_string()),
        );
        assert_eq!(
            extract_content_type(r#"{"Content-Type":"text/html"}"#),
            Some("text/html".to_string()),
        );
        assert_eq!(extract_content_type("invalid"), None);
    }

    #[test]
    fn http_record_count_returns_correct_count() {
        let conn = create_test_db();
        conn.execute(
            "INSERT INTO requests (source, rewritten_url, method, response_status,
             response_headers, response_body)
             VALUES ('http', '/test', 'GET', 200, '{}', X'00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO requests (source, dns_query, dns_resolved)
             VALUES ('dns', 'test.com', '[]')",
            [],
        )
        .unwrap();

        let handler = ReplayHandler {
            conn: Mutex::new(conn),
        };
        assert_eq!(handler.http_record_count(), 1);
    }
}
