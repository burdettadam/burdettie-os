//! Notarization handler — Apple notarization ticket validation.
//!
//! Apple's notarization service (`api.apple-cloudkit.com`) verifies that a
//! binary has been scanned for malware and issued a ticket.  Tools like
//! `stapler` and `spctl` query this endpoint with a CDHash (SHA-256 of the
//! code directory).
//!
//! We return protocol-correct responses that indicate all binaries are
//! notarized.  This prevents Gatekeeper-style blocks in guest tools.
//!
//! Endpoints:
//! - `POST /api/v1/notarize/tickets/lookup`  — CDHash → ticket lookup
//! - `GET  /api/v1/notarize/tickets/<id>`    — ticket retrieval
//! - `POST /api/v1/notarize/submit`          — binary submission (accepted)
//! - `/api/v1/notarize/*`                    — generic notarization

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct NotarizeHandler;

impl Handler for NotarizeHandler {
    fn handle(
        &self,
        path: &str,
        method: &str,
        body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/notarize") {
            return None;
        }

        super::status::inc_notarize();

        log::info!(
            "burdettie-services: notarize {method} {path} ({} bytes)",
            body.len()
        );

        if path.starts_with("/api/v1/notarize/tickets/lookup") {
            return Some(handle_ticket_lookup(body));
        }

        if path.starts_with("/api/v1/notarize/tickets/") {
            return Some(handle_ticket_get(path));
        }

        if path.starts_with("/api/v1/notarize/submit") {
            return Some(handle_submit());
        }

        // Generic notarization acknowledgement
        Some(HandlerResponse::json(
            br#"{"status":"success","message":"Notarization accepted by BurdettieOS","server":"burdettie"}"#.to_vec(),
        ))
    }
}

/// Handle a ticket lookup by CDHash.
///
/// Request body is JSON: `{"records":{"recordName":"2/<cdhash>"}}`
///
/// Returns an Apple-compatible CloudKit response with a signed ticket.
fn handle_ticket_lookup(body: &[u8]) -> HandlerResponse {
    // Try to extract the CDHash from the request
    let cdhash = extract_cdhash(body).unwrap_or_else(|| "0".repeat(64));

    // Build a response in Apple's CloudKit notarization format
    let response = serde_json::json!({
        "records": [{
            "recordName": format!("2/{cdhash}"),
            "created": {
                "timestamp": 1712361600000_u64,
                "userRecordName": "_burdettie_system"
            },
            "fields": {
                "signedTicket": {
                    "value": build_signed_ticket_base64(&cdhash),
                    "type": "BYTES"
                },
                "cdhash": {
                    "value": cdhash,
                    "type": "STRING"
                },
                "status": {
                    "value": "ACCEPTED",
                    "type": "STRING"
                }
            }
        }]
    });

    HandlerResponse::json(response.to_string().into_bytes())
}

/// Handle a ticket retrieval by ID.
fn handle_ticket_get(path: &str) -> HandlerResponse {
    let ticket_id = path
        .strip_prefix("/api/v1/notarize/tickets/")
        .unwrap_or("unknown");

    let response = serde_json::json!({
        "id": ticket_id,
        "status": "valid",
        "statusCode": 0,
        "statusMessage": "Package Approved",
        "server": "burdettie"
    });

    HandlerResponse::json(response.to_string().into_bytes())
}

/// Handle a notarization submission.
fn handle_submit() -> HandlerResponse {
    let response = serde_json::json!({
        "id": "burdettie-notarization-0001",
        "status": "Accepted",
        "statusCode": 0,
        "statusMessage": "Successfully received submission",
        "server": "burdettie"
    });

    HandlerResponse::json(response.to_string().into_bytes())
}

/// Extract CDHash from a JSON request body.
fn extract_cdhash(body: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    // Try Apple's format: records.recordName = "2/<cdhash>"
    let record_name = v
        .pointer("/records/recordName")
        .or_else(|| v.pointer("/records/0/recordName"))
        .and_then(|v| v.as_str())?;
    record_name.strip_prefix("2/").map(|s| s.to_string())
}

/// Build a base64-encoded signed ticket plist.
///
/// Apple's notarization tickets are DER-encoded CMS (PKCS#7) structures
/// containing a binary property list with CDHash and signature info.  We
/// build a minimal binary plist ticket that tools will accept.
fn build_signed_ticket_base64(cdhash: &str) -> String {
    use plist::Value;
    use std::collections::BTreeMap;

    // Build the ticket plist
    let mut ticket = BTreeMap::new();
    ticket.insert("cdhash".to_string(), Value::String(cdhash.to_string()));
    ticket.insert("ticketVersion".to_string(), Value::Integer(2.into()));
    ticket.insert(
        "signedDate".to_string(),
        Value::String("2026-04-06T00:00:00Z".to_string()),
    );
    ticket.insert(
        "issuer".to_string(),
        Value::String("BurdettieOS Notary".to_string()),
    );
    ticket.insert("status".to_string(), Value::Integer(0.into())); // 0 = valid

    let ticket_value = Value::Dictionary(ticket.into_iter().collect());

    // Serialize to binary plist (matches Apple's real ticket format), then base64-encode
    let mut buf = Vec::new();
    ticket_value.to_writer_binary(&mut buf).unwrap_or_default();

    base64_encode(&buf)
}

/// Minimal base64 encoder (no external dependency).
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_lookup_returns_cdhash() {
        let body = br#"{"records":{"recordName":"2/abc123def456"}}"#;
        let resp = handle_ticket_lookup(body);
        let json: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
        assert_eq!(
            json["records"][0]["fields"]["cdhash"]["value"],
            "abc123def456"
        );
        assert_eq!(json["records"][0]["fields"]["status"]["value"], "ACCEPTED");
    }

    #[test]
    fn ticket_lookup_with_missing_cdhash_uses_placeholder() {
        let body = b"{}";
        let resp = handle_ticket_lookup(body);
        let json: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
        // Should have a 64-char zero placeholder
        let cdhash = json["records"][0]["fields"]["cdhash"]["value"]
            .as_str()
            .unwrap();
        assert_eq!(cdhash.len(), 64);
    }

    #[test]
    fn signed_ticket_is_base64_plist() {
        let encoded = build_signed_ticket_base64("deadbeef");
        // Must be valid base64
        assert!(encoded
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));
        assert!(!encoded.is_empty());
    }

    #[test]
    fn base64_encode_basic() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
        assert_eq!(base64_encode(b"Hi"), "SGk=");
        assert_eq!(base64_encode(b"ABC"), "QUJD");
    }

    #[test]
    fn submit_returns_accepted() {
        let resp = handle_submit();
        assert_eq!(resp.status_code, 200);
        let json: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
        assert_eq!(json["status"], "Accepted");
    }
}
