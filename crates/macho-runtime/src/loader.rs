//! BFS dependency loading and dylib path resolution.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use macho_exec::{normalize_install_name, ImageRegistry};
use macho_parse::{parse, ParsedMachO};
use platform_core::fs::FilesystemProvider;
use runtime_diagnostics::DiagEvent;

use crate::config::RuntimeConfig;

use crate::error::RuntimeError;
use crate::tracer::RuntimeTracer;

/// Aggregate timing + counter stats for BFS dependency loading.
#[derive(Debug, Clone, Default)]
pub struct DependencyLoadStats {
    pub direct_deps_requested: usize,
    pub deps_enqueued: usize,
    pub subdeps_enqueued: usize,
    pub system_libraries_skipped: usize,
    pub already_loaded_skipped: usize,
    pub resolve_misses: usize,
    pub resolved: usize,
    pub read_failures: usize,
    pub parse_failures: usize,
    pub load_failures: usize,
    pub images_loaded: usize,
    pub bytes_read: usize,
    pub resolve_time: Duration,
    pub read_time: Duration,
    pub parse_time: Duration,
    pub registry_load_time: Duration,
}

impl DependencyLoadStats {
    pub fn total_time(&self) -> Duration {
        self.resolve_time + self.read_time + self.parse_time + self.registry_load_time
    }

    pub fn record_to(&self, tracer: &RuntimeTracer) {
        tracer.record_perf_timing("map_images", "deps_resolve_us", self.resolve_time);
        tracer.record_perf_timing("map_images", "deps_read_us", self.read_time);
        tracer.record_perf_timing("map_images", "deps_parse_us", self.parse_time);
        tracer.record_perf_timing(
            "map_images",
            "deps_registry_load_us",
            self.registry_load_time,
        );
        tracer.record_perf_timing("map_images", "deps_total_us", self.total_time());
        tracer.record_perf_counter(
            "map_images",
            "direct_deps_requested",
            self.direct_deps_requested as u64,
        );
        tracer.record_perf_counter("map_images", "deps_enqueued", self.deps_enqueued as u64);
        tracer.record_perf_counter(
            "map_images",
            "subdeps_enqueued",
            self.subdeps_enqueued as u64,
        );
        tracer.record_perf_counter(
            "map_images",
            "system_libraries_skipped",
            self.system_libraries_skipped as u64,
        );
        tracer.record_perf_counter(
            "map_images",
            "already_loaded_skipped",
            self.already_loaded_skipped as u64,
        );
        tracer.record_perf_counter("map_images", "resolve_misses", self.resolve_misses as u64);
        tracer.record_perf_counter("map_images", "resolved", self.resolved as u64);
        tracer.record_perf_counter("map_images", "read_failures", self.read_failures as u64);
        tracer.record_perf_counter("map_images", "parse_failures", self.parse_failures as u64);
        tracer.record_perf_counter("map_images", "load_failures", self.load_failures as u64);
        tracer.record_perf_counter("map_images", "images_loaded", self.images_loaded as u64);
        tracer.record_perf_counter("map_images", "bytes_read", self.bytes_read as u64);
    }
}

/// BFS-load dylib dependencies, skipping system libraries.
pub(crate) fn load_dependencies(
    main_parsed: &ParsedMachO,
    registry: &mut ImageRegistry,
    binary_dir: &Path,
    fs: &dyn FilesystemProvider,
    config: &RuntimeConfig,
) -> Result<DependencyLoadStats, RuntimeError> {
    let mut queue: VecDeque<(String, Vec<String>, PathBuf)> = VecDeque::new();
    let mut stats = DependencyLoadStats::default();

    // Seed with main binary's direct deps
    let rpaths = main_parsed.rpaths.clone();
    for dlib in &main_parsed.dylibs {
        if is_system_library(&dlib.name) {
            stats.system_libraries_skipped += 1;
            continue;
        }
        stats.direct_deps_requested += 1;
        stats.deps_enqueued += 1;
        queue.push_back((dlib.name.clone(), rpaths.clone(), binary_dir.to_path_buf()));
    }

    while let Some((raw_name, rpaths, loader_dir)) = queue.pop_front() {
        let install_name = normalize_install_name(&raw_name);
        if registry.get(&install_name).is_some() {
            stats.already_loaded_skipped += 1;
            continue; // already loaded
        }

        runtime_diagnostics::emit(DiagEvent::ResourceRequest {
            kind: "dylib".into(),
            name: install_name.clone(),
        });

        let resolve_start = Instant::now();
        let resolved_path = resolve_dylib_path(&install_name, &rpaths, binary_dir, &loader_dir, fs);
        stats.resolve_time += resolve_start.elapsed();

        let Some(path) = resolved_path else {
            log::debug!("Dependency not found locally: {install_name}");
            stats.resolve_misses += 1;
            runtime_diagnostics::emit(DiagEvent::ResourceFailed {
                kind: "dylib".into(),
                name: install_name.clone(),
                reason: "not found on disk".into(),
            });
            continue;
        };
        stats.resolved += 1;

        let read_start = Instant::now();
        let dylib_data = match fs.read(&path) {
            Ok(d) => d,
            Err(e) => {
                stats.read_time += read_start.elapsed();
                stats.read_failures += 1;
                log::warn!("Failed to read {}: {e}", path.display());
                runtime_diagnostics::emit(DiagEvent::ResourceFailed {
                    kind: "dylib".into(),
                    name: install_name.clone(),
                    reason: format!("read error: {e}"),
                });
                continue;
            }
        };
        stats.read_time += read_start.elapsed();
        stats.bytes_read += dylib_data.len();

        // Parse to discover sub-dependencies before loading
        let parse_start = Instant::now();
        let sub_parsed = match parse(&dylib_data) {
            Ok(p) => p,
            Err(e) => {
                stats.parse_time += parse_start.elapsed();
                stats.parse_failures += 1;
                log::warn!("Failed to parse {install_name}: {e}");
                runtime_diagnostics::emit(DiagEvent::ResourceFailed {
                    kind: "dylib".into(),
                    name: install_name.clone(),
                    reason: format!("parse error: {e}"),
                });
                continue;
            }
        };
        stats.parse_time += parse_start.elapsed();

        // Every locally loaded image must pass the same admission evaluator as
        // the main executable before its children are considered or it is mapped.
        crate::security::evaluate_security(&sub_parsed, &path, config)?;

        // Enqueue sub-dependencies
        let sub_rpaths = sub_parsed.rpaths.clone();
        let sub_loader_dir = path.parent().unwrap_or(binary_dir).to_path_buf();
        for sub_dlib in &sub_parsed.dylibs {
            if is_system_library(&sub_dlib.name) {
                stats.system_libraries_skipped += 1;
                continue;
            }
            if registry.get(&sub_dlib.name).is_none() {
                stats.subdeps_enqueued += 1;
                stats.deps_enqueued += 1;
                queue.push_back((
                    sub_dlib.name.clone(),
                    sub_rpaths.clone(),
                    sub_loader_dir.clone(),
                ));
            }
        }

        let load_start = Instant::now();
        match registry.load_preparsed(&install_name, Arc::new(sub_parsed), Arc::from(dylib_data)) {
            Ok(_) => {
                stats.registry_load_time += load_start.elapsed();
                stats.images_loaded += 1;
                log::info!("Loaded dependency: {install_name} ({})", path.display());
                runtime_diagnostics::emit(DiagEvent::ResourceResolved {
                    kind: "dylib".into(),
                    name: install_name,
                    addr: 0, // no single address for an image
                });
            }
            Err(e) => {
                stats.registry_load_time += load_start.elapsed();
                stats.load_failures += 1;
                log::warn!("Failed to load {install_name}: {e}");
                runtime_diagnostics::emit(DiagEvent::ResourceFailed {
                    kind: "dylib".into(),
                    name: install_name,
                    reason: format!("load error: {e}"),
                });
            }
        }
    }

    Ok(stats)
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
    loader_dir: &Path,
    fs: &dyn FilesystemProvider,
) -> Option<PathBuf> {
    if let Some(rest) = name.strip_prefix("@rpath/") {
        for rpath in rpaths {
            let resolved = if rpath == "@executable_path" {
                binary_dir.to_path_buf()
            } else if rpath == "@loader_path" {
                loader_dir.to_path_buf()
            } else if let Some(r) = rpath.strip_prefix("@executable_path/") {
                binary_dir.join(r)
            } else if let Some(r) = rpath.strip_prefix("@loader_path/") {
                loader_dir.join(r)
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
        let candidate = loader_dir.join(rest);
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

/// Load insert libraries (from `DYLD_INSERT_LIBRARIES`) into the registry
/// before normal dependency loading.  Each library is parsed and loaded;
/// its sub-dependencies are NOT loaded (they'll be resolved normally later
/// through the dependency BFS).
///
/// Returns the number of libraries successfully loaded.
pub(crate) fn load_insert_libraries(
    paths: &[std::path::PathBuf],
    registry: &mut ImageRegistry,
    fs: &dyn FilesystemProvider,
    config: &RuntimeConfig,
) -> Result<usize, RuntimeError> {
    let mut loaded = 0;
    for path in paths {
        let install_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("insert_lib")
            .to_string();
        if registry.get(&install_name).is_some() {
            log::debug!("insert library already loaded: {install_name}");
            continue;
        }
        let data = match fs.read(path) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("DYLD_INSERT_LIBRARIES: cannot read {}: {e}", path.display());
                continue;
            }
        };
        let parsed = match parse(&data) {
            Ok(p) => p,
            Err(e) => {
                log::warn!(
                    "DYLD_INSERT_LIBRARIES: cannot parse {}: {e}",
                    path.display()
                );
                continue;
            }
        };
        // Inserted libraries are executable inputs too; reject them before mapping.
        crate::security::evaluate_security(&parsed, path, config)?;
        match registry.load_preparsed(&install_name, Arc::new(parsed), Arc::from(data)) {
            Ok(_) => {
                log::info!("DYLD_INSERT_LIBRARIES: loaded {}", path.display());
                loaded += 1;
            }
            Err(e) => {
                log::warn!("DYLD_INSERT_LIBRARIES: cannot load {}: {e}", path.display());
            }
        }
    }
    Ok(loaded)
}
