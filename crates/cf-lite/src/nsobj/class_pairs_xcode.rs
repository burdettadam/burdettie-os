//! Xcode private framework class/metaclass pair declarations.
//!
//! Stub classes for symbols referenced by heavy Xcode tools (momc,
//! xcindex-test, etc.) that come from private frameworks like
//! DVTFoundation, IDEFoundation, AuthKit, CoreSimulator, etc.

use std::cell::UnsafeCell;

use super::{nsobject_class_addr, nsobject_metaclass_addr, SyncArray};

// 16 stub classes — all children of NSObject

define_class_pair!(child ACSURLSESSION_CLASS_DATA, ACSURLSESSION_METACLASS_DATA, ACSURLSESSION_INIT,
    ensure: ensure_acsurlsession_init,
    class_addr: acsurlsession_class_addr,
    meta_addr: acsurlsession_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child AKAPPLEIDAUTHINAPP_CLASS_DATA, AKAPPLEIDAUTHINAPP_METACLASS_DATA, AKAPPLEIDAUTHINAPP_INIT,
    ensure: ensure_akappleidauthinapp_init,
    class_addr: akappleidauthinapp_class_addr,
    meta_addr: akappleidauthinapp_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child CPPROVISIONINGMGR_CLASS_DATA, CPPROVISIONINGMGR_METACLASS_DATA, CPPROVISIONINGMGR_INIT,
    ensure: ensure_cpprovisioningmgr_init,
    class_addr: cpprovisioningmgr_class_addr,
    meta_addr: cpprovisioningmgr_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child DVTDARWINUPSESSION_CLASS_DATA, DVTDARWINUPSESSION_METACLASS_DATA, DVTDARWINUPSESSION_INIT,
    ensure: ensure_dvtdarwinupsession_init,
    class_addr: dvtdarwinupsession_class_addr,
    meta_addr: dvtdarwinupsession_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child DVTFILEPATH_CLASS_DATA, DVTFILEPATH_METACLASS_DATA, DVTFILEPATH_INIT,
    ensure: ensure_dvtfilepath_init,
    class_addr: dvtfilepath_class_addr,
    meta_addr: dvtfilepath_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child DVTFOUNDATION_CLASS_DATA, DVTFOUNDATION_METACLASS_DATA, DVTFOUNDATION_INIT,
    ensure: ensure_dvtfoundation_init,
    class_addr: dvtfoundation_class_addr,
    meta_addr: dvtfoundation_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child DVTLOGASPECT_CLASS_DATA, DVTLOGASPECT_METACLASS_DATA, DVTLOGASPECT_INIT,
    ensure: ensure_dvtlogaspect_init,
    class_addr: dvtlogaspect_class_addr,
    meta_addr: dvtlogaspect_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child DVTMACROEXPN_CLASS_DATA, DVTMACROEXPN_METACLASS_DATA, DVTMACROEXPN_INIT,
    ensure: ensure_dvtmacroexpn_init,
    class_addr: dvtmacroexpn_class_addr,
    meta_addr: dvtmacroexpn_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child DVTPLATFORM_CLASS_DATA, DVTPLATFORM_METACLASS_DATA, DVTPLATFORM_INIT,
    ensure: ensure_dvtplatform_init,
    class_addr: dvtplatform_class_addr,
    meta_addr: dvtplatform_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child IDEINDEX_CLASS_DATA, IDEINDEX_METACLASS_DATA, IDEINDEX_INIT,
    ensure: ensure_ideindex_init,
    class_addr: ideindex_class_addr,
    meta_addr: ideindex_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child IDEWORKSPACE_CLASS_DATA, IDEWORKSPACE_METACLASS_DATA, IDEWORKSPACE_INIT,
    ensure: ensure_ideworkspace_init,
    class_addr: ideworkspace_class_addr,
    meta_addr: ideworkspace_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child PBXPROJECT_CLASS_DATA, PBXPROJECT_METACLASS_DATA, PBXPROJECT_INIT,
    ensure: ensure_pbxproject_init,
    class_addr: pbxproject_class_addr,
    meta_addr: pbxproject_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child XCSIGNINGINPUT_CLASS_DATA, XCSIGNINGINPUT_METACLASS_DATA, XCSIGNINGINPUT_INIT,
    ensure: ensure_xcsigninginput_init,
    class_addr: xcsigninginput_class_addr,
    meta_addr: xcsigninginput_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child XCDEVICE_CLASS_DATA, XCDEVICE_METACLASS_DATA, XCDEVICE_INIT,
    ensure: ensure_xcdevice_init,
    class_addr: xcdevice_class_addr,
    meta_addr: xcdevice_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child SIMRUNTIME_CLASS_DATA, SIMRUNTIME_METACLASS_DATA, SIMRUNTIME_INIT,
    ensure: ensure_simruntime_init,
    class_addr: simruntime_class_addr,
    meta_addr: simruntime_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child SIMDEVICETYPE_CLASS_DATA, SIMDEVICETYPE_METACLASS_DATA, SIMDEVICETYPE_INIT,
    ensure: ensure_simdevicetype_init,
    class_addr: simdevicetype_class_addr,
    meta_addr: simdevicetype_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

// Additional DVT/IDE classes needed for Xcode tool init paths

define_class_pair!(child DVTDEVELOPERPATHS_CLASS_DATA, DVTDEVELOPERPATHS_METACLASS_DATA, DVTDEVELOPERPATHS_INIT,
    ensure: ensure_dvtdeveloperpaths_init,
    class_addr: dvtdeveloperpaths_class_addr,
    meta_addr: dvtdeveloperpaths_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child DVTPLUGINMGR_CLASS_DATA, DVTPLUGINMGR_METACLASS_DATA, DVTPLUGINMGR_INIT,
    ensure: ensure_dvtpluginmgr_init,
    class_addr: dvtpluginmgr_class_addr,
    meta_addr: dvtpluginmgr_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);

define_class_pair!(child IDEINITIALIZER_CLASS_DATA, IDEINITIALIZER_METACLASS_DATA, IDEINITIALIZER_INIT,
    ensure: ensure_ideinitializer_init,
    class_addr: ideinitializer_class_addr,
    meta_addr: ideinitializer_metaclass_addr,
    parent_class: nsobject_class_addr,
    parent_meta: nsobject_metaclass_addr);
