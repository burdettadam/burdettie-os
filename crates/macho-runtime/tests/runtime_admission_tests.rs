//! Admission routing tests using generated, project-owned Mach-O metadata.
//!
//! No SDK, compiled fixture, or proprietary application is required. The images
//! have a mappable segment but no entry point or initializers: these tests check
//! policy routing and denial propagation, not guest execution, cryptographic
//! authenticity, or OS-level confinement.

#![cfg(any(target_os = "macos", target_os = "linux"))]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use macho_runtime::typestate::Loader;
use macho_runtime::{Runtime, RuntimeConfig, RuntimeError};
use trust_policy::{CodeSignatureValidator, SecurityGate, SecurityVerdict, TrustPolicy};

static TEST_LOCK: Mutex<()> = Mutex::new(());
static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);
const DENIAL: &str = "admission-regression-denied";

#[derive(Clone, Copy, Debug)]
enum Route {
    Native,
    Interpreted,
    Typestate,
}

const ROUTES: [Route; 3] = [Route::Native, Route::Interpreted, Route::Typestate];

impl Route {
    fn load(self, path: &Path, config: RuntimeConfig) -> Result<(), RuntimeError> {
        macho_runtime::reset_runtime_state_for_reuse();
        match self {
            Self::Native => Runtime::load_and_run(path, &config).map(|_| ()),
            Self::Interpreted => Runtime::load_and_run_interpreted(path, &config).map(|_| ()),
            Self::Typestate => Loader::parse(path, config)?
                .check_security()?
                .map_images()
                .map(|_| ()),
        }
    }
}

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    fn new() -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let name = format!("burdettie-admission-{}-{id}", std::process::id());
        let path = std::env::temp_dir().join(name);
        std::fs::create_dir(&path).expect("create unique test directory");
        Self(path)
    }

    fn write(&self, name: &str, file_type: u32, dependencies: &[&str]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).expect("create fixture parent");
        std::fs::write(&path, image(file_type, dependencies)).expect("write generated image");
        path
    }

    fn write_with_rpaths(
        &self,
        name: &str,
        dependencies: &[&str],
        rpaths: &[&str],
    ) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).expect("create fixture parent");
        let mut bytes = image(6, dependencies);
        let old_size = u32::from_le_bytes(bytes[20..24].try_into().unwrap()) as usize;
        let mut end = 32 + old_size;
        for rpath in rpaths {
            let length = (12 + rpath.len() + 1).div_ceil(8) * 8;
            assert!(end + length < bytes.len());
            bytes[end..end + 4].copy_from_slice(&0x8000_001c_u32.to_le_bytes());
            bytes[end + 4..end + 8]
                .copy_from_slice(&u32::try_from(length).unwrap().to_le_bytes());
            bytes[end + 8..end + 12].copy_from_slice(&12_u32.to_le_bytes());
            bytes[end + 12..end + 12 + rpath.len()].copy_from_slice(rpath.as_bytes());
            end += length;
        }
        let count = u32::try_from(1 + dependencies.len() + rpaths.len()).unwrap();
        bytes[16..20].copy_from_slice(&count.to_le_bytes());
        bytes[20..24].copy_from_slice(&u32::try_from(end - 32).unwrap().to_le_bytes());
        let parsed = macho_parse::parse(&bytes).expect("rpath fixture must parse");
        assert_eq!(parsed.rpaths, rpaths);
        std::fs::write(&path, bytes).expect("write generated rpath image");
        path
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Thin ARM64 Mach-O with one 16-KiB __TEXT segment and optional dylib commands.
/// There is deliberately no LC_MAIN, thread state, or constructor section.
fn image(file_type: u32, dependencies: &[&str]) -> Vec<u8> {
    let mut segment = vec![0_u8; 72];
    segment[0..4].copy_from_slice(&0x19_u32.to_le_bytes()); // LC_SEGMENT_64
    segment[4..8].copy_from_slice(&72_u32.to_le_bytes());
    segment[8..14].copy_from_slice(b"__TEXT");
    segment[24..32].copy_from_slice(&0x1_0000_0000_u64.to_le_bytes());
    segment[32..40].copy_from_slice(&0x4000_u64.to_le_bytes());
    segment[48..56].copy_from_slice(&0x4000_u64.to_le_bytes());
    segment[56..60].copy_from_slice(&7_u32.to_le_bytes()); // maximum RWX
    segment[60..64].copy_from_slice(&5_u32.to_le_bytes()); // initial RX
    let mut commands = segment;
    for name in dependencies {
        let length = (24 + name.len() + 1).div_ceil(8) * 8;
        let mut command = vec![0_u8; length];
        command[0..4].copy_from_slice(&0xc_u32.to_le_bytes()); // LC_LOAD_DYLIB
        command[4..8].copy_from_slice(&u32::try_from(length).unwrap().to_le_bytes());
        command[8..12].copy_from_slice(&24_u32.to_le_bytes());
        command[24..24 + name.len()].copy_from_slice(name.as_bytes());
        commands.extend_from_slice(&command);
    }
    let mut bytes = Vec::new();
    for value in [
        0xfeed_facf,
        0x0100_000c, // CPU_TYPE_ARM64
        0,
        file_type,
        u32::try_from(1 + dependencies.len()).unwrap(),
        u32::try_from(commands.len()).unwrap(),
        0,
        0,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&commands);
    assert!(bytes.len() < 0x4000);
    bytes.resize(0x4000, 0);
    let parsed = macho_parse::parse(&bytes).expect("generated fixture must parse");
    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(parsed.dylibs.len(), dependencies.len());
    assert!(parsed.entry.is_none());
    bytes
}

struct RecordingSignatureGate {
    denied_path: PathBuf,
    seen: Arc<Mutex<Vec<PathBuf>>>,
}

impl SecurityGate for RecordingSignatureGate {
    fn name(&self) -> &str {
        "test.signature"
    }

    fn is_enforcing(&self) -> bool {
        true
    }
}

impl CodeSignatureValidator for RecordingSignatureGate {
    fn validate_signature(&self, path: &Path, _has_signature: bool) -> SecurityVerdict {
        self.seen.lock().unwrap().push(path.to_path_buf());
        if path == self.denied_path {
            SecurityVerdict::Deny {
                reason: DENIAL.to_string(),
            }
        } else {
            SecurityVerdict::Allow
        }
    }
}

fn config_for(main: &Path, denied_path: &Path, seen: &Arc<Mutex<Vec<PathBuf>>>) -> RuntimeConfig {
    let mut policy = TrustPolicy::permissive();
    policy.code_signature = Box::new(RecordingSignatureGate {
        denied_path: denied_path.to_path_buf(),
        seen: seen.clone(),
    });
    let mut config = RuntimeConfig::permissive(main);
    config.security_policy = Some(policy);
    config.shim_set.objc = false;
    config.shim_set.foundation = false;
    config
}

fn assert_denied(route: Route, result: Result<(), RuntimeError>) {
    match result {
        Err(RuntimeError::SecurityDenied { reasons }) => {
            assert!(reasons.iter().any(|reason| reason.contains(DENIAL)));
        }
        other => panic!("{route:?}: expected the configured denial, got {other:?}"),
    }
}

#[test]
fn every_route_rejects_a_denied_main_image() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    for route in ROUTES {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let config = config_for(&main, &main, &seen);
        assert_denied(route, route.load(&main, config));
        assert_eq!(*seen.lock().unwrap(), vec![main.clone()]);
    }
}

#[test]
fn every_route_rejects_a_denied_direct_dependency() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &["@loader_path/denied.dylib"]);
    let denied = directory.write("denied.dylib", 6, &[]);
    for route in ROUTES {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let config = config_for(&main, &denied, &seen);
        assert_denied(route, route.load(&main, config));
        assert_eq!(*seen.lock().unwrap(), vec![main.clone(), denied.clone()]);
    }
}

#[test]
fn every_route_rejects_a_denied_transitive_dependency() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &["@loader_path/parent.dylib"]);
    let parent = directory.write("parent.dylib", 6, &["@loader_path/denied.dylib"]);
    let denied = directory.write("denied.dylib", 6, &[]);
    for route in ROUTES {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let config = config_for(&main, &denied, &seen);
        assert_denied(route, route.load(&main, config));
        assert_eq!(
            *seen.lock().unwrap(),
            vec![main.clone(), parent.clone(), denied.clone()]
        );
    }
}

#[test]
fn every_route_rejects_a_denied_explicit_insert_library() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let denied = directory.write("denied.dylib", 6, &[]);
    for route in ROUTES {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut config = config_for(&main, &denied, &seen);
        config.insert_libraries.push(denied.clone());
        assert_denied(route, route.load(&main, config));
        assert_eq!(*seen.lock().unwrap(), vec![main.clone(), denied.clone()]);
    }
}

#[test]
fn every_route_rejects_a_denied_environment_insert_library() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let denied = directory.write("denied.dylib", 6, &[]);
    for route in ROUTES {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut config = config_for(&main, &denied, &seen);
        config.env_overrides.push((
            "DYLD_INSERT_LIBRARIES".to_string(),
            format!(": {} :", denied.display()),
        ));
        assert_denied(route, route.load(&main, config));
        assert_eq!(*seen.lock().unwrap(), vec![main.clone(), denied.clone()]);
    }
}

#[test]
fn allowed_graph_is_mappable_and_retains_the_report() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &["@loader_path/allowed.dylib"]);
    let dependency = directory.write("allowed.dylib", 6, &[]);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let config = config_for(&main, &directory.0.join("not-present"), &seen);
    let checked = Loader::parse(&main, config)
        .expect("parse")
        .check_security()
        .expect("allow main");
    assert!(checked.security_report().unwrap().allowed);
    assert!(checked.security_report().unwrap().denied_reasons.is_empty());
    let mapped = checked.map_images().expect("map admitted graph");
    assert_eq!(mapped.images_loaded(), 2);
    assert_eq!(*seen.lock().unwrap(), vec![main, dependency]);
}

#[test]
fn permissive_admission_is_not_reported_as_evaluated() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let checked = Loader::parse(&main, RuntimeConfig::permissive(&main))
        .expect("parse")
        .check_security()
        .expect("permissive admission");
    assert!(checked.security_report().is_none());
    assert_eq!(checked.images_loaded(), 0);
}

#[test]
fn cached_main_bytes_do_not_cache_an_allow_decision() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    Loader::parse(&main, RuntimeConfig::permissive(&main))
        .expect("populate parse cache")
        .check_security()
        .expect("initial permissive admission");
    for route in ROUTES {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let config = config_for(&main, &main, &seen);
        assert_denied(route, route.load(&main, config));
        assert_eq!(*seen.lock().unwrap(), vec![main.clone()]);
    }
}

fn insert_library(config: &mut RuntimeConfig, path: &Path, via_environment: bool) {
    if via_environment {
        config.env_overrides.push((
            "DYLD_INSERT_LIBRARIES".to_string(),
            format!(": {} :", path.display()),
        ));
    } else {
        config.insert_libraries.push(path.to_path_buf());
    }
}

#[test]
fn every_route_rejects_an_inserted_librarys_direct_dependency() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let inserted = directory.write("plugins/inserted.dylib", 6, &["@loader_path/denied.dylib"]);
    let denied = directory.write("plugins/denied.dylib", 6, &[]);
    for route in ROUTES {
        for via_environment in [false, true] {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let mut config = config_for(&main, &denied, &seen);
            insert_library(&mut config, &inserted, via_environment);
            assert_denied(route, route.load(&main, config));
            assert_eq!(
                *seen.lock().unwrap(),
                vec![main.clone(), inserted.clone(), denied.clone()]
            );
        }
    }
}

#[test]
fn every_route_rejects_an_inserted_librarys_transitive_dependency() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let inserted = directory.write(
        "plugins/inserted.dylib",
        6,
        &["@loader_path/deps/parent.dylib"],
    );
    let parent = directory.write("plugins/deps/parent.dylib", 6, &["@loader_path/denied.dylib"]);
    let denied = directory.write("plugins/deps/denied.dylib", 6, &[]);
    for route in ROUTES {
        for via_environment in [false, true] {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let mut config = config_for(&main, &denied, &seen);
            insert_library(&mut config, &inserted, via_environment);
            assert_denied(route, route.load(&main, config));
            assert_eq!(
                *seen.lock().unwrap(),
                vec![main.clone(), inserted.clone(), parent.clone(), denied.clone()]
            );
        }
    }
}

#[test]
fn inserted_dependencies_use_the_actual_executable_directory() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let inserted = directory.write(
        "plugins/inserted.dylib",
        6,
        &["@executable_path/denied.dylib"],
    );
    let denied = directory.write("denied.dylib", 6, &[]);
    for route in ROUTES {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut config = config_for(&main, &denied, &seen);
        // Process metadata must not redirect the dependency resolver.
        config.process_path = directory.0.join("unrelated/fake-main");
        config.insert_libraries.push(inserted.clone());
        assert_denied(route, route.load(&main, config));
        assert_eq!(
            *seen.lock().unwrap(),
            vec![main.clone(), inserted.clone(), denied.clone()]
        );
    }
}

#[test]
fn inserted_dependencies_use_their_roots_rpaths() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for (rpath, dependency_name) in [
        ("@loader_path/private", "plugins/private/denied.dylib"),
        ("@executable_path/private", "private/denied.dylib"),
    ] {
        let directory = FixtureDirectory::new();
        let main = directory.write("main", 2, &[]);
        let inserted = directory.write_with_rpaths(
            "plugins/inserted.dylib",
            &["@rpath/denied.dylib"],
            &[rpath],
        );
        let denied = directory.write(dependency_name, 6, &[]);
        for route in ROUTES {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let mut config = config_for(&main, &denied, &seen);
            config.insert_libraries.push(inserted.clone());
            assert_denied(route, route.load(&main, config));
            assert_eq!(
                *seen.lock().unwrap(),
                vec![main.clone(), inserted.clone(), denied.clone()]
            );
        }
    }
}

#[test]
fn every_inserted_root_is_traversed() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let first = directory.write("first.dylib", 6, &[]);
    let second = directory.write("second.dylib", 6, &["@loader_path/denied.dylib"]);
    let denied = directory.write("denied.dylib", 6, &[]);
    for route in ROUTES {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut config = config_for(&main, &denied, &seen);
        config.insert_libraries = vec![first.clone(), second.clone()];
        assert_denied(route, route.load(&main, config));
        assert_eq!(
            *seen.lock().unwrap(),
            vec![main.clone(), first.clone(), second.clone(), denied.clone()]
        );
    }
}

#[test]
fn inserted_and_main_roots_share_an_admitted_dependency() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let deps = &["@executable_path/shared.dylib"];
    let main = directory.write("main", 2, deps);
    let first = directory.write("plugins/first.dylib", 6, deps);
    let second = directory.write("plugins/second.dylib", 6, deps);
    let shared = directory.write("shared.dylib", 6, &[]);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut config = config_for(&main, &directory.0.join("not-present"), &seen);
    config.insert_libraries = vec![first.clone(), second.clone()];
    let mapped = Loader::parse(&main, config)
        .expect("parse")
        .check_security()
        .expect("allow main")
        .map_images()
        .expect("map shared dependency once");
    assert_eq!(mapped.images_loaded(), 4);
    assert_eq!(*seen.lock().unwrap(), vec![main, first, second, shared]);
}

#[test]
fn inserted_dependency_cycles_terminate_without_duplicate_mapping() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let inserted = directory.write("inserted.dylib", 6, &["@loader_path/a.dylib"]);
    let a = directory.write("a.dylib", 6, &["@loader_path/b.dylib"]);
    let b = directory.write("b.dylib", 6, &["@loader_path/a.dylib"]);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut config = config_for(&main, &directory.0.join("not-present"), &seen);
    config.insert_libraries.push(inserted.clone());
    let mapped = Loader::parse(&main, config)
        .expect("parse")
        .check_security()
        .expect("allow main")
        .map_images()
        .expect("map cyclic graph");
    assert_eq!(mapped.images_loaded(), 4);
    assert_eq!(*seen.lock().unwrap(), vec![main, inserted, a, b]);
}

#[test]
fn absent_policy_still_loads_the_inserted_dependency_closure() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let directory = FixtureDirectory::new();
    let main = directory.write("main", 2, &[]);
    let inserted = directory.write("inserted.dylib", 6, &["@loader_path/parent.dylib"]);
    directory.write("parent.dylib", 6, &["@loader_path/leaf.dylib"]);
    directory.write("leaf.dylib", 6, &[]);
    let mut config = RuntimeConfig::permissive(&main);
    config.shim_set.objc = false;
    config.shim_set.foundation = false;
    config.insert_libraries.push(inserted);
    let checked = Loader::parse(&main, config)
        .expect("parse")
        .check_security()
        .expect("permissive admission");
    assert!(checked.security_report().is_none());
    let mapped = checked.map_images().expect("map complete inserted graph");
    assert_eq!(mapped.images_loaded(), 4);
}

#[test]
fn parent_relative_dependencies_are_resolved_before_name_normalization() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for inserted_root in [false, true] {
        let directory = FixtureDirectory::new();
        directory.write("denied.dylib", 6, &[]);
        let main = if inserted_root {
            directory.write("main", 2, &[])
        } else {
            directory.write("nested/main", 2, &["@loader_path/../denied.dylib"])
        };
        let inserted = inserted_root.then(|| {
            directory.write(
                "nested/inserted.dylib",
                6,
                &["@loader_path/../denied.dylib"],
            )
        });
        let denied = directory.0.join("nested/../denied.dylib");
        for route in ROUTES {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let mut config = config_for(&main, &denied, &seen);
            if let Some(path) = &inserted {
                config.insert_libraries.push(path.clone());
            }
            assert_denied(route, route.load(&main, config));
            let mut expected = vec![main.clone()];
            expected.extend(inserted.iter().cloned());
            expected.push(denied.clone());
            assert_eq!(*seen.lock().unwrap(), expected);
        }
    }
}
