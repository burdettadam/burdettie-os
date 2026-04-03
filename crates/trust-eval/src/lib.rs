//! X.509 certificate chain validation using `rustls-webpki`.
//!
//! Provides real certificate validation for the Security.framework
//! shim layer. Uses Mozilla's root certificate store via `webpki-roots`.
//!
//! # Apple Stack Mapping
//!
//! ```text
//! Apple:  Security.framework → SecTrustEvaluateWithError
//! Us:     trust-eval         → validate_trust() using webpki + mozilla roots
//! ```

use pki_types::{CertificateDer, UnixTime};
use webpki::{EndEntityCert, KeyUsage, ALL_VERIFICATION_ALGS};

/// Errors from certificate validation.
#[derive(Debug)]
pub enum ValidationError {
    /// The leaf certificate could not be parsed.
    InvalidLeafCert(String),
    /// Chain validation failed.
    ChainValidation(String),
    /// No certificates provided.
    EmptyChain,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLeafCert(e) => write!(f, "invalid leaf certificate: {e}"),
            Self::ChainValidation(e) => write!(f, "chain validation failed: {e}"),
            Self::EmptyChain => write!(f, "no certificates in chain"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validate a certificate chain against Mozilla's root store.
///
/// `certs_der` should contain DER-encoded certificates, leaf first,
/// followed by any intermediates. The root does not need to be included —
/// it will be matched against the bundled Mozilla root store.
///
/// Returns `true` if the chain validates, `false` otherwise.
pub fn validate_trust(certs_der: &[&[u8]]) -> bool {
    match validate_chain(certs_der) {
        Ok(()) => true,
        Err(e) => {
            log::debug!("trust-eval: validation failed: {e}");
            false
        }
    }
}

/// Validate a certificate chain, returning a detailed error on failure.
pub fn validate_chain(certs_der: &[&[u8]]) -> Result<(), ValidationError> {
    if certs_der.is_empty() {
        return Err(ValidationError::EmptyChain);
    }

    let leaf_der = CertificateDer::from(certs_der[0]);
    let ee = EndEntityCert::try_from(&leaf_der)
        .map_err(|e| ValidationError::InvalidLeafCert(format!("{e}")))?;

    let intermediate_certs: Vec<CertificateDer<'_>> = certs_der[1..]
        .iter()
        .map(|der| CertificateDer::from(*der))
        .collect();

    let time = UnixTime::now();
    ee.verify_for_usage(
        ALL_VERIFICATION_ALGS,
        webpki_roots::TLS_SERVER_ROOTS,
        &intermediate_certs,
        time,
        KeyUsage::server_auth(),
        None,
        None,
    )
    .map_err(|e| ValidationError::ChainValidation(format!("{e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_chain_fails() {
        assert!(!validate_trust(&[]));
    }

    #[test]
    fn garbage_cert_fails() {
        assert!(!validate_trust(&[b"not a certificate"]));
    }
}
