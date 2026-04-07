//! Time endpoint handler.
//!
//! NTP uses UDP port 123, not HTTP.  Any request arriving here via domain
//! rewrite is an HTTP probe.  We return the current Unix timestamp so
//! callers have a reasonable time reference.
//!
//! Endpoint:
//! - `/api/v1/time` — time.apple.com, time-macos.apple.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct TimeHandler;

impl Handler for TimeHandler {
    fn handle(&self, path: &str, _method: &str, _body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/time") {
            return None;
        }

        log::debug!("burdettie-services: time {path}");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let body = format!(
            r#"{{"timestamp":{now},"server":"burdettie","protocol":"http-not-ntp"}}"#
        );

        Some(HandlerResponse::json(body.into_bytes()))
    }
}
