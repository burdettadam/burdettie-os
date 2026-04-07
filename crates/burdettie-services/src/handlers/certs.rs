//! Certificate validation handler — OCSP responder and CRL endpoint.
//!
//! Provides protocol-correct responses for Apple's certificate validation:
//!
//! - OCSP: RFC 6960 `OCSPResponse` with `certStatus=good` (DER-encoded)
//! - CRL:  X.509 v2 empty CRL with BurdettieOS issuer (DER-encoded)
//! - Validate: JSON success for generic cert validation endpoints
//!
//! Endpoints:
//! - `/api/v1/certs/ocsp`     — ocsp.apple.com, ocsp2.apple.com
//! - `/api/v1/certs/crl`      — crl.apple.com
//! - `/api/v1/certs/validate`  — certs.apple.com, valid.apple.com

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

        super::status::inc_certs();

        log::info!(
            "burdettie-services: certs {method} {path} ({} bytes)",
            body.len()
        );

        if path.starts_with("/api/v1/certs/ocsp") {
            Some(HandlerResponse::ok(
                "application/ocsp-response",
                build_ocsp_good_response(body),
            ))
        } else if path.starts_with("/api/v1/certs/crl") {
            Some(HandlerResponse::ok(
                "application/pkix-crl",
                build_empty_crl(),
            ))
        } else {
            Some(HandlerResponse::json(
                br#"{"status":"valid","server":"burdettie"}"#.to_vec(),
            ))
        }
    }
}

// ── DER construction helpers ────────────────────────────────────

/// Wrap `content` in a DER SEQUENCE tag (0x30).
fn der_sequence(content: &[u8]) -> Vec<u8> {
    der_wrap(0x30, content)
}

/// Wrap `content` in a DER context-specific CONSTRUCTED tag [n].
fn der_context(tag_num: u8, content: &[u8]) -> Vec<u8> {
    der_wrap(0xa0 | tag_num, content)
}

/// Wrap `content` in a DER tag with proper length encoding.
fn der_wrap(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    der_push_length(&mut out, content.len());
    out.extend_from_slice(content);
    out
}

/// Encode a DER length (supports short and long form up to 64K).
fn der_push_length(out: &mut Vec<u8>, len: usize) {
    if len < 0x80 {
        out.push(len as u8);
    } else if len < 0x100 {
        out.push(0x81);
        out.push(len as u8);
    } else {
        out.push(0x82);
        out.push((len >> 8) as u8);
        out.push(len as u8);
    }
}

/// DER ENUMERATED value.
fn der_enumerated(val: u8) -> Vec<u8> {
    vec![0x0a, 0x01, val]
}

/// DER INTEGER from bytes (unsigned, adds leading 0x00 if high bit set).
fn der_integer(val: &[u8]) -> Vec<u8> {
    if val.is_empty() {
        return vec![0x02, 0x01, 0x00];
    }
    let needs_pad = val[0] & 0x80 != 0;
    let mut out = vec![0x02];
    let len = val.len() + if needs_pad { 1 } else { 0 };
    der_push_length(&mut out, len);
    if needs_pad {
        out.push(0x00);
    }
    out.extend_from_slice(val);
    out
}

/// DER OCTET STRING.
fn der_octet_string(val: &[u8]) -> Vec<u8> {
    der_wrap(0x04, val)
}

/// DER OID from pre-encoded bytes.
fn der_oid(encoded: &[u8]) -> Vec<u8> {
    der_wrap(0x06, encoded)
}

/// DER GeneralizedTime from a fixed timestamp.
fn der_generalized_time(time_str: &str) -> Vec<u8> {
    let bytes = time_str.as_bytes();
    der_wrap(0x18, bytes)
}

/// DER UTCTime from a fixed timestamp.
fn der_utc_time(time_str: &str) -> Vec<u8> {
    let bytes = time_str.as_bytes();
    der_wrap(0x17, bytes)
}

/// DER BIT STRING (with 0 unused bits).
fn der_bit_string(content: &[u8]) -> Vec<u8> {
    let mut out = vec![0x03];
    der_push_length(&mut out, content.len() + 1);
    out.push(0x00); // 0 unused bits
    out.extend_from_slice(content);
    out
}

/// DER PrintableString.
fn der_printable_string(s: &str) -> Vec<u8> {
    der_wrap(0x13, s.as_bytes())
}

/// DER SET wrapping.
fn der_set(content: &[u8]) -> Vec<u8> {
    der_wrap(0x31, content)
}

/// Build a minimal RDN for "CN=BurdettieOS CA".
fn burdettie_issuer_name() -> Vec<u8> {
    // OID 2.5.4.3 = commonName
    let cn_oid = der_oid(&[0x55, 0x04, 0x03]);
    let cn_val = der_printable_string("BurdettieOS CA");
    let attr_pair = der_sequence(&[cn_oid.as_slice(), cn_val.as_slice()].concat());
    let rdn = der_set(&attr_pair);
    der_sequence(&rdn)
}

// ── OCSP Response (RFC 6960) ────────────────────────────────────

/// Build a complete RFC 6960 OCSPResponse with responseStatus=successful (0)
/// and a BasicOCSPResponse containing a single certStatus=good response.
///
/// Structure:
/// ```text
/// OCSPResponse ::= SEQUENCE {
///   responseStatus  ENUMERATED { successful (0) },
///   responseBytes   [0] EXPLICIT SEQUENCE {
///     responseType  OID (id-pkix-ocsp-basic),
///     response      OCTET STRING (BasicOCSPResponse DER)
///   }
/// }
/// ```
fn build_ocsp_good_response(request_body: &[u8]) -> Vec<u8> {
    let basic = build_basic_ocsp_response(request_body);

    // OID 1.3.6.1.5.5.7.48.1.1 = id-pkix-ocsp-basic
    let response_type = der_oid(&[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x01]);
    let response_octets = der_octet_string(&basic);

    let response_bytes = der_sequence(&[response_type.as_slice(), response_octets.as_slice()].concat());
    let response_bytes_tagged = der_context(0, &response_bytes);

    let status = der_enumerated(0); // successful
    der_sequence(&[status.as_slice(), response_bytes_tagged.as_slice()].concat())
}

/// Build a BasicOCSPResponse with one SingleResponse (certStatus=good).
///
/// ```text
/// BasicOCSPResponse ::= SEQUENCE {
///   tbsResponseData   ResponseData,
///   signatureAlgorithm AlgorithmIdentifier,
///   signature          BIT STRING
/// }
/// ```
fn build_basic_ocsp_response(request_body: &[u8]) -> Vec<u8> {
    let tbs = build_response_data(request_body);

    // AlgorithmIdentifier: sha256WithRSAEncryption (1.2.840.113549.1.1.11)
    let sig_alg_oid = der_oid(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b]);
    let sig_alg = der_sequence(&[sig_alg_oid.as_slice(), &[0x05, 0x00]].concat()); // + NULL params

    // Dummy signature (64 bytes of zeros) — no client actually verifies these
    // since the cert chain terminates at a BurdettieOS root
    let dummy_sig = der_bit_string(&[0u8; 64]);

    der_sequence(&[tbs.as_slice(), sig_alg.as_slice(), dummy_sig.as_slice()].concat())
}

/// Build the ResponseData (the signed portion).
///
/// ```text
/// ResponseData ::= SEQUENCE {
///   version           [0] EXPLICIT INTEGER DEFAULT v1,
///   responderID       ResponderID,
///   producedAt        GeneralizedTime,
///   responses         SEQUENCE OF SingleResponse
/// }
/// ```
fn build_response_data(request_body: &[u8]) -> Vec<u8> {
    // ResponderID ::= CHOICE { byName [1] Name }
    let name = burdettie_issuer_name();
    let responder_id = der_context(1, &name);

    // ProducedAt
    let produced_at = der_generalized_time("20260406000000Z");

    // SingleResponse — certStatus=good, echoing the request's CertID
    let single = build_single_response_good(request_body);
    let responses = der_sequence(&single);

    der_sequence(
        &[responder_id.as_slice(), produced_at.as_slice(), responses.as_slice()].concat(),
    )
}

/// Build a SingleResponse with certStatus=good.
///
/// ```text
/// SingleResponse ::= SEQUENCE {
///   certID       CertID,
///   certStatus   CertStatus (good [0] IMPLICIT NULL),
///   thisUpdate   GeneralizedTime,
///   nextUpdate   [0] EXPLICIT GeneralizedTime OPTIONAL
/// }
/// ```
fn build_single_response_good(request_body: &[u8]) -> Vec<u8> {
    let cert_id = extract_cert_id_from_request(request_body)
        .unwrap_or_else(build_cert_id_placeholder);

    // certStatus: good = [0] IMPLICIT NULL (tag 0x80, length 0)
    let cert_status_good: &[u8] = &[0x80, 0x00];

    let this_update = der_generalized_time("20260406000000Z");
    let next_update = der_context(0, &der_generalized_time("20260407000000Z"));

    der_sequence(
        &[
            cert_id.as_slice(),
            cert_status_good,
            this_update.as_slice(),
            next_update.as_slice(),
        ]
        .concat(),
    )
}

/// Extract the CertID from a DER-encoded OCSPRequest body.
///
/// OCSPRequest → TBSRequest → requestList[0] → reqCert (CertID)
/// The CertID is a SEQUENCE that we echo back verbatim in the response.
fn extract_cert_id_from_request(body: &[u8]) -> Option<Vec<u8>> {
    // Walk: OCSPRequest(SEQUENCE) → TBSRequest(SEQUENCE) → requestList(SEQUENCE) → Request(SEQUENCE) → CertID(SEQUENCE)
    let ocsp_req = der_unwrap_sequence(body)?;
    let tbs_req = der_unwrap_sequence(ocsp_req)?;
    // Skip optional version [0] tag if present
    let rest = if !tbs_req.is_empty() && tbs_req[0] == 0xa0 {
        let (_, after_version) = der_read_tlv(tbs_req)?;
        after_version
    } else {
        tbs_req
    };
    let request_list_content = der_unwrap_sequence(rest)?;
    let request_content = der_unwrap_sequence(request_list_content)?;
    // reqCert is the first element — a SEQUENCE (the CertID itself)
    // Return the full TLV bytes (tag + length + content)
    let hlen = der_header_len(request_content)?;
    let clen = der_content_len(request_content)?;
    let total = hlen + clen;
    if total > request_content.len() { return None; }
    Some(request_content[..total].to_vec())
}

/// Unwrap the contents of a DER SEQUENCE (tag 0x30).
fn der_unwrap_sequence(data: &[u8]) -> Option<&[u8]> {
    if data.is_empty() || data[0] != 0x30 { return None; }
    let (content, _) = der_read_tlv(data)?;
    Some(content)
}

/// Read a DER TLV: returns (content_slice, rest_after_tlv).
fn der_read_tlv(data: &[u8]) -> Option<(&[u8], &[u8])> {
    if data.is_empty() { return None; }
    let hlen = der_header_len(data)?;
    let content_len = der_content_len(data)?;
    let total = hlen + content_len;
    if total > data.len() { return None; }
    Some((&data[hlen..total], &data[total..]))
}

/// Compute the header length (tag + length bytes) of a DER element.
fn der_header_len(data: &[u8]) -> Option<usize> {
    if data.len() < 2 { return None; }
    let len_byte = data[1];
    if len_byte < 0x80 {
        Some(2)
    } else {
        let num_len_bytes = (len_byte & 0x7f) as usize;
        if data.len() < 2 + num_len_bytes { return None; }
        Some(2 + num_len_bytes)
    }
}

/// Compute the content length from DER length encoding.
fn der_content_len(data: &[u8]) -> Option<usize> {
    if data.len() < 2 { return None; }
    let len_byte = data[1];
    if len_byte < 0x80 {
        Some(len_byte as usize)
    } else {
        let num_len_bytes = (len_byte & 0x7f) as usize;
        if num_len_bytes > 4 || data.len() < 2 + num_len_bytes { return None; }
        let mut len = 0usize;
        for i in 0..num_len_bytes {
            len = (len << 8) | data[2 + i] as usize;
        }
        Some(len)
    }
}

/// Fallback: build a CertID with placeholder hashes when request parsing fails.
fn build_cert_id_placeholder() -> Vec<u8> {
    // SHA-256 OID = 2.16.840.1.101.3.4.2.1
    let sha256_oid = der_oid(&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01]);
    let hash_alg = der_sequence(&[sha256_oid.as_slice(), &[0x05, 0x00]].concat());

    // 32-byte placeholder hashes (SHA-256 output size)
    let issuer_name_hash = der_octet_string(&[0xAA; 32]);
    let issuer_key_hash = der_octet_string(&[0xBB; 32]);
    let serial = der_integer(&[0x01]);

    der_sequence(
        &[
            hash_alg.as_slice(),
            issuer_name_hash.as_slice(),
            issuer_key_hash.as_slice(),
            serial.as_slice(),
        ]
        .concat(),
    )
}

// ── CRL (X.509 v2) ─────────────────────────────────────────────

/// Build a valid but empty X.509 v2 CRL (no revoked certs).
///
/// ```text
/// CertificateList ::= SEQUENCE {
///   tbsCertList       TBSCertList,
///   signatureAlgorithm AlgorithmIdentifier,
///   signatureValue     BIT STRING
/// }
///
/// TBSCertList ::= SEQUENCE {
///   version            INTEGER { v2(1) },
///   signature          AlgorithmIdentifier,
///   issuer             Name,
///   thisUpdate         Time,
///   nextUpdate         Time,
///   -- revokedCertificates OMITTED (empty CRL)
/// }
/// ```
fn build_empty_crl() -> Vec<u8> {
    // AlgorithmIdentifier: sha256WithRSAEncryption
    let sig_alg_oid = der_oid(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b]);
    let sig_alg = der_sequence(&[sig_alg_oid.as_slice(), &[0x05, 0x00]].concat());

    let version = der_integer(&[0x01]); // v2
    let issuer = burdettie_issuer_name();
    let this_update = der_utc_time("260406000000Z");
    let next_update = der_utc_time("260407000000Z");

    // TBSCertList — no revokedCertificates field (empty CRL)
    let tbs = der_sequence(
        &[
            version.as_slice(),
            sig_alg.as_slice(),
            issuer.as_slice(),
            this_update.as_slice(),
            next_update.as_slice(),
        ]
        .concat(),
    );

    let dummy_sig = der_bit_string(&[0u8; 64]);

    der_sequence(&[tbs.as_slice(), sig_alg.as_slice(), dummy_sig.as_slice()].concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocsp_response_is_valid_der() {
        let resp = build_ocsp_good_response(&[]);
        // Must start with SEQUENCE tag
        assert_eq!(resp[0], 0x30, "OCSP response must be a SEQUENCE");
        // Must contain ENUMERATED 0 (successful)
        assert!(resp.windows(3).any(|w| w == [0x0a, 0x01, 0x00]),
            "must contain responseStatus=successful");
        // Must contain id-pkix-ocsp-basic OID
        let basic_oid = [0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x01];
        assert!(resp.windows(basic_oid.len()).any(|w| w == basic_oid),
            "must contain id-pkix-ocsp-basic OID");
        // Must contain certStatus=good ([0] IMPLICIT NULL = 0x80, 0x00)
        assert!(resp.windows(2).any(|w| w == [0x80, 0x00]),
            "must contain certStatus=good");
        // Sanity: should be > 100 bytes (a real response, not a 5-byte stub)
        assert!(resp.len() > 100, "OCSP response should be substantial: {} bytes", resp.len());
    }

    #[test]
    fn crl_is_valid_der() {
        let crl = build_empty_crl();
        assert_eq!(crl[0], 0x30, "CRL must be a SEQUENCE");
        // Must contain version v2 (INTEGER 1)
        assert!(crl.windows(3).any(|w| w == [0x02, 0x01, 0x01]),
            "CRL must contain version=v2");
        // Must contain sha256WithRSAEncryption OID
        let sha256rsa_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
        assert!(crl.windows(sha256rsa_oid.len()).any(|w| w == sha256rsa_oid),
            "CRL must contain sha256WithRSAEncryption OID");
        assert!(crl.len() > 50, "CRL should be substantial: {} bytes", crl.len());
    }

    #[test]
    fn der_helpers_roundtrip() {
        let seq = der_sequence(&[0x02, 0x01, 0x05]); // SEQUENCE { INTEGER 5 }
        assert_eq!(seq, vec![0x30, 0x03, 0x02, 0x01, 0x05]);

        let long = der_wrap(0x30, &[0u8; 200]);
        assert_eq!(long[0], 0x30);
        assert_eq!(long[1], 0x81); // long form
        assert_eq!(long[2], 200);
        assert_eq!(long.len(), 203);
    }

    #[test]
    fn ocsp_echoes_request_cert_id() {
        // Build a minimal OCSP request with known hashes
        let issuer_name_hash = der_octet_string(&[0x11; 32]);
        let issuer_key_hash = der_octet_string(&[0x22; 32]);
        let serial = der_integer(&[0x42]);
        let sha256_oid = der_oid(&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01]);
        let hash_alg = der_sequence(&[sha256_oid.as_slice(), &[0x05, 0x00]].concat());
        let cert_id = der_sequence(
            &[hash_alg.as_slice(), issuer_name_hash.as_slice(), issuer_key_hash.as_slice(), serial.as_slice()].concat(),
        );
        let request = der_sequence(&cert_id);
        let request_list = der_sequence(&request);
        let tbs_request = der_sequence(&request_list);
        let ocsp_request = der_sequence(&tbs_request);

        let resp = build_ocsp_good_response(&ocsp_request);
        // The response should contain the issuer hashes from the request, not 0xAA/0xBB
        assert!(resp.windows(32).any(|w| w == [0x11; 32]),
            "response must contain echoed issuerNameHash 0x11");
        assert!(resp.windows(32).any(|w| w == [0x22; 32]),
            "response must contain echoed issuerKeyHash 0x22");
        // Should NOT contain placeholder hashes
        assert!(!resp.windows(32).any(|w| w == [0xAA; 32]),
            "response must NOT contain placeholder 0xAA hash");
    }

    #[test]
    fn ocsp_falls_back_to_placeholder_on_bad_request() {
        let resp = build_ocsp_good_response(&[0xFF, 0xFF]); // garbage input
        // Should still be valid DER with placeholder hashes
        assert_eq!(resp[0], 0x30);
        assert!(resp.windows(32).any(|w| w == [0xAA; 32]),
            "bad request should produce placeholder 0xAA hash");
    }
}
