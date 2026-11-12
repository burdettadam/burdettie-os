//! Software update catalog handler.
//!
//! Returns minimal valid catalog plists so tools like `softwareupdate` don't
//! error out.
//!
//! Endpoints:
//! - `/api/v1/swupdate/scan`  — primary catalog scan (sucatalog format)
//! - `/api/v1/swupdate/*`     — all software update domains

use super::{Handler, HandlerResponse};
use tiny_http::Request;

/// Minimal empty software update catalog plist (sucatalog format).
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
    <key>IndexDate</key>
    <date>2026-04-06T00:00:00Z</date>
</dict>
</plist>"#;

/// Configuration plist for Rosetta 2 / system config endpoint.
///
/// This is the merged version: contains both SwupdateHandler's `RosettaEnabled`
/// and ConfigHandler's `RosettaUpdateAvailable` + `AdditionalComponentsAvailable`.
const CONFIG_PLIST: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>RosettaEnabled</key>
    <false/>
    <key>RosettaUpdateAvailable</key>
    <false/>
    <key>AdditionalComponentsAvailable</key>
    <false/>
</dict>
</plist>"#;

pub struct SwupdateHandler;

impl Handler for SwupdateHandler {
    fn handle(
        &self,
        path: &str,
        method: &str,
        body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if path.starts_with("/api/v1/swupdate/") {
            log::info!(
                "burdettie-services: swupdate {method} {path} ({} bytes)",
                body.len()
            );
            return Some(HandlerResponse::plist_xml(EMPTY_CATALOG_PLIST.to_vec()));
        }

        if path.starts_with("/api/v1/config") {
            log::info!("burdettie-services: config {method} {path}");
            return Some(HandlerResponse::plist_xml(CONFIG_PLIST.to_vec()));
        }

        None
    }
}
