//! BFS dependency loading and dylib path resolution.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use macho_exec::ImageRegistry;
use macho_parse::{parse, ParsedMachO};
use platform_core::fs::FilesystemProvider;
use runtime_diagnostics::DiagEvent;

use crate::error::RuntimeError;

/// BFS-load dylib dependencies, skipping system libraries.
pub(crate) fn load_dependencies(
    main_parsed: &ParsedMachO,
    registry: &mut ImageRegistry,
    binary_dir: &Path,
    fs: &dyn FilesystemProvider,
) -> Result<(), RuntimeError> {
    let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();

    // Seed with main binary's direct deps
    let rpaths = main_parsed.rpaths.clone();
    for dlib in &main_parsed.dylibs {
        if is_system_library(&dlib.name) {
            continue;
        }
        queue.push_back((dlib.name.clone(), rpaths.clone()));
    }

    while let Some((install_name, rpaths)) = queue.pop_front() {
        if registry.get(&install_name).is_some() {
            continue; // already loaded
        }

        runtime_diagnostics::emit(DiagEvent::ResourceRequest {
            kind: "dylib".into(),
            name: install_name.clone(),
        });

        let Some(path) = resolve_dylib_path(&install_name, &rpaths, binary_dir, fs) else {
            log::debug!("Dependency not found locally: {install_name}");
            runtime_diagnostics::emit(DiagEvent::ResourceFailed {
                kind: "dylib".into(),
                name: install_name.clone(),
                reason: "not found on disk".into(),
            });
            continue;
        };

        let dylib_data = match fs.read(&path) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("Failed to read {}: {e}", path.display());
                runtime_diagnostics::emit(DiagEvent::ResourceFailed {
                    kind: "dylib".into(),
                    name: install_name.clone(),
                    reason: format!("read error: {e}"),
                });
                continue;
            }
        };

        // Parse to discover sub-dependencies before loading
        let sub_parsed = match parse(&dylib_data) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("Failed to parse {install_name}: {e}");
                runtime_diagnostics::emit(DiagEvent::ResourceFailed {
                    kind: "dylib".into(),
                    name: install_name.clone(),
                    reason: format!("parse error: {e}"),
                });
                continue;
            }
        };

        // Enqueue sub-dependencies
        let sub_rpaths = sub_parsed.rpaths.clone();
        for sub_dlib in &sub_parsed.dylibs {
            if is_system_library(&sub_dlib.name) {
                continue;
            }
            if registry.get(&sub_dlib.name).is_none() {
                queue.push_back((sub_dlib.name.clone(), sub_rpaths.clone()));
            }
        }

        match registry.load(&install_name, dylib_data) {
            Ok(_) => {
                log::info!("Loaded dependency: {install_name} ({})", path.display());
                runtime_diagnostics::emit(DiagEvent::ResourceResolved {
                    kind: "dylib".into(),
                    name: install_name,
                    addr: 0, // no single address for an image
                });
            }
            Err(e) => {
                log::warn!("Failed to load {install_name}: {e}");
                runtime_diagnostics::emit(DiagEvent::ResourceFailed {
                    kind: "dylib".into(),
                    name: install_name,
                    reason: format!("load error: {e}"),
                });
            }
        }
    }

    Ok(())
}

/// Check if a dylib name refers to a system library that should be
/// resolved via the host process (StubResolver) rather than loaded
/// from disk.
pub(crate) fn is_system_library(name: &str) -> bool {
    name.starts_with("/usr/lib/") || name.starts_with("/System/") || name.starts_with("/Library/")
}

/// Resolve a dylib install name to a local file path.
///
/// Handles `@rpath/`, `@executable_path/`, `@loader_path/` prefixes
/// by searching relative to the binary directory.
pub(crate) fn resolve_dylib_path(
    name: &str,
    rpaths: &[String],
    binary_dir: &Path,
    fs: &dyn FilesystemProvider,
) -> Option<PathBuf> {
    if let Some(rest) = name.strip_prefix("@rpath/") {
        for rpath in rpaths {
            let resolved = if rpath == "@executable_path" || rpath == "@loader_path" {
                binary_dir.to_path_buf()
            } else if let Some(r) = rpath.strip_prefix("@executable_path/") {
                binary_dir.join(r)
            } else if let Some(r) = rpath.strip_prefix("@loader_path/") {
                binary_dir.join(r)
            } else {
                PathBuf::from(rpath)
            };
            let candidate = resolved.join(rest);
            if fs.exists(&candidate) {
                return Some(candidate);
            }
        }
        // Also try binary_dir directly as fallback
        let candidate = binary_dir.join(rest);
        if fs.exists(&candidate) {
            return Some(candidate);
        }
    } else if let Some(rest) = name.strip_prefix("@executable_path/") {
        let candidate = binary_dir.join(rest);
        if fs.exists(&candidate) {
            return Some(candidate);
        }
    } else if let Some(rest) = name.strip_prefix("@loader_path/") {
        let candidate = binary_dir.join(rest);
        if fs.exists(&candidate) {
            return Some(candidate);
        }
    } else {
        let path = PathBuf::from(name);
        if fs.exists(&path) {
            return Some(path);
        }
    }
    None
}
