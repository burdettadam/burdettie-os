//! DNS-over-HTTPS (DoH) handler.
//!
//! Endpoints:
//! - `/api/v1/dns/resolve` — doh.dns.apple.com equivalent
//!
//! Returns DNS JSON responses in the RFC 8484 JSON variant that Apple's
//! DoH resolver uses.  Supports:
//! - A records (type 1)   → 127.0.0.1
//! - AAAA records (type 28) → ::1
//! - CNAME records (type 5) → returned as prefix for apple.com domains
//! - NXDOMAIN (Status 3)   → for explicitly blocked domains
//!
//! All responses are sandboxed — guest DNS never reaches the internet.

use super::{Handler, HandlerResponse};
use tiny_http::Request;

/// DNS record types
const DNS_TYPE_A: u32 = 1;
const DNS_TYPE_CNAME: u32 = 5;
const DNS_TYPE_AAAA: u32 = 28;

/// Domains that should return NXDOMAIN (blocked).
const NXDOMAIN_PATTERNS: &[&str] = &[
    "telemetry.apple.com",
    "xp.apple.com",
    "metrics.apple.com",
    "securemetrics.apple.com",
];

pub struct DnsHandler;

impl Handler for DnsHandler {
    fn handle(
        &self,
        path: &str,
        method: &str,
        _body: &[u8],
        request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/api/v1/dns/") {
            return None;
        }

        super::status::inc_dns();

        log::info!("burdettie-services: dns {method} {path}");

        let name =
            extract_query_param(request.url(), "name").unwrap_or_else(|| "localhost".to_string());

        let qtype: u32 = extract_query_param(request.url(), "type")
            .and_then(|t| t.parse().ok())
            .unwrap_or(DNS_TYPE_A);

        let response = build_dns_response(&name, qtype);

        Some(HandlerResponse::ok(
            "application/dns-json",
            response.to_string().into_bytes(),
        ))
    }
}

/// Build a complete DNS JSON response.
fn build_dns_response(name: &str, qtype: u32) -> serde_json::Value {
    // Check NXDOMAIN first
    let lower = name.to_ascii_lowercase();
    if NXDOMAIN_PATTERNS
        .iter()
        .any(|p| lower == *p || lower.ends_with(&format!(".{p}")))
    {
        return build_nxdomain(name, qtype);
    }

    let mut answers = Vec::new();

    // If the name looks like a CDN alias (e.g., *.apple.com.edgekey.net),
    // add a CNAME record chaining back to the canonical name.
    let canonical = resolve_cname(name);
    if let Some(ref cname_target) = canonical {
        answers.push(serde_json::json!({
            "name": name,
            "type": DNS_TYPE_CNAME,
            "TTL": 3600,
            "data": cname_target
        }));
    }
    let resolved_name = canonical.as_deref().unwrap_or(name);

    match qtype {
        DNS_TYPE_A => {
            answers.push(serde_json::json!({
                "name": resolved_name,
                "type": DNS_TYPE_A,
                "TTL": 300,
                "data": "127.0.0.1"
            }));
        }
        DNS_TYPE_AAAA => {
            answers.push(serde_json::json!({
                "name": resolved_name,
                "type": DNS_TYPE_AAAA,
                "TTL": 300,
                "data": "::1"
            }));
        }
        _ => {
            // For any other type, return A record as fallback
            answers.push(serde_json::json!({
                "name": resolved_name,
                "type": DNS_TYPE_A,
                "TTL": 300,
                "data": "127.0.0.1"
            }));
        }
    }

    serde_json::json!({
        "Status": 0,
        "TC": false,
        "RD": true,
        "RA": true,
        "AD": false,
        "CD": false,
        "Question": [{
            "name": name,
            "type": qtype
        }],
        "Answer": answers
    })
}

/// Build an NXDOMAIN response (Status 3, no Answer section).
fn build_nxdomain(name: &str, qtype: u32) -> serde_json::Value {
    serde_json::json!({
        "Status": 3,
        "TC": false,
        "RD": true,
        "RA": true,
        "AD": false,
        "CD": false,
        "Question": [{
            "name": name,
            "type": qtype
        }],
        "Authority": [{
            "name": ".",
            "type": 6,
            "TTL": 1800,
            "data": "a.root-servers.net. nstld.verisign-grs.com. 2024040600 1800 900 604800 86400"
        }]
    })
}

/// Resolve CDN-style CNAME aliases.
///
/// Apple uses CDN edge domains like `*.akamaiedge.net` and `*.apple.com.edgekey.net`.
/// We chain these back to the canonical Apple domain.
fn resolve_cname(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".edgekey.net")
        || lower.ends_with(".akamaiedge.net")
        || lower.ends_with(".cloudfront.net")
        || lower.ends_with(".akadns.net")
    {
        // Strip the CDN suffix and try to recover a plausible Apple domain
        if lower.contains("apple") {
            return Some("apple.com".to_string());
        }
        return Some(format!("{lower}.burdettie-cdn.local"));
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_resolves_to_localhost() {
        let resp = build_dns_response("developer.apple.com", DNS_TYPE_A);
        assert_eq!(resp["Status"], 0);
        assert_eq!(resp["Answer"][0]["data"], "127.0.0.1");
        assert_eq!(resp["Answer"][0]["type"], DNS_TYPE_A);
    }

    #[test]
    fn aaaa_record_resolves_to_ipv6_localhost() {
        let resp = build_dns_response("developer.apple.com", DNS_TYPE_AAAA);
        assert_eq!(resp["Status"], 0);
        assert_eq!(resp["Answer"][0]["data"], "::1");
        assert_eq!(resp["Answer"][0]["type"], DNS_TYPE_AAAA);
    }

    #[test]
    fn nxdomain_for_telemetry() {
        let resp = build_dns_response("telemetry.apple.com", DNS_TYPE_A);
        assert_eq!(resp["Status"], 3);
        assert!(resp.get("Answer").is_none());
        assert!(resp["Authority"].is_array());
    }

    #[test]
    fn nxdomain_for_telemetry_subdomain() {
        let resp = build_dns_response("collector.telemetry.apple.com", DNS_TYPE_A);
        assert_eq!(resp["Status"], 3);
    }

    #[test]
    fn cname_for_cdn_domain() {
        let resp = build_dns_response("developer.apple.com.edgekey.net", DNS_TYPE_A);
        assert_eq!(resp["Status"], 0);
        // First answer should be CNAME
        assert_eq!(resp["Answer"][0]["type"], DNS_TYPE_CNAME);
        // Second answer should be A record
        assert_eq!(resp["Answer"][1]["type"], DNS_TYPE_A);
        assert_eq!(resp["Answer"][1]["data"], "127.0.0.1");
    }

    #[test]
    fn extract_query_params_work() {
        assert_eq!(
            extract_query_param("/resolve?name=foo.com&type=28", "name"),
            Some("foo.com".to_string())
        );
        assert_eq!(
            extract_query_param("/resolve?name=foo.com&type=28", "type"),
            Some("28".to_string())
        );
        assert_eq!(extract_query_param("/resolve?name=foo.com", "type"), None);
    }
}
