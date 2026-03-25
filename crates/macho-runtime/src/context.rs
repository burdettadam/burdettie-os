//! Runtime execution context — mutable state during a run.

use libsystem_shims::{ShimConfig, ShimTable};

use crate::config::ShimSet;

/// Mutable runtime state created at the start of execution.
pub struct RuntimeContext {
    /// Symbol → address shim table.
    pub shim_table: ShimTable,
}

impl RuntimeContext {
    /// Build the runtime context: create shim table and install shims
    /// according to the configuration.
    pub fn new(shim_config: &ShimConfig, shim_set: &ShimSet) -> Self {
        let mut table = ShimTable::new();

        if shim_set.process {
            libsystem_shims::ProcessShims::install(&mut table, shim_config);
        }
        if shim_set.tls {
            libsystem_shims::TlsShims::install(&mut table);
        }
        if shim_set.dispatch {
            libsystem_shims::DispatchShims::install(&mut table);
        }
        if shim_set.pthread {
            libsystem_shims::PthreadShims::install(&mut table);
        }
        if shim_set.mach {
            libsystem_shims::MachShims::install(&mut table);
        }
        if shim_set.lifecycle {
            libsystem_shims::LifecycleShims::install(&mut table);
        }
        if shim_set.ptrauth {
            libsystem_shims::PtrauthShims::install(&mut table);
        }
        if shim_set.objc {
            objc_runtime::ObjcShims::install(&mut table);
        }
        if shim_set.foundation {
            cf_lite::FoundationShims::install(&mut table);
        }
        if shim_set.random {
            libsystem_shims::RandomShims::install(&mut table);
        }

        log::info!("RuntimeContext: installed {} shims", table.len());

        Self { shim_table: table }
    }
}
