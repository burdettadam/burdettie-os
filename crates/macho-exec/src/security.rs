//! Security-gated execution: evaluate trust policy before calling entry.
//!
//! Combines the trust-policy security evaluation with the controlled
//! entry mechanism. If the security report denies execution, returns
//! `ExecError::SecurityDenied` instead of running the binary.

use std::path::Path;

use macho_parse::ParsedMachO;
use macho_types::load_command::LoadCommandPayload;
use macho_types::EntryPoint;
use trust_policy::{CodeSignatureInfo, SecurityReport, TrustPolicy};

use crate::{EntryResult, ExecError, MappedImage};

/// Execute a mapped binary with security policy enforcement.
///
/// Evaluates the binary against the given `TrustPolicy`. If the
/// security report denies execution, returns `ExecError::SecurityDenied`.
/// Otherwise, proceeds with `call_entry`.
///
/// Returns the `EntryResult` along with the `SecurityReport` for
/// inspection by the caller.
pub fn checked_entry(
    image: &MappedImage,
    entry: &EntryPoint,
    parsed: &ParsedMachO,
    binary_path: &Path,
    policy: &TrustPolicy,
) -> Result<(EntryResult, SecurityReport), ExecError> {
    // Reject encrypted binaries — we cannot map decrypted pages
    for cmd in &parsed.commands {
        if let LoadCommandPayload::EncryptionInfo { crypt_id, .. } = &cmd.payload {
            if *crypt_id != 0 {
                return Err(ExecError::EncryptedBinary {
                    crypt_id: *crypt_id,
                });
            }
        }
    }

    // Build CodeSignatureInfo from the parsed code signature
    let cs_info = parsed
        .code_signature
        .as_ref()
        .map(CodeSignatureInfo::from_parsed)
        .unwrap_or_default();

    // Evaluate security gates
    let report = policy.evaluate_binary_parsed(binary_path, parsed.header.flags, &cs_info);

    // Enforce: if denied, do not execute
    if !report.allowed {
        let reasons = report.denied_reasons.join("; ");
        return Err(ExecError::SecurityDenied { reason: reasons });
    }

    log::info!(
        "Security check passed for {} (hardened={}, strip_dyld_env={})",
        binary_path.display(),
        report.hardened_runtime,
        report.strip_dyld_env,
    );

    // Proceed with execution (call_entry handles protect() internally)
    let result = crate::entry::call_entry(image, entry)?;
    Ok((result, report))
}
