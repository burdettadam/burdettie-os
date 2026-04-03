//! Runtime execution context — mutable state during a run.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use libsystem_shims::{ShimConfig, ShimTable};

use crate::config::ShimSet;

/// Reserve near the full-set shim count up front to avoid repeated HashMap growth
/// during startup. The full runtime currently installs roughly the mid-400s shims.
const SHIM_TABLE_INITIAL_CAPACITY: usize = 512;

/// Per-ShimSet shim table cache. Key is a bitmask of the 10 ShimSet flags.
/// Building a ~450-entry shim table costs ~16us; cloning a pre-built one costs
/// ~2us. The cache persists for the process lifetime (intentional — shims are
/// all static function pointers that never change between runs).
static SHIM_TABLE_CACHE: OnceLock<Mutex<HashMap<u16, ShimTable>>> = OnceLock::new();

fn shim_table_cache() -> &'static Mutex<HashMap<u16, ShimTable>> {
    SHIM_TABLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn shim_set_key(s: &ShimSet) -> u16 {
    (s.process as u16)
        | ((s.tls as u16) << 1)
        | ((s.dispatch as u16) << 2)
        | ((s.pthread as u16) << 3)
        | ((s.mach as u16) << 4)
        | ((s.lifecycle as u16) << 5)
        | ((s.ptrauth as u16) << 6)
        | ((s.objc as u16) << 7)
        | ((s.foundation as u16) << 8)
        | ((s.random as u16) << 9)
}

/// Mutable runtime state created at the start of execution.
pub struct RuntimeContext {
    /// Symbol → address shim table.
    pub shim_table: ShimTable,
}

impl RuntimeContext {
    /// Build the runtime context: create shim table and install shims
    /// according to the configuration.
    pub fn new(shim_config: &ShimConfig, shim_set: &ShimSet) -> Self {
        let key = shim_set_key(shim_set);

        // Fast path: clone a pre-built table (~2us) instead of rebuilding (~16us).
        // ProcessShims stores process identity in a OnceLock<ProcessState> separate
        // from the table entries, so cloning the cached table is always correct.
        // The PROCESS_STATE OnceLock is populated on the first (cold) build below.
        {
            let guard = shim_table_cache()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(cached) = guard.get(&key) {
                return Self { shim_table: cached.clone() };
            }
        }

        // Cold path: build the full table, then cache it for subsequent runs.
        let mut table = ShimTable::with_capacity(SHIM_TABLE_INITIAL_CAPACITY);

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
        if shim_set.darwin_stubs {
            #[cfg(target_os = "macos")]
            libsystem_shims::install_darwin_stubs(&mut table);
        }

        log::info!("RuntimeContext: installed {} shims", table.len());

        // Populate cache; use entry() to avoid overwriting a concurrently-stored entry.
        shim_table_cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(key)
            .or_insert_with(|| table.clone());

        Self { shim_table: table }
    }
}
