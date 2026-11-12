//! Diagnostics assets handler.
//!
//! `diagassets.apple.com` serves hardware diagnostics manifests as plists.
//! We return an empty manifest — no diagnostics assets available.
//!
//! Endpoint:
//! - `/api/v1/diagnostics/assets` — diagassets.apple.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct DiagnosticsHandler;

impl Handler for DiagnosticsHandler {
    fn handle(
        &self,
        path: &str,
        _method: &str,
        _body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/diagnostics/") {
            return None;
        }

        log::debug!("burdettie-services: diagnostics {path}");

        // Return an empty diagnostics asset manifest
        Some(HandlerResponse::plist_xml(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Assets</key>
    <array/>
    <key>AssetGeneration</key>
    <integer>0</integer>
</dict>
</plist>"#.to_vec(),
        ))
    }
}
