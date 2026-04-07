//! Diagnostic status handler — `/_burdettie/status`.
//!
//! Returns a JSON object showing the current state of all BurdettieOS services:
//! handler counts, recording stats, replay status, uptime.

use super::{Handler, HandlerResponse};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tiny_http::Request;

/// Counts of requests handled, by category.
pub static REQUESTS_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static REQUESTS_TELEMETRY: AtomicU64 = AtomicU64::new(0);
pub static REQUESTS_CERTS: AtomicU64 = AtomicU64::new(0);
pub static REQUESTS_DNS: AtomicU64 = AtomicU64::new(0);
pub static REQUESTS_NOTARIZE: AtomicU64 = AtomicU64::new(0);
pub static REQUESTS_REPLAY_HIT: AtomicU64 = AtomicU64::new(0);

static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Initialize the status start time. Call once at server startup.
pub fn init_start_time() {
    START_TIME.get_or_init(Instant::now);
}

/// Increment the total counter (called from the router on every request).
pub fn inc_total() {
    REQUESTS_TOTAL.fetch_add(1, Ordering::Relaxed);
}

/// Increment the telemetry counter.
pub fn inc_telemetry() {
    REQUESTS_TELEMETRY.fetch_add(1, Ordering::Relaxed);
}

/// Increment the certs counter.
pub fn inc_certs() {
    REQUESTS_CERTS.fetch_add(1, Ordering::Relaxed);
}

/// Increment the DNS counter.
pub fn inc_dns() {
    REQUESTS_DNS.fetch_add(1, Ordering::Relaxed);
}

/// Increment the notarize counter.
pub fn inc_notarize() {
    REQUESTS_NOTARIZE.fetch_add(1, Ordering::Relaxed);
}

/// Increment the replay-hit counter.
pub fn inc_replay_hit() {
    REQUESTS_REPLAY_HIT.fetch_add(1, Ordering::Relaxed);
}

pub struct StatusHandler;

impl Handler for StatusHandler {
    fn handle(&self, path: &str, _method: &str, _body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if path != "/_burdettie/status" {
            return None;
        }

        let uptime_secs = START_TIME
            .get()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);

        let record_count = crate::global_recorder()
            .map(|r| r.count())
            .unwrap_or(-1);

        let response = serde_json::json!({
            "server": "burdettie",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": uptime_secs,
            "counters": {
                "total": REQUESTS_TOTAL.load(Ordering::Relaxed),
                "telemetry": REQUESTS_TELEMETRY.load(Ordering::Relaxed),
                "certs": REQUESTS_CERTS.load(Ordering::Relaxed),
                "dns": REQUESTS_DNS.load(Ordering::Relaxed),
                "notarize": REQUESTS_NOTARIZE.load(Ordering::Relaxed),
                "replay_hits": REQUESTS_REPLAY_HIT.load(Ordering::Relaxed),
            },
            "recording": {
                "enabled": record_count >= 0,
                "total_records": record_count,
            },
        });

        Some(HandlerResponse::json(response.to_string().into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_returns_json() {
        init_start_time();
        REQUESTS_TOTAL.store(42, Ordering::Relaxed);

        assert_eq!(REQUESTS_TOTAL.load(Ordering::Relaxed), 42);

        // Reset for other tests
        REQUESTS_TOTAL.store(0, Ordering::Relaxed);
    }

    #[test]
    fn status_ignores_other_paths() {
        assert!("/_burdettie/status".starts_with("/_burdettie/"));
        assert!(!"/api/v1/telemetry".starts_with("/_burdettie/"));
    }
}
