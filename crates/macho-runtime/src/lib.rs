//! Runtime orchestration for burdetty-os.
//!
//! `macho-runtime` provides a single entry point — `Runtime::load_and_run` —
//! that takes a Mach-O binary path and a configuration, then drives the
//! full pipeline:
//!
//! 1. Parse the binary
//! 2. Evaluate security policy
//! 3. Resolve and load dependencies (BFS)
//! 4. Map all images via `ImageRegistry`
//! 5. Install shims into the resolution chain
//! 6. Apply fixups (rebases + binds) across all images
//! 7. Protect all images (mprotect)
//! 8. Run constructors for all images in load order
//! 9. Transfer to entry point
//!
//! The runtime **never** re-implements parsing, mapping, or fixup logic.
//! It orchestrates the loader core crates and provides the execution
//! environment (shims, TLS, process identity) that real programs expect.

pub mod config;
pub mod context;
pub mod error;
pub mod interp_bridge;
mod loader;
mod objc_setup;
pub mod resolver;
mod security;
pub mod tracer;

pub use config::{RuntimeConfig, ShimSet};
pub use context::RuntimeContext;
pub use error::RuntimeError;
pub use resolver::CompositeResolver;
pub use tracer::{EventKind, RuntimeEvent, RuntimeTracer, TRACER_SCHEMA_VERSION};

use std::path::Path;

use libsystem_shims::ShimConfig;
use macho_exec::{call_constructors, ImageRegistry, StubResolver};
use macho_objc::{extract_initializers, parse_objc_metadata};
use trust_policy::SecurityReport;

/// Result of a complete runtime execution.
#[derive(Debug)]
pub struct RuntimeResult {
    /// Process exit code from entry point.
    pub exit_code: i32,
    /// Security evaluation report (if policy was applied).
    pub security_report: Option<SecurityReport>,
    /// Number of images loaded (including main binary).
    pub images_loaded: usize,
    /// Number of shims installed.
    pub shims_installed: usize,
    /// Number of constructors/initializers run.
    pub initializers_run: usize,
    /// Runtime event tracer with collected events (if tracing was enabled).
    pub tracer: Option<RuntimeTracer>,
}

/// Top-level runtime facade.
pub struct Runtime;

impl Runtime {
    /// Load a Mach-O binary and run it to completion.
    ///
    /// This is the primary entry point for the runtime layer.
    /// It orchestrates parsing, security, dependency resolution,
    /// mapping (via `ImageRegistry`), shim installation, fixups,
    /// protection, initializers, and entry.
    pub fn load_and_run(
        binary_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<RuntimeResult, RuntimeError> {
        let tracer = config
            .tracer
            .clone()
            .unwrap_or_else(RuntimeTracer::disabled);

        // Initialize platform hasher for code signature verification
        #[cfg(target_os = "macos")]
        macho_parse::set_platform_hasher(std::sync::Arc::new(platform_darwin_host::DarwinHasher));
        #[cfg(target_os = "linux")]
        macho_parse::set_platform_hasher(std::sync::Arc::new(platform_linux::LinuxHasher));

        // 1. Read binary data
        let data = config
            .platform
            .fs()
            .read(binary_path)
            .map_err(|e| RuntimeError::Io {
                path: binary_path.to_path_buf(),
                source: e,
            })?;

        // 2. Parse (handles fat binaries automatically, preferring arm64)
        let parsed = macho_parse::parse_auto(&data, macho_types::CpuType::Arm64)?;
        tracer.record(
            EventKind::Pipeline,
            format!(
                "parsed {} ({} segments, {} commands)",
                binary_path.display(),
                parsed.segments.len(),
                parsed.commands.len(),
            ),
        );

        // For mapping we need the raw slice data matching the parsed arch
        let main_data = if macho_parse::is_fat(&data) {
            macho_parse::fat_slice_for_cpu(&data, macho_types::CpuType::Arm64)?.to_vec()
        } else {
            data
        };

        // 3. Security evaluation
        let security_report = security::evaluate_security(&parsed, binary_path, config)?;
        tracer.record(
            EventKind::Pipeline,
            format!(
                "security: {}",
                if security_report.is_some() {
                    "evaluated"
                } else {
                    "skipped"
                },
            ),
        );

        // 4. Build shim table
        let shim_config = ShimConfig {
            process_name: config.process_name.clone(),
            executable_path: config.process_path.to_string_lossy().to_string(),
            argc: 1,
            argv: vec![config.process_name.clone()],
            env: config.env_overrides.clone(),
        };
        let ctx = RuntimeContext::new(&shim_config, &config.shim_set);
        tracer.record(
            EventKind::Pipeline,
            format!("shims installed: {}", ctx.shim_table.len(),),
        );

        // 5. Build composite resolver: shims → host fallback
        #[cfg(target_os = "macos")]
        let host_symbols: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        #[cfg(target_os = "linux")]
        let host_symbols: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_linux::LinuxSymbolResolver::new());

        // Install host symbol resolver for objc-runtime exception bridge
        objc_runtime::set_host_symbols(host_symbols.clone());

        // Install platform allocator for objc-runtime object memory
        #[cfg(target_os = "macos")]
        objc_runtime::set_allocator(
            std::sync::Arc::new(platform_darwin_host::DarwinAllocator),
        );
        #[cfg(target_os = "linux")]
        objc_runtime::set_allocator(
            std::sync::Arc::new(platform_linux::LinuxAllocator),
        );

        // Install platform threading for objc-runtime @synchronized
        #[cfg(target_os = "macos")]
        objc_runtime::set_threading(
            std::sync::Arc::new(platform_darwin_host::PosixThreading),
        );
        #[cfg(target_os = "linux")]
        objc_runtime::set_threading(
            std::sync::Arc::new(platform_linux::LinuxThreading),
        );

        let stub_resolver = if config.host_fallback {
            Some(StubResolver::new(host_symbols))
        } else {
            None
        };
        let composite = CompositeResolver::new(&ctx.shim_table, stub_resolver.as_ref());

        // 6. Load dependencies + main binary into ImageRegistry
        #[cfg(target_os = "macos")]
        let unwind_registry: std::sync::Arc<dyn platform_core::unwind::UnwindRegistry> =
            std::sync::Arc::new(platform_darwin_host::DarwinUnwindRegistry);
        #[cfg(target_os = "linux")]
        let unwind_registry: std::sync::Arc<dyn platform_core::unwind::UnwindRegistry> =
            std::sync::Arc::new(platform_linux::LinuxUnwindRegistry::new());
        let mut registry = ImageRegistry::new(
            config.platform.memory(),
            unwind_registry,
        );
        let binary_dir = binary_path.parent().unwrap_or(Path::new("."));

        // BFS dependency loading (skip system libraries)
        loader::load_dependencies(&parsed, &mut registry, binary_dir, config.platform.fs())?;

        // Load the main binary last
        let app_idx = registry.load("__main__", main_data)?;
        for loaded in registry.images() {
            tracer.record(
                EventKind::ImageLoad,
                format!(
                    "'{}' mapped at {:#x} ({} segments)",
                    loaded.install_name,
                    loaded
                        .image
                        .segments
                        .first()
                        .map(|s| s.base as u64)
                        .unwrap_or(0),
                    loaded.image.segments.len(),
                ),
            );
        }
        log::info!("Loaded {} images (including main)", registry.images().len());

        // 7. Apply fixups across all images with shim-aware resolver
        registry.apply_all_fixups(&composite)?;
        tracer.record(EventKind::Pipeline, "fixups applied to all images");

        // 7.5. Register ObjC metadata from all loaded images (two-phase).
        //
        // Phase A: Register classes from on-disk data. Class structs have valid
        // vmaddrs in on-disk data because they're defined in the same image.
        // Same-image categories are also merged here via register_image().
        //
        // Phase B: Merge cross-image categories by reading the resolved `cls`
        // pointer from mapped memory (post-fixup). This handles cases where a
        // category in image B targets a class defined in image A — the bind
        // slot is 0x0 on disk but contains the resolved address after fixups.
        if config.shim_set.objc {
            let mut objc_rt = objc_runtime::ObjcRuntime::new();
            let mut image_metadata: Vec<(macho_objc::ObjcMetadata, i64)> = Vec::new();

            // Phase A: register classes + same-image categories
            for loaded in registry.images() {
                let metadata = parse_objc_metadata(&loaded.parsed, &loaded.data);
                if metadata.presence.has_objc {
                    objc_rt.register_image(&metadata, loaded.image.slide);
                    for cls in &metadata.class_refs {
                        tracer.record(
                            EventKind::ClassRegistration,
                            format!(
                                "'{}' from '{}'",
                                cls.name.as_deref().unwrap_or("<anon>"),
                                loaded.install_name,
                            ),
                        );
                    }
                    log::info!(
                        "ObjC: registered {} classes from '{}'",
                        metadata.class_refs.len(),
                        loaded.install_name,
                    );
                    image_metadata.push((metadata, loaded.image.slide));
                }
            }

            // Phase B: merge cross-image categories from mapped memory
            for (metadata, slide) in &image_metadata {
                if !metadata.categories.is_empty() {
                    for cat in &metadata.categories {
                        tracer.record(
                            EventKind::CategoryMerge,
                            format!(
                                "category '{}' ({} instance, {} class methods)",
                                cat.name.as_deref().unwrap_or("<anon>"),
                                cat.instance_methods.len(),
                                cat.class_methods.len(),
                            ),
                        );
                    }
                    objc_rt.merge_categories_post_fixup(metadata, *slide);
                }
            }

            // Phase C: fix up __objc_selrefs to point at interned selectors
            for (metadata, slide) in &image_metadata {
                if !metadata.selref_addrs.is_empty() {
                    tracer.record(
                        EventKind::SelectorRegistration,
                        format!("{} selrefs fixed up", metadata.selref_addrs.len(),),
                    );
                    objc_rt.fixup_selrefs(metadata, *slide);
                }
            }

            // Phase D: register synthetic Foundation classes (NSObject, NSString, etc.)
            if config.shim_set.foundation {
                objc_setup::register_foundation_classes(&mut objc_rt, config, &tracer);
            }

            // Phase E: repair superclass addrs from mapped memory (post-fixup).
            // register_image reads superclass from on-disk data, where
            // bind-encoded refs (e.g., _OBJC_CLASS_$_NSObject) are unresolved.
            // After fixup the mapped memory has the correct addresses.
            unsafe {
                objc_rt.repair_superclass_addrs();
            }

            // Store in global — shim functions will read from it
            *objc_runtime::OBJC_RUNTIME
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(objc_rt);
        }

        // 8. Protect all images (mprotect: RX for __TEXT, R for __DATA_CONST)
        for loaded in registry.images() {
            loaded.image.protect()?;
        }
        tracer.record(
            EventKind::Pipeline,
            format!("mprotect applied to {} images", registry.images().len(),),
        );

        // 8.1. Unwind info is now registered automatically by ImageRegistry::load()
        // via the UnwindRegistry trait (DarwinUnwindRegistry on macOS).
        tracer.record(
            EventKind::Pipeline,
            format!(
                "unwind info registered for {} images",
                registry.images().len(),
            ),
        );

        // 8.5. Dispatch ObjC +load methods (after mprotect, before constructors)
        let mut load_count = 0;
        if config.shim_set.objc {
            let guard = objc_runtime::OBJC_RUNTIME
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(rt) = guard.as_ref() {
                let loads = rt.collect_load_methods();
                load_count = loads.len();
                // Must drop the lock before calling into loaded code
                drop(guard);
                for (class_name, imp) in &loads {
                    log::info!("Calling +[{} load] at {:#x}", class_name, imp);
                    tracer.record(
                        EventKind::Pipeline,
                        format!("+[{} load] at {:#x}", class_name, imp,),
                    );
                    let f: extern "C" fn() = unsafe { std::mem::transmute(*imp) };
                    f();
                }
            }
        }
        tracer.record(
            EventKind::Pipeline,
            format!("{} +load methods dispatched", load_count,),
        );

        // 9. Run constructors for all images in load order
        let mut total_initializers = 0;
        for loaded in registry.images() {
            let inits = extract_initializers(&loaded.parsed, &loaded.data);
            if !inits.constructors.is_empty() {
                total_initializers += call_constructors(&loaded.image, &inits)?;
            }
        }

        tracer.record(
            EventKind::Pipeline,
            format!("{} constructors/initializers run", total_initializers,),
        );

        // 10. Call entry point on the main binary
        let app = &registry.images()[app_idx];
        let entry = app
            .parsed
            .entry
            .as_ref()
            .ok_or(RuntimeError::NoEntryPoint)?;

        tracer.record(EventKind::Pipeline, "calling entry point".to_string());

        // Build entry args from RuntimeConfig
        let entry_args = macho_exec::EntryArgs {
            program_name: config.process_name.clone(),
            args: Vec::new(),
            env: config
                .env_overrides
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect(),
        };
        let entry_result =
            macho_exec::call_entry_with_args(&app.image, entry, Some(&entry_args))?;
        tracer.record(
            EventKind::Pipeline,
            format!("entry point returned exit_code={}", entry_result.exit_code,),
        );

        let shims_installed = ctx.shim_table.len();
        let images_loaded = registry.images().len();

        Ok(RuntimeResult {
            exit_code: entry_result.exit_code,
            security_report,
            images_loaded,
            shims_installed,
            initializers_run: total_initializers + load_count,
            tracer: if tracer.is_enabled() {
                Some(tracer)
            } else {
                None
            },
        })
    }

    /// Load a Mach-O binary and run it via the ARM64 interpreter.
    ///
    /// Same pipeline as [`load_and_run`] through phase 7.5 (parse, map, shim,
    /// fixup, ObjC registration), then executes through the interpreter instead
    /// of native execution. This enables running ObjC/Foundation fixtures on
    /// any architecture.
    pub fn load_and_run_interpreted(
        binary_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<RuntimeResult, RuntimeError> {
        // Initialize platform hasher for code signature verification
        #[cfg(target_os = "macos")]
        macho_parse::set_platform_hasher(std::sync::Arc::new(platform_darwin_host::DarwinHasher));
        #[cfg(target_os = "linux")]
        macho_parse::set_platform_hasher(std::sync::Arc::new(platform_linux::LinuxHasher));

        // 1. Read binary
        let data = config
            .platform
            .fs()
            .read(binary_path)
            .map_err(|e| RuntimeError::Io {
                path: binary_path.to_path_buf(),
                source: e,
            })?;

        // 2. Parse
        let parsed = macho_parse::parse_auto(&data, macho_types::CpuType::Arm64)?;
        let main_data = if macho_parse::is_fat(&data) {
            macho_parse::fat_slice_for_cpu(&data, macho_types::CpuType::Arm64)?.to_vec()
        } else {
            data
        };

        // 4. Build shim table
        let shim_config = ShimConfig {
            process_name: config.process_name.clone(),
            executable_path: config.process_path.to_string_lossy().to_string(),
            argc: 1,
            argv: vec![config.process_name.clone()],
            env: config.env_overrides.clone(),
        };
        let ctx = RuntimeContext::new(&shim_config, &config.shim_set);

        // 5. Build composite resolver
        #[cfg(target_os = "macos")]
        let host_symbols: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        #[cfg(target_os = "linux")]
        let host_symbols: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_linux::LinuxSymbolResolver::new());

        objc_runtime::set_host_symbols(host_symbols.clone());
        #[cfg(target_os = "macos")]
        objc_runtime::set_allocator(
            std::sync::Arc::new(platform_darwin_host::DarwinAllocator),
        );
        #[cfg(target_os = "linux")]
        objc_runtime::set_allocator(
            std::sync::Arc::new(platform_linux::LinuxAllocator),
        );
        #[cfg(target_os = "macos")]
        objc_runtime::set_threading(
            std::sync::Arc::new(platform_darwin_host::PosixThreading),
        );
        #[cfg(target_os = "linux")]
        objc_runtime::set_threading(
            std::sync::Arc::new(platform_linux::LinuxThreading),
        );

        let stub_resolver = if config.host_fallback {
            Some(StubResolver::new(host_symbols))
        } else {
            None
        };
        let composite = CompositeResolver::new(&ctx.shim_table, stub_resolver.as_ref());

        // 6. Load + map
        #[cfg(target_os = "macos")]
        let unwind_registry: std::sync::Arc<dyn platform_core::unwind::UnwindRegistry> =
            std::sync::Arc::new(platform_darwin_host::DarwinUnwindRegistry);
        #[cfg(target_os = "linux")]
        let unwind_registry: std::sync::Arc<dyn platform_core::unwind::UnwindRegistry> =
            std::sync::Arc::new(platform_linux::LinuxUnwindRegistry::new());
        let mut registry = ImageRegistry::new(config.platform.memory(), unwind_registry);
        let binary_dir = binary_path.parent().unwrap_or(Path::new("."));
        loader::load_dependencies(&parsed, &mut registry, binary_dir, config.platform.fs())?;
        let app_idx = registry.load("__main__", main_data)?;

        // 7. Apply fixups
        registry.apply_all_fixups(&composite)?;

        // 7.5. ObjC registration
        if config.shim_set.objc {
            let mut objc_rt = objc_runtime::ObjcRuntime::new();
            let mut image_metadata: Vec<(macho_objc::ObjcMetadata, i64)> = Vec::new();

            for loaded in registry.images() {
                let metadata = parse_objc_metadata(&loaded.parsed, &loaded.data);
                if metadata.presence.has_objc {
                    objc_rt.register_image(&metadata, loaded.image.slide);
                    image_metadata.push((metadata, loaded.image.slide));
                }
            }

            for (metadata, slide) in &image_metadata {
                if !metadata.categories.is_empty() {
                    objc_rt.merge_categories_post_fixup(metadata, *slide);
                }
            }
            for (metadata, slide) in &image_metadata {
                if !metadata.selref_addrs.is_empty() {
                    objc_rt.fixup_selrefs(metadata, *slide);
                }
            }
            if config.shim_set.foundation {
                let tracer = RuntimeTracer::disabled();
                objc_setup::register_foundation_classes(&mut objc_rt, config, &tracer);
            }
            unsafe {
                objc_rt.repair_superclass_addrs();
            }
            *objc_runtime::OBJC_RUNTIME
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(objc_rt);
        }

        // 8. Protect segments
        for loaded in registry.images() {
            loaded.image.protect()?;
        }

        // 10. Fire +load methods from app binary only (not system dylibs,
        //     whose +load IMPs depend on the full host ObjC runtime).
        if config.shim_set.objc {
            let app_ranges: Vec<(u64, u64)> = registry.images()[app_idx]
                .image
                .segments
                .iter()
                .map(|s| (s.base as u64, s.base as u64 + s.size as u64))
                .collect();
            let guard = objc_runtime::OBJC_RUNTIME
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(rt) = guard.as_ref() {
                let loads = rt.collect_load_methods();
                drop(guard);
                for (class_name, imp) in &loads {
                    let in_app = app_ranges.iter().any(|&(lo, hi)| *imp >= lo && *imp < hi);
                    if !in_app {
                        log::debug!(
                            "Interpreter: skipping +[{} load] at {:#x} (outside app)",
                            class_name,
                            imp
                        );
                        continue;
                    }
                    log::info!(
                        "Interpreter: calling +[{} load] at {:#x}",
                        class_name,
                        imp
                    );
                    let f: extern "C" fn() = unsafe { std::mem::transmute(*imp) };
                    f();
                }
            }
        }

        // 11. Interpreter execution
        let app = &registry.images()[app_idx];
        let entry = app
            .parsed
            .entry
            .as_ref()
            .ok_or(RuntimeError::NoEntryPoint)?;

        // Build guest memory ranges for the interpreter bridge.
        let guest_ranges: Vec<(u64, u64)> = registry
            .images()
            .iter()
            .flat_map(|loaded| {
                loaded.image.segments.iter().map(|s| {
                    (s.base as u64, s.base as u64 + s.size as u64)
                })
            })
            .collect();

        // Build segment list from ALL loaded images for the interpreter
        // memory model (so it can fetch instructions from dylib stubs).
        let all_segments: Vec<(u64, *mut u8, usize, bool)> = registry
            .images()
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != app_idx)
            .flat_map(|(_, loaded)| {
                loaded.image.segments.iter().map(|s| {
                    let writable = s.name != "__TEXT";
                    (s.base as u64, s.base, s.size, writable)
                })
            })
            .collect();

        let ffi_bridge = interp_bridge::build_interpreter_bridge(&guest_ranges);

        let entry_args = macho_exec::EntryArgs {
            program_name: config.process_name.clone(),
            args: Vec::new(),
            env: config
                .env_overrides
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect(),
        };

        let entry_result = macho_exec::call_entry_interpreted(
            &app.image,
            entry,
            Some(&entry_args),
            ffi_bridge,
            &all_segments,
        )?;

        Ok(RuntimeResult {
            exit_code: entry_result.exit_code,
            security_report: None,
            images_loaded: registry.images().len(),
            shims_installed: ctx.shim_table.len(),
            initializers_run: 0,
            tracer: None,
        })
    }

    /// Resolve an `.app` bundle path to the executable inside.
    ///
    /// Given a path like `Foo.app`, reads `Contents/Info.plist` for
    /// `CFBundleExecutable` and returns `Foo.app/Contents/MacOS/<exe>`.
    /// Returns `None` if the path is not an `.app` bundle.
    pub fn resolve_app_bundle(bundle_path: &Path) -> Option<std::path::PathBuf> {
        let name = bundle_path.file_name()?.to_str()?;
        if !name.ends_with(".app") {
            return None;
        }

        let contents = bundle_path.join("Contents");
        let macos = contents.join("MacOS");

        // Try Info.plist for CFBundleExecutable
        let plist_path = contents.join("Info.plist");
        if let Ok(data) = std::fs::read_to_string(&plist_path) {
            if let Some(exe_name) = parse_cf_bundle_executable(&data) {
                let exe = macos.join(&exe_name);
                if exe.exists() {
                    return Some(exe);
                }
            }
        }

        // Fallback: bundle name without .app suffix
        let stem = name.strip_suffix(".app")?;
        let exe = macos.join(stem);
        if exe.exists() {
            return Some(exe);
        }

        // Last resort: any executable in Contents/MacOS
        if let Ok(entries) = std::fs::read_dir(&macos) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    return Some(path);
                }
            }
        }

        None
    }

}

/// Minimal plist parser: extract CFBundleExecutable from Info.plist XML.
fn parse_cf_bundle_executable(plist_xml: &str) -> Option<String> {
    // Look for <key>CFBundleExecutable</key> followed by <string>...</string>
    let key_tag = "<key>CFBundleExecutable</key>";
    let key_pos = plist_xml.find(key_tag)?;
    let after_key = &plist_xml[key_pos + key_tag.len()..];

    let string_start = after_key.find("<string>")?;
    let value_start = string_start + "<string>".len();
    let value_end = after_key[value_start..].find("</string>")?;

    let value = after_key[value_start..value_start + value_end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
