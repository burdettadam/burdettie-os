//! iCloud services handler.
//!
//! Catches traffic destined for iCloud, iCloud-content, CloudKit, and
//! the iCloud gateway.  For CloudKit notarization paths (ticket lookup,
//! submission), delegates to notarization-compatible responses.
//!
//! Endpoints:
//! - `/api/v1/icloud`          — *.icloud.com wildcard
//! - `/api/v1/icloud/content`  — *.icloud-content.com
//! - `/api/v1/icloud/cloudkit` — *.apple-cloudkit.com (incl. notarization)
//! - `/api/v1/icloud/gateway`  — gateway.icloud.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct ICloudHandler;

impl Handler for ICloudHandler {
    fn handle(
        &self,
        path: &str,
        method: &str,
        body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/icloud") {
            return None;
        }

        log::info!(
            "burdettie-services: icloud {method} {path} ({} bytes)",
            body.len()
        );

        // CloudKit notarization — ticket lookup
        if path.contains("/tickets/lookup") && method == "POST" {
            return Some(handle_cloudkit_ticket_lookup(body));
        }

        // CloudKit notarization — ticket retrieval
        if path.contains("/tickets/") && method == "GET" {
            return Some(HandlerResponse::json(
                br#"{"status":"valid","statusCode":0,"statusMessage":"Package Approved","server":"burdettie"}"#.to_vec(),
            ));
        }

        // CloudKit notarization — submission
        if path.contains("/submit") && method == "POST" {
            return Some(HandlerResponse::json(
                br#"{"status":"Accepted","id":"burdettie-notarization-0001","message":"Package accepted by BurdettieOS","server":"burdettie"}"#.to_vec(),
            ));
        }

        // iCloud content / gateway — return empty container list
        if path.starts_with("/api/v1/icloud/content") {
            return Some(HandlerResponse::json(
                br#"{"tokens":[],"server":"burdettie"}"#.to_vec(),
            ));
        }

        if path.starts_with("/api/v1/icloud/gateway") {
            return Some(HandlerResponse::json(
                br#"{"status":"ok","containers":[],"server":"burdettie"}"#.to_vec(),
            ));
        }

        // Generic iCloud — not signed in, no containers
        Some(HandlerResponse::json(
            br#"{"accountStatus":"NOT_AVAILABLE","containers":[],"server":"burdettie"}"#.to_vec(),
        ))
    }
}

/// Handle a CloudKit notarization ticket lookup by CDHash.
fn handle_cloudkit_ticket_lookup(body: &[u8]) -> HandlerResponse {
    // Try to extract CDHash from the JSON request body
    let cdhash = extract_cdhash(body)
        .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());

    // Build a signed ticket as a base64-encoded plist
    let ticket_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>cdhash</key>
    <string>{cdhash}</string>
    <key>ticketVersion</key>
    <integer>2</integer>
    <key>issuer</key>
    <string>BurdettieOS Notary</string>
    <key>status</key>
    <integer>0</integer>
</dict>
</plist>"#
    );

    let ticket_b64 = base64_encode(ticket_plist.as_bytes());

    let response = serde_json::json!({
        "records": [{
            "recordName": format!("2/{cdhash}"),
            "fields": {
                "signedTicket": {
                    "value": ticket_b64,
                    "type": "BYTES",
                },
            },
        }],
        "status": "ACCEPTED",
    });

    HandlerResponse::json(response.to_string().into_bytes())
}

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(B64_CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(B64_CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(B64_CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(B64_CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Extract CDHash from CloudKit-style JSON request body.
fn extract_cdhash(body: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    // Try CloudKit format: {"records":[{"recordName":"2/<cdhash>"}]}
    let records = v.get("records")?.as_array()?;
    let name = records.first()?.get("recordName")?.as_str()?;
    let hash = name.strip_prefix("2/")?;
    Some(hash.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icloud_generic_returns_not_available() {
        let h = ICloudHandler;
        let req = tiny_http::TestRequest::new().into();
        let resp = h.handle("/api/v1/icloud", "GET", b"", &req).unwrap();
        assert_eq!(resp.status_code, 200);
        let body = String::from_utf8(resp.body).unwrap();
        assert!(body.contains("NOT_AVAILABLE"));
    }

    #[test]
    fn cloudkit_ticket_lookup() {
        let h = ICloudHandler;
        let body = br#"{"records":[{"recordName":"2/abc123"}]}"#;
        let req = tiny_http::TestRequest::new().into();
        let resp = h
            .handle("/api/v1/icloud/cloudkit/tickets/lookup", "POST", body, &req)
            .unwrap();
        assert_eq!(resp.status_code, 200);
        let body = String::from_utf8(resp.body).unwrap();
        assert!(body.contains("ACCEPTED"));
        assert!(body.contains("abc123"));
    }

    #[test]
    fn content_returns_empty_tokens() {
        let h = ICloudHandler;
        let req = tiny_http::TestRequest::new().into();
        let resp = h
            .handle("/api/v1/icloud/content", "GET", b"", &req)
            .unwrap();
        let body = String::from_utf8(resp.body).unwrap();
        assert!(body.contains("tokens"));
    }

    #[test]
    fn ignores_non_icloud_paths() {
        let h = ICloudHandler;
        let req = tiny_http::TestRequest::new().into();
        assert!(h.handle("/api/v1/telemetry/xp", "GET", b"", &req).is_none());
    }
}
