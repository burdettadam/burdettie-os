//! Notarization handler — stubs for Apple notarization service.
//!
//! Endpoints:
//! - `/api/v1/notarize`   — api.apple-cloudkit.com (notarization tickets)
//! - `/api/v1/notarize/*` — sub-paths for ticket lookup/validation

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct NotarizeHandler;

impl Handler for NotarizeHandler {
    fn handle(&self, path: &str, method: &str, _body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/notarize") {
            return None;
        }

        log::info!("burdettie-services: notarize {method} {path}");

        // Return a "notarized" response — tools checking notarization should
        // see this as valid so they don't block execution.
        Some(HandlerResponse::json(
            br#"{"status":"success","message":"Notarization accepted by BurdettieOS","ticket_status":"valid"}"#.to_vec(),
        ))
    }
}
