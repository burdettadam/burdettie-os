//! macOS Recovery handler.
//!
//! `osrecovery.apple.com` and `oscdn.apple.com` serve macOS Recovery images,
//! chunklists, and BaseSystem.  We return "no recovery images available"
//! rather than serving multi-GB downloads.
//!
//! Endpoints:
//! - `/api/v1/recovery`     — osrecovery.apple.com
//! - `/api/v1/recovery/cdn` — oscdn.apple.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct RecoveryHandler;

impl Handler for RecoveryHandler {
    fn handle(
        &self,
        path: &str,
        _method: &str,
        _body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/recovery") {
            return None;
        }

        log::debug!("burdettie-services: recovery {path}");

        // Return an empty recovery catalog
        Some(HandlerResponse::plist_xml(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>InstallationRecoveryImages</key>
    <array/>
</dict>
</plist>"#.to_vec(),
        ))
    }
}
