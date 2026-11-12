//! Configuration endpoint handler — serves configuration plists.
//!
//! Endpoints:
//! - `/api/v1/config`   — configuration.apple.com (Rosetta 2, system config)
//! - `/api/v1/config/*` — sub-paths for specific config keys

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct ConfigHandler;

impl Handler for ConfigHandler {
    fn handle(
        &self,
        path: &str,
        _method: &str,
        _body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/config") {
            return None;
        }

        log::info!("burdettie-services: config {path}");

        // Return a minimal configuration plist that tells the guest system
        // everything is up to date and no additional components are needed.
        Some(HandlerResponse::plist_xml(CONFIG_PLIST.as_bytes().to_vec()))
    }
}

/// Minimal configuration plist — tells the guest no updates/components needed.
const CONFIG_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>RosettaUpdateAvailable</key>
    <false/>
    <key>AdditionalComponentsAvailable</key>
    <false/>
</dict>
</plist>"#;
