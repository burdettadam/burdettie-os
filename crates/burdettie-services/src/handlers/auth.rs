//! Apple ID / authentication stub handler.
//!
//! Endpoints:
//! - `/api/v1/auth/gsa/*`   — gsa.apple.com (Apple ID auth gateway)
//! - `/api/v1/auth/idmsa/*` — idmsa.apple.com (Apple ID auth)

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct AuthHandler;

impl Handler for AuthHandler {
    fn handle(&self, path: &str, _method: &str, _body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/auth/") {
            return None;
        }

        log::info!("burdettie-services: auth {path}");

        // Return an "unauthorized" response — tools should handle this
        // gracefully without crashing.  This is safer than returning fake
        // credentials.
        Some(HandlerResponse::with_status(
            401,
            "application/json",
            br#"{"error":"authentication_required","message":"BurdettieOS does not provide Apple ID authentication"}"#.to_vec(),
        ))
    }
}
