//! Captive portal / connectivity check handler.
//!
//! Apple's captive portal check (`captive.apple.com`) expects a specific HTML
//! response.  Returning it tells the OS "you have internet connectivity."
//!
//! Endpoints:
//! - `/api/v1/network/captive`  — captive.apple.com
//! - `/api/v1/network/probe`   — probe.icloud.com
//! - `/api/v1/network/pong`    — pong.icloud.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

/// The exact HTML response Apple expects from captive.apple.com.
/// Source: <https://captive.apple.com/> returns exactly this.
const CAPTIVE_SUCCESS_HTML: &[u8] =
    b"<HTML><HEAD><TITLE>Success</TITLE></HEAD><BODY>Success</BODY></HTML>";

pub struct CaptiveHandler;

impl Handler for CaptiveHandler {
    fn handle(
        &self,
        path: &str,
        _method: &str,
        _body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/network/") {
            return None;
        }

        log::debug!("burdettie-services: connectivity check {path}");

        // All connectivity endpoints return "Success" — this tells the guest
        // it has internet access.
        Some(HandlerResponse::html(CAPTIVE_SUCCESS_HTML.to_vec()))
    }
}
