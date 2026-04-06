//! Xcode component handler — stubs for Xcode download and devimage endpoints.
//!
//! Endpoints:
//! - `/api/v1/xcode/devimages/*`  — devimages-cdn.apple.com (component catalog)
//! - `/api/v1/xcode/download/*`   — download.developer.apple.com
//! - `/api/v1/xcode/*`            — other Xcode endpoints

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct XcodeHandler;

impl Handler for XcodeHandler {
    fn handle(&self, path: &str, method: &str, body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/xcode/") {
            return None;
        }

        log::info!(
            "burdettie-services: xcode {method} {path} ({} bytes)",
            body.len()
        );

        if path.starts_with("/api/v1/xcode/devimages") {
            // Return an empty component catalog — no additional downloads needed.
            return Some(HandlerResponse::plist_xml(
                EMPTY_DEVIMAGES_PLIST.as_bytes().to_vec(),
            ));
        }

        if path.starts_with("/api/v1/xcode/download") {
            // download.developer.apple.com — return 404 (no packages available)
            return Some(HandlerResponse::with_status(
                404,
                "application/json",
                br#"{"error":"not_available","message":"Package not available on BurdettieOS","server":"burdettie"}"#.to_vec(),
            ));
        }

        // Generic Xcode endpoint acknowledgement
        Some(HandlerResponse::json(
            br#"{"status":"ok","server":"burdettie","service":"xcode"}"#.to_vec(),
        ))
    }
}

/// Minimal plist that says "no components need downloading".
const EMPTY_DEVIMAGES_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>images</key>
    <array/>
    <key>version</key>
    <integer>1</integer>
    <key>sdks</key>
    <dict/>
</dict>
</plist>"#;
