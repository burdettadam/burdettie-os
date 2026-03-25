use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Description of a test fixture binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureDescriptor {
    pub name: String,
    pub binary_path: PathBuf,
    pub description: String,
    pub expected_dylibs: Vec<String>,
    pub expected_entry_type: String,
    pub expected_segments: Vec<String>,
    pub tags: Vec<String>,
}

/// Returns the path to the fixtures directory relative to workspace root.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("fixtures"))
        .unwrap_or_else(|| PathBuf::from("fixtures"))
}

/// Read a fixture binary by relative path, returning `None` if not built.
pub fn read_fixture(name: &str) -> Option<Vec<u8>> {
    std::fs::read(fixtures_dir().join(name)).ok()
}

/// Path to a specific fixture directory.
pub fn fixture_path(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

/// Path to the "app" binary inside a fixture directory.
pub fn fixture_binary(name: &str) -> PathBuf {
    fixture_path(name).join("app")
}

/// Load a fixture descriptor from a JSON file.
pub fn load_descriptor(path: &Path) -> Result<FixtureDescriptor, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(path)?;
    let desc: FixtureDescriptor = serde_json::from_str(&data)?;
    Ok(desc)
}

/// List known fixture names.
pub fn known_fixtures() -> Vec<&'static str> {
    vec![
        "simple_cli",
        "dylib_chain",
        "constructor_test",
        "objc_foundation",
        "framework_style",
        "malformed",
        "return42",
        "fat_binary",
        "objc_classes",
        "weak_dylib",
        "tlv_basic",
        "process_info",
        "dispatch_test",
        "objc_minimal",
        "objc_category",
        "real_cli",
        "collection_enum",
        "nsdate_basics",
        "nsuserdefaults_basics",
        "nsset_basics",
        "nsurl_basics",
        "nsnull_basics",
        "nsdateformatter_basics",
        "nscharacterset_basics",
        "nsnotification_basics",
        "nsvalue_basics",
        "nsuuid_basics",
        "nslocale_basics",
        "nstimezone_basics",
        "nscalendar_basics",
        "nsnumberformatter_basics",
        "nsindexset_basics",
        "nssortdescriptor_basics",
        "nstimer_basics",
        "try_catch_test",
    ]
}
