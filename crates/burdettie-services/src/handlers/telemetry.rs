//! Telemetry handler — accepts and records Apple diagnostics/analytics traffic.
//!
//! Endpoints:
//! - `/api/v1/telemetry/xp/*`     — xp.apple.com (crash reports, install events)
//! - `/api/v1/telemetry/metrics/*` — metrics.icloud.com
//! - `/api/v1/telemetry/submissions/*` — iphonesubmissions.apple.com
//! - `/api/v1/telemetry/xp-cdn/*` — xp-cdn.apple.com (content caching reports)

use super::{Handler, HandlerResponse};
use tiny_http::Request;

/// Accepts all telemetry POSTs and returns 200.  The request body is stored
/// by the recorder for later protocol analysis.
pub struct TelemetryHandler;

impl Handler for TelemetryHandler {
    fn handle(
        &self,
        path: &str,
        method: &str,
        body: &[u8],
        _request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/telemetry/") {
            return None;
        }

        super::status::inc_telemetry();

        log::info!(
            "burdettie-services: telemetry {method} {path} ({} bytes)",
            body.len()
        );

        // Accept everything — Apple telemetry endpoints return 200 with a small
        // JSON acknowledgement.
        Some(HandlerResponse::json(
            br#"{"status":"ok","server":"burdettie"}"#.to_vec(),
        ))
    }
}
