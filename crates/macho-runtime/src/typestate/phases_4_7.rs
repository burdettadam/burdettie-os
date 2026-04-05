use std::marker::PhantomData;
use macho_exec::call_constructors;
use macho_objc::extract_initializers;

use crate::error::RuntimeError;
use crate::tracer::EventKind;
use crate::{objc_setup, CompositeResolver, RuntimeResult};

use super::{FixedUp, Initialized, Loader, Mapped, Protected, Registered};

// ── Phase 4: Apply fixups ───────────────────────────────────────────

impl Loader<Mapped> {
    /// Apply rebase + bind fixups across all mapped images.
    pub fn apply_fixups(mut self) -> Result<Loader<FixedUp>, RuntimeError> {
        {
            let ctx = self.inner.ctx.as_ref().expect("ctx set in map_images");
            let data_symbols = macho_exec::DataSymbolStore::global();
            let composite =
                CompositeResolver::new(&ctx.shim_table, data_symbols, self.inner.stub_resolver.as_ref());
            let registry = self.inner
                .registry
                .as_mut()
                .expect("registry set in map_images");
            registry.apply_all_fixups(&composite)?;
            // Patch TLV thunks before protect() makes __DATA read-only
            for loaded in registry.images() {
                macho_exec::patch_tlv_thunks(&loaded.image, &loaded.parsed, &loaded.data);
            }
            // Strip PAC-authenticated branches when hardware PAC is unavailable
            for loaded in registry.images() {
                macho_exec::ptrauth::strip_pac_branches(&loaded.image);
            }
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
            let registry = self.inner.registry.as_ref().expect("registry set");
            let _stats = objc_setup::run_registration_pipeline(
                registry,
                &self.inner.config,
                &self.inner.tracer,
            );
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
                .read()
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
