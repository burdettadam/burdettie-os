//! Dynamic class creation shims: objc_allocateClassPair,
//! objc_registerClassPair, objc_disposeClassPair, objc_duplicateClass,
//! class_addIvar, class_addProperty.

use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::class::{ClassInfo, IvarEntry, PropertyEntry, OBJC_RUNTIME};

// ── Synthetic address allocator ───────────────────────────────────

/// Counter for generating unique class/metaclass addresses.
/// High sentinel range avoids collision with real mapped addresses.
static NEXT_SYNTH_ADDR: AtomicU64 = AtomicU64::new(0xD000_0000_0001_0000);

fn alloc_synth_addr() -> u64 {
    // Each allocation bumps by 0x1000 (page-aligned gap) to avoid aliasing.
    NEXT_SYNTH_ADDR.fetch_add(0x1000, Ordering::Relaxed)
}

// ── Staged class pairs ────────────────────────────────────────────

struct StagedClassPair {
    class: ClassInfo,
    metaclass: ClassInfo,
    metaclass_addr: u64,
}

static STAGED_CLASSES: Mutex<Option<HashMap<u64, StagedClassPair>>> = Mutex::new(None);

trait StagedClassInit {
    fn insert(&mut self, key: u64, val: StagedClassPair);
    fn remove(&mut self, key: &u64) -> Option<StagedClassPair>;
    fn get_mut(&mut self, key: &u64) -> Option<&mut StagedClassPair>;
}

impl StagedClassInit for std::sync::MutexGuard<'_, Option<HashMap<u64, StagedClassPair>>> {
    fn insert(&mut self, key: u64, val: StagedClassPair) {
        self.get_or_insert_with(HashMap::new).insert(key, val);
    }
    fn remove(&mut self, key: &u64) -> Option<StagedClassPair> {
        self.as_mut()?.remove(key)
    }
    fn get_mut(&mut self, key: &u64) -> Option<&mut StagedClassPair> {
        self.as_mut()?.get_mut(key)
    }
}

// ── Shims ─────────────────────────────────────────────────────────

/// `Class objc_allocateClassPair(Class superclass, const char *name, size_t extraBytes)`
///
/// Creates a new class and metaclass pair. The class is not yet visible
/// to `objc_getClass`; call `objc_registerClassPair` to finalize.
pub extern "C" fn shim_objc_allocate_class_pair(
    superclass: u64,
    name: *const c_char,
    _extra_bytes: u64,
) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }

    // Check if the name is already registered or staged.
    {
        let guard = match OBJC_RUNTIME.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(rt) = guard.as_ref() {
            if rt.get_class(name_str).is_some() {
                return 0; // already exists
            }
        }
    }

    // Resolve superclass info.
    let (superclass_addr, instance_size, is_root) = if superclass == 0 {
        (None, 8u32, true) // root class — minimum 8 bytes for ISA
    } else {
        let guard = match OBJC_RUNTIME.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(rt) = guard.as_ref() {
            if let Some(super_info) = rt.get_class_by_addr(superclass) {
                (Some(superclass), super_info.instance_size, false)
            } else {
                return 0; // superclass not found
            }
        } else {
            return 0;
        }
    };

    let cls_addr = alloc_synth_addr();
    let meta_addr = alloc_synth_addr();

    let class_info = ClassInfo {
        name: name_str.to_string(),
        mapped_addr: cls_addr,
        methods: Vec::new(),
        class_methods: Vec::new(),
        ivars: Vec::new(),
        properties: Vec::new(),
        protocol_names: Vec::new(),
        instance_size,
        is_root,
        is_meta: false,
        superclass_addr: superclass_addr,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };

    let metaclass_info = ClassInfo {
        name: name_str.to_string(),
        mapped_addr: meta_addr,
        methods: Vec::new(),
        class_methods: Vec::new(),
        ivars: Vec::new(),
        properties: Vec::new(),
        protocol_names: Vec::new(),
        instance_size: 40, // metaclass instance size (isa + superclass + cache + vtable + data)
        is_root: false,
        is_meta: true,
        superclass_addr: superclass_addr, // metaclass super = superclass's metaclass (simplified)
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };

    STAGED_CLASSES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(
            cls_addr,
            StagedClassPair {
                class: class_info,
                metaclass: metaclass_info,
                metaclass_addr: meta_addr,
            },
        );

    log::debug!(
        "objc_allocateClassPair: allocated '{}' at {:#x} (meta {:#x})",
        name_str,
        cls_addr,
        meta_addr
    );

    cls_addr
}

/// `void objc_registerClassPair(Class cls)`
///
/// Finalizes a previously allocated class pair and makes it visible to
/// `objc_getClass` and method dispatch.
pub extern "C" fn shim_objc_register_class_pair(cls: u64) {
    if cls == 0 {
        return;
    }

    let pair = STAGED_CLASSES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&cls);

    let Some(pair) = pair else { return };

    let mut guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return };

    let name = pair.class.name.clone();
    let meta_addr = pair.metaclass_addr;

    rt.register_synthetic_class(pair.class);
    rt.register_metaclass_addr(meta_addr, &name);

    // Also register the metaclass ClassInfo so class method lookup works.
    rt.register_synthetic_class(pair.metaclass);

    log::debug!(
        "objc_registerClassPair: registered '{}' at {:#x}",
        name,
        cls
    );
}

/// `void objc_disposeClassPair(Class cls)`
///
/// Destroys a previously created class pair. In our runtime this removes
/// the class from the class table. Only works for dynamically created classes.
pub extern "C" fn shim_objc_dispose_class_pair(cls: u64) {
    if cls == 0 {
        return;
    }

    // First check if it's still in staging (never registered).
    {
        let mut staged = STAGED_CLASSES
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if staged.remove(&cls).is_some() {
            log::debug!("objc_disposeClassPair: removed staged class {:#x}", cls);
            return;
        }
    }

    // Only dispose dynamically-created classes (high sentinel range).
    if cls < 0xD000_0000_0000_0000 {
        log::warn!(
            "objc_disposeClassPair: refusing to dispose non-dynamic class {:#x}",
            cls
        );
        return;
    }

    let mut guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return };

    rt.remove_class_by_addr(cls);
}

/// `Class objc_duplicateClass(Class original, const char *name, size_t extraBytes)`
///
/// Creates a copy of a class with a new name. Used by KVO to create
/// `NSKVONotifying_ClassName` subclasses.
pub extern "C" fn shim_objc_duplicate_class(
    original: u64,
    name: *const c_char,
    _extra_bytes: u64,
) -> u64 {
    if original == 0 || name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }

    let mut guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return 0 };

    // Already exists?
    if rt.get_class(name_str).is_some() {
        return 0;
    }

    let Some(orig) = rt.get_class_by_addr(original) else {
        return 0;
    };

    let new_addr = alloc_synth_addr();
    let meta_addr = alloc_synth_addr();

    let mut dup = orig.clone();
    dup.name = name_str.to_string();
    dup.mapped_addr = new_addr;
    dup.superclass_addr = Some(original); // duplicate is a subclass of original

    let mut meta = ClassInfo {
        name: name_str.to_string(),
        mapped_addr: meta_addr,
        methods: Vec::new(),
        class_methods: dup.class_methods.clone(),
        ivars: Vec::new(),
        properties: Vec::new(),
        protocol_names: Vec::new(),
        instance_size: 40,
        is_root: false,
        is_meta: true,
        superclass_addr: dup.superclass_addr,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    meta.rebuild_indices();

    rt.register_synthetic_class(dup);
    rt.register_metaclass_addr(meta_addr, name_str);
    rt.register_synthetic_class(meta);

    log::debug!(
        "objc_duplicateClass: duplicated {:#x} as '{}' at {:#x}",
        original,
        name_str,
        new_addr
    );

    new_addr
}

// ── class_addIvar / class_addProperty ─────────────────────────────

/// `BOOL class_addIvar(Class cls, const char *name, size_t size,
///     uint8_t alignment, const char *types)`
///
/// Adds an instance variable to an unregistered class. Cannot be called
/// on classes that have already been registered.
pub extern "C" fn shim_class_add_ivar(
    cls: u64,
    name: *const c_char,
    size: u64,
    _alignment: u8,
    types: *const c_char,
) -> u8 {
    if cls == 0 || name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }
    let type_str = if types.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(types) }
            .to_str()
            .unwrap_or("")
    };

    let mut staged = STAGED_CLASSES
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let Some(pair) = staged.get_mut(&cls) else {
        // Not staged — refuse (Apple semantics: can't add ivars after registration)
        return 0;
    };

    // Duplicate name check
    if pair.class.ivars.iter().any(|iv| iv.name == name_str) {
        return 0;
    }

    let offset = pair.class.instance_size;
    let ivar_size = size as u32;

    pair.class.ivars.push(IvarEntry {
        name: name_str.to_string(),
        type_encoding: type_str.to_string(),
        offset,
        size: ivar_size,
    });
    pair.class.instance_size += ivar_size;

    1
}

/// `BOOL class_addProperty(Class cls, const char *name,
///     const objc_property_attribute_t *attributes, unsigned int attributeCount)`
///
/// Adds a property to a class. Works on both staged and registered classes.
pub extern "C" fn shim_class_add_property(
    cls: u64,
    name: *const c_char,
    _attributes: u64,
    _attribute_count: u32,
) -> u8 {
    if cls == 0 || name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }

    // Try staged classes first.
    {
        let mut staged = STAGED_CLASSES
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(pair) = staged.get_mut(&cls) {
            if pair.class.properties.iter().any(|p| p.name == name_str) {
                return 0;
            }
            pair.class.properties.push(PropertyEntry {
                name: name_str.to_string(),
                attributes: String::new(), // TODO: parse attribute structs if needed
            });
            return 1;
        }
    }

    // Fall back to registered classes.
    let mut guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return 0 };
    let Some(class) = rt.get_class_by_addr_mut(cls) else {
        return 0;
    };

    if class.properties.iter().any(|p| p.name == name_str) {
        return 0;
    }
    class.properties.push(PropertyEntry {
        name: name_str.to_string(),
        attributes: String::new(),
    });
    1
}
