//! NSNib / NSCoder stubs for AppKit.
//!
//! Apps that use nib-based UI call `+[NSBundle loadNibNamed:owner:]` or
//! `[[NSNib alloc] initWithNibNamed:bundle:]` on launch. Our stubs
//! report success without actually loading nibs — programmatic-only
//! Cocoa apps don't need real nib loading.

/// `+[NSNib alloc]` — returns a minimal object.
pub extern "C" fn shim_nsnib_alloc(cls: u64, _cmd: u64) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, 16) as u64;
    unsafe {
        *(ptr as *mut u64) = cls;
    }
    ptr
}

/// `-[NSNib initWithNibNamed:bundle:]` → self (pretend we loaded it).
pub extern "C" fn shim_nsnib_init_with_name(obj: u64, _cmd: u64, _name: u64, _bundle: u64) -> u64 {
    log::debug!("NSNib initWithNibNamed:bundle: — stub (nib loading not supported)");
    obj
}

/// `-[NSNib instantiateWithOwner:topLevelObjects:]` → YES (no-op).
pub extern "C" fn shim_nsnib_instantiate(_obj: u64, _cmd: u64, _owner: u64, _objects: u64) -> u8 {
    log::debug!("NSNib instantiateWithOwner:topLevelObjects: — stub");
    1
}
