//! Apple Push Notification service handler.
//!
//! APNs uses persistent TCP/TLS connections with binary framing — not HTTP.
//! Since BurdettieOS only intercepts HTTP traffic via domain rewriting,
//! any APNs request arriving here is an HTTP probe or a miscategorized request.
//! We return a clean "not connected" response.
//!
//! Endpoint:
//! - `/api/v1/push` — *.push.apple.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct PushHandler;

impl Handler for PushHandler {
    fn handle(&self, path: &str, _method: &str, _body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/push") {
            return None;
        }

        log::debug!("burdettie-services: push {path}");

        Some(HandlerResponse::json(
            br#"{"status":"not_connected","message":"APNs not available in BurdettieOS","server":"burdettie"}"#.to_vec(),
        ))
    }
}
