//! NSWorkspace shim: workspace singleton stub.

use std::sync::OnceLock;

static SHARED_WORKSPACE: OnceLock<u64> = OnceLock::new();

fn get_or_create_shared_workspace() -> u64 {
    *SHARED_WORKSPACE.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj == 0 { return 0; }
        unsafe { *(obj as *mut u64) = crate::nsobj::nsworkspace_class_addr(); }
        obj
    })
}

/// `+[NSWorkspace sharedWorkspace]`
pub extern "C" fn shim_nsworkspace_shared_workspace(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_shared_workspace()
}
