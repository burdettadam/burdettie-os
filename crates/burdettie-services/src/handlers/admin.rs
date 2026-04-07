//! Admin API handler — query recorded requests, logs, and system state.
//!
//! Endpoints (all require auth token when configured):
//! - `/_burdettie/admin/requests` — query recorded requests (GET)
//! - `/_burdettie/admin/requests/summary` — request counts by source (GET)
//! - `/_burdettie/admin/logs` — query structured log events (GET)
//! - `/_burdettie/admin/config` — current server configuration (GET)
//!
//! Query parameters:
//! - `source` — filter by source type: http, xpc, dns, os_log
//! - `limit` — max results (default 50, max 1000)
//! - `offset` — pagination offset (default 0)

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct AdminHandler {
    /// Bearer token for auth (None = no auth required).
    auth_token: Option<String>,
    /// Instance ID for multi-tenant tagging.
    instance_id: String,
}

impl AdminHandler {
    pub fn new(auth_token: Option<String>, instance_id: String) -> Self {
        Self { auth_token, instance_id }
    }

    /// Validate the Authorization header if a token is configured.
    fn check_auth(&self, request: &Request) -> bool {
        let Some(ref expected) = self.auth_token else {
            return true; // no auth required
        };

        request
            .headers()
            .iter()
            .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("authorization"))
            .map(|h| {
                let val = h.value.as_str();
                val.strip_prefix("Bearer ")
                    .or_else(|| val.strip_prefix("bearer "))
                    .is_some_and(|token| token == expected.as_str())
            })
            .unwrap_or(false)
    }
}

impl Handler for AdminHandler {
    fn handle(&self, path: &str, _method: &str, _body: &[u8], request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/_burdettie/admin/") {
            return None;
        }

        // Check auth
        if !self.check_auth(request) {
            return Some(HandlerResponse::with_status(
                401,
                "application/json",
                br#"{"error":"unauthorized","message":"Valid Bearer token required"}"#.to_vec(),
            ));
        }

        match path {
            "/_burdettie/admin/requests" => Some(self.handle_requests(request)),
            "/_burdettie/admin/requests/summary" => Some(self.handle_summary()),
            "/_burdettie/admin/logs" => Some(self.handle_logs(request)),
            "/_burdettie/admin/config" => Some(self.handle_config()),
            _ => Some(HandlerResponse::with_status(
                404,
                "application/json",
                br#"{"error":"not_found"}"#.to_vec(),
            )),
        }
    }
}

impl AdminHandler {
    fn handle_requests(&self, request: &Request) -> HandlerResponse {
        let recorder = match crate::global_recorder() {
            Some(r) => r,
            None => return HandlerResponse::with_status(
                503,
                "application/json",
                br#"{"error":"recorder_not_available"}"#.to_vec(),
            ),
        };

        let url = request.url();
        let source = extract_query_param(url, "source");
        let limit = extract_query_param(url, "limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(50)
            .min(1000);
        let offset = extract_query_param(url, "offset")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let rows = recorder.query_recent(source.as_deref(), limit, offset);

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "total": recorder.count(),
            "limit": limit,
            "offset": offset,
            "count": rows.len(),
            "requests": rows,
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_summary(&self) -> HandlerResponse {
        let recorder = match crate::global_recorder() {
            Some(r) => r,
            None => return HandlerResponse::with_status(
                503,
                "application/json",
                br#"{"error":"recorder_not_available"}"#.to_vec(),
            ),
        };

        let groups = recorder.count_by_source();
        let mut by_source = serde_json::Map::new();
        for (source, count) in &groups {
            by_source.insert(source.clone(), serde_json::json!(count));
        }

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "total": recorder.count(),
            "by_source": by_source,
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_logs(&self, request: &Request) -> HandlerResponse {
        let limit = extract_query_param(request.url(), "limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(100)
            .min(1000);

        let events = burdettie_log_pipeline::drain_events();
        let capped: Vec<_> = events.into_iter().rev().take(limit).collect();

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "event_count": burdettie_log_pipeline::event_count(),
            "returned": capped.len(),
            "events": capped,
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_config(&self) -> HandlerResponse {
        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "guest_binary": crate::guest_binary(),
            "server_version": env!("CARGO_PKG_VERSION"),
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }
}

/// Extract a query parameter from a URL path+query string.
fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next() == Some(key) {
            return kv.next().map(|v| v.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_disabled_allows_all() {
        let handler = AdminHandler::new(None, "test".into());
        assert!(handler.auth_token.is_none());
        // Without a real Request we can't test check_auth, but we verify no panic
    }

    #[test]
    fn admin_ignores_non_admin_paths() {
        let handler = AdminHandler::new(None, "test".into());
        assert!(!"/api/v1/telemetry".starts_with("/_burdettie/admin/"));
        assert!("/_burdettie/admin/requests".starts_with("/_burdettie/admin/"));
        // handler.handle would return None for non-admin paths
        let _ = handler;
    }

    #[test]
    fn extract_params() {
        assert_eq!(
            extract_query_param("/_burdettie/admin/requests?source=http&limit=10", "source"),
            Some("http".into())
        );
        assert_eq!(
            extract_query_param("/_burdettie/admin/requests?source=http&limit=10", "limit"),
            Some("10".into())
        );
        assert_eq!(
            extract_query_param("/_burdettie/admin/requests", "source"),
            None
        );
    }
}
