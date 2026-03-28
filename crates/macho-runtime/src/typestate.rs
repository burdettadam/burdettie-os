//! Typestate loader pipeline.
//!
//! Encodes the Mach-O loader lifecycle as a compile-time state machine.
//! Each phase of loading is a distinct type; transitions are enforced by
//! the type system — you cannot skip or reorder phases.
//!
//! # Phases
//!
//! ```text
//! Parsed → Checked → Mapped → FixedUp → Registered → Protected → Initialized
//!                                                                      ↓
//!                                                                RuntimeResult
//! ```
//!
//! # Example
//!
//! ```no_run
//! use macho_runtime::typestate::Loader;
//! use macho_runtime::RuntimeConfig;
//! use std::path::Path;
//!
//! let path = Path::new("fixtures/return42/app");
//! let config = RuntimeConfig::permissive(path);
//! let result = Loader::parse(path, config).unwrap()
//!     .check_security().unwrap()
//!     .map_images().unwrap()
//!     .apply_fixups().unwrap()
//!     .register_objc().unwrap()
//!     .protect().unwrap()
//!     .initialize().unwrap()
//!     .run().unwrap();
//! assert_eq!(result.exit_code, 42);
//! ```

use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libsystem_shims::ShimConfig;
use macho_exec::{call_constructors, ImageRegistry, StubResolver};
use macho_objc::{extract_initializers, parse_objc_metadata};
use macho_parse::ParsedMachO;
use trust_policy::SecurityReport;

use crate::config::RuntimeConfig;
use crate::context::RuntimeContext;
use crate::error::RuntimeError;
use crate::tracer::{EventKind, RuntimeTracer};
use crate::{loader, objc_setup, security, CompositeResolver, RuntimeResult};

// ── Phase markers (sealed) ──────────────────────────────────────────

mod sealed {
    pub trait Phase {}
}

/// Binary parsed, ready for security check.
pub struct Parsed;
/// Security evaluated, ready to map images.
pub struct Checked;
/// All images mapped into memory, ready for fixups.
pub struct Mapped;
/// Fixups applied (rebase + bind), ready for ObjC registration.
pub struct FixedUp;
/// ObjC metadata registered, ready to protect segments.
pub struct Registered;
/// Segment permissions finalized, ready for initialization.
pub struct Protected;
/// +load methods and constructors executed, ready to call entry.
pub struct Initialized;

impl sealed::Phase for Parsed {}
impl sealed::Phase for Checked {}
impl sealed::Phase for Mapped {}
impl sealed::Phase for FixedUp {}
impl sealed::Phase for Registered {}
impl sealed::Phase for Protected {}
impl sealed::Phase for Initialized {}

// ── Loader core ─────────────────────────────────────────────────────

/// Typestate Mach-O loader pipeline.
///
/// Each transition method consumes `self` and returns the next phase.
/// The compiler prevents calling methods in the wrong order.
pub struct Loader<S: sealed::Phase> {
    inner: LoaderInner,
    _phase: PhantomData<S>,
}

struct LoaderInner {
    binary_path: PathBuf,
    config: RuntimeConfig,
    tracer: RuntimeTracer,
    parsed: ParsedMachO,
    main_data: Vec<u8>,
    // Populated after check_security
    security_report: Option<SecurityReport>,
    // Populated after map_images
    ctx: Option<RuntimeContext>,
    #[allow(dead_code)]
    host_symbols: Option<Arc<dyn platform_core::symbols::HostSymbolResolver>>,
    stub_resolver: Option<StubResolver>,
    registry: Option<ImageRegistry>,
    app_idx: usize,
    initializers_run: usize,
}

// ── Common accessors (all phases) ───────────────────────────────────

impl<S: sealed::Phase> Loader<S> {
    /// The parsed Mach-O binary.
    pub fn parsed(&self) -> &ParsedMachO {
        &self.inner.parsed
    }

    /// Path to the original binary.
    pub fn binary_path(&self) -> &Path {
        &self.inner.binary_path
    }

    /// Security report (populated after `check_security`).
    pub fn security_report(&self) -> Option<&SecurityReport> {
        self.inner.security_report.as_ref()
    }

    /// Number of images loaded (populated after `map_images`).
    pub fn images_loaded(&self) -> usize {
        self.inner
            .registry
            .as_ref()
            .map(|r| r.images().len())
            .unwrap_or(0)
    }
}

// ── Phase 1: Parse ──────────────────────────────────────────────────

impl Loader<Parsed> {
    /// Read and parse a Mach-O binary (handles fat binaries).
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

        let data = config
            .platform
            .fs()
            .read(binary_path)
            .map_err(|e| RuntimeError::Io {
                path: binary_path.to_path_buf(),
                source: e,
            })?;

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

        let main_data = if macho_parse::is_fat(&data) {
            macho_parse::fat_slice_for_cpu(&data, macho_types::CpuType::Arm64)?.to_vec()
        } else {
            data
        };

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
        let shim_config = ShimConfig {
            process_name: inner.config.process_name.clone(),
            executable_path: inner.config.process_path.to_string_lossy().to_string(),
            argc: 1,
            argv: vec![inner.config.process_name.clone()],
            env: inner.config.env_overrides.clone(),
        };
        let ctx = RuntimeContext::new(&shim_config, &inner.config.shim_set);
        inner.tracer.record(
            EventKind::Pipeline,
            format!("shims installed: {}", ctx.shim_table.len()),
        );

        // Host symbol resolver
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

        let stub_resolver = if inner.config.host_fallback {
            Some(StubResolver::new(host_symbols.clone()))
        } else {
            None
        };

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
        loader::load_dependencies(
            &inner.parsed,
            &mut registry,
            binary_dir,
            inner.config.platform.fs(),
        )?;

        let main_data = std::mem::take(&mut inner.main_data);
        let app_idx = registry.load("__main__", main_data)?;

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
        inner.stub_resolver = stub_resolver;
        inner.registry = Some(registry);
        inner.app_idx = app_idx;

        Ok(Loader {
            inner: self.inner,
            _phase: PhantomData,
        })
    }
}

// ── Phase 4: Apply fixups ───────────────────────────────────────────

impl Loader<Mapped> {
    /// Apply rebase + bind fixups across all mapped images.
    pub fn apply_fixups(self) -> Result<Loader<FixedUp>, RuntimeError> {
        {
            let ctx = self.inner.ctx.as_ref().expect("ctx set in map_images");
            let composite =
                CompositeResolver::new(&ctx.shim_table, self.inner.stub_resolver.as_ref());
            self.inner
                .registry
                .as_ref()
                .expect("registry set in map_images")
                .apply_all_fixups(&composite)?;
        }
        self.inner
            .tracer
            .record(EventKind::Pipeline, "fixups applied to all images".to_string());
        Ok(Loader {
            inner: self.inner,
            _phase: PhantomData,
        })
    }
}

// ── Phase 5: ObjC registration ──────────────────────────────────────

impl Loader<FixedUp> {
    /// Register ObjC metadata, merge categories, fix selrefs, and install
    /// synthetic Foundation classes.
    pub fn register_objc(self) -> Result<Loader<Registered>, RuntimeError> {
        if self.inner.config.shim_set.objc {
            let mut objc_rt = objc_runtime::ObjcRuntime::new();
            let mut image_metadata: Vec<(macho_objc::ObjcMetadata, i64)> = Vec::new();

            let registry = self.inner.registry.as_ref().expect("registry set");

            // Phase A: register classes + same-image categories
            for loaded in registry.images() {
                let metadata = parse_objc_metadata(&loaded.parsed, &loaded.data);
                if metadata.presence.has_objc {
                    objc_rt.register_image(&metadata, loaded.image.slide);
                    for cls in &metadata.class_refs {
                        self.inner.tracer.record(
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

            // Phase B: cross-image categories
            for (metadata, slide) in &image_metadata {
                if !metadata.categories.is_empty() {
                    for cat in &metadata.categories {
                        self.inner.tracer.record(
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

            // Phase C: selref fixups
            for (metadata, slide) in &image_metadata {
                if !metadata.selref_addrs.is_empty() {
                    self.inner.tracer.record(
                        EventKind::SelectorRegistration,
                        format!("{} selrefs fixed up", metadata.selref_addrs.len()),
                    );
                    objc_rt.fixup_selrefs(metadata, *slide);
                }
            }

            // Phase D: synthetic Foundation classes
            if self.inner.config.shim_set.foundation {
                objc_setup::register_foundation_classes(
                    &mut objc_rt,
                    &self.inner.config,
                    &self.inner.tracer,
                );
            }

            // Phase E: superclass repair from mapped memory
            unsafe {
                objc_rt.repair_superclass_addrs();
            }

            *objc_runtime::OBJC_RUNTIME
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(objc_rt);
        }

        Ok(Loader {
            inner: self.inner,
            _phase: PhantomData,
        })
    }
}

// ── Phase 6: Protect segments ───────────────────────────────────────

impl Loader<Registered> {
    /// Finalize segment permissions (RX for `__TEXT`, R for `__DATA_CONST`).
    pub fn protect(self) -> Result<Loader<Protected>, RuntimeError> {
        let image_count = {
            let registry = self.inner.registry.as_ref().expect("registry set");
            for loaded in registry.images() {
                loaded.image.protect()?;
            }
            registry.images().len()
        };
        self.inner.tracer.record(
            EventKind::Pipeline,
            format!("mprotect applied to {image_count} images"),
        );
        Ok(Loader {
            inner: self.inner,
            _phase: PhantomData,
        })
    }
}

// ── Phase 7: Initialize (+load + constructors) ──────────────────────

impl Loader<Protected> {
    /// Dispatch ObjC +load methods and run C/C++ constructors.
    pub fn initialize(mut self) -> Result<Loader<Initialized>, RuntimeError> {
        let mut init_count = 0;

        // +load methods
        if self.inner.config.shim_set.objc {
            let guard = objc_runtime::OBJC_RUNTIME
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(rt) = guard.as_ref() {
                let loads = rt.collect_load_methods();
                init_count += loads.len();
                drop(guard);
                for (class_name, imp) in &loads {
                    log::info!("Calling +[{class_name} load] at {imp:#x}");
                    self.inner.tracer.record(
                        EventKind::Pipeline,
                        format!("+[{class_name} load] at {imp:#x}"),
                    );
                    let f: extern "C" fn() = unsafe { std::mem::transmute(*imp) };
                    f();
                }
            } else {
                drop(guard);
            }
        }

        // Constructors
        {
            let registry = self.inner.registry.as_ref().expect("registry set");
            for loaded in registry.images() {
                let inits = extract_initializers(&loaded.parsed, &loaded.data);
                if !inits.constructors.is_empty() {
                    init_count += call_constructors(&loaded.image, &inits)?;
                }
            }
        }

        self.inner.tracer.record(
            EventKind::Pipeline,
            format!("{init_count} constructors/initializers run"),
        );
        self.inner.initializers_run = init_count;

        Ok(Loader {
            inner: self.inner,
            _phase: PhantomData,
        })
    }
}

// ── Final: Run entry point ──────────────────────────────────────────

impl Loader<Initialized> {
    /// Transfer control to the binary's `LC_MAIN` entry point.
    pub fn run(self) -> Result<RuntimeResult, RuntimeError> {
        let Loader { inner, .. } = self;

        let registry = inner.registry.as_ref().expect("registry set");
        let app = &registry.images()[inner.app_idx];
        let entry = app
            .parsed
            .entry
            .as_ref()
            .ok_or(RuntimeError::NoEntryPoint)?;

        inner
            .tracer
            .record(EventKind::Pipeline, "calling entry point".to_string());

        let entry_args = macho_exec::EntryArgs {
            program_name: inner.config.process_name.clone(),
            args: Vec::new(),
            env: inner
                .config
                .env_overrides
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect(),
        };
        let entry_result =
            macho_exec::call_entry_with_args(&app.image, entry, Some(&entry_args))?;

        inner.tracer.record(
            EventKind::Pipeline,
            format!("entry point returned exit_code={}", entry_result.exit_code),
        );

        let shims_installed = inner
            .ctx
            .as_ref()
            .map(|c| c.shim_table.len())
            .unwrap_or(0);

        Ok(RuntimeResult {
            exit_code: entry_result.exit_code,
            security_report: inner.security_report,
            images_loaded: registry.images().len(),
            shims_installed,
            initializers_run: inner.initializers_run,
            tracer: if inner.tracer.is_enabled() {
                Some(inner.tracer)
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn typestate_return42() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/return42/return42");
        if !path.exists() {
            eprintln!("Skipping: fixture not found");
            return;
        }
        let config = RuntimeConfig::permissive(&path);
        let result = Loader::parse(&path, config)
            .expect("parse")
            .check_security()
            .expect("security")
            .map_images()
            .expect("map")
            .apply_fixups()
            .expect("fixups")
            .register_objc()
            .expect("objc")
            .protect()
            .expect("protect")
            .initialize()
            .expect("init")
            .run()
            .expect("run");
        assert_eq!(result.exit_code, 42);
    }
}
