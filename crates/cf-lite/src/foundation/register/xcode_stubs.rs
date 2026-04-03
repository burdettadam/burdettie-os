//! Xcode private-framework stub class registrations.
//!
//! Registers minimal synthetic ObjC classes for private-framework
//! classes referenced by Xcode tools. Each class inherits from
//! NSObject and has no custom methods — unknown selectors fall through
//! to the unrecognized selector handler which returns 0.

use std::collections::HashMap;

use objc_runtime::ClassInfo;

use crate::nsobj;

/// Stub class descriptor: maps a class name to its addr functions.
struct StubClassDesc {
    name: &'static str,
    class_addr: fn() -> u64,
    meta_addr: fn() -> u64,
}

/// All Xcode private-framework stub classes.
const XCODE_STUB_CLASSES: &[StubClassDesc] = &[
    StubClassDesc { name: "ACSURLSession", class_addr: nsobj::acsurlsession_class_addr, meta_addr: nsobj::acsurlsession_metaclass_addr },
    StubClassDesc { name: "AKAppleIDAuthenticationInAppContext", class_addr: nsobj::akappleidauthinapp_class_addr, meta_addr: nsobj::akappleidauthinapp_metaclass_addr },
    StubClassDesc { name: "CPProvisioningManager", class_addr: nsobj::cpprovisioningmgr_class_addr, meta_addr: nsobj::cpprovisioningmgr_metaclass_addr },
    StubClassDesc { name: "DVTDarwinupSession", class_addr: nsobj::dvtdarwinupsession_class_addr, meta_addr: nsobj::dvtdarwinupsession_metaclass_addr },
    StubClassDesc { name: "DVTFilePath", class_addr: nsobj::dvtfilepath_class_addr, meta_addr: nsobj::dvtfilepath_metaclass_addr },
    StubClassDesc { name: "DVTFoundation", class_addr: nsobj::dvtfoundation_class_addr, meta_addr: nsobj::dvtfoundation_metaclass_addr },
    StubClassDesc { name: "DVTLogAspect", class_addr: nsobj::dvtlogaspect_class_addr, meta_addr: nsobj::dvtlogaspect_metaclass_addr },
    StubClassDesc { name: "DVTMacroExpansionScope", class_addr: nsobj::dvtmacroexpn_class_addr, meta_addr: nsobj::dvtmacroexpn_metaclass_addr },
    StubClassDesc { name: "DVTPlatform", class_addr: nsobj::dvtplatform_class_addr, meta_addr: nsobj::dvtplatform_metaclass_addr },
    StubClassDesc { name: "IDEIndex", class_addr: nsobj::ideindex_class_addr, meta_addr: nsobj::ideindex_metaclass_addr },
    StubClassDesc { name: "IDEWorkspace", class_addr: nsobj::ideworkspace_class_addr, meta_addr: nsobj::ideworkspace_metaclass_addr },
    StubClassDesc { name: "PBXProject", class_addr: nsobj::pbxproject_class_addr, meta_addr: nsobj::pbxproject_metaclass_addr },
    StubClassDesc { name: "XCSigningInput", class_addr: nsobj::xcsigninginput_class_addr, meta_addr: nsobj::xcsigninginput_metaclass_addr },
    StubClassDesc { name: "XCDevice", class_addr: nsobj::xcdevice_class_addr, meta_addr: nsobj::xcdevice_metaclass_addr },
    StubClassDesc { name: "SimRuntime", class_addr: nsobj::simruntime_class_addr, meta_addr: nsobj::simruntime_metaclass_addr },
    StubClassDesc { name: "SimDeviceType", class_addr: nsobj::simdevicetype_class_addr, meta_addr: nsobj::simdevicetype_metaclass_addr },
];

/// Register all Xcode stub classes in the ObjC runtime.
///
/// Each class is registered as a minimal NSObject subclass with no
/// custom methods. This allows `objc_getClass("DVTFilePath")` to
/// succeed and enables `[[DVTFilePath alloc] init]` to return a
/// valid (but empty) object.
pub fn register_xcode_stub_classes(rt: &mut objc_runtime::ObjcRuntime) {
    for desc in XCODE_STUB_CLASSES {
        let class_addr = (desc.class_addr)();
        let metaclass_addr = (desc.meta_addr)();

        let info = ClassInfo {
            name: desc.name.to_string(),
            mapped_addr: class_addr,
            methods: vec![],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: Vec::new(),
            instance_size: 16,
            is_root: false,
            is_meta: false,
            superclass_addr: Some(nsobj::nsobject_class_addr()),
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        rt.register_synthetic_class(info);
        rt.register_metaclass_addr(metaclass_addr, desc.name);
    }
    log::debug!("xcode_stubs: registered {} stub classes", XCODE_STUB_CLASSES.len());
}

/// Install Xcode stub class/metaclass data symbols in the ShimTable.
pub fn install_xcode_stub_class_symbols(table: &mut libsystem_shims::ShimTable) {
    use libsystem_shims::register_shims;

    register_shims!(table,
        ("_OBJC_CLASS_$_ACSURLSession", nsobj::acsurlsession_class_addr(), "ACSURLSession class (Xcode stub)"),
        ("_OBJC_METACLASS_$_ACSURLSession", nsobj::acsurlsession_metaclass_addr(), "ACSURLSession metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_AKAppleIDAuthenticationInAppContext", nsobj::akappleidauthinapp_class_addr(), "AKAppleIDAuthenticationInAppContext class (Xcode stub)"),
        ("_OBJC_METACLASS_$_AKAppleIDAuthenticationInAppContext", nsobj::akappleidauthinapp_metaclass_addr(), "AKAppleIDAuthenticationInAppContext metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_CPProvisioningManager", nsobj::cpprovisioningmgr_class_addr(), "CPProvisioningManager class (Xcode stub)"),
        ("_OBJC_METACLASS_$_CPProvisioningManager", nsobj::cpprovisioningmgr_metaclass_addr(), "CPProvisioningManager metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_DVTDarwinupSession", nsobj::dvtdarwinupsession_class_addr(), "DVTDarwinupSession class (Xcode stub)"),
        ("_OBJC_METACLASS_$_DVTDarwinupSession", nsobj::dvtdarwinupsession_metaclass_addr(), "DVTDarwinupSession metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_DVTFilePath", nsobj::dvtfilepath_class_addr(), "DVTFilePath class (Xcode stub)"),
        ("_OBJC_METACLASS_$_DVTFilePath", nsobj::dvtfilepath_metaclass_addr(), "DVTFilePath metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_DVTFoundation", nsobj::dvtfoundation_class_addr(), "DVTFoundation class (Xcode stub)"),
        ("_OBJC_METACLASS_$_DVTFoundation", nsobj::dvtfoundation_metaclass_addr(), "DVTFoundation metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_DVTLogAspect", nsobj::dvtlogaspect_class_addr(), "DVTLogAspect class (Xcode stub)"),
        ("_OBJC_METACLASS_$_DVTLogAspect", nsobj::dvtlogaspect_metaclass_addr(), "DVTLogAspect metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_DVTMacroExpansionScope", nsobj::dvtmacroexpn_class_addr(), "DVTMacroExpansionScope class (Xcode stub)"),
        ("_OBJC_METACLASS_$_DVTMacroExpansionScope", nsobj::dvtmacroexpn_metaclass_addr(), "DVTMacroExpansionScope metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_DVTPlatform", nsobj::dvtplatform_class_addr(), "DVTPlatform class (Xcode stub)"),
        ("_OBJC_METACLASS_$_DVTPlatform", nsobj::dvtplatform_metaclass_addr(), "DVTPlatform metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_IDEIndex", nsobj::ideindex_class_addr(), "IDEIndex class (Xcode stub)"),
        ("_OBJC_METACLASS_$_IDEIndex", nsobj::ideindex_metaclass_addr(), "IDEIndex metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_IDEWorkspace", nsobj::ideworkspace_class_addr(), "IDEWorkspace class (Xcode stub)"),
        ("_OBJC_METACLASS_$_IDEWorkspace", nsobj::ideworkspace_metaclass_addr(), "IDEWorkspace metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_PBXProject", nsobj::pbxproject_class_addr(), "PBXProject class (Xcode stub)"),
        ("_OBJC_METACLASS_$_PBXProject", nsobj::pbxproject_metaclass_addr(), "PBXProject metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_XCSigningInput", nsobj::xcsigninginput_class_addr(), "XCSigningInput class (Xcode stub)"),
        ("_OBJC_METACLASS_$_XCSigningInput", nsobj::xcsigninginput_metaclass_addr(), "XCSigningInput metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_XCDevice", nsobj::xcdevice_class_addr(), "XCDevice class (Xcode stub)"),
        ("_OBJC_METACLASS_$_XCDevice", nsobj::xcdevice_metaclass_addr(), "XCDevice metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_SimRuntime", nsobj::simruntime_class_addr(), "SimRuntime class (Xcode stub)"),
        ("_OBJC_METACLASS_$_SimRuntime", nsobj::simruntime_metaclass_addr(), "SimRuntime metaclass (Xcode stub)"),
        ("_OBJC_CLASS_$_SimDeviceType", nsobj::simdevicetype_class_addr(), "SimDeviceType class (Xcode stub)"),
        ("_OBJC_METACLASS_$_SimDeviceType", nsobj::simdevicetype_metaclass_addr(), "SimDeviceType metaclass (Xcode stub)")
    );
    log::debug!("xcode_stubs: registered 32 class/metaclass data symbols");
}
