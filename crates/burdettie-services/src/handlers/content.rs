//! Content / App Store handler.
//!
//! Handles traffic from iTunes, App Store, MZStatic, and enterprise app
//! validation (PPQ) domains.  Returns "not available" responses — we don't
//! serve App Store content.
//!
//! Endpoints:
//! - `/api/v1/content/itunes`   — *.itunes.apple.com
//! - `/api/v1/content/apps`     — *.apps.apple.com
//! - `/api/v1/content/mzstatic` — *.mzstatic.com
//! - `/api/v1/apps/validate`    — ppq.apple.com (enterprise validation)

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct ContentHandler;

impl Handler for ContentHandler {
    fn handle(
        &self,
        path: &str,
        _method: &str,
        _body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        let is_content = path.starts_with("/api/v1/content/");
        let is_validate = path.starts_with("/api/v1/apps/validate");

        if !is_content && !is_validate {
            return None;
        }

        log::debug!("burdettie-services: content {path}");

        if is_validate {
            // Enterprise app validation — report as valid
            return Some(HandlerResponse::json(
                br#"{"status":"valid","message":"Enterprise app accepted by BurdettieOS","server":"burdettie"}"#.to_vec(),
            ));
        }

        if path.starts_with("/api/v1/content/mzstatic") {
            // Static assets — 404 (we don't serve actual App Store binaries)
            return Some(HandlerResponse::with_status(
                404,
                "application/json",
                br#"{"error":"not_found","message":"MZStatic content not available in BurdettieOS","server":"burdettie"}"#.to_vec(),
            ));
        }

        // Generic iTunes / App Store response
        Some(HandlerResponse::json(
            br#"{"status":"not_available","storefront":"burdettie","results":[],"server":"burdettie"}"#.to_vec(),
        ))
    }
}
