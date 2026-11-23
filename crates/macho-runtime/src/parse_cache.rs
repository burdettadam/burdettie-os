//! Content-bound per-process binary parse cache shared across execution pipelines.
//!
//! Every lookup receives the bytes just read through the configured filesystem
//! provider. A path hit is reusable only when the SHA-256 of those source bytes
//! matches the cached source revision. This deliberately trades the old
//! path-only no-I/O fast path for freshness: the exact bytes parsed, admitted,
//! and mapped must be the bytes observed for this invocation.
//!
//! The cache is intentionally not cleared by runtime reset. Unchanged source
//! bytes can reuse their immutable parse result across benchmark iterations;
//! replaced or modified bytes miss and are reparsed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use macho_parse::ParsedMachO;
use sha2::{Digest, Sha256};

pub(crate) struct CachedBinary {
    /// Selected thin image bytes passed to the mapper.
    pub(crate) data: Arc<[u8]>,
    pub(crate) parsed: Arc<ParsedMachO>,
    /// Digest of the complete file read from FilesystemProvider, before fat slicing.
    source_sha256: [u8; 32],
}

static BINARY_PARSE_CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedBinary>>> = OnceLock::new();

pub(crate) fn binary_parse_cache() -> &'static Mutex<HashMap<PathBuf, CachedBinary>> {
    BINARY_PARSE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn digest(source_data: &[u8]) -> [u8; 32] {
    Sha256::digest(source_data).into()
}

/// Look up a cached parse result only when the current source bytes match.
///
/// Callers must read through their configured FilesystemProvider first and
/// pass those exact bytes here. The returned data and parsed object were created
/// from the same source revision. A miss leaves parsing and cache replacement
/// to the caller.
pub(crate) fn lookup(path: &Path, source_data: &[u8]) -> Option<(Arc<[u8]>, Arc<ParsedMachO>)> {
    let source_sha256 = digest(source_data);
    let mut guard = binary_parse_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if let Some(cached) = guard.get(path) {
        if cached.source_sha256 == source_sha256 {
            return Some((cached.data.clone(), cached.parsed.clone()));
        }
    }

    // Preserve canonical aliases, but never let an alias bypass content binding.
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if canonical == path {
        return None;
    }
    if let Some(cached) = guard.get(&canonical) {
        if cached.source_sha256 == source_sha256 {
            let data = cached.data.clone();
            let parsed = cached.parsed.clone();
            guard.insert(
                path.to_path_buf(),
                CachedBinary {
                    data: data.clone(),
                    parsed: parsed.clone(),
                    source_sha256,
                },
            );
            return Some((data, parsed));
        }
    }
    None
}

/// Insert an entry under both raw and canonical paths, binding it to the
/// complete source bytes read for this invocation. Data may be a selected
/// architecture slice of a universal/fat binary.
pub(crate) fn insert_both(
    raw: &Path,
    canonical: PathBuf,
    source_data: &[u8],
    data: Arc<[u8]>,
    parsed: Arc<ParsedMachO>,
) {
    let source_sha256 = digest(source_data);
    let mut guard = binary_parse_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if raw != canonical.as_path() {
        guard.insert(
            raw.to_path_buf(),
            CachedBinary {
                data: data.clone(),
                parsed: parsed.clone(),
                source_sha256,
            },
        );
    }
    guard.insert(
        canonical,
        CachedBinary {
            data,
            parsed,
            source_sha256,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("burdettie-parse-cache-{}-{id}", std::process::id()));
            std::fs::create_dir(&path).expect("create isolated cache test directory");
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Minimal valid ARM64 MH_EXECUTE with one file-backed __TEXT segment.
    fn image(marker: u8) -> Vec<u8> {
        let mut segment = vec![0_u8; 72];
        segment[0..4].copy_from_slice(&0x19_u32.to_le_bytes()); // LC_SEGMENT_64
        segment[4..8].copy_from_slice(&72_u32.to_le_bytes());
        segment[8..14].copy_from_slice(b"__TEXT");
        segment[24..32].copy_from_slice(&0x1_0000_0000_u64.to_le_bytes());
        segment[32..40].copy_from_slice(&0x4000_u64.to_le_bytes());
        segment[48..56].copy_from_slice(&0x4000_u64.to_le_bytes());
        segment[56..60].copy_from_slice(&7_u32.to_le_bytes());
        segment[60..64].copy_from_slice(&5_u32.to_le_bytes());

        let mut bytes = Vec::with_capacity(0x4000);
        for value in [0xfeed_facf_u32, 0x0100_000c_u32, 0, 2, 1, 72, 0, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&segment);
        bytes.resize(0x4000, 0);
        // Change an otherwise ignored payload byte without changing file size
        // or parsed load-command metadata.
        bytes[0x3fff] = marker;
        bytes
    }

    #[test]
    fn unchanged_source_bytes_reuse_the_same_immutable_parse() {
        let directory = TempDirectory::new();
        let path = directory.0.join("app");
        let source = image(1);
        std::fs::write(&path, &source).unwrap();
        let parsed = Arc::new(macho_parse::parse(&source).expect("valid fixture"));
        let mapped: Arc<[u8]> = Arc::from(source.clone());
        let canonical = path.canonicalize().unwrap();

        insert_both(&path, canonical, &source, mapped.clone(), parsed.clone());

        let (cached_data, cached_parsed) =
            lookup(&path, &std::fs::read(&path).unwrap()).expect("matching source cache hit");
        assert!(Arc::ptr_eq(&cached_data, &mapped));
        assert!(Arc::ptr_eq(&cached_parsed, &parsed));
    }

    #[test]
    fn same_path_and_same_size_replacement_invalidates_raw_and_canonical_aliases() {
        let directory = TempDirectory::new();
        let file = directory.0.join("app");
        let raw_alias = directory.0.join(".").join("app");
        let original = image(1);
        let replacement = image(2);
        assert_eq!(original.len(), replacement.len());
        std::fs::write(&file, &original).unwrap();

        let parsed = Arc::new(macho_parse::parse(&original).expect("valid original"));
        let mapped: Arc<[u8]> = Arc::from(original.clone());
        let canonical = file.canonicalize().unwrap();
        insert_both(&raw_alias, canonical.clone(), &original, mapped, parsed);

        assert!(lookup(&raw_alias, &original).is_some());
        assert!(lookup(&canonical, &original).is_some());

        // The replacement changes only a payload byte; size, path and parsed
        // load-command metadata remain the same. Content identity must still miss.
        std::fs::write(&file, &replacement).unwrap();
        assert!(lookup(&raw_alias, &replacement).is_none());
        assert!(lookup(&canonical, &replacement).is_none());
    }

    #[test]
    fn changed_source_is_not_hidden_by_a_stale_raw_alias() {
        let directory = TempDirectory::new();
        let file = directory.0.join("app");
        let raw_alias = directory.0.join(".").join("app");
        let original = image(3);
        let replacement = image(4);
        std::fs::write(&file, &original).unwrap();

        let parsed = Arc::new(macho_parse::parse(&original).expect("valid original"));
        let mapped: Arc<[u8]> = Arc::from(original.clone());
        let canonical = file.canonicalize().unwrap();
        insert_both(&raw_alias, canonical, &original, mapped, parsed);

        assert!(lookup(&raw_alias, &replacement).is_none());
        assert!(lookup(&raw_alias, &original).is_some());
    }
}
