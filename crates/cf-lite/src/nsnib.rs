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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const NIB_SIZE: usize = 16;

    // --- shim_nsnib_alloc ---

    #[test]
    fn alloc_returns_nonzero_with_isa() {
        let cls = crate::nsobj::nsnib_class_addr();
        let obj = shim_nsnib_alloc(cls, 0);
        assert_ne!(obj, 0);
        // ISA should be set to cls
        assert_eq!(read_field(obj, 0), cls);
        free_obj(obj);
    }

    #[test]
    fn alloc_with_zero_cls_sets_isa_zero() {
        let obj = shim_nsnib_alloc(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), 0);
        free_obj(obj);
    }

    // --- shim_nsnib_init_with_name ---

    #[test]
    fn init_with_name_returns_self() {
        let obj = alloc_obj(NIB_SIZE, crate::nsobj::nsnib_class_addr());
        let result = shim_nsnib_init_with_name(obj, 0, 0, 0);
        assert_eq!(result, obj);
        free_obj(obj);
    }

    #[test]
    fn init_with_name_nil_returns_nil() {
        let result = shim_nsnib_init_with_name(0, 0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn init_with_name_ignores_args() {
        let obj = alloc_obj(NIB_SIZE, crate::nsobj::nsnib_class_addr());
        let name = make_string("MainMenu");
        let bundle = alloc_obj_fake(16);
        let result = shim_nsnib_init_with_name(obj, 0, name, bundle);
        assert_eq!(result, obj);
        free_obj(obj);
        free_obj(name);
        free_obj(bundle);
    }

    // --- shim_nsnib_instantiate ---

    #[test]
    fn instantiate_returns_yes() {
        let obj = alloc_obj(NIB_SIZE, crate::nsobj::nsnib_class_addr());
        let result = shim_nsnib_instantiate(obj, 0, 0, 0);
        assert_eq!(result, 1);
        free_obj(obj);
    }

    #[test]
    fn instantiate_nil_returns_yes() {
        // Stub always returns 1 regardless of receiver
        let result = shim_nsnib_instantiate(0, 0, 0, 0);
        assert_eq!(result, 1);
    }

    #[test]
    fn instantiate_ignores_owner_and_objects() {
        let obj = alloc_obj(NIB_SIZE, crate::nsobj::nsnib_class_addr());
        let owner = alloc_obj_fake(16);
        let objects = alloc_u64_buf(4);
        let result = shim_nsnib_instantiate(obj, 0, owner, objects);
        assert_eq!(result, 1);
        free_obj(obj);
        free_obj(owner);
        free_buf(objects);
    }
}
