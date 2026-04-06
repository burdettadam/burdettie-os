//! Catch-all handler for any endpoint not matched by a specific handler.
//!
//! Records the request and returns a generic 200 so the guest doesn't error out.

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct CatchallHandler;

impl Handler for CatchallHandler {
    fn handle(&self, path: &str, method: &str, body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        // Accept everything that starts with /api/v1/
        if !path.starts_with("/api/v1/") {
            return None;
        }

        log::info!(
            "burdettie-services: catchall {method} {path} ({} bytes)",
            body.len()
        );

        Some(HandlerResponse::json(
            br#"{"status":"ok","handler":"catchall","server":"burdettie"}"#.to_vec(),
        ))
    }
}
