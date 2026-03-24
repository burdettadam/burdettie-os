//! Code signature (SuperBlob → CodeDirectory) parser.
//!
//! Parses the LC_CODE_SIGNATURE data from the __LINKEDIT segment.
//! All fields are big-endian on disk.

use macho_types::codesig::*;

use crate::ParseError;

/// Parse a code signature from the raw LC_CODE_SIGNATURE data.
///
/// `cs_data` is the byte slice starting at `data_off` with length `data_size`
/// as specified by the LC_CODE_SIGNATURE load command.
pub fn parse_code_signature(cs_data: &[u8]) -> Result<CodeSignature, ParseError> {
    if cs_data.len() < 12 {
        return Err(ParseError::Other("code signature data too short".into()));
    }

    let magic = read_be_u32(cs_data, 0);
    if magic != CSMAGIC_EMBEDDED_SIGNATURE {
        return Err(ParseError::Other(format!(
            "bad SuperBlob magic: {magic:#010x} (expected {CSMAGIC_EMBEDDED_SIGNATURE:#010x})"
        )));
    }

    let length = read_be_u32(cs_data, 4);
    let count = read_be_u32(cs_data, 8);

    // Parse blob index entries (each is 8 bytes: type + offset)
    let index_size = count as usize * 8;
    if 12 + index_size > cs_data.len() {
        return Err(ParseError::Other(
            "SuperBlob index extends beyond data".into(),
        ));
    }

    let mut index = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let pos = 12 + i * 8;
        let slot_type = CsSlotType::from(read_be_u32(cs_data, pos));
        let offset = read_be_u32(cs_data, pos + 4);
        index.push(BlobIndex { slot_type, offset });
    }

    let super_blob = SuperBlob {
        magic,
        length,
        count,
        index: index.clone(),
    };

    // Parse individual blobs
    let mut code_directory = None;
    let mut entitlements_plist = None;
    let mut has_cms_signature = false;

    for entry in &index {
        let off = entry.offset as usize;
        if off + 8 > cs_data.len() {
            continue;
        }
        let blob_magic = read_be_u32(cs_data, off);
        let blob_length = read_be_u32(cs_data, off + 4) as usize;

        if off + blob_length > cs_data.len() {
            continue;
        }

        let blob_data = &cs_data[off..off + blob_length];

        match blob_magic {
            CSMAGIC_CODEDIRECTORY => {
                if code_directory.is_none() {
                    match parse_code_directory(blob_data) {
                        Ok(cd) => code_directory = Some(cd),
                        Err(e) => log::warn!("CodeDirectory parse failed: {e}"),
                    }
                }
            }
            CSMAGIC_ENTITLEMENTS => {
                // Entitlements blob: magic(4) + length(4) + plist XML
                if blob_data.len() > 8 {
                    let plist_bytes = &blob_data[8..];
                    if let Ok(s) = std::str::from_utf8(plist_bytes) {
                        entitlements_plist = Some(s.to_string());
                    }
                }
            }
            CSMAGIC_BLOBWRAPPER => {
                has_cms_signature = true;
            }
            _ => {}
        }
    }

    Ok(CodeSignature {
        super_blob,
        code_directory,
        entitlements_plist,
        has_cms_signature,
    })
}

/// Parse a CodeDirectory blob.
fn parse_code_directory(cd_data: &[u8]) -> Result<CodeDirectory, ParseError> {
    // Minimum CodeDirectory size: header through hashOffset (at least 44 bytes)
    if cd_data.len() < 44 {
        return Err(ParseError::Other("CodeDirectory too short".into()));
    }

    // magic(4) + length(4) already validated by caller
    let version = read_be_u32(cd_data, 8);
    let flags = CsFlags::from_bits_truncate(read_be_u32(cd_data, 12));
    let hash_offset = read_be_u32(cd_data, 16);
    let ident_offset = read_be_u32(cd_data, 20);
    let n_special_slots = read_be_u32(cd_data, 24);
    let n_code_slots = read_be_u32(cd_data, 28);
    let code_limit = read_be_u32(cd_data, 32);
    let hash_size = cd_data[36];
    let hash_type = CsHashType::from(cd_data[37]);
    // cd_data[38] is platform, cd_data[39] is pageSize log2
    let page_size_log2 = cd_data[39];

    // Extract identity string (null-terminated at ident_offset)
    let identity = read_cstring(cd_data, ident_offset as usize);

    // Extract team ID (if version >= 0x20200, teamOffset is at offset 52)
    let team_id = if version >= 0x20200 && cd_data.len() >= 56 {
        let team_offset = read_be_u32(cd_data, 52);
        if team_offset > 0 {
            read_cstring(cd_data, team_offset as usize)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Extract page hashes
    let mut page_hashes = Vec::with_capacity(n_code_slots as usize);
    let h_off = hash_offset as usize;
    for i in 0..n_code_slots as usize {
        let start = h_off + i * hash_size as usize;
        let end = start + hash_size as usize;
        if end <= cd_data.len() {
            page_hashes.push(cd_data[start..end].to_vec());
        }
    }

    // Compute CDHash (SHA-256 of entire CodeDirectory blob)
    let cd_hash = sha256(cd_data);

    Ok(CodeDirectory {
        version,
        flags,
        hash_offset,
        ident_offset,
        n_special_slots,
        n_code_slots,
        code_limit,
        hash_type,
        hash_size,
        page_size_log2,
        identity,
        team_id,
        cd_hash,
        page_hashes,
    })
}

/// Read a big-endian u32 at the given offset.
fn read_be_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Read a null-terminated C string from `data` at `offset`.
fn read_cstring(data: &[u8], offset: usize) -> String {
    if offset >= data.len() {
        return String::new();
    }
    let slice = &data[offset..];
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..end]).into_owned()
}

/// Compute SHA-256 hash via the platform hasher.
fn sha256(data: &[u8]) -> Vec<u8> {
    // Use the global platform hasher if set, otherwise fall back to CC_SHA256
    if let Some(hasher) = crate::platform_hasher() {
        hasher.sha256(data).to_vec()
    } else {
        extern "C" {
            fn CC_SHA256(data: *const u8, len: u32, md: *mut u8) -> *mut u8;
        }
        let mut hash = [0u8; 32];
        unsafe {
            CC_SHA256(data.as_ptr(), data.len() as u32, hash.as_mut_ptr());
        }
        hash.to_vec()
    }
}

/// Result of page hash verification.
#[derive(Debug)]
pub struct PageHashResult {
    /// Number of page slots that were verified successfully.
    pub verified: usize,
    /// Mismatched slots, if any.
    pub mismatches: Vec<PageHashMismatch>,
}

/// A page hash mismatch between the code signature and the actual binary data.
#[derive(Debug)]
pub struct PageHashMismatch {
    /// Page slot index (0-based).
    pub slot: usize,
    /// File offset range of the page.
    pub file_range: std::ops::Range<usize>,
}

/// Verify that the page hashes in a CodeDirectory match the actual binary data.
///
/// This is the key integrity check that AMFI/AppleMobileFileIntegrity performs
/// at page-in time. Each page of the signed region is SHA-256 hashed and
/// compared against the stored hash in the CodeDirectory.
///
/// `binary_data` is the full binary file data (not just the code signature).
pub fn verify_page_hashes(binary_data: &[u8], cd: &CodeDirectory) -> PageHashResult {
    let page_size = 1usize << cd.page_size_log2;
    let code_limit = cd.code_limit as usize;
    let mut verified = 0;
    let mut mismatches = Vec::new();

    for (i, expected) in cd.page_hashes.iter().enumerate() {
        let start = i * page_size;
        if start >= code_limit || start >= binary_data.len() {
            break;
        }
        let end = (start + page_size).min(code_limit).min(binary_data.len());
        let actual = sha256(&binary_data[start..end]);

        if actual.as_slice() == expected.as_slice() {
            verified += 1;
        } else {
            mismatches.push(PageHashMismatch {
                slot: i,
                file_range: start..end,
            });
        }
    }

    PageHashResult {
        verified,
        mismatches,
    }
}
