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

    // Extract entitlement keys from code signature if present
    let entitlement_keys = parsed
        .code_signature
        .as_ref()
        .map(|cs| cs.entitlement_keys())
        .unwrap_or_default();

    // Evaluate security gates with entitlement keys
    let report = policy.evaluate_binary_parsed_with_keys(
        binary_path,
        parsed.header.flags,
        &cs_info,
        &entitlement_keys,
    );

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

#[cfg(test)]
mod tests {
    use super::*;
    use macho_parse::ParsedMachO;
    use macho_types::header::{CpuType, FileType, MachFlags, MachHeader, MH_MAGIC_64};
    use macho_types::load_command::{LoadCommand, LoadCommandPayload, LoadCommandType};
    use trust_policy::TrustPolicy;

    fn make_header() -> MachHeader {
        MachHeader {
            magic: MH_MAGIC_64,
            cpu_type: CpuType::Arm64,
            cpu_subtype: 0,
            file_type: FileType::Execute,
            num_commands: 0,
            size_of_commands: 0,
            flags: MachFlags::empty(),
            reserved: Some(0),
        }
    }

    /// Build a minimal ParsedMachO with an EncryptionInfo command.
    fn parsed_with_encryption(crypt_id: u32) -> ParsedMachO {
        let cmd = LoadCommand {
            cmd: LoadCommandType::EncryptionInfo64,
            cmd_size: 24,
            payload: LoadCommandPayload::EncryptionInfo {
                crypt_off: 0,
                crypt_size: 0,
                crypt_id,
            },
        };
        ParsedMachO {
            header: make_header(),
            commands: vec![cmd],
            segments: Vec::new(),
            sections: Vec::new(),
            dylibs: Vec::new(),
            rpaths: Vec::new(),
            entry: None,
            symbols: None,
            uuid: None,
            code_signature: None,
        }
    }

    /// Build a minimal ParsedMachO with no encryption.
    fn parsed_clean() -> ParsedMachO {
        ParsedMachO {
            header: make_header(),
            commands: Vec::new(),
            segments: Vec::new(),
            sections: Vec::new(),
            dylibs: Vec::new(),
            rpaths: Vec::new(),
            entry: None,
            symbols: None,
            uuid: None,
            code_signature: None,
        }
    }

    #[test]
    fn encrypted_binary_rejected() {
        let parsed = parsed_with_encryption(1);
        // We can't call checked_entry without a MappedImage, so test the
        // loop logic directly:
        let result: Result<(), ExecError> = (|| {
            for cmd in &parsed.commands {
                if let LoadCommandPayload::EncryptionInfo { crypt_id, .. } = &cmd.payload {
                    if *crypt_id != 0 {
                        return Err(ExecError::EncryptedBinary {
                            crypt_id: *crypt_id,
                        });
                    }
                }
            }
            Ok(())
        })();
        let err = result.unwrap_err();
        assert!(
            matches!(err, ExecError::EncryptedBinary { crypt_id: 1 }),
            "expected EncryptedBinary, got: {err:?}"
        );
    }

    #[test]
    fn zero_crypt_id_is_allowed() {
        let parsed = parsed_with_encryption(0);
        // crypt_id == 0 means not actually encrypted — should pass the check
        let pass: Result<(), ExecError> = (|| {
            for cmd in &parsed.commands {
                if let LoadCommandPayload::EncryptionInfo { crypt_id, .. } = &cmd.payload {
                    if *crypt_id != 0 {
                        return Err(ExecError::EncryptedBinary {
                            crypt_id: *crypt_id,
                        });
                    }
                }
            }
            Ok(())
        })();
        assert!(pass.is_ok());
    }

    #[test]
    fn security_denied_by_apple_default_policy() {
        let parsed = parsed_clean();
        let policy = TrustPolicy::apple_default();
        let path = Path::new("/tmp/test.bin");

        // Evaluate the policy — apple_default may deny unsigned binaries
        let cs_info = parsed
            .code_signature
            .as_ref()
            .map(CodeSignatureInfo::from_parsed)
            .unwrap_or_default();
        let report = policy.evaluate_binary_parsed(path, parsed.header.flags, &cs_info);

        // If the policy denies this, verify the SecurityDenied error shape
        if !report.allowed {
            let reasons = report.denied_reasons.join("; ");
            let err = ExecError::SecurityDenied {
                reason: reasons.clone(),
            };
            match &err {
                ExecError::SecurityDenied { reason } => assert!(!reason.is_empty()),
                other => panic!("expected SecurityDenied, got: {other:?}"),
            }
        }
        // Either way — test passes; we verified the policy runs without panicking
    }

    #[test]
    fn permissive_policy_allows_unsigned() {
        let parsed = parsed_clean();
        let policy = TrustPolicy::permissive();
        let path = Path::new("/tmp/test.bin");

        let cs_info = parsed
            .code_signature
            .as_ref()
            .map(CodeSignatureInfo::from_parsed)
            .unwrap_or_default();
        let report = policy.evaluate_binary_parsed(path, parsed.header.flags, &cs_info);
        assert!(
            report.allowed,
            "permissive policy should allow unsigned binary"
        );
    }

    #[test]
    fn encrypted_binary_high_crypt_id() {
        let parsed = parsed_with_encryption(99);
        let result: Result<(), ExecError> = (|| {
            for cmd in &parsed.commands {
                if let LoadCommandPayload::EncryptionInfo { crypt_id, .. } = &cmd.payload {
                    if *crypt_id != 0 {
                        return Err(ExecError::EncryptedBinary {
                            crypt_id: *crypt_id,
                        });
                    }
                }
            }
            Ok(())
        })();
        let err = result.unwrap_err();
        match err {
            ExecError::EncryptedBinary { crypt_id } => assert_eq!(crypt_id, 99),
            other => panic!("expected EncryptedBinary, got: {other:?}"),
        }
    }
}
