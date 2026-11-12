pub(crate) mod cfconstant_string;
mod register;
mod shim_classes;
mod shim_core;

pub(crate) use cfconstant_string::cfconstant_string_class_addr;

use libsystem_shims::table::ShimTable;
#[cfg(target_os = "macos")]
use std::sync::OnceLock;

const HOST_ISA_ALIAS_SYMBOLS: [(&str, &str); 4] = [
    ("OBJC_CLASS_$_NSConstantArray", "NSArray"),
    ("OBJC_CLASS_$_NSConstantDictionary", "NSConstantDictionary"),
    (
        "OBJC_CLASS_$_NSConstantIntegerNumber",
        "NSConstantIntegerNumber",
    ),
    ("OBJC_CLASS_$_NSException", "NSException"),
];

#[cfg(target_os = "macos")]
static HOST_ISA_ALIAS_CACHE: OnceLock<Vec<(u64, &'static str)>> = OnceLock::new();

fn nsstring_from_str(value: &str) -> u64 {
    let cstr = std::ffi::CString::new(value).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

fn xcode_app_info_plist_path() -> Option<std::path::PathBuf> {
    let process_path = libsystem_shims::process::process_path()?;
    let bundle_root = crate::nsbundle::detect_app_bundle_root(process_path)?;

    let contents_plist = std::path::Path::new(&bundle_root).join("Contents/Info.plist");
    if contents_plist.exists() {
        return Some(contents_plist);
    }

    let flat_plist = std::path::Path::new(&bundle_root).join("Info.plist");
    flat_plist.exists().then_some(flat_plist)
}

fn xcode_app_version_plist_path() -> Option<std::path::PathBuf> {
    let info_path = xcode_app_info_plist_path()?;
    if let Some(bundle_root) = info_path.parent().and_then(|contents| contents.parent()) {
        let version_plist = bundle_root.join("Contents/version.plist");
        if version_plist.exists() {
            return Some(version_plist);
        }
    }

    Some(info_path)
}

fn xcode_info_plist_value(key: &str) -> u64 {
    let Some(info_path) = xcode_app_info_plist_path() else {
        return 0;
    };

    let mut candidate_paths = Vec::with_capacity(2);
    if let Some(version_path) = xcode_app_version_plist_path() {
        candidate_paths.push(version_path);
    }
    candidate_paths.push(info_path);

    let key_obj = nsstring_from_str(key);
    if key_obj == 0 {
        return 0;
    }

    for path in candidate_paths {
        let Some(path_str) = path.to_str() else {
            continue;
        };
        let dict = crate::nspropertylistserialization::parse_plist_file(path_str);
        if dict == 0 {
            continue;
        }
        let value = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key_obj);
        if value != 0 {
            return value;
        }
    }

    0
}

pub extern "C" fn shim_dvt_system_prerequisites_paths_license_host_app_info_plist_path(
    _cls: u64,
    _cmd: u64,
) -> u64 {
    let Some(path) = xcode_app_info_plist_path() else {
        return 0;
    };

    let Some(path_str) = path.to_str() else {
        return 0;
    };

    nsstring_from_str(path_str)
}

pub extern "C" fn shim_dvt_system_prerequisites_paths_host_app_version_plist_path(
    _cls: u64,
    _cmd: u64,
) -> u64 {
    let Some(path) = xcode_app_version_plist_path() else {
        return 0;
    };

    let Some(path_str) = path.to_str() else {
        return 0;
    };

    nsstring_from_str(path_str)
}

pub extern "C" fn shim_dvt_system_prerequisites_paths_host_app_product_build_version(
    _cls: u64,
    _cmd: u64,
) -> u64 {
    xcode_info_plist_value("ProductBuildVersion")
}

fn install_guest_class_method(
    rt: &mut objc_runtime::ObjcRuntime,
    class_name: &str,
    selector: &'static str,
    imp: u64,
    type_encoding: &'static str,
) {
    let Some(class_addr) = rt.get_class(class_name).map(|class| class.mapped_addr) else {
        log::debug!(
            "FoundationShims: guest compatibility target class '{}' not present",
            class_name
        );
        return;
    };
    let Some(meta_addr) = rt.dispatch_isa_for_receiver(class_addr) else {
        log::debug!(
            "FoundationShims: guest compatibility target class '{}' has no dispatch metaclass",
            class_name
        );
        return;
    };

    if rt.add_method_to_class(
        meta_addr,
        objc_runtime::MethodEntry {
            selector: selector.into(),
            imp,
            type_encoding: type_encoding.into(),
        },
    ) {
        log::debug!(
            "FoundationShims: installed guest compatibility class method +[{} {}]",
            class_name,
            selector
        );
    }
}

fn resolve_host_isa_aliases(
    symbols: &dyn platform_core::symbols::HostSymbolResolver,
) -> Vec<(u64, &'static str)> {
    HOST_ISA_ALIAS_SYMBOLS
        .iter()
        .filter_map(|(sym_name, target_class)| {
            symbols
                .resolve(sym_name)
                .map(|addr| (addr as u64, *target_class))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn host_isa_aliases(
    symbols: &dyn platform_core::symbols::HostSymbolResolver,
) -> Vec<(u64, &'static str)> {
    if let Some(aliases) = HOST_ISA_ALIAS_CACHE.get() {
        return aliases.clone();
    }

    let aliases = resolve_host_isa_aliases(symbols);
    if !aliases.is_empty() {
        let _ = HOST_ISA_ALIAS_CACHE.set(aliases.clone());
    }
    aliases
}

/// Install Foundation/CF-lite shims into a `ShimTable`.
pub struct FoundationShims;

impl FoundationShims {
    pub fn install(table: &mut ShimTable) {
        shim_core::install_core_shims(table);
        shim_classes::install_class_symbols(table);
        log::debug!("FoundationShims: registered Foundation/CF shims");
    }

    /// Register tagged pointer class table entries.
    ///
    /// On arm64, tagged pointers encode a 3-bit class index in bits 60-62.
    /// Apple's canonical mapping: 2 = NSNumber, 3 = NSString, 6 = NSDate.
    pub fn register_tagged_classes(rt: &mut objc_runtime::ObjcRuntime) {
        use crate::nsobj;
        rt.register_tagged_class(2, nsobj::nsnumber_class_addr());
        rt.register_tagged_class(3, nsobj::nsstring_class_addr());
        rt.register_tagged_class(6, nsobj::nsdate_class_addr());
        log::debug!(
            "FoundationShims: registered tagged pointer classes (NSNumber=2, NSString=3, NSDate=6)"
        );
    }

    /// Register host Foundation ISA addresses as aliases for our shim classes.
    ///
    /// The compiler embeds constant objects (`@[]`, `@{}`, `@(int)`) with ISA
    /// pointers to the host Foundation classes (NSConstantArray, etc.). This
    /// maps those host addresses to our synthetic classes so dispatch works
    /// without falling back to the host `objc_msgSend`.
    #[cfg(target_os = "macos")]
    pub fn register_host_isa_aliases(rt: &mut objc_runtime::ObjcRuntime) {
        let symbols = crate::pal_symbols();

        for (addr, target_class) in host_isa_aliases(symbols) {
            rt.register_isa_alias(addr, target_class);
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn register_host_isa_aliases(_rt: &mut objc_runtime::ObjcRuntime) {}

    /// Install narrow compatibility methods on guest-defined classes.
    ///
    /// If DVT classes weren't loaded from a real DVTFoundation framework,
    /// they are created from our class pair statics so the methods have
    /// somewhere to live.
    pub fn register_guest_compat_methods(rt: &mut objc_runtime::ObjcRuntime) {
        // Ensure DVTSystemPrerequisitesPaths exists in the ObjC runtime
        // using the real class pair address (not a synthetic/fake address).
        if rt.get_class("DVTSystemPrerequisitesPaths").is_none() {
            let class_addr = crate::nsobj::dvtsystemprerequisitespaths_class_addr();
            let meta_addr = crate::nsobj::dvtsystemprerequisitespaths_metaclass_addr();
            rt.register_synthetic_class(objc_runtime::ClassInfo {
                name: "DVTSystemPrerequisitesPaths".to_string(),
                mapped_addr: class_addr,
                methods: vec![],
                class_methods: vec![],
                ivars: vec![],
                properties: vec![],
                protocol_names: vec![],
                instance_size: 8,
                is_root: false,
                is_meta: false,
                superclass_addr: None,
                method_index: std::collections::HashMap::new(),
                class_method_index: std::collections::HashMap::new(),
            });
            rt.register_metaclass_addr(meta_addr, "DVTSystemPrerequisitesPaths");
            log::debug!(
                "FoundationShims: registered DVTSystemPrerequisitesPaths at {:#x} / meta {:#x}",
                class_addr,
                meta_addr,
            );
        }

        install_guest_class_method(
            rt,
            "DVTSystemPrerequisitesPaths",
            "licenseHostAppInfoPlistPath",
            shim_dvt_system_prerequisites_paths_license_host_app_info_plist_path as *const ()
                as u64,
            "@@:",
        );
        install_guest_class_method(
            rt,
            "DVTSystemPrerequisitesPaths",
            "hostAppVersionPlistPath",
            shim_dvt_system_prerequisites_paths_host_app_version_plist_path as *const () as u64,
            "@@:",
        );
        install_guest_class_method(
            rt,
            "DVTSystemPrerequisitesPaths",
            "hostAppProductBuildVersion",
            shim_dvt_system_prerequisites_paths_host_app_product_build_version as *const () as u64,
            "@@:",
        );
    }
}

mod impl_appkit;
mod impl_foundation;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct TestSymbolResolver {
        symbols: HashMap<&'static str, u64>,
    }

    impl platform_core::symbols::HostSymbolResolver for TestSymbolResolver {
        fn resolve(&self, name: &str) -> Option<*const ()> {
            self.symbols
                .get(name)
                .copied()
                .map(|addr| addr as *const ())
        }

        fn resolve_in(&self, _handle: *mut (), name: &str) -> Option<*const ()> {
            self.resolve(name)
        }

        fn open_library(&self, _path: &str) -> Option<*mut ()> {
            None
        }

        fn default_handle(&self) -> *mut () {
            std::ptr::null_mut()
        }
    }

    #[test]
    fn foundation_shims_registered() {
        let mut table = ShimTable::new();
        FoundationShims::install(&mut table);
        assert!(table.resolve("_NSLog").is_some());
        assert!(table.resolve("___CFConstantStringClassReference").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSObject").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSObject").is_some());
        assert!(table.resolve("_CFStringCreateWithCString").is_some());
        assert!(table.resolve("_CFStringGetLength").is_some());
        assert!(table.resolve("_CFStringGetCStringPtr").is_some());
        assert!(table.resolve("_kCFStringEncodingUTF8").is_some());
        assert!(table.resolve("_CFRelease").is_some());
        assert!(table.resolve("_kCFAllocatorDefault").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSString").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSString").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSArray").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSArray").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSNumber").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSNumber").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSDictionary").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSDictionary").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableArray").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSMutableArray").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableDictionary").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSMutableDictionary")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableString").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSMutableString").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSData").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSData").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSProcessInfo").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSProcessInfo").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSError").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSError").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSBundle").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSBundle").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSFileManager").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSFileManager").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSException").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSException").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSEnumerator").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSEnumerator").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSDate").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSDate").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSUserDefaults").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSUserDefaults").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSSet").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSSet").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableSet").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSMutableSet").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSURL").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSURL").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSNull").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSNull").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSDateFormatter").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSDateFormatter").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSCharacterSet").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSCharacterSet").is_some());
        assert!(table
            .resolve("_OBJC_CLASS_$_NSNotificationCenter")
            .is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSNotificationCenter")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSValue").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSValue").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSUUID").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSUUID").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSLocale").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSLocale").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSTimeZone").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSTimeZone").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSCalendar").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSCalendar").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSNumberFormatter").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSNumberFormatter")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSIndexSet").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSIndexSet").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMutableIndexSet").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSMutableIndexSet")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSSortDescriptor").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSSortDescriptor")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSTimer").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSTimer").is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSMethodSignature").is_some());
        assert!(table
            .resolve("_OBJC_METACLASS_$_NSMethodSignature")
            .is_some());
        assert!(table.resolve("_OBJC_CLASS_$_NSInvocation").is_some());
        assert!(table.resolve("_OBJC_METACLASS_$_NSInvocation").is_some());
        // Symbol count grows as shims are added; assert it doesn't regress below a known minimum.
        #[cfg(target_os = "macos")]
        assert!(
            table.len() >= 172,
            "expected ≥172 symbols, got {}",
            table.len()
        );
        #[cfg(not(target_os = "macos"))]
        assert!(
            table.len() >= 162,
            "expected ≥162 symbols, got {}",
            table.len()
        );
    }

    #[test]
    fn cfconstant_string_class_nonzero() {
        let addr = cfconstant_string_class_addr();
        assert_ne!(addr, 0);
    }

    #[test]
    fn cfconstant_string_isa_points_to_self() {
        let addr = cfconstant_string_class_addr();
        // The fake class ISA points to itself. When the host class is used
        // (via upgrade_cfstring_isa_to_host), this test still checks the
        // fake class because cfconstant_string_class_addr returns fake first.
        let isa = unsafe { std::ptr::read_unaligned(addr as *const u64) };
        assert_eq!(isa, addr);
    }

    #[test]
    fn guest_compat_methods_attach_to_dvt_paths_metaclass() {
        let class_addr = 0xD470_1000;
        let meta_addr = 0xD470_2000;
        let mut rt = objc_runtime::ObjcRuntime::new();

        rt.register_synthetic_class(objc_runtime::ClassInfo {
            name: "DVTSystemPrerequisitesPaths".to_string(),
            mapped_addr: class_addr,
            methods: vec![],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 8,
            is_root: false,
            is_meta: false,
            superclass_addr: Some(crate::nsobj::nsobject_class_addr()),
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        });
        rt.register_metaclass_addr(meta_addr, "DVTSystemPrerequisitesPaths");

        FoundationShims::register_guest_compat_methods(&mut rt);

        let class = rt
            .get_class("DVTSystemPrerequisitesPaths")
            .expect("compat class should stay registered");
        assert!(class
            .find_class_method("licenseHostAppInfoPlistPath")
            .is_some());
        assert!(class.find_class_method("hostAppVersionPlistPath").is_some());
        assert!(class
            .find_class_method("hostAppProductBuildVersion")
            .is_some());
    }

    #[test]
    fn resolve_host_isa_aliases_skips_missing_symbols() {
        let mut resolver = TestSymbolResolver::default();
        resolver
            .symbols
            .insert("OBJC_CLASS_$_NSConstantArray", 0x1000);
        resolver.symbols.insert("OBJC_CLASS_$_NSException", 0x2000);

        let aliases = resolve_host_isa_aliases(&resolver);

        assert_eq!(aliases, vec![(0x1000, "NSArray"), (0x2000, "NSException")]);
    }
}
