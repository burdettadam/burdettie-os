//! NSNib stubs for AppKit.
//!
//! Apps that use nib-based UI call `+[NSBundle loadNibNamed:owner:]` or
//! `[[NSNib alloc] initWithNibNamed:bundle:]` on launch. The bundle's
//! `loadNibNamed:` now uses the Apple keyed archive decoder to load
//! real nib files; `NSNib` stores the name/bundle for deferred loading.
//!
//! Object layout (24 bytes):
//!   offset 0:  ISA    (u64)
//!   offset 8:  name   (u64 — NSString nib name)
//!   offset 16: bundle (u64 — NSBundle, 0 = mainBundle)

const NAME_OFF: usize = 8;
const BUNDLE_OFF: usize = 16;

/// `+[NSNib alloc]` — returns a minimal object.
pub extern "C" fn shim_nsnib_alloc(cls: u64, _cmd: u64) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, 24) as u64;
    unsafe {
        *(ptr as *mut u64) = cls;
    }
    ptr
}

/// `-[NSNib initWithNibNamed:bundle:]` — stores name and bundle.
pub extern "C" fn shim_nsnib_init_with_name(obj: u64, _cmd: u64, name: u64, bundle: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + NAME_OFF) as *mut u64) = name;
        *((obj as usize + BUNDLE_OFF) as *mut u64) = bundle;
    }
    log::debug!("NSNib initWithNibNamed:bundle:");
    obj
}

/// `-[NSNib instantiateWithOwner:topLevelObjects:]`
///
/// Loads the nib via NSBundle loadNibNamed: if a name was stored.
pub extern "C" fn shim_nsnib_instantiate(obj: u64, _cmd: u64, owner: u64, objects: u64) -> u8 {
    if obj == 0 {
        return 1;
    } // stub: YES
    let name = unsafe { *((obj as usize + NAME_OFF) as *const u64) };
    if name == 0 {
        return 1;
    } // no name stored

    let bundle = unsafe { *((obj as usize + BUNDLE_OFF) as *const u64) };
    let bundle = if bundle == 0 {
        crate::nsbundle::get_or_create_main_bundle()
    } else {
        bundle
    };

    crate::nsbundle::shim_nsbundle_load_nib_named(bundle, 0, name, owner, objects).max(1)
    // return at least YES for compat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const NIB_SIZE: usize = 24;

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
