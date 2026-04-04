//! Interpreted execution pipeline (load_and_run_interpreted).

use std::path::Path;

use libsystem_shims::ShimConfig;
use macho_exec::{ImageRegistry, StubResolver, DataSymbolStore};

use crate::config::RuntimeConfig;
use crate::context::RuntimeContext;
use crate::error::RuntimeError;
use crate::resolver::CompositeResolver;
use crate::tracer::RuntimeTracer;
use crate::RuntimeResult;
use crate::{interp_bridge, loader, objc_setup};

/// Drive the ARM64 interpreted execution pipeline.
///
/// Same setup steps as the native pipeline through fixups + ObjC registration,
/// then executes via the ARM64 interpreter instead of native entry transfer.
pub(super) fn run_interpreted(
    binary_path: &Path,
    config: &RuntimeConfig,
) -> Result<RuntimeResult, RuntimeError> {
    // Initialize platform hasher for code signature verification
    #[cfg(target_os = "macos")]
    macho_parse::set_platform_hasher(std::sync::Arc::new(platform_darwin_host::DarwinHasher));
    #[cfg(target_os = "linux")]
    macho_parse::set_platform_hasher(std::sync::Arc::new(platform_linux::LinuxHasher));

    // 1–2. Read + parse — fast path returns cached Arc<[u8]> + Arc<ParsedMachO>.
    let (main_data, arc_parsed) =
        if let Some(cached) = crate::parse_cache::lookup(binary_path) {
            cached
        } else {
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

    let stub_resolver = StubResolver::new(host_symbols);
    let data_symbols = DataSymbolStore::global();
    let composite = CompositeResolver::new(&ctx.shim_table, data_symbols, Some(&stub_resolver));

    // 6. Load + map
    #[cfg(target_os = "macos")]
    let unwind_registry: std::sync::Arc<dyn platform_core::unwind::UnwindRegistry> =
        std::sync::Arc::new(platform_darwin_host::DarwinUnwindRegistry);
    #[cfg(target_os = "linux")]
    let unwind_registry: std::sync::Arc<dyn platform_core::unwind::UnwindRegistry> =
        std::sync::Arc::new(platform_linux::LinuxUnwindRegistry::new());
    let mut registry = ImageRegistry::new(config.platform.memory(), unwind_registry);
    let binary_dir = binary_path.parent().unwrap_or(Path::new("."));

    // 5.5. Load DYLD_INSERT_LIBRARIES (if any, and not blocked by hardened runtime).
    {
        let mut insert_paths = config.insert_libraries.clone();
        for (key, val) in &config.env_overrides {
            if key == "DYLD_INSERT_LIBRARIES" {
                for p in val.split(':') {
                    let p = p.trim();
                    if !p.is_empty() {
                        insert_paths.push(std::path::PathBuf::from(p));
                    }
                }
            }
        }
        if !insert_paths.is_empty() {
            let count =
                loader::load_insert_libraries(&insert_paths, &mut registry, config.platform.fs());
            log::info!("DYLD_INSERT_LIBRARIES: loaded {count}/{} libraries", insert_paths.len());
        }
    }

    let _dep_stats =
        loader::load_dependencies(&arc_parsed, &mut registry, binary_dir, config.platform.fs())?;
    let app_idx = registry.load_preparsed("__main__", arc_parsed, main_data)?;

    // 7. Apply fixups
    registry.apply_all_fixups(&composite)?;

    // 7.5. ObjC registration (phases A–E, no event tracing on interpreted path)
    if config.shim_set.objc {
        let _stats =
            objc_setup::run_registration_pipeline(&registry, config, &RuntimeTracer::disabled());
    }

    // 8. Protect segments
    for loaded in registry.images() {
        loaded.image.protect()?;
    }

    // 10. Build interpreter infrastructure (needed for +load and main).
    let app = &registry.images()[app_idx];
    let entry = app
        .parsed
        .entry
        .as_ref()
        .ok_or(RuntimeError::NoEntryPoint)?
        .clone();

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

    // 10b. Fire +load methods from app binary via the interpreter.
    if config.shim_set.objc {
        let app_ranges: Vec<(u64, u64)> = registry.images()[app_idx]
            .image
            .segments
            .iter()
            .map(|s| (s.base as u64, s.base as u64 + s.size as u64))
            .collect();
        let guard = objc_runtime::OBJC_RUNTIME
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(rt) = guard.as_ref() {
            let loads = rt.collect_load_methods();
            drop(guard);

            // Collect +load IMPs in app range.
            let app_loads: Vec<_> = loads
                .iter()
                .filter(|(_, imp)| app_ranges.iter().any(|&(lo, hi)| *imp >= lo && *imp < hi))
                .collect();

            if !app_loads.is_empty() {
                // Build segment list for the +load interpreter.
                let mut load_segments: Vec<(u64, *mut u8, usize, bool)> = registry
                    .images()[app_idx]
                    .image
                    .segments
                    .iter()
                    .map(|s| {
                        let writable = s.name != "__TEXT";
                        (s.base as u64, s.base, s.size, writable)
                    })
                    .collect();
                load_segments.extend_from_slice(&all_segments);

                for (class_name, imp) in &app_loads {
                    log::info!(
                        "Interpreter: calling +[{} load] at {:#x} via interpreter",
                        class_name,
                        imp
                    );
                    let ffi = interp_bridge::build_interpreter_bridge(&guest_ranges);
                    // SAFETY: segments are valid for the duration of this block.
                    let mut mem = unsafe {
                        arm64_translate::Memory::from_segments(&load_segments)
                    };
                    mem.host_passthrough = true;
                    let mut cpu = arm64_translate::CpuState::new();
                    cpu.pc = *imp;
                    cpu.sp = mem.initial_sp();
                    cpu.x[30] = 0; // RET → 0 triggers ReturnedToHost exit
                    let mut interp = arm64_translate::Interpreter::new(cpu, mem, ffi);
                    let _ = interp.run();
                }
            }
        }
    }

    // 11. Interpreter execution of main()
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
        &registry.images()[app_idx].image,
        &entry,
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
