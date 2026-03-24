use serde::{Deserialize, Serialize};

/// Entry point for the main executable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryPoint {
    /// LC_MAIN style — offset relative to __TEXT segment.
    Main { entry_offset: u64, stack_size: u64 },
    /// LC_UNIXTHREAD style — absolute address in thread state.
    UnixThread { entry_address: u64 },
}
