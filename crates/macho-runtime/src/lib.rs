//! Runtime orchestration for darwin-loader-lab.
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
mod loader;
pub mod resolver;
pub mod tracer;

pub use config::{RuntimeConfig, ShimSet};
pub use context::RuntimeContext;
pub use error::RuntimeError;
pub use resolver::CompositeResolver;
pub use tracer::{EventKind, RuntimeEvent, RuntimeTracer, TRACER_SCHEMA_VERSION};

use std::path::Path;

use libsystem_shims::ShimConfig;
use macho_exec::{call_constructors, call_entry, ImageRegistry, StubResolver};
use macho_objc::{extract_initializers, parse_objc_metadata};
use macho_parse::ParsedMachO;
use trust_policy::{CodeSignatureInfo, SecurityReport};

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
        macho_parse::set_platform_hasher(std::sync::Arc::new(platform_darwin_host::DarwinHasher));

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
        let security_report = Self::evaluate_security(&parsed, binary_path, config)?;
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
        let host_symbols: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        let stub_resolver = if config.host_fallback {
            Some(StubResolver::new(host_symbols))
        } else {
            None
        };
        let composite = CompositeResolver::new(&ctx.shim_table, stub_resolver.as_ref());

        // 6. Load dependencies + main binary into ImageRegistry
        let mut registry = ImageRegistry::new(config.platform.memory());
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
                // Initialize cf-lite's platform provider for Foundation shims
                cf_lite::set_platform(config.platform.clone());

                cf_lite::FoundationShims::register_nsobject_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsstring_proper_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsstring_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsarray_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsnumber_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsdictionary_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsmutablearray_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsmutabledictionary_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsmutablestring_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsdata_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsprocessinfo_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nserror_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsbundle_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsfilemanager_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsexception_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsenumerator_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsdate_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsuserdefaults_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsset_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsmutableset_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsurl_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsnull_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsdateformatter_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nscharacterset_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsnotificationcenter_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsvalue_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsuuid_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nslocale_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nstimezone_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nscalendar_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsnumberformatter_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsindexset_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsmutableindexset_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nssortdescriptor_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nstimer_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsthread_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nslock_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsrecursivelock_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nscondition_class(&mut objc_rt);
                cf_lite::FoundationShims::register_nsconditionlock_class(&mut objc_rt);
                tracer.record(
                    EventKind::ClassRegistration,
                    "40 synthetic Foundation classes registered",
                );
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

        // 8.1. Register unwind info for all mapped images so that
        // @try/@catch/@finally and C++ exceptions work through mapped code.
        // libunwind needs to know about our images' __unwind_info / __eh_frame
        // sections to find landing pads during stack unwinding.
        for loaded in registry.images() {
            macho_exec::register_unwind_info(&loaded.parsed.segments, loaded.image.slide);
        }
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
        let entry_result = call_entry(&app.image, entry)?;
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

    /// Evaluate security policy, returning the report or an error if denied.
    fn evaluate_security(
        parsed: &ParsedMachO,
        binary_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<Option<SecurityReport>, RuntimeError> {
        let Some(policy) = &config.security_policy else {
            return Ok(None);
        };

        // Reject encrypted binaries
        for cmd in &parsed.commands {
            if let macho_types::load_command::LoadCommandPayload::EncryptionInfo {
                crypt_id, ..
            } = &cmd.payload
            {
                if *crypt_id != 0 {
                    return Err(RuntimeError::EncryptedBinary {
                        crypt_id: *crypt_id,
                    });
                }
            }
        }

        let cs_info = parsed
            .code_signature
            .as_ref()
            .map(CodeSignatureInfo::from_parsed)
            .unwrap_or_default();

        let report = policy.evaluate_binary_parsed(binary_path, parsed.header.flags, &cs_info);

        if !report.allowed {
            return Err(RuntimeError::SecurityDenied {
                reasons: report.denied_reasons.clone(),
            });
        }

        log::info!(
            "Security check passed for {} (hardened={}, strip_dyld_env={})",
            binary_path.display(),
            report.hardened_runtime,
            report.strip_dyld_env,
        );

        Ok(Some(report))
    }
}
