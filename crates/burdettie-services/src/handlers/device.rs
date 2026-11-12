//! Device activation handler — returns success responses for activation checks.
//!
//! Endpoints:
//! - `/api/v1/device/activate` — albert.apple.com (device activation)
//! - `/api/v1/device/ips`     — static.ips.apple.com
//! - `/api/v1/device/tbsc`    — tbsc.apple.com
//! - `/api/v1/device/humb`    — humb.apple.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

/// Apple's activation server returns a plist with activation status.
const ACTIVATION_SUCCESS_PLIST: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Status</key>
    <string>Activated</string>
    <key>AccountTokenCertificate</key>
    <data></data>
    <key>DeviceCertificate</key>
    <data></data>
</dict>
</plist>"#;

pub struct DeviceHandler;

impl Handler for DeviceHandler {
    fn handle(
        &self,
        path: &str,
        method: &str,
        body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/device/") {
            return None;
        }

        log::info!(
            "burdettie-services: device {method} {path} ({} bytes)",
            body.len()
        );

        // Return an activation success plist for all device endpoints.
        Some(HandlerResponse::plist_xml(
            ACTIVATION_SUCCESS_PLIST.to_vec(),
        ))
    }
}
