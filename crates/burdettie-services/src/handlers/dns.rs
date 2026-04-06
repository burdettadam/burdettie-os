//! DNS-over-HTTPS (DoH) handler.
//!
//! Endpoints:
//! - `/api/v1/dns/resolve` — doh.dns.apple.com equivalent
//!
//! Returns a minimal DNS JSON response for queries routed here.
//! Apple's DoH uses the `application/dns-json` format (RFC 8484 JSON variant).

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct DnsHandler;

impl Handler for DnsHandler {
    fn handle(&self, path: &str, method: &str, _body: &[u8], request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/dns/") {
            return None;
        }

        log::info!("burdettie-services: dns {method} {path}");

        // Extract the "name" query parameter if present
        let name = extract_query_param(request.url(), "name")
            .unwrap_or_else(|| "localhost".to_string());

        // Return a minimal DNS JSON response resolving everything to 127.0.0.1
        // This keeps guest DNS queries fully sandboxed.
        let response = serde_json::json!({
            "Status": 0,
            "TC": false,
            "RD": true,
            "RA": true,
            "AD": false,
            "CD": false,
            "Question": [{
                "name": name,
                "type": 1
            }],
            "Answer": [{
                "name": name,
                "type": 1,
                "TTL": 300,
                "data": "127.0.0.1"
            }]
        });

        Some(HandlerResponse::ok(
            "application/dns-json",
            response.to_string().into_bytes(),
        ))
    }
}

/// Extract a query parameter value from a URL path+query string.
fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next() == Some(key) {
            return kv.next().map(|v| v.to_string());
        }
    }
    None
}
