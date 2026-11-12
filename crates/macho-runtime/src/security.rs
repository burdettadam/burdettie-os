use std::path::Path;

use macho_parse::ParsedMachO;
use trust_policy::{CodeSignatureInfo, SecurityReport};

use crate::config::RuntimeConfig;
use crate::error::RuntimeError;

/// Evaluate security policy, returning the report or an error if denied.
pub(crate) fn evaluate_security(
    parsed: &ParsedMachO,
    binary_path: &Path,
    config: &RuntimeConfig,
) -> Result<Option<SecurityReport>, RuntimeError> {
    let Some(policy) = &config.security_policy else {
        return Ok(None);
    };

    // Reject encrypted binaries
    for cmd in &parsed.commands {
        if let macho_types::load_command::LoadCommandPayload::EncryptionInfo { crypt_id, .. } =
            &cmd.payload
        {
            if *crypt_id != 0 {
                return Err(RuntimeError::EncryptedBinary {
                    crypt_id: *crypt_id,
                });
            }
        }
    }

    let cs_info = parsed
        .code_signature
        .as_ref()
        .map(CodeSignatureInfo::from_parsed)
        .unwrap_or_default();

    // Extract entitlement keys from code signature if present
    let entitlement_keys = parsed
        .code_signature
        .as_ref()
        .map(|cs| cs.entitlement_keys())
        .unwrap_or_default();

    let report = policy.evaluate_binary_parsed_with_keys(
        binary_path,
        parsed.header.flags,
        &cs_info,
        &entitlement_keys,
    );

    if !report.allowed {
        return Err(RuntimeError::SecurityDenied {
            reasons: report.denied_reasons.clone(),
        });
    }

    log::info!(
        "Security check passed for {} (hardened={}, strip_dyld_env={})",
        binary_path.display(),
        report.hardened_runtime,
        report.strip_dyld_env,
    );

    Ok(Some(report))
}
