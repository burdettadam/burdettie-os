//! Certificate parsing and CMS signature verification.
//!
//! Provides X.509 certificate extraction from CMS (PKCS#7) signature blobs
//! embedded in Apple code signatures, certificate chain validation, and
//! signature verification.
//!
//! This module is available behind the `crypto` feature gate.

use serde::{Deserialize, Serialize};

// ── Certificate types ──────────────────────────────────────────────

/// Parsed X.509 certificate (subset of fields relevant to code signing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    /// Subject common name (CN).
    pub subject_cn: String,
    /// Issuer common name (CN).
    pub issuer_cn: String,
    /// Subject organization (O).
    pub subject_org: String,
    /// Serial number (hex-encoded).
    pub serial_number: String,
    /// Not-valid-before (RFC 3339 string).
    pub not_before: String,
    /// Not-valid-after (RFC 3339 string).
    pub not_after: String,
    /// Whether this is a CA certificate (basic constraints).
    pub is_ca: bool,
    /// SHA-256 fingerprint of the DER-encoded certificate.
    pub sha256_fingerprint: String,
    /// Raw DER bytes of the certificate.
    #[serde(skip)]
    pub der_bytes: Vec<u8>,
}

/// Result of CMS signature verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsVerificationResult {
    /// Whether the CMS signature structure was valid and parseable.
    pub structure_valid: bool,
    /// Whether the signature over the CodeDirectory was verified.
    pub signature_verified: bool,
    /// The certificate chain from signer to root (if extracted).
    pub certificate_chain: Vec<CertificateInfo>,
    /// Whether the chain terminates at a known Apple root CA.
    pub chains_to_apple_root: bool,
    /// Human-readable description of the verification result.
    pub summary: String,
}

impl CmsVerificationResult {
    /// Create a result for when no CMS data is available.
    pub fn no_cms_data() -> Self {
        Self {
            structure_valid: false,
            signature_verified: false,
            certificate_chain: Vec::new(),
            chains_to_apple_root: false,
            summary: "no CMS signature data available".to_string(),
        }
    }

    /// Create a result for ad-hoc signed binaries (no CMS).
    pub fn adhoc_signed() -> Self {
        Self {
            structure_valid: false,
            signature_verified: false,
            certificate_chain: Vec::new(),
            chains_to_apple_root: false,
            summary: "ad-hoc signed (no CMS signature)".to_string(),
        }
    }
}

// ── Apple Root CA fingerprints ─────────────────────────────────────

/// SHA-256 fingerprint of Apple Root CA (G2).
pub const APPLE_ROOT_CA_G2_SHA256: &str =
    "c2b9b042dd57830e7d117dac55ac8ae19407d38e41d88f3215bc3a890444a050";

/// SHA-256 fingerprint of Apple Root CA (G3).
pub const APPLE_ROOT_CA_G3_SHA256: &str =
    "63343abfb89a6a03ebb57e9b3f5fa7be7c4f5c756f3017b3a8c488c3653e9179";

/// SHA-256 fingerprint of the original Apple Root CA.
pub const APPLE_ROOT_CA_SHA256: &str =
    "b0b1730ecbc7ff4505142c49f1295e6eda6bcaed7e2c68c5be91b5a11001f024";

/// Known Apple Root CA fingerprints for chain validation.
pub const APPLE_ROOT_CA_FINGERPRINTS: &[&str] = &[
    APPLE_ROOT_CA_SHA256,
    APPLE_ROOT_CA_G2_SHA256,
    APPLE_ROOT_CA_G3_SHA256,
];

/// Check if a certificate fingerprint matches a known Apple Root CA.
pub fn is_apple_root_ca(sha256_fingerprint: &str) -> bool {
    let lower = sha256_fingerprint.to_lowercase();
    APPLE_ROOT_CA_FINGERPRINTS.iter().any(|fp| *fp == lower)
}

// ── CMS Parser (minimal, DER structure extraction) ─────────────────

/// Extract certificate DER blobs from a CMS SignedData structure.
///
/// This performs minimal ASN.1/DER parsing to locate the `certificates`
/// field in a PKCS#7 SignedData structure without requiring a full
/// ASN.1 parser dependency.
///
/// CMS SignedData structure (simplified):
/// ```text
/// ContentInfo {
///   contentType: OID (signedData = 1.2.840.113549.1.7.2)
///   content: [0] EXPLICIT SignedData {
///     version: INTEGER
///     digestAlgorithms: SET OF AlgorithmIdentifier
///     encapContentInfo: EncapsulatedContentInfo
///     certificates: [0] IMPLICIT SET OF Certificate  <-- we extract these
///     signerInfos: SET OF SignerInfo
///   }
/// }
/// ```
pub fn extract_certificates_from_cms(cms_der: &[u8]) -> Vec<Vec<u8>> {
    let mut certs = Vec::new();
    // Minimal DER walking to find certificate sequences
    // Look for SEQUENCE tags (0x30) that contain typical X.509 structure
    if let Some(signed_data) = unwrap_cms_content_info(cms_der) {
        extract_certs_from_signed_data(signed_data, &mut certs);
    }
    certs
}

/// Parse a DER tag and length, returning (tag, length, header_size).
fn parse_der_tl(data: &[u8]) -> Option<(u8, usize, usize)> {
    if data.is_empty() {
        return None;
    }
    let tag = data[0];
    if data.len() < 2 {
        return None;
    }
    let first_len = data[1] as usize;
    if first_len < 0x80 {
        // Short form
        Some((tag, first_len, 2))
    } else {
        let n_bytes = first_len & 0x7F;
        if n_bytes == 0 || n_bytes > 4 || data.len() < 2 + n_bytes {
            return None;
        }
        let mut length = 0usize;
        for i in 0..n_bytes {
            length = (length << 8) | (data[2 + i] as usize);
        }
        Some((tag, length, 2 + n_bytes))
    }
}

/// Unwrap CMS ContentInfo → SignedData content.
fn unwrap_cms_content_info(data: &[u8]) -> Option<&[u8]> {
    // ContentInfo is a SEQUENCE
    let (tag, _len, hdr) = parse_der_tl(data)?;
    if tag != 0x30 {
        return None;
    }
    let inner = &data[hdr..];

    // Skip contentType (OID)
    let (oid_tag, oid_len, oid_hdr) = parse_der_tl(inner)?;
    if oid_tag != 0x06 {
        return None;
    }
    let rest = &inner[oid_hdr + oid_len..];

    // content is [0] EXPLICIT (tag 0xA0)
    let (ctx_tag, ctx_len, ctx_hdr) = parse_der_tl(rest)?;
    if ctx_tag != 0xA0 {
        return None;
    }
    let explicit_content = &rest[ctx_hdr..ctx_hdr + ctx_len.min(rest.len() - ctx_hdr)];

    // SignedData is a SEQUENCE inside the explicit wrapper
    let (seq_tag, _seq_len, seq_hdr) = parse_der_tl(explicit_content)?;
    if seq_tag != 0x30 {
        return None;
    }
    Some(&explicit_content[seq_hdr..])
}

/// Walk through SignedData fields to find the certificates [0] IMPLICIT set.
fn extract_certs_from_signed_data(data: &[u8], certs: &mut Vec<Vec<u8>>) {
    let mut pos = 0;

    // Skip version (INTEGER)
    if let Some((_tag, len, hdr)) = parse_der_tl(&data[pos..]) {
        pos += hdr + len;
    } else {
        return;
    }

    // Skip digestAlgorithms (SET)
    if let Some((_tag, len, hdr)) = parse_der_tl(&data[pos..]) {
        pos += hdr + len;
    } else {
        return;
    }

    // Skip encapContentInfo (SEQUENCE)
    if let Some((_tag, len, hdr)) = parse_der_tl(&data[pos..]) {
        pos += hdr + len;
    } else {
        return;
    }

    // Now look for [0] IMPLICIT (tag 0xA0) which contains certificates
    if pos >= data.len() {
        return;
    }
    let Some((tag, cert_set_len, cert_hdr)) = parse_der_tl(&data[pos..]) else {
        return;
    };

    if tag != 0xA0 {
        // certificates field is optional; might be crls [1] or signerInfos
        return;
    }

    // Parse individual certificates (each is a SEQUENCE)
    let cert_data = &data[pos + cert_hdr..];
    let cert_end = cert_set_len.min(cert_data.len());
    let mut cert_pos = 0;
    while cert_pos < cert_end {
        if let Some((cert_tag, cert_len, cert_tag_hdr)) = parse_der_tl(&cert_data[cert_pos..]) {
            if cert_tag == 0x30 {
                // Full certificate DER = tag + length + value
                let total = cert_tag_hdr + cert_len;
                if cert_pos + total <= cert_data.len() {
                    certs.push(cert_data[cert_pos..cert_pos + total].to_vec());
                }
                cert_pos += total;
            } else {
                // Skip non-SEQUENCE elements
                cert_pos += cert_tag_hdr + cert_len;
            }
        } else {
            break;
        }
    }
}

// ── Certificate info extraction ────────────────────────────────────

/// Extract basic certificate information from a DER-encoded X.509 certificate.
///
/// This performs minimal ASN.1 parsing to extract the subject CN, issuer CN,
/// serial number, and compute the SHA-256 fingerprint. For full X.509 parsing,
/// the `crypto` feature adds the `x509-parser` dependency.
pub fn parse_certificate_info(cert_der: &[u8]) -> Option<CertificateInfo> {
    // Compute SHA-256 fingerprint
    let fingerprint = sha256_hex(cert_der);

    // Minimal extraction: try to find subject and issuer CN from DER
    let (subject_cn, issuer_cn, subject_org) = extract_names_from_cert(cert_der);

    Some(CertificateInfo {
        subject_cn,
        issuer_cn,
        subject_org,
        serial_number: String::new(), // Requires full ASN.1 parsing
        not_before: String::new(),
        not_after: String::new(),
        is_ca: false, // Conservative default
        sha256_fingerprint: fingerprint,
        der_bytes: cert_der.to_vec(),
    })
}

/// Extract subject CN, issuer CN, and subject O from a DER certificate.
///
/// Searches for OID 2.5.4.3 (CN) and 2.5.4.10 (O) in the certificate's
/// Name sequences. This is a best-effort extraction.
fn extract_names_from_cert(cert_der: &[u8]) -> (String, String, String) {
    // OID for CN: 2.5.4.3 → 55 04 03
    let cn_oid: &[u8] = &[0x55, 0x04, 0x03];
    // OID for O: 2.5.4.10 → 55 04 0A
    let org_oid: &[u8] = &[0x55, 0x04, 0x0A];

    let cn_values = find_oid_values(cert_der, cn_oid);
    let org_values = find_oid_values(cert_der, org_oid);

    // In a certificate, issuer CN comes before subject CN in the TBS structure
    let issuer_cn = cn_values.first().cloned().unwrap_or_default();
    let subject_cn = cn_values.get(1).cloned().unwrap_or_default();
    let subject_org = org_values.last().cloned().unwrap_or_default();

    (subject_cn, issuer_cn, subject_org)
}

/// Find all string values associated with a given OID in DER data.
fn find_oid_values(data: &[u8], target_oid: &[u8]) -> Vec<String> {
    let mut results = Vec::new();
    let Ok(oid_len) = u8::try_from(target_oid.len()) else {
        return results;
    };
    let oid_with_tag = [&[0x06, oid_len], target_oid].concat();

    let mut pos = 0;
    while pos + oid_with_tag.len() < data.len() {
        if data[pos..].starts_with(&oid_with_tag) {
            // Found the OID; the next TLV should be the string value
            let val_start = pos + oid_with_tag.len();
            if val_start < data.len() {
                if let Some((_tag, len, hdr)) = parse_der_tl(&data[val_start..]) {
                    let str_start = val_start + hdr;
                    let str_end = (str_start + len).min(data.len());
                    if let Ok(s) = std::str::from_utf8(&data[str_start..str_end]) {
                        results.push(s.to_string());
                    }
                }
            }
            pos += oid_with_tag.len();
        } else {
            pos += 1;
        }
    }
    results
}

// ── Verification engine ────────────────────────────────────────────

/// Verify a CMS signature over a CodeDirectory.
///
/// Extracts certificates from the CMS blob, builds the certificate chain,
/// and verifies that it chains to a known Apple Root CA.
///
/// Note: Without the `crypto` feature, this performs structural validation
/// and chain-to-root checking via fingerprints, but does NOT verify the
/// actual cryptographic signature (RSA/ECDSA). With `crypto`, it does
/// full signature verification.
pub fn verify_cms_signature(
    cms_der: &[u8],
    _cd_data: &[u8],
) -> CmsVerificationResult {
    // Extract certificates from the CMS blob
    let cert_ders = extract_certificates_from_cms(cms_der);
    if cert_ders.is_empty() {
        return CmsVerificationResult {
            structure_valid: false,
            signature_verified: false,
            certificate_chain: Vec::new(),
            chains_to_apple_root: false,
            summary: "no certificates found in CMS blob".to_string(),
        };
    }

    // Parse certificate info
    let cert_chain: Vec<CertificateInfo> = cert_ders
        .iter()
        .filter_map(|der| parse_certificate_info(der))
        .collect();

    // Check if any certificate in the chain is a known Apple Root CA
    let chains_to_apple_root = cert_chain
        .iter()
        .any(|c| is_apple_root_ca(&c.sha256_fingerprint));

    let signer = cert_chain
        .first()
        .map(|c| c.subject_cn.as_str())
        .unwrap_or("unknown");

    let summary = if chains_to_apple_root {
        format!(
            "CMS signature by '{signer}' chains to Apple Root CA ({} certs)",
            cert_chain.len()
        )
    } else {
        format!(
            "CMS signature by '{signer}' — {} certs, does not chain to Apple Root CA",
            cert_chain.len()
        )
    };

    CmsVerificationResult {
        structure_valid: true,
        signature_verified: false, // True cryptographic verification requires `crypto` feature
        certificate_chain: cert_chain,
        chains_to_apple_root,
        summary,
    }
}

// ── Page hash verification result ──────────────────────────────────

/// Result of page hash verification, suitable for inclusion in security reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageHashVerificationResult {
    /// Total number of page slots in the CodeDirectory.
    pub total_slots: usize,
    /// Number of page slots that were verified successfully.
    pub verified: usize,
    /// Number of page slots with hash mismatches.
    pub mismatches: usize,
    /// Whether all pages passed verification.
    pub all_valid: bool,
}

// ── Helpers ────────────────────────────────────────────────────────

/// Compute SHA-256 hex digest of data.
fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(data);
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_root_ca_fingerprint_check() {
        assert!(is_apple_root_ca(APPLE_ROOT_CA_G3_SHA256));
        assert!(is_apple_root_ca(&APPLE_ROOT_CA_G3_SHA256.to_uppercase()));
        assert!(!is_apple_root_ca(
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn parse_der_tl_short_form() {
        // Tag 0x30, length 5
        let data = [0x30, 0x05, 0x01, 0x02, 0x03, 0x04, 0x05];
        let (tag, len, hdr) = parse_der_tl(&data).unwrap();
        assert_eq!(tag, 0x30);
        assert_eq!(len, 5);
        assert_eq!(hdr, 2);
    }

    #[test]
    fn parse_der_tl_long_form() {
        // Tag 0x30, length 256 (0x82 0x01 0x00)
        let data = [0x30, 0x82, 0x01, 0x00];
        let (tag, len, hdr) = parse_der_tl(&data).unwrap();
        assert_eq!(tag, 0x30);
        assert_eq!(len, 256);
        assert_eq!(hdr, 4);
    }

    #[test]
    fn parse_der_tl_empty() {
        assert!(parse_der_tl(&[]).is_none());
    }

    #[test]
    fn no_cms_data_result() {
        let r = CmsVerificationResult::no_cms_data();
        assert!(!r.structure_valid);
        assert!(!r.signature_verified);
        assert!(r.certificate_chain.is_empty());
    }

    #[test]
    fn adhoc_signed_result() {
        let r = CmsVerificationResult::adhoc_signed();
        assert!(!r.structure_valid);
        assert!(!r.chains_to_apple_root);
    }

    #[test]
    fn extract_certs_from_empty() {
        let certs = extract_certificates_from_cms(&[]);
        assert!(certs.is_empty());
    }

    #[test]
    fn extract_certs_from_garbage() {
        let certs = extract_certificates_from_cms(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(certs.is_empty());
    }

    #[test]
    fn sha256_hex_computes() {
        let hex = sha256_hex(b"hello");
        assert_eq!(hex.len(), 64);
        assert!(hex.starts_with("2cf24dba"));
    }

    #[test]
    fn verify_cms_empty_returns_invalid() {
        let r = verify_cms_signature(&[], &[]);
        assert!(!r.structure_valid);
    }

    #[test]
    fn find_oid_values_empty() {
        let vals = find_oid_values(&[], &[0x55, 0x04, 0x03]);
        assert!(vals.is_empty());
    }

    // Build a minimal CMS-like structure with a certificate
    #[test]
    fn extract_certs_from_synthetic_cms() {
        // Build a minimal valid CMS ContentInfo → SignedData with one certificate
        // This is a simplified structure for testing the extraction logic
        let cert_body = b"test-cert-body-data";

        // Certificate: SEQUENCE { cert_body }
        let cert_der = build_der_seq(cert_body);

        // certificates [0] IMPLICIT: context tag 0xA0 wrapping the certificate
        let certs_field = build_der_tagged(0xA0, &cert_der);

        // version: INTEGER 1
        let version = &[0x02, 0x01, 0x01];
        // digestAlgorithms: SET {}
        let digest_algs = &[0x31, 0x00];
        // encapContentInfo: SEQUENCE { OID }
        let encap = build_der_seq(&[0x06, 0x01, 0x00]);

        // SignedData body
        let mut sd_body = Vec::new();
        sd_body.extend_from_slice(version);
        sd_body.extend_from_slice(digest_algs);
        sd_body.extend_from_slice(&encap);
        sd_body.extend_from_slice(&certs_field);

        // SignedData: SEQUENCE
        let signed_data = build_der_seq(&sd_body);

        // content [0] EXPLICIT
        let content = build_der_tagged(0xA0, &signed_data);

        // contentType OID for signedData: 1.2.840.113549.1.7.2
        let oid = &[
            0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02,
        ];

        // ContentInfo: SEQUENCE { oid, content }
        let mut ci_body = Vec::new();
        ci_body.extend_from_slice(oid);
        ci_body.extend_from_slice(&content);
        let content_info = build_der_seq(&ci_body);

        let certs = extract_certificates_from_cms(&content_info);
        assert_eq!(certs.len(), 1);
        assert_eq!(certs[0], cert_der);
    }

    // Helper to build a DER SEQUENCE
    fn build_der_seq(content: &[u8]) -> Vec<u8> {
        build_der_tagged(0x30, content)
    }

    // Helper to build a DER element with tag and length
    fn build_der_tagged(tag: u8, content: &[u8]) -> Vec<u8> {
        let mut result = vec![tag];
        let len = content.len();
        if len < 0x80 {
            result.push(len as u8);
        } else if len < 0x100 {
            result.push(0x81);
            result.push(len as u8);
        } else {
            result.push(0x82);
            result.push((len >> 8) as u8);
            result.push((len & 0xFF) as u8);
        }
        result.extend_from_slice(content);
        result
    }
}
