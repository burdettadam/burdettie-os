//! Class registrations for CoreData, AVFoundation, CoreLocation, CoreBluetooth stub classes.

use super::me;
use crate::{avfoundation_stubs as av, coredata_stubs as cd, corelocation_stubs as cl, nsobj};
use objc_runtime::ClassInfo;
use std::collections::HashMap;

// ── CoreData ──────────────────────────────────────────────────────────────────

pub fn register_nsmanagedobjectcontext_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmanagedobjectcontext_class_addr();
    let metaclass_addr = nsobj::nsmanagedobjectcontext_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSManagedObjectContext".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cd::shim_nsmoc_init as *const () as u64, "@@:"),
            me(
                "initWithConcurrencyType:",
                cd::shim_nsmoc_init_with_concurrency_type as *const () as u64,
                "@@:Q",
            ),
            me(
                "performBlock:",
                cd::shim_nsmoc_perform_block as *const () as u64,
                "v@:@",
            ),
            me(
                "performBlockAndWait:",
                cd::shim_nsmoc_perform_block as *const () as u64,
                "v@:@",
            ),
            me("save:", cd::shim_nsmoc_save as *const () as u64, "B@:^@"),
            me(
                "executeFetchRequest:error:",
                cd::shim_nsmoc_execute_fetch_request as *const () as u64,
                "@@:@^@",
            ),
            me(
                "persistentStoreCoordinator",
                cd::shim_nsmoc_persistent_store_coordinator as *const () as u64,
                "@@:",
            ),
            me(
                "setPersistentStoreCoordinator:",
                cd::shim_nsmoc_set_persistent_store_coordinator as *const () as u64,
                "v@:@",
            ),
            me(
                "mergeChangesFromContextDidSaveNotification:",
                cd::shim_nsmoc_merge_changes_from_context_did_save as *const () as u64,
                "v@:@",
            ),
            me(
                "undoManager",
                cd::shim_nsmoc_undo_manager as *const () as u64,
                "@@:",
            ),
            me(
                "hasChanges",
                cd::shim_nsmoc_has_changes as *const () as u64,
                "B@:",
            ),
            me("reset", cd::shim_nsmoc_reset as *const () as u64, "v@:"),
            me(
                "rollback",
                cd::shim_nsmoc_rollback as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![
            me("new", cd::shim_nsmoc_new as *const () as u64, "@@:"),
            me("alloc", cd::shim_nsmoc_alloc as *const () as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cd::NSMANAGEDOBJECTCONTEXT_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSManagedObjectContext");
}

pub fn register_nspersistentstorecoordinator_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nspersistentstorecoordinator_class_addr();
    let metaclass_addr = nsobj::nspersistentstorecoordinator_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSPersistentStoreCoordinator".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithManagedObjectModel:",
                cd::shim_nspsc_init_with_model as *const () as u64,
                "@@:@",
            ),
            me(
                "addPersistentStoreWithType:configuration:URL:options:error:",
                cd::shim_nspsc_add_store as *const () as u64,
                "@@:@@@@^@",
            ),
            me(
                "managedObjectModel",
                cd::shim_nspsc_managed_object_model as *const () as u64,
                "@@:",
            ),
            me(
                "persistentStores",
                cd::shim_nspsc_persistent_stores as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me("new", cd::shim_nspsc_new as *const () as u64, "@@:"),
            me("alloc", cd::shim_nspsc_alloc as *const () as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cd::NSPERSISTENTSTORECOORDINATOR_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSPersistentStoreCoordinator");
}

pub fn register_nsfetchrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsfetchrequest_class_addr();
    let metaclass_addr = nsobj::nsfetchrequest_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSFetchRequest".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithEntityName:",
                cd::shim_nsfr_init_with_entity_name as *const () as u64,
                "@@:@",
            ),
            me(
                "entityName",
                cd::shim_nsfr_entity_name as *const () as u64,
                "@@:",
            ),
            me(
                "predicate",
                cd::shim_nsfr_predicate as *const () as u64,
                "@@:",
            ),
            me(
                "setPredicate:",
                cd::shim_nsfr_set_predicate as *const () as u64,
                "v@:@",
            ),
            me(
                "sortDescriptors",
                cd::shim_nsfr_sort_descriptors as *const () as u64,
                "@@:",
            ),
            me(
                "setSortDescriptors:",
                cd::shim_nsfr_set_sort_descriptors as *const () as u64,
                "v@:@",
            ),
            me(
                "fetchLimit",
                cd::shim_nsfr_fetch_limit as *const () as u64,
                "Q@:",
            ),
            me(
                "setFetchLimit:",
                cd::shim_nsfr_set_fetch_limit as *const () as u64,
                "v@:Q",
            ),
        ],
        class_methods: vec![
            me(
                "fetchRequestWithEntityName:",
                cd::shim_nsfr_fetch_request_with_entity_name as *const () as u64,
                "@@:@",
            ),
            me("alloc", cd::shim_nsfr_alloc as *const () as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cd::NSFETCHREQUEST_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSFetchRequest");
}

pub fn register_nsmanagedobject_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmanagedobject_class_addr();
    let metaclass_addr = nsobj::nsmanagedobject_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSManagedObject".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithEntity:insertIntoManagedObjectContext:",
                cd::shim_nsmo_init_with_entity_context as *const () as u64,
                "@@:@@",
            ),
            me(
                "valueForKey:",
                cd::shim_nsmo_value_for_key as *const () as u64,
                "@@:@",
            ),
            me(
                "setValue:forKey:",
                cd::shim_nsmo_set_value_for_key as *const () as u64,
                "v@:@@",
            ),
            me("entity", cd::shim_nsmo_entity as *const () as u64, "@@:"),
            me(
                "managedObjectContext",
                cd::shim_nsmo_managed_object_context as *const () as u64,
                "@@:",
            ),
            me(
                "isDeleted",
                cd::shim_nsmo_is_deleted as *const () as u64,
                "B@:",
            ),
            me(
                "isInserted",
                cd::shim_nsmo_is_inserted as *const () as u64,
                "B@:",
            ),
            me(
                "isUpdated",
                cd::shim_nsmo_is_updated as *const () as u64,
                "B@:",
            ),
        ],
        class_methods: vec![me("alloc", cd::shim_nsmo_alloc as *const () as u64, "@@:")],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cd::NSMANAGEDOBJECT_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSManagedObject");
}

pub fn register_nsentitydescription_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsentitydescription_class_addr();
    let metaclass_addr = nsobj::nsentitydescription_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSEntityDescription".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cd::shim_nsed_init as *const () as u64, "@@:"),
            me("name", cd::shim_nsed_name as *const () as u64, "@@:"),
            me(
                "properties",
                cd::shim_nsed_properties as *const () as u64,
                "@@:",
            ),
            me(
                "managedObjectClassName",
                cd::shim_nsed_managed_object_class_name as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me("alloc", cd::shim_nsed_alloc as *const () as u64, "@@:"),
            me(
                "entityForName:inManagedObjectContext:",
                cd::shim_nsed_entity_for_name as *const () as u64,
                "@@:@@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cd::NSENTITYDESCRIPTION_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSEntityDescription");
}

pub fn register_nsmanagedobjectmodel_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmanagedobjectmodel_class_addr();
    let metaclass_addr = nsobj::nsmanagedobjectmodel_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSManagedObjectModel".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cd::shim_nsmom_init as *const () as u64, "@@:"),
            me(
                "initWithContentsOfURL:",
                cd::shim_nsmom_init_with_contents_of_url as *const () as u64,
                "@@:@",
            ),
            me(
                "entities",
                cd::shim_nsmom_entities as *const () as u64,
                "@@:",
            ),
            me(
                "versionIdentifiers",
                cd::shim_nsmom_entity_versions_by_name as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me("alloc", cd::shim_nsmom_alloc as *const () as u64, "@@:"),
            me(
                "mergedModelFromBundles:",
                cd::shim_nsmom_merged_model_from_bundles as *const () as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cd::NSMANAGEDOBJECTMODEL_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSManagedObjectModel");
}

// ── AVFoundation ──────────────────────────────────────────────────────────────

pub fn register_avaudioplayer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::avaudioplayer_class_addr();
    let metaclass_addr = nsobj::avaudioplayer_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVAudioPlayer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithContentsOfURL:error:",
                av::shim_avaudioplayer_init_with_url as *const () as u64,
                "@@:@^@",
            ),
            me(
                "initWithData:error:",
                av::shim_avaudioplayer_init_with_data as *const () as u64,
                "@@:@^@",
            ),
            me(
                "play",
                av::shim_avaudioplayer_play as *const () as u64,
                "B@:",
            ),
            me(
                "pause",
                av::shim_avaudioplayer_pause as *const () as u64,
                "v@:",
            ),
            me(
                "stop",
                av::shim_avaudioplayer_stop as *const () as u64,
                "v@:",
            ),
            me(
                "prepareToPlay",
                av::shim_avaudioplayer_prepare_to_play as *const () as u64,
                "B@:",
            ),
            me(
                "isPlaying",
                av::shim_avaudioplayer_is_playing as *const () as u64,
                "B@:",
            ),
            me(
                "duration",
                av::shim_avaudioplayer_duration as *const () as u64,
                "d@:",
            ),
            me(
                "currentTime",
                av::shim_avaudioplayer_current_time as *const () as u64,
                "d@:",
            ),
            me(
                "setCurrentTime:",
                av::shim_avaudioplayer_set_current_time as *const () as u64,
                "v@:d",
            ),
            me(
                "volume",
                av::shim_avaudioplayer_volume as *const () as u64,
                "f@:",
            ),
            me(
                "setVolume:",
                av::shim_avaudioplayer_set_volume as *const () as u64,
                "v@:f",
            ),
            me(
                "numberOfLoops",
                av::shim_avaudioplayer_number_of_loops as *const () as u64,
                "q@:",
            ),
            me(
                "setNumberOfLoops:",
                av::shim_avaudioplayer_set_number_of_loops as *const () as u64,
                "v@:q",
            ),
            me("url", av::shim_avaudioplayer_url as *const () as u64, "@@:"),
            me(
                "delegate",
                av::shim_avaudioplayer_delegate as *const () as u64,
                "@@:",
            ),
            me(
                "setDelegate:",
                av::shim_avaudioplayer_set_delegate as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            av::shim_avaudioplayer_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: av::AVAUDIOPLAYER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVAudioPlayer");
}

pub fn register_avaudiorecorder_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::avaudiorecorder_class_addr();
    let metaclass_addr = nsobj::avaudiorecorder_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVAudioRecorder".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithURL:settings:error:",
                av::shim_avaudiorecorder_init_with_url as *const () as u64,
                "@@:@@^@",
            ),
            me(
                "record",
                av::shim_avaudiorecorder_record as *const () as u64,
                "B@:",
            ),
            me(
                "stop",
                av::shim_avaudiorecorder_stop as *const () as u64,
                "v@:",
            ),
            me(
                "isRecording",
                av::shim_avaudiorecorder_is_recording as *const () as u64,
                "B@:",
            ),
            me(
                "deleteRecording",
                av::shim_avaudiorecorder_delete_recording as *const () as u64,
                "B@:",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            av::shim_avaudiorecorder_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: av::AVAUDIORECORDER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVAudioRecorder");
}

pub fn register_avaudioengine_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::avaudioengine_class_addr();
    let metaclass_addr = nsobj::avaudioengine_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVAudioEngine".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                av::shim_avaudioengine_init as *const () as u64,
                "@@:",
            ),
            me(
                "startAndReturnError:",
                av::shim_avaudioengine_start as *const () as u64,
                "B@:^@",
            ),
            me(
                "stop",
                av::shim_avaudioengine_stop as *const () as u64,
                "v@:",
            ),
            me(
                "isRunning",
                av::shim_avaudioengine_is_running as *const () as u64,
                "B@:",
            ),
            me(
                "mainMixerNode",
                av::shim_avaudioengine_main_mixer_node as *const () as u64,
                "@@:",
            ),
            me(
                "outputNode",
                av::shim_avaudioengine_output_node as *const () as u64,
                "@@:",
            ),
            me(
                "inputNode",
                av::shim_avaudioengine_input_node as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me("new", av::shim_avaudioengine_new as *const () as u64, "@@:"),
            me(
                "alloc",
                av::shim_avaudioengine_alloc as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: av::AVAUDIOENGINE_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVAudioEngine");
}

pub fn register_avcapturedevice_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::avcapturedevice_class_addr();
    let metaclass_addr = nsobj::avcapturedevice_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVCaptureDevice".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "localizedName",
                av::shim_avcapturedevice_local_name as *const () as u64,
                "@@:",
            ),
            me(
                "uniqueID",
                av::shim_avcapturedevice_unique_id as *const () as u64,
                "@@:",
            ),
            me(
                "hasMediaType:",
                av::shim_avcapturedevice_has_media_type as *const () as u64,
                "B@:@",
            ),
        ],
        class_methods: vec![
            me(
                "alloc",
                av::shim_avcapturedevice_alloc as *const () as u64,
                "@@:",
            ),
            me(
                "defaultDeviceWithMediaType:",
                av::shim_avcapturedevice_default_device_with_media_type as *const () as u64,
                "@@:@",
            ),
            me(
                "defaultDeviceWithDeviceType:mediaType:position:",
                av::shim_avcapturedevice_default_device as *const () as u64,
                "@@:@@@",
            ),
            me(
                "devicesWithMediaType:",
                av::shim_avcapturedevice_devices_with_media_type as *const () as u64,
                "@@:@",
            ),
            me(
                "authorizationStatusForMediaType:",
                av::shim_avcapturedevice_authorization_status_for_media_type as *const () as u64,
                "Q@:@",
            ),
            me(
                "requestAccessForMediaType:completionHandler:",
                av::shim_avcapturedevice_request_access as *const () as u64,
                "v@:@@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: av::AVCAPTUREDEVICE_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVCaptureDevice");
}

pub fn register_avcapturesession_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::avcapturesession_class_addr();
    let metaclass_addr = nsobj::avcapturesession_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVCaptureSession".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                av::shim_avcapturesession_init as *const () as u64,
                "@@:",
            ),
            me(
                "addInput:",
                av::shim_avcapturesession_add_input as *const () as u64,
                "v@:@",
            ),
            me(
                "addOutput:",
                av::shim_avcapturesession_add_output as *const () as u64,
                "v@:@",
            ),
            me(
                "removeInput:",
                av::shim_avcapturesession_remove_input as *const () as u64,
                "v@:@",
            ),
            me(
                "removeOutput:",
                av::shim_avcapturesession_remove_output as *const () as u64,
                "v@:@",
            ),
            me(
                "canAddInput:",
                av::shim_avcapturesession_can_add_input as *const () as u64,
                "B@:@",
            ),
            me(
                "canAddOutput:",
                av::shim_avcapturesession_can_add_output as *const () as u64,
                "B@:@",
            ),
            me(
                "startRunning",
                av::shim_avcapturesession_start_running as *const () as u64,
                "v@:",
            ),
            me(
                "stopRunning",
                av::shim_avcapturesession_stop_running as *const () as u64,
                "v@:",
            ),
            me(
                "isRunning",
                av::shim_avcapturesession_is_running as *const () as u64,
                "B@:",
            ),
            me(
                "sessionPreset",
                av::shim_avcapturesession_session_preset as *const () as u64,
                "@@:",
            ),
            me(
                "setSessionPreset:",
                av::shim_avcapturesession_set_session_preset as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![
            me(
                "new",
                av::shim_avcapturesession_new as *const () as u64,
                "@@:",
            ),
            me(
                "alloc",
                av::shim_avcapturesession_alloc as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: av::AVCAPTURESESSION_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVCaptureSession");
}

pub fn register_avplayer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::avplayer_class_addr();
    let metaclass_addr = nsobj::avplayer_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVPlayer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("play", av::shim_avplayer_play as *const () as u64, "v@:"),
            me("pause", av::shim_avplayer_pause as *const () as u64, "v@:"),
            me("rate", av::shim_avplayer_rate as *const () as u64, "f@:"),
            me(
                "setRate:",
                av::shim_avplayer_set_rate as *const () as u64,
                "v@:f",
            ),
            me(
                "status",
                av::shim_avplayer_status as *const () as u64,
                "Q@:",
            ),
            me(
                "currentItem",
                av::shim_avplayer_current_item as *const () as u64,
                "@@:",
            ),
            me(
                "volume",
                av::shim_avplayer_volume as *const () as u64,
                "f@:",
            ),
            me(
                "setVolume:",
                av::shim_avplayer_set_volume as *const () as u64,
                "v@:f",
            ),
        ],
        class_methods: vec![
            me("alloc", av::shim_avplayer_alloc as *const () as u64, "@@:"),
            me(
                "playerWithURL:",
                av::shim_avplayer_player_with_url as *const () as u64,
                "@@:@",
            ),
            me(
                "playerWithPlayerItem:",
                av::shim_avplayer_player_with_player_item as *const () as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: av::AVPLAYER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVPlayer");
}

// ── CoreLocation ──────────────────────────────────────────────────────────────

pub fn register_cllocationmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::cllocationmanager_class_addr();
    let metaclass_addr = nsobj::cllocationmanager_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CLLocationManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                cl::shim_cllocationmanager_init as *const () as u64,
                "@@:",
            ),
            me(
                "delegate",
                cl::shim_cllocationmanager_delegate as *const () as u64,
                "@@:",
            ),
            me(
                "setDelegate:",
                cl::shim_cllocationmanager_set_delegate as *const () as u64,
                "v@:@",
            ),
            me(
                "desiredAccuracy",
                cl::shim_cllocationmanager_desired_accuracy as *const () as u64,
                "d@:",
            ),
            me(
                "setDesiredAccuracy:",
                cl::shim_cllocationmanager_set_desired_accuracy as *const () as u64,
                "v@:d",
            ),
            me(
                "distanceFilter",
                cl::shim_cllocationmanager_distance_filter as *const () as u64,
                "d@:",
            ),
            me(
                "setDistanceFilter:",
                cl::shim_cllocationmanager_set_distance_filter as *const () as u64,
                "v@:d",
            ),
            me(
                "requestWhenInUseAuthorization",
                cl::shim_cllocationmanager_request_when_in_use_authorization as *const () as u64,
                "v@:",
            ),
            me(
                "requestAlwaysAuthorization",
                cl::shim_cllocationmanager_request_always_authorization as *const () as u64,
                "v@:",
            ),
            me(
                "startUpdatingLocation",
                cl::shim_cllocationmanager_start_updating_location as *const () as u64,
                "v@:",
            ),
            me(
                "stopUpdatingLocation",
                cl::shim_cllocationmanager_stop_updating_location as *const () as u64,
                "v@:",
            ),
            me(
                "requestLocation",
                cl::shim_cllocationmanager_request_location as *const () as u64,
                "v@:",
            ),
            me(
                "startUpdatingHeading",
                cl::shim_cllocationmanager_start_updating_heading as *const () as u64,
                "v@:",
            ),
            me(
                "stopUpdatingHeading",
                cl::shim_cllocationmanager_stop_updating_heading as *const () as u64,
                "v@:",
            ),
            me(
                "startMonitoringSignificantLocationChanges",
                cl::shim_cllocationmanager_start_monitoring_significant_location_changes
                    as *const () as u64,
                "v@:",
            ),
            me(
                "stopMonitoringSignificantLocationChanges",
                cl::shim_cllocationmanager_stop_monitoring_significant_location_changes as *const ()
                    as u64,
                "v@:",
            ),
            me(
                "location",
                cl::shim_cllocationmanager_location as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "new",
                cl::shim_cllocationmanager_new as *const () as u64,
                "@@:",
            ),
            me(
                "alloc",
                cl::shim_cllocationmanager_alloc as *const () as u64,
                "@@:",
            ),
            me(
                "authorizationStatus",
                cl::shim_cllocationmanager_authorization_status as *const () as u64,
                "i@:",
            ),
            me(
                "locationServicesEnabled",
                cl::shim_cllocationmanager_location_services_enabled as *const () as u64,
                "B@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cl::CLLOCATIONMANAGER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CLLocationManager");
}

pub fn register_cllocation_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::cllocation_class_addr();
    let metaclass_addr = nsobj::cllocation_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CLLocation".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithLatitude:longitude:",
                cl::shim_cllocation_init_with_lat_lon as *const () as u64,
                "@@:dd",
            ),
            me(
                "coordinate",
                cl::shim_cllocation_coordinate_latitude as *const () as u64,
                "d@:",
            ), // simplified: returns lat
            me(
                "altitude",
                cl::shim_cllocation_altitude as *const () as u64,
                "d@:",
            ),
            me(
                "horizontalAccuracy",
                cl::shim_cllocation_horizontal_accuracy as *const () as u64,
                "d@:",
            ),
            me(
                "verticalAccuracy",
                cl::shim_cllocation_vertical_accuracy as *const () as u64,
                "d@:",
            ),
            me(
                "speed",
                cl::shim_cllocation_speed as *const () as u64,
                "d@:",
            ),
            me(
                "course",
                cl::shim_cllocation_course as *const () as u64,
                "d@:",
            ),
            me(
                "timestamp",
                cl::shim_cllocation_timestamp as *const () as u64,
                "@@:",
            ),
            me(
                "distanceFromLocation:",
                cl::shim_cllocation_distance_from_location as *const () as u64,
                "d@:@",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            cl::shim_cllocation_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cl::CLLOCATION_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CLLocation");
}

pub fn register_clgeocoder_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::clgeocoder_class_addr();
    let metaclass_addr = nsobj::clgeocoder_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CLGeocoder".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cl::shim_clgeocoder_init as *const () as u64, "@@:"),
            me(
                "geocodeAddressString:completionHandler:",
                cl::shim_clgeocoder_geocode_address_string as *const () as u64,
                "v@:@@",
            ),
            me(
                "reverseGeocodeLocation:completionHandler:",
                cl::shim_clgeocoder_reverse_geocode_location as *const () as u64,
                "v@:@@",
            ),
            me(
                "cancelGeocode",
                cl::shim_clgeocoder_cancel_geocode as *const () as u64,
                "v@:",
            ),
            me(
                "isGeocoding",
                cl::shim_clgeocoder_is_geocoding as *const () as u64,
                "B@:",
            ),
        ],
        class_methods: vec![
            me("new", cl::shim_clgeocoder_new as *const () as u64, "@@:"),
            me(
                "alloc",
                cl::shim_clgeocoder_alloc as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cl::CLGEOCODER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CLGeocoder");
}

// ── CoreBluetooth ─────────────────────────────────────────────────────────────

pub fn register_cbcentralmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::cbcentralmanager_class_addr();
    let metaclass_addr = nsobj::cbcentralmanager_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CBCentralManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithDelegate:queue:",
                cl::shim_cbcentralmanager_init_with_delegate as *const () as u64,
                "@@:@@",
            ),
            me(
                "initWithDelegate:queue:options:",
                cl::shim_cbcentralmanager_init_with_delegate_options as *const () as u64,
                "@@:@@@",
            ),
            me(
                "state",
                cl::shim_cbcentralmanager_state as *const () as u64,
                "Q@:",
            ),
            me(
                "delegate",
                cl::shim_cbcentralmanager_delegate as *const () as u64,
                "@@:",
            ),
            me(
                "scanForPeripheralsWithServices:options:",
                cl::shim_cbcentralmanager_scan_for_peripherals as *const () as u64,
                "v@:@@",
            ),
            me(
                "stopScan",
                cl::shim_cbcentralmanager_stop_scan as *const () as u64,
                "v@:",
            ),
            me(
                "connectPeripheral:options:",
                cl::shim_cbcentralmanager_connect as *const () as u64,
                "v@:@@",
            ),
            me(
                "cancelPeripheralConnection:",
                cl::shim_cbcentralmanager_cancel_connection as *const () as u64,
                "v@:@",
            ),
            me(
                "isScanning",
                cl::shim_cbcentralmanager_is_scanning as *const () as u64,
                "B@:",
            ),
        ],
        class_methods: vec![
            me(
                "alloc",
                cl::shim_cbcentralmanager_alloc as *const () as u64,
                "@@:",
            ),
            me(
                "authorization",
                cl::shim_cbcentralmanager_authorization as *const () as u64,
                "Q@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cl::CBCENTRALMANAGER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CBCentralManager");
}

pub fn register_cbperipheralmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::cbperipheralmanager_class_addr();
    let metaclass_addr = nsobj::cbperipheralmanager_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CBPeripheralManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithDelegate:queue:",
                cl::shim_cbperipheralmanager_init_with_delegate as *const () as u64,
                "@@:@@",
            ),
            me(
                "state",
                cl::shim_cbperipheralmanager_state as *const () as u64,
                "Q@:",
            ),
            me(
                "delegate",
                cl::shim_cbperipheralmanager_delegate as *const () as u64,
                "@@:",
            ),
            me(
                "startAdvertising:",
                cl::shim_cbperipheralmanager_start_advertising as *const () as u64,
                "v@:@",
            ),
            me(
                "stopAdvertising",
                cl::shim_cbperipheralmanager_stop_advertising as *const () as u64,
                "v@:",
            ),
            me(
                "isAdvertising",
                cl::shim_cbperipheralmanager_is_advertising as *const () as u64,
                "B@:",
            ),
            me(
                "addService:",
                cl::shim_cbperipheralmanager_add as *const () as u64,
                "v@:@",
            ),
            me(
                "removeService:",
                cl::shim_cbperipheralmanager_remove as *const () as u64,
                "v@:@",
            ),
            me(
                "removeAllServices",
                cl::shim_cbperipheralmanager_remove_all_services as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            cl::shim_cbperipheralmanager_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: cl::CBPERIPHERALMANAGER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CBPeripheralManager");
}
