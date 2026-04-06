//! Software update catalog handler.
//!
//! Returns minimal valid catalog plists so tools like `softwareupdate` don't
//! error out.
//!
//! Endpoints:
//! - `/api/v1/swupdate/*` — all software update domains

use super::{Handler, HandlerResponse};
use tiny_http::Request;

/// Minimal empty software update catalog plist.
const EMPTY_CATALOG_PLIST: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CatalogVersion</key>
    <integer>2</integer>
    <key>ApplePostURL</key>
    <string>https://swscan.apple.com/report</string>
    <key>Products</key>
    <dict/>
</dict>
</plist>"#;

pub struct SwupdateHandler;

impl Handler for SwupdateHandler {
    fn handle(&self, path: &str, method: &str, body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/swupdate/") {
            return None;
        }

        log::info!(
            "burdettie-services: swupdate {method} {path} ({} bytes)",
            body.len()
        );

        // Return an empty catalog — no updates available.
        Some(HandlerResponse::plist_xml(EMPTY_CATALOG_PLIST.to_vec()))
    }
}
