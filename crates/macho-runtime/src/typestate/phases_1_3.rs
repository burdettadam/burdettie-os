use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use libsystem_shims::ShimConfig;
use macho_exec::{ImageRegistry, StubResolver, DataSymbolStore};
use macho_parse::ParsedMachO;

use crate::config::RuntimeConfig;
use crate::context::RuntimeContext;
use crate::error::RuntimeError;
use crate::parse_cache::{insert_both, lookup};
use crate::tracer::{EventKind, RuntimeTracer};
use crate::{loader, security};

use super::{Checked, Loader, LoaderInner, Mapped, Parsed};

// ── Phase 1: Parse ──────────────────────────────────────────────────

impl Loader<Parsed> {
    /// Read and parse a Mach-O binary (handles fat binaries).
    ///
    /// Results are cached per binary path: subsequent calls for the same
    /// path skip file I/O and re-parsing, returning Arc clones instead.
    pub fn parse(binary_path: &Path, config: RuntimeConfig) -> Result<Self, RuntimeError> {
        #[cfg(target_os = "macos")]
        macho_parse::set_platform_hasher(Arc::new(platform_darwin_host::DarwinHasher));
        #[cfg(target_os = "linux")]
        macho_parse::set_platform_hasher(Arc::new(platform_linux::LinuxHasher));

        let mut config = config;
        let tracer = config
            .tracer
            .take()
            .unwrap_or_else(RuntimeTracer::disabled);

        // Fast path: try raw path first (no realpath syscall), then canonical.
        if let Some((main_data, parsed)) = lookup(binary_path) {
            tracer.record(
                EventKind::Pipeline,
                format!(
                    "parsed {} (cached: {} segments, {} commands)",
                    binary_path.display(),
                    parsed.segments.len(),
                    parsed.commands.len(),
                ),
            );
            return Ok(Loader {
                inner: LoaderInner {
                    binary_path: binary_path.to_path_buf(),
                    config,
                    tracer,
                    parsed,
                    main_data,
                    security_report: None,
                    ctx: None,
                    host_symbols: None,
                    stub_resolver: None,
                    data_symbols: None,
                    registry: None,
                    app_idx: 0,
                    initializers_run: 0,
                },
                _phase: PhantomData,
            });
        }

        // Slow path: read file, parse, and populate the cache.
        let data = config
            .platform
            .fs()
            .read(binary_path)
            .map_err(|e| RuntimeError::Io {
                path: binary_path.to_path_buf(),
                source: e,
            })?;

        let parsed_result = macho_parse::parse_auto(&data, macho_types::CpuType::Arm64)?;
        tracer.record(
            EventKind::Pipeline,
            format!(
                "parsed {} ({} segments, {} commands)",
                binary_path.display(),
                parsed_result.segments.len(),
                parsed_result.commands.len(),
            ),
        );

        // Convert to Arc types — zero-copy for thin binaries, slice copy for fat.
        let main_data: Arc<[u8]> = if macho_parse::is_fat(&data) {
            let slice = macho_parse::fat_slice_for_cpu(&data, macho_types::CpuType::Arm64)?;
            Arc::from(slice)
        } else {
            Arc::from(data)
        };
        let parsed: Arc<ParsedMachO> = Arc::new(parsed_result);

        let canonical = binary_path.canonicalize().unwrap_or_else(|_| binary_path.to_path_buf());
        insert_both(binary_path, canonical, main_data.clone(), parsed.clone());

        Ok(Loader {
            inner: LoaderInner {
                binary_path: binary_path.to_path_buf(),
                config,
                tracer,
                parsed,
                main_data,
                security_report: None,
                ctx: None,
                host_symbols: None,
                stub_resolver: None,
                data_symbols: None,
                registry: None,
                app_idx: 0,
                initializers_run: 0,
            },
            _phase: PhantomData,
        })
    }
}

// ── Phase 2: Security check ─────────────────────────────────────────

impl Loader<Parsed> {
    /// Evaluate the security policy against the parsed binary.
    pub fn check_security(mut self) -> Result<Loader<Checked>, RuntimeError> {
        self.inner.security_report = security::evaluate_security(
            &self.inner.parsed,
            &self.inner.binary_path,
            &self.inner.config,
        )?;
        self.inner.tracer.record(
            EventKind::Pipeline,
            format!(
                "security: {}",
                if self.inner.security_report.is_some() {
                    "evaluated"
                } else {
                    "skipped"
                },
            ),
        );
        Ok(Loader {
            inner: self.inner,
            _phase: PhantomData,
        })
    }
}

// ── Phase 3: Map images ─────────────────────────────────────────────

impl Loader<Checked> {
    /// Build shim table, load dependencies, and map all images into memory.
    pub fn map_images(mut self) -> Result<Loader<Mapped>, RuntimeError> {
        let inner = &mut self.inner;

        // Build shim table
        let shim_setup_start = Instant::now();
        let shim_config = ShimConfig {
            process_name: inner.config.process_name.clone(),
            executable_path: inner.config.process_path.to_string_lossy().to_string(),
            argc: 1,
            argv: vec![inner.config.process_name.clone()],
            env: inner.config.env_overrides.clone(),
        };
        let ctx = RuntimeContext::new(&shim_config, &inner.config.shim_set);
        let shim_setup_time = shim_setup_start.elapsed();
        inner.tracer.record(
            EventKind::Pipeline,
            format!("shims installed: {}", ctx.shim_table.len()),
        );
        inner
            .tracer
            .record_perf_timing("map_images", "shim_setup_us", shim_setup_time);
        inner
            .tracer
            .record_perf_counter("map_images", "shims_installed", ctx.shim_table.len() as u64);

        // Host symbol resolver
        let host_setup_start = Instant::now();
        #[cfg(target_os = "macos")]
        let host_symbols: Arc<dyn platform_core::symbols::HostSymbolResolver> =
            Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        #[cfg(target_os = "linux")]
        let host_symbols: Arc<dyn platform_core::symbols::HostSymbolResolver> =
            Arc::new(platform_linux::LinuxSymbolResolver::new());

        objc_runtime::set_host_symbols(host_symbols.clone());

        #[cfg(target_os = "macos")]
        objc_runtime::set_allocator(Arc::new(platform_darwin_host::DarwinAllocator));
        #[cfg(target_os = "linux")]
        objc_runtime::set_allocator(Arc::new(platform_linux::LinuxAllocator));

        #[cfg(target_os = "macos")]
        objc_runtime::set_threading(Arc::new(platform_darwin_host::PosixThreading));
        #[cfg(target_os = "linux")]
        objc_runtime::set_threading(Arc::new(platform_linux::LinuxThreading));

        let host_setup_time = host_setup_start.elapsed();
        inner
            .tracer
            .record_perf_timing("map_images", "host_setup_us", host_setup_time);

        let stub_resolver_start = Instant::now();
        let stub_resolver = StubResolver::new(host_symbols.clone());
        let data_symbols = DataSymbolStore::new();
        inner.tracer.record_perf_timing(
            "map_images",
            "stub_resolver_setup_us",
            stub_resolver_start.elapsed(),
        );

        // Create ImageRegistry and load all images
        #[cfg(target_os = "macos")]
        let unwind_registry: Arc<dyn platform_core::unwind::UnwindRegistry> =
            Arc::new(platform_darwin_host::DarwinUnwindRegistry);
        #[cfg(target_os = "linux")]
        let unwind_registry: Arc<dyn platform_core::unwind::UnwindRegistry> =
            Arc::new(platform_linux::LinuxUnwindRegistry::new());

        let mut registry =
            ImageRegistry::new(inner.config.platform.memory(), unwind_registry);
        let binary_dir = inner.binary_path.parent().unwrap_or(Path::new("."));
        let dep_stats = loader::load_dependencies(
            &inner.parsed,
            &mut registry,
            binary_dir,
            inner.config.platform.fs(),
        )?;
        dep_stats.record_to(&inner.tracer);

        // Arc clones are O(1) refcount increments — no heap allocation.
        let main_data = inner.main_data.clone();
        let main_parsed = inner.parsed.clone();
        let main_load_start = Instant::now();
        let app_idx = registry.load_preparsed("__main__", main_parsed, main_data)?;
        let main_load_time = main_load_start.elapsed();
        inner
            .tracer
            .record_perf_timing("map_images", "main_image_load_us", main_load_time);
        inner
            .tracer
            .record_perf_counter("map_images", "mapped_images_total", registry.images().len() as u64);

        for loaded in registry.images() {
            inner.tracer.record(
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

        inner.ctx = Some(ctx);
        inner.host_symbols = Some(host_symbols);
        inner.stub_resolver = Some(stub_resolver);
        inner.data_symbols = Some(data_symbols);
        inner.registry = Some(registry);
        inner.app_idx = app_idx;

        Ok(Loader {
            inner: self.inner,
            _phase: PhantomData,
        })
    }
}

