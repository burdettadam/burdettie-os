//! Certificate validation handler (OCSP/CRL stubs).
//!
//! Endpoints:
//! - `/api/v1/certs/ocsp`    — ocsp.apple.com, ocsp2.apple.com
//! - `/api/v1/certs/crl`     — crl.apple.com
//! - `/api/v1/certs/validate` — certs.apple.com, valid.apple.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct CertsHandler;

impl Handler for CertsHandler {
    fn handle(&self, path: &str, method: &str, body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/certs/") {
            return None;
        }

        log::info!(
            "burdettie-services: certs {method} {path} ({} bytes)",
            body.len()
        );

        if path.starts_with("/api/v1/certs/ocsp") {
            // OCSP: return "good" status.
            // A minimal OCSP response indicating the cert is valid.
            // Real OCSP responses are ASN.1-encoded — for now return a stub
            // that records the request for protocol analysis.
            Some(HandlerResponse::ok(
                "application/ocsp-response",
                // OCSP "successful" response (status=0) — minimal DER stub
                // The recorder captures the full request for later analysis.
                vec![0x30, 0x03, 0x0a, 0x01, 0x00], // SEQUENCE { ENUM { 0 } } = successful
            ))
        } else if path.starts_with("/api/v1/certs/crl") {
            // CRL: return empty CRL.
            Some(HandlerResponse::ok(
                "application/pkix-crl",
                Vec::new(),
            ))
        } else {
            // Generic cert validation: return 200.
            Some(HandlerResponse::json(
                br#"{"status":"valid"}"#.to_vec(),
            ))
        }
    }
}
