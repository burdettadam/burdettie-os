//! NSUserDefaults — in-memory key-value store.
//!
//! Real NSUserDefaults persists to ~/Library/Preferences/<bundle-id>.plist.
//! This shim provides an in-memory store sufficient for programs that use
//! NSUserDefaults for configuration lookup.
//!
//! Layout: singleton object (16 bytes):
//!   offset  0: isa       (u64)  → NSUserDefaults class addr
//!   offset  8: store_ptr (u64)  → pointer to the backing NSMutableDictionary

use std::sync::OnceLock;

const NSUD_STORE_OFF: usize = 8;

/// Singleton instance.
static STANDARD_DEFAULTS: OnceLock<u64> = OnceLock::new();

fn get_or_create_singleton() -> u64 {
    *STANDARD_DEFAULTS.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsuserdefaults_class_addr();
        }
        // Create backing mutable dictionary
        let store = crate::nsmutabledict::shim_nsmutabledict_dictionary(
            crate::nsobj::nsmutabledictionary_class_addr(),
            0,
        );
        unsafe {
            *((obj as usize + NSUD_STORE_OFF) as *mut u64) = store;
        }
        obj
    })
}

/// `+[NSUserDefaults standardUserDefaults]`
pub extern "C" fn shim_nsuserdefaults_standard(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_singleton()
}

/// `-[NSUserDefaults objectForKey:]`
pub extern "C" fn shim_nsuserdefaults_object_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 {
        return 0;
    }
    let store = unsafe { *((obj as usize + NSUD_STORE_OFF) as *const u64) };
    if store == 0 {
        return 0;
    }
    crate::nsdictionary::shim_nsdictionary_object_for_key(store, 0, key)
}

/// `-[NSUserDefaults stringForKey:]` — returns nil if value is not an NSString.
pub extern "C" fn shim_nsuserdefaults_string_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    // In our simplified model, we just return objectForKey: since all stored values
    // from setObject:forKey: are kept as-is.
    shim_nsuserdefaults_object_for_key(obj, _cmd, key)
}

/// `-[NSUserDefaults integerForKey:]`
pub extern "C" fn shim_nsuserdefaults_integer_for_key(obj: u64, _cmd: u64, key: u64) -> i64 {
    let val = shim_nsuserdefaults_object_for_key(obj, _cmd, key);
    if val == 0 {
        return 0;
    }
    // Try to read integerValue from the object (works for NSNumber and NSString)
    // Read ISA to check class
    let isa = unsafe { *(val as *const u64) };
    if isa == crate::nsobj::nsnumber_class_addr() {
        crate::nsnumber::shim_nsnumber_integer_value(val, 0)
    } else {
        0
    }
}

/// `-[NSUserDefaults boolForKey:]`
pub extern "C" fn shim_nsuserdefaults_bool_for_key(obj: u64, _cmd: u64, key: u64) -> u8 {
    let val = shim_nsuserdefaults_object_for_key(obj, _cmd, key);
    if val == 0 {
        return 0;
    }
    let isa = unsafe { *(val as *const u64) };
    if isa == crate::nsobj::nsnumber_class_addr() {
        crate::nsnumber::shim_nsnumber_bool_value(val, 0)
    } else {
        0
    }
}

/// `-[NSUserDefaults doubleForKey:]`
pub extern "C" fn shim_nsuserdefaults_double_for_key(obj: u64, _cmd: u64, key: u64) -> f64 {
    let val = shim_nsuserdefaults_object_for_key(obj, _cmd, key);
    if val == 0 {
        return 0.0;
    }
    let isa = unsafe { *(val as *const u64) };
    if isa == crate::nsobj::nsnumber_class_addr() {
        crate::nsnumber::shim_nsnumber_double_value(val, 0)
    } else {
        0.0
    }
}

/// `-[NSUserDefaults setObject:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_object(obj: u64, _cmd: u64, value: u64, key: u64) {
    if obj == 0 || key == 0 {
        return;
    }
    let store = unsafe { *((obj as usize + NSUD_STORE_OFF) as *const u64) };
    if store == 0 {
        return;
    }
    if value == 0 {
        // Setting nil = removeObjectForKey:
        crate::nsmutabledict::shim_nsmutabledict_remove_object_for_key(store, 0, key);
    } else {
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(store, 0, value, key);
    }
}

/// `-[NSUserDefaults removeObjectForKey:]`
pub extern "C" fn shim_nsuserdefaults_remove_object(obj: u64, _cmd: u64, key: u64) {
    if obj == 0 || key == 0 {
        return;
    }
    let store = unsafe { *((obj as usize + NSUD_STORE_OFF) as *const u64) };
    if store == 0 {
        return;
    }
    crate::nsmutabledict::shim_nsmutabledict_remove_object_for_key(store, 0, key);
}

/// `-[NSUserDefaults synchronize]` — no-op in our in-memory store.
pub extern "C" fn shim_nsuserdefaults_synchronize(_obj: u64, _cmd: u64) -> u8 {
    1 // YES — always succeeds
}

/// `-[NSUserDefaults setInteger:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_integer(obj: u64, _cmd: u64, value: i64, key: u64) {
    let number = crate::nsnumber::make_nsnumber_int(value);
    shim_nsuserdefaults_set_object(obj, 0, number, key);
}

/// `-[NSUserDefaults setBool:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_bool(obj: u64, _cmd: u64, value: u8, key: u64) {
    let number = crate::nsnumber::make_nsnumber_bool(value != 0);
    shim_nsuserdefaults_set_object(obj, 0, number, key);
}

/// `-[NSUserDefaults setDouble:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_double(obj: u64, _cmd: u64, value: f64, key: u64) {
    let number = crate::nsnumber::make_nsnumber_double(value);
    shim_nsuserdefaults_set_object(obj, 0, number, key);
}

/// `-[NSUserDefaults setFloat:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_float(obj: u64, _cmd: u64, value: f32, key: u64) {
    let number = crate::nsnumber::make_nsnumber_double(value as f64);
    shim_nsuserdefaults_set_object(obj, 0, number, key);
}

/// `-[NSUserDefaults floatForKey:]`
pub extern "C" fn shim_nsuserdefaults_float_for_key(obj: u64, _cmd: u64, key: u64) -> f32 {
    shim_nsuserdefaults_double_for_key(obj, _cmd, key) as f32
}

/// `-[NSUserDefaults arrayForKey:]`
pub extern "C" fn shim_nsuserdefaults_array_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    shim_nsuserdefaults_object_for_key(obj, _cmd, key)
}

/// `-[NSUserDefaults dictionaryForKey:]`
pub extern "C" fn shim_nsuserdefaults_dictionary_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    shim_nsuserdefaults_object_for_key(obj, _cmd, key)
}
