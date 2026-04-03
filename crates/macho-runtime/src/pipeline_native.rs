//! Native execution pipeline (load_and_run).

use std::path::Path;

use libsystem_shims::ShimConfig;
use macho_exec::{call_constructors, ImageRegistry, StubResolver, DataSymbolStore};
use macho_objc::extract_initializers;

use crate::config::RuntimeConfig;
use crate::context::RuntimeContext;
use crate::error::RuntimeError;
use crate::resolver::CompositeResolver;
use crate::tracer::{EventKind, RuntimeTracer};
use crate::RuntimeResult;
use crate::{loader, objc_setup, security};
use std::time::Instant;

/// Drive the full native execution pipeline.
pub(super) fn run_native(
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

    // 1–2. Read + parse — fast path returns cached Arc<[u8]> + Arc<ParsedMachO>
    // without any file I/O or realpath syscalls on iterations after the first.
    let (main_data, arc_parsed) =
        if let Some(cached) = crate::parse_cache::lookup(binary_path) {
            cached
        } else {
            // Slow path: read file and parse, then populate cache.
            let data = config
                .platform
                .fs()
                .read(binary_path)
                .map_err(|e| RuntimeError::Io {
                    path: binary_path.to_path_buf(),
                    source: e,
                })?;
            let parsed_result = macho_parse::parse_auto(&data, macho_types::CpuType::Arm64)?;
            let main_data: std::sync::Arc<[u8]> = if macho_parse::is_fat(&data) {
                let slice =
                    macho_parse::fat_slice_for_cpu(&data, macho_types::CpuType::Arm64)?;
                std::sync::Arc::from(slice)
            } else {
                std::sync::Arc::from(data)
            };
            let arc_parsed = std::sync::Arc::new(parsed_result);
            let canonical =
                binary_path.canonicalize().unwrap_or_else(|_| binary_path.to_path_buf());
            crate::parse_cache::insert_both(
                binary_path,
                canonical,
                main_data.clone(),
                arc_parsed.clone(),
            );
            (main_data, arc_parsed)
        };
    tracer.record(
        EventKind::Pipeline,
        format!(
            "parsed {} ({} segments, {} commands)",
            binary_path.display(),
            arc_parsed.segments.len(),
            arc_parsed.commands.len(),
        ),
    );

    // 3. Security evaluation
    let security_report = security::evaluate_security(&arc_parsed, binary_path, config)?;
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
    let shim_setup_start = Instant::now();
    let shim_config = ShimConfig {
        process_name: config.process_name.clone(),
        executable_path: config.process_path.to_string_lossy().to_string(),
        argc: 1,
        argv: vec![config.process_name.clone()],
        env: config.env_overrides.clone(),
    };
    let ctx = RuntimeContext::new(&shim_config, &config.shim_set);
    let shim_setup_time = shim_setup_start.elapsed();
    tracer.record(
        EventKind::Pipeline,
        format!("shims installed: {}", ctx.shim_table.len(),),
    );
    tracer.record_perf_timing("map_images", "shim_setup_us", shim_setup_time);
    tracer.record_perf_counter("map_images", "shims_installed", ctx.shim_table.len() as u64);

    // 5. Build composite resolver: shims → host fallback
    let host_setup_start = Instant::now();
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
    tracer.record_perf_timing("map_images", "host_setup_us", host_setup_start.elapsed());

    let stub_resolver_start = Instant::now();
    let stub_resolver = StubResolver::new(host_symbols);
    tracer.record_perf_timing(
        "map_images",
        "stub_resolver_setup_us",
        stub_resolver_start.elapsed(),
    );
    let data_symbols = DataSymbolStore::new();
    let composite = CompositeResolver::new(&ctx.shim_table, &data_symbols, Some(&stub_resolver));

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
    let dep_stats =
        loader::load_dependencies(&arc_parsed, &mut registry, binary_dir, config.platform.fs())?;
    dep_stats.record_to(&tracer);

    // Load the main binary last
    let main_load_start = Instant::now();
    let app_idx = registry.load_preparsed("__main__", arc_parsed, main_data)?;
    tracer.record_perf_timing("map_images", "main_image_load_us", main_load_start.elapsed());
    tracer.record_perf_counter("map_images", "mapped_images_total", registry.images().len() as u64);
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

    // 7.3. Patch TLV thunks before protect() makes __DATA read-only
    for loaded in registry.images() {
        macho_exec::patch_tlv_thunks(&loaded.image, &loaded.parsed, &loaded.data);
    }

    // 7.5. Register ObjC metadata (phases A–E)
    if config.shim_set.objc {
        let _stats = objc_setup::run_registration_pipeline(&registry, config, &tracer);
    }

    // 8. Dispatch ObjC +load methods (before constructors, __DATA_CONST still RW)
    // Phase 1: patch known crash sites while __TEXT is still writable.
    for loaded in registry.images() {
        macho_exec::patch::patch_known_crashes(&loaded.image, &loaded.parsed);
    }
    // Phase 2: make __TEXT executable so +load and constructors can run.
    for loaded in registry.images() {
        loaded.image.protect_executable()?;
    }
    let mut load_count = 0;
    if config.shim_set.objc {
        let guard = objc_runtime::OBJC_RUNTIME
            .read()
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

    // 9.5. Protect all images (mprotect: RX for __TEXT, R for __DATA_CONST)
    // This MUST run after constructors — matching dyld behavior where
    // __DATA_CONST stays writable during initialization.
    for loaded in registry.images() {
        loaded.image.protect()?;
    }
    tracer.record(
        EventKind::Pipeline,
        format!("mprotect applied to {} images", registry.images().len(),),
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
