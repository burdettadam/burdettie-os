//! Code signature structures (SuperBlob, CodeDirectory, entitlements).
//!
//! These are the binary types used within the LC_CODE_SIGNATURE data
//! referenced by the __LINKEDIT segment. All multi-byte integers are
//! stored in **big-endian** byte order.

use serde::{Deserialize, Serialize};

// ── Magic constants ────────────────────────────────────────────────

/// SuperBlob magic — `CSMAGIC_EMBEDDED_SIGNATURE`
pub const CSMAGIC_EMBEDDED_SIGNATURE: u32 = 0xFADE_0CC0;
/// CodeDirectory magic
pub const CSMAGIC_CODEDIRECTORY: u32 = 0xFADE_0C02;
/// Entitlements plist blob magic
pub const CSMAGIC_ENTITLEMENTS: u32 = 0xFADE_7171;
/// CMS (PKCS#7) signature blob magic
pub const CSMAGIC_BLOBWRAPPER: u32 = 0xFADE_0B01;

// ── Slot types ─────────────────────────────────────────────────────

/// Slot type identifiers in a SuperBlob index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum CsSlotType {
    CodeDirectory = 0,
    InfoSlot = 1,
    Requirements = 2,
    ResourceDir = 3,
    Application = 4,
    Entitlements = 5,
    AlternateCodeDirectories = 0x1000,
    CmsSignature = 0x10000,
    Unknown(u32),
}

impl From<u32> for CsSlotType {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::CodeDirectory,
            1 => Self::InfoSlot,
            2 => Self::Requirements,
            3 => Self::ResourceDir,
            4 => Self::Application,
            5 => Self::Entitlements,
            0x1000 => Self::AlternateCodeDirectories,
            0x10000 => Self::CmsSignature,
            other => Self::Unknown(other),
        }
    }
}

// ── Code signature flags (CS_*) ────────────────────────────────────

use bitflags::bitflags;

bitflags! {
    /// Code-signing flags from the CodeDirectory `flags` field.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CsFlags: u32 {
        /// This code has been validated (the system sets this at runtime).
        const VALID           = 0x0000_0001;
        /// Ad-hoc signed (no identity).
        const ADHOC           = 0x0000_0002;
        /// Has an Info.plist.
        const GET_TASK_ALLOW  = 0x0000_0004;
        /// Installer package.
        const INSTALLER       = 0x0000_0008;
        /// Force hardened runtime (CS_RUNTIME).
        const RUNTIME         = 0x0001_0000;
        /// Signed by linker (linker-signed).
        const LINKER_SIGNED   = 0x0002_0000;
        /// Restrict DYLD environment variables (CS_RESTRICT).
        const RESTRICT        = 0x0000_0800;
        /// Enforcement mode.
        const ENFORCEMENT     = 0x0000_1000;
        /// Require library validation.
        const REQUIRE_LV      = 0x0000_2000;
    }
}

impl Serialize for CsFlags {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CsFlags {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_truncate(bits))
    }
}

// ── Hash types ─────────────────────────────────────────────────────

/// Hash algorithm identifier from the CodeDirectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CsHashType {
    Sha1 = 1,
    Sha256 = 2,
    Sha256Truncated = 3,
    Sha384 = 4,
    Unknown(u8),
}

impl From<u8> for CsHashType {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Sha1,
            2 => Self::Sha256,
            3 => Self::Sha256Truncated,
            4 => Self::Sha384,
            other => Self::Unknown(other),
        }
    }
}

// ── Parsed structures ──────────────────────────────────────────────

/// An entry in the SuperBlob's index table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobIndex {
    pub slot_type: CsSlotType,
    pub offset: u32,
}

/// Parsed SuperBlob (embedded signature container).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperBlob {
    pub magic: u32,
    pub length: u32,
    pub count: u32,
    pub index: Vec<BlobIndex>,
}

/// Parsed CodeDirectory from the SuperBlob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDirectory {
    /// CodeDirectory version (e.g., 0x20400).
    pub version: u32,
    /// Code-signing flags (adhoc, runtime, restrict, etc.).
    pub flags: CsFlags,
    /// Offset within the CodeDirectory to the start of page hashes.
    pub hash_offset: u32,
    /// Offset within the CodeDirectory to the signing identity string.
    pub ident_offset: u32,
    /// Number of special hash slots (negative indices).
    pub n_special_slots: u32,
    /// Number of code (page) hash slots.
    pub n_code_slots: u32,
    /// Size of the code region being signed (bytes).
    pub code_limit: u32,
    /// Hash algorithm used.
    pub hash_type: CsHashType,
    /// Size of each hash output (bytes).
    pub hash_size: u8,
    /// Log2 of the page size used for hashing.
    pub page_size_log2: u8,
    /// Signing identity string (e.g., "app" or bundle ID).
    pub identity: String,
    /// Team identifier (empty for ad-hoc/linker-signed).
    pub team_id: String,
    /// The computed CDHash (SHA-256 hash of the CodeDirectory blob itself).
    pub cd_hash: Vec<u8>,
    /// Individual page hashes.
    pub page_hashes: Vec<Vec<u8>>,
}

/// Parsed code signature from LC_CODE_SIGNATURE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSignature {
    /// The SuperBlob container.
    pub super_blob: SuperBlob,
    /// The primary CodeDirectory (if found).
    pub code_directory: Option<CodeDirectory>,
    /// Raw entitlements plist XML (if found).
    pub entitlements_plist: Option<String>,
    /// Whether a CMS (PKCS#7) signature blob is present.
    pub has_cms_signature: bool,
    /// Raw CMS (PKCS#7) signature blob data (DER-encoded SignedData).
    /// This is the content of the CSMAGIC_BLOBWRAPPER blob, minus the 8-byte header.
    #[serde(skip)]
    pub cms_data: Option<Vec<u8>>,
    /// Raw CodeDirectory blob data (for CMS signature verification).
    /// The CMS signature signs over this exact byte sequence.
    #[serde(skip)]
    pub cd_data: Option<Vec<u8>>,
}

impl CodeSignature {
    /// Extract entitlement keys and their boolean values from the plist XML.
    ///
    /// Scans `<key>...</key>` tags and checks whether the following element is
    /// `<true/>` or `<false/>`. Keys followed by other elements default to `true`.
    pub fn entitlement_keys(&self) -> Vec<(String, bool)> {
        let Some(xml) = &self.entitlements_plist else {
            return Vec::new();
        };
        extract_entitlement_keys(xml)
    }
}

/// Lightweight plist key/value scanner — no XML parser dependency.
fn extract_entitlement_keys(xml: &str) -> Vec<(String, bool)> {
    let mut result = Vec::new();
    let key_open = "<key>";
    let key_close = "</key>";

    let mut search_from = 0;
    while let Some(start) = xml[search_from..].find(key_open) {
        let abs_start = search_from + start + key_open.len();
        let Some(end) = xml[abs_start..].find(key_close) else {
            break;
        };
        let abs_end = abs_start + end;
        let key = xml[abs_start..abs_end].trim().to_string();

        // Look at what follows the closing </key>
        let after = xml[abs_end + key_close.len()..].trim_start();
        let value = if after.starts_with("<true/>") {
            true
        } else if after.starts_with("<false/>") {
            false
        } else {
            // non-boolean value (string, array, etc.) — treat as present/true
            true
        };

        result.push((key, value));
        search_from = abs_end + key_close.len();
    }
    result
}
