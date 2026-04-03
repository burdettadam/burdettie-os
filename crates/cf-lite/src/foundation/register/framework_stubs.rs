//! Class registrations for CoreData, AVFoundation, CoreLocation, CoreBluetooth stub classes.

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{coredata_stubs as cd, avfoundation_stubs as av, corelocation_stubs as cl, nsobj};

// ── CoreData ──────────────────────────────────────────────────────────────────

pub fn register_nsmanagedobjectcontext_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::nsmanagedobjectcontext_class_addr();
    let metaclass_addr = nsobj::nsmanagedobjectcontext_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSManagedObjectContext".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cd::shim_nsmoc_init as u64, "@@:"),
            me("initWithConcurrencyType:", cd::shim_nsmoc_init_with_concurrency_type as u64, "@@:Q"),
            me("performBlock:", cd::shim_nsmoc_perform_block as u64, "v@:@"),
            me("performBlockAndWait:", cd::shim_nsmoc_perform_block as u64, "v@:@"),
            me("save:", cd::shim_nsmoc_save as u64, "B@:^@"),
            me("executeFetchRequest:error:", cd::shim_nsmoc_execute_fetch_request as u64, "@@:@^@"),
            me("persistentStoreCoordinator", cd::shim_nsmoc_persistent_store_coordinator as u64, "@@:"),
            me("setPersistentStoreCoordinator:", cd::shim_nsmoc_set_persistent_store_coordinator as u64, "v@:@"),
            me("mergeChangesFromContextDidSaveNotification:", cd::shim_nsmoc_merge_changes_from_context_did_save as u64, "v@:@"),
            me("undoManager", cd::shim_nsmoc_undo_manager as u64, "@@:"),
            me("hasChanges", cd::shim_nsmoc_has_changes as u64, "B@:"),
            me("reset", cd::shim_nsmoc_reset as u64, "v@:"),
            me("rollback", cd::shim_nsmoc_rollback as u64, "v@:"),
        ],
        class_methods: vec![
            me("new", cd::shim_nsmoc_new as u64, "@@:"),
            me("alloc", cd::shim_nsmoc_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cd::NSMANAGEDOBJECTCONTEXT_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSManagedObjectContext");
}

pub fn register_nspersistentstorecoordinator_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::nspersistentstorecoordinator_class_addr();
    let metaclass_addr = nsobj::nspersistentstorecoordinator_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSPersistentStoreCoordinator".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithManagedObjectModel:", cd::shim_nspsc_init_with_model as u64, "@@:@"),
            me("addPersistentStoreWithType:configuration:URL:options:error:", cd::shim_nspsc_add_store as u64, "@@:@@@@^@"),
            me("managedObjectModel", cd::shim_nspsc_managed_object_model as u64, "@@:"),
            me("persistentStores", cd::shim_nspsc_persistent_stores as u64, "@@:"),
        ],
        class_methods: vec![
            me("new", cd::shim_nspsc_new as u64, "@@:"),
            me("alloc", cd::shim_nspsc_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cd::NSPERSISTENTSTORECOORDINATOR_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSPersistentStoreCoordinator");
}

pub fn register_nsfetchrequest_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::nsfetchrequest_class_addr();
    let metaclass_addr = nsobj::nsfetchrequest_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSFetchRequest".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithEntityName:", cd::shim_nsfr_init_with_entity_name as u64, "@@:@"),
            me("entityName", cd::shim_nsfr_entity_name as u64, "@@:"),
            me("predicate", cd::shim_nsfr_predicate as u64, "@@:"),
            me("setPredicate:", cd::shim_nsfr_set_predicate as u64, "v@:@"),
            me("sortDescriptors", cd::shim_nsfr_sort_descriptors as u64, "@@:"),
            me("setSortDescriptors:", cd::shim_nsfr_set_sort_descriptors as u64, "v@:@"),
            me("fetchLimit", cd::shim_nsfr_fetch_limit as u64, "Q@:"),
            me("setFetchLimit:", cd::shim_nsfr_set_fetch_limit as u64, "v@:Q"),
        ],
        class_methods: vec![
            me("fetchRequestWithEntityName:", cd::shim_nsfr_fetch_request_with_entity_name as u64, "@@:@"),
            me("alloc", cd::shim_nsfr_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cd::NSFETCHREQUEST_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSFetchRequest");
}

pub fn register_nsmanagedobject_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::nsmanagedobject_class_addr();
    let metaclass_addr = nsobj::nsmanagedobject_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSManagedObject".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithEntity:insertIntoManagedObjectContext:", cd::shim_nsmo_init_with_entity_context as u64, "@@:@@"),
            me("valueForKey:", cd::shim_nsmo_value_for_key as u64, "@@:@"),
            me("setValue:forKey:", cd::shim_nsmo_set_value_for_key as u64, "v@:@@"),
            me("entity", cd::shim_nsmo_entity as u64, "@@:"),
            me("managedObjectContext", cd::shim_nsmo_managed_object_context as u64, "@@:"),
            me("isDeleted", cd::shim_nsmo_is_deleted as u64, "B@:"),
            me("isInserted", cd::shim_nsmo_is_inserted as u64, "B@:"),
            me("isUpdated", cd::shim_nsmo_is_updated as u64, "B@:"),
        ],
        class_methods: vec![
            me("alloc", cd::shim_nsmo_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cd::NSMANAGEDOBJECT_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSManagedObject");
}

pub fn register_nsentitydescription_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::nsentitydescription_class_addr();
    let metaclass_addr = nsobj::nsentitydescription_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSEntityDescription".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cd::shim_nsed_init as u64, "@@:"),
            me("name", cd::shim_nsed_name as u64, "@@:"),
            me("properties", cd::shim_nsed_properties as u64, "@@:"),
            me("managedObjectClassName", cd::shim_nsed_managed_object_class_name as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", cd::shim_nsed_alloc as u64, "@@:"),
            me("entityForName:inManagedObjectContext:", cd::shim_nsed_entity_for_name as u64, "@@:@@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cd::NSENTITYDESCRIPTION_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSEntityDescription");
}

pub fn register_nsmanagedobjectmodel_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::nsmanagedobjectmodel_class_addr();
    let metaclass_addr = nsobj::nsmanagedobjectmodel_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "NSManagedObjectModel".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cd::shim_nsmom_init as u64, "@@:"),
            me("initWithContentsOfURL:", cd::shim_nsmom_init_with_contents_of_url as u64, "@@:@"),
            me("entities", cd::shim_nsmom_entities as u64, "@@:"),
            me("versionIdentifiers", cd::shim_nsmom_entity_versions_by_name as u64, "@@:"),
        ],
        class_methods: vec![
            me("alloc", cd::shim_nsmom_alloc as u64, "@@:"),
            me("mergedModelFromBundles:", cd::shim_nsmom_merged_model_from_bundles as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cd::NSMANAGEDOBJECTMODEL_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "NSManagedObjectModel");
}

// ── AVFoundation ──────────────────────────────────────────────────────────────

pub fn register_avaudioplayer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::avaudioplayer_class_addr();
    let metaclass_addr = nsobj::avaudioplayer_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVAudioPlayer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithContentsOfURL:error:", av::shim_avaudioplayer_init_with_url as u64, "@@:@^@"),
            me("initWithData:error:", av::shim_avaudioplayer_init_with_data as u64, "@@:@^@"),
            me("play", av::shim_avaudioplayer_play as u64, "B@:"),
            me("pause", av::shim_avaudioplayer_pause as u64, "v@:"),
            me("stop", av::shim_avaudioplayer_stop as u64, "v@:"),
            me("prepareToPlay", av::shim_avaudioplayer_prepare_to_play as u64, "B@:"),
            me("isPlaying", av::shim_avaudioplayer_is_playing as u64, "B@:"),
            me("duration", av::shim_avaudioplayer_duration as u64, "d@:"),
            me("currentTime", av::shim_avaudioplayer_current_time as u64, "d@:"),
            me("setCurrentTime:", av::shim_avaudioplayer_set_current_time as u64, "v@:d"),
            me("volume", av::shim_avaudioplayer_volume as u64, "f@:"),
            me("setVolume:", av::shim_avaudioplayer_set_volume as u64, "v@:f"),
            me("numberOfLoops", av::shim_avaudioplayer_number_of_loops as u64, "q@:"),
            me("setNumberOfLoops:", av::shim_avaudioplayer_set_number_of_loops as u64, "v@:q"),
            me("url", av::shim_avaudioplayer_url as u64, "@@:"),
            me("delegate", av::shim_avaudioplayer_delegate as u64, "@@:"),
            me("setDelegate:", av::shim_avaudioplayer_set_delegate as u64, "v@:@"),
        ],
        class_methods: vec![
            me("alloc", av::shim_avaudioplayer_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: av::AVAUDIOPLAYER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVAudioPlayer");
}

pub fn register_avaudiorecorder_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::avaudiorecorder_class_addr();
    let metaclass_addr = nsobj::avaudiorecorder_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVAudioRecorder".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithURL:settings:error:", av::shim_avaudiorecorder_init_with_url as u64, "@@:@@^@"),
            me("record", av::shim_avaudiorecorder_record as u64, "B@:"),
            me("stop", av::shim_avaudiorecorder_stop as u64, "v@:"),
            me("isRecording", av::shim_avaudiorecorder_is_recording as u64, "B@:"),
            me("deleteRecording", av::shim_avaudiorecorder_delete_recording as u64, "B@:"),
        ],
        class_methods: vec![
            me("alloc", av::shim_avaudiorecorder_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: av::AVAUDIORECORDER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVAudioRecorder");
}

pub fn register_avaudioengine_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::avaudioengine_class_addr();
    let metaclass_addr = nsobj::avaudioengine_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVAudioEngine".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", av::shim_avaudioengine_init as u64, "@@:"),
            me("startAndReturnError:", av::shim_avaudioengine_start as u64, "B@:^@"),
            me("stop", av::shim_avaudioengine_stop as u64, "v@:"),
            me("isRunning", av::shim_avaudioengine_is_running as u64, "B@:"),
            me("mainMixerNode", av::shim_avaudioengine_main_mixer_node as u64, "@@:"),
            me("outputNode", av::shim_avaudioengine_output_node as u64, "@@:"),
            me("inputNode", av::shim_avaudioengine_input_node as u64, "@@:"),
        ],
        class_methods: vec![
            me("new", av::shim_avaudioengine_new as u64, "@@:"),
            me("alloc", av::shim_avaudioengine_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: av::AVAUDIOENGINE_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVAudioEngine");
}

pub fn register_avcapturedevice_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::avcapturedevice_class_addr();
    let metaclass_addr = nsobj::avcapturedevice_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVCaptureDevice".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("localizedName", av::shim_avcapturedevice_local_name as u64, "@@:"),
            me("uniqueID", av::shim_avcapturedevice_unique_id as u64, "@@:"),
            me("hasMediaType:", av::shim_avcapturedevice_has_media_type as u64, "B@:@"),
        ],
        class_methods: vec![
            me("alloc", av::shim_avcapturedevice_alloc as u64, "@@:"),
            me("defaultDeviceWithMediaType:", av::shim_avcapturedevice_default_device_with_media_type as u64, "@@:@"),
            me("defaultDeviceWithDeviceType:mediaType:position:", av::shim_avcapturedevice_default_device as u64, "@@:@@@"),
            me("devicesWithMediaType:", av::shim_avcapturedevice_devices_with_media_type as u64, "@@:@"),
            me("authorizationStatusForMediaType:", av::shim_avcapturedevice_authorization_status_for_media_type as u64, "Q@:@"),
            me("requestAccessForMediaType:completionHandler:", av::shim_avcapturedevice_request_access as u64, "v@:@@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: av::AVCAPTUREDEVICE_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVCaptureDevice");
}

pub fn register_avcapturesession_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::avcapturesession_class_addr();
    let metaclass_addr = nsobj::avcapturesession_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVCaptureSession".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", av::shim_avcapturesession_init as u64, "@@:"),
            me("addInput:", av::shim_avcapturesession_add_input as u64, "v@:@"),
            me("addOutput:", av::shim_avcapturesession_add_output as u64, "v@:@"),
            me("removeInput:", av::shim_avcapturesession_remove_input as u64, "v@:@"),
            me("removeOutput:", av::shim_avcapturesession_remove_output as u64, "v@:@"),
            me("canAddInput:", av::shim_avcapturesession_can_add_input as u64, "B@:@"),
            me("canAddOutput:", av::shim_avcapturesession_can_add_output as u64, "B@:@"),
            me("startRunning", av::shim_avcapturesession_start_running as u64, "v@:"),
            me("stopRunning", av::shim_avcapturesession_stop_running as u64, "v@:"),
            me("isRunning", av::shim_avcapturesession_is_running as u64, "B@:"),
            me("sessionPreset", av::shim_avcapturesession_session_preset as u64, "@@:"),
            me("setSessionPreset:", av::shim_avcapturesession_set_session_preset as u64, "v@:@"),
        ],
        class_methods: vec![
            me("new", av::shim_avcapturesession_new as u64, "@@:"),
            me("alloc", av::shim_avcapturesession_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: av::AVCAPTURESESSION_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVCaptureSession");
}

pub fn register_avplayer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::avplayer_class_addr();
    let metaclass_addr = nsobj::avplayer_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "AVPlayer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("play", av::shim_avplayer_play as u64, "v@:"),
            me("pause", av::shim_avplayer_pause as u64, "v@:"),
            me("rate", av::shim_avplayer_rate as u64, "f@:"),
            me("setRate:", av::shim_avplayer_set_rate as u64, "v@:f"),
            me("status", av::shim_avplayer_status as u64, "Q@:"),
            me("currentItem", av::shim_avplayer_current_item as u64, "@@:"),
            me("volume", av::shim_avplayer_volume as u64, "f@:"),
            me("setVolume:", av::shim_avplayer_set_volume as u64, "v@:f"),
        ],
        class_methods: vec![
            me("alloc", av::shim_avplayer_alloc as u64, "@@:"),
            me("playerWithURL:", av::shim_avplayer_player_with_url as u64, "@@:@"),
            me("playerWithPlayerItem:", av::shim_avplayer_player_with_player_item as u64, "@@:@"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: av::AVPLAYER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "AVPlayer");
}

// ── CoreLocation ──────────────────────────────────────────────────────────────

pub fn register_cllocationmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::cllocationmanager_class_addr();
    let metaclass_addr = nsobj::cllocationmanager_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CLLocationManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cl::shim_cllocationmanager_init as u64, "@@:"),
            me("delegate", cl::shim_cllocationmanager_delegate as u64, "@@:"),
            me("setDelegate:", cl::shim_cllocationmanager_set_delegate as u64, "v@:@"),
            me("desiredAccuracy", cl::shim_cllocationmanager_desired_accuracy as u64, "d@:"),
            me("setDesiredAccuracy:", cl::shim_cllocationmanager_set_desired_accuracy as u64, "v@:d"),
            me("distanceFilter", cl::shim_cllocationmanager_distance_filter as u64, "d@:"),
            me("setDistanceFilter:", cl::shim_cllocationmanager_set_distance_filter as u64, "v@:d"),
            me("requestWhenInUseAuthorization", cl::shim_cllocationmanager_request_when_in_use_authorization as u64, "v@:"),
            me("requestAlwaysAuthorization", cl::shim_cllocationmanager_request_always_authorization as u64, "v@:"),
            me("startUpdatingLocation", cl::shim_cllocationmanager_start_updating_location as u64, "v@:"),
            me("stopUpdatingLocation", cl::shim_cllocationmanager_stop_updating_location as u64, "v@:"),
            me("requestLocation", cl::shim_cllocationmanager_request_location as u64, "v@:"),
            me("startUpdatingHeading", cl::shim_cllocationmanager_start_updating_heading as u64, "v@:"),
            me("stopUpdatingHeading", cl::shim_cllocationmanager_stop_updating_heading as u64, "v@:"),
            me("startMonitoringSignificantLocationChanges", cl::shim_cllocationmanager_start_monitoring_significant_location_changes as u64, "v@:"),
            me("stopMonitoringSignificantLocationChanges", cl::shim_cllocationmanager_stop_monitoring_significant_location_changes as u64, "v@:"),
            me("location", cl::shim_cllocationmanager_location as u64, "@@:"),
        ],
        class_methods: vec![
            me("new", cl::shim_cllocationmanager_new as u64, "@@:"),
            me("alloc", cl::shim_cllocationmanager_alloc as u64, "@@:"),
            me("authorizationStatus", cl::shim_cllocationmanager_authorization_status as u64, "i@:"),
            me("locationServicesEnabled", cl::shim_cllocationmanager_location_services_enabled as u64, "B@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cl::CLLOCATIONMANAGER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CLLocationManager");
}

pub fn register_cllocation_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::cllocation_class_addr();
    let metaclass_addr = nsobj::cllocation_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CLLocation".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithLatitude:longitude:", cl::shim_cllocation_init_with_lat_lon as u64, "@@:dd"),
            me("coordinate", cl::shim_cllocation_coordinate_latitude as u64, "d@:"), // simplified: returns lat
            me("altitude", cl::shim_cllocation_altitude as u64, "d@:"),
            me("horizontalAccuracy", cl::shim_cllocation_horizontal_accuracy as u64, "d@:"),
            me("verticalAccuracy", cl::shim_cllocation_vertical_accuracy as u64, "d@:"),
            me("speed", cl::shim_cllocation_speed as u64, "d@:"),
            me("course", cl::shim_cllocation_course as u64, "d@:"),
            me("timestamp", cl::shim_cllocation_timestamp as u64, "@@:"),
            me("distanceFromLocation:", cl::shim_cllocation_distance_from_location as u64, "d@:@"),
        ],
        class_methods: vec![
            me("alloc", cl::shim_cllocation_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cl::CLLOCATION_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CLLocation");
}

pub fn register_clgeocoder_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::clgeocoder_class_addr();
    let metaclass_addr = nsobj::clgeocoder_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CLGeocoder".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", cl::shim_clgeocoder_init as u64, "@@:"),
            me("geocodeAddressString:completionHandler:", cl::shim_clgeocoder_geocode_address_string as u64, "v@:@@"),
            me("reverseGeocodeLocation:completionHandler:", cl::shim_clgeocoder_reverse_geocode_location as u64, "v@:@@"),
            me("cancelGeocode", cl::shim_clgeocoder_cancel_geocode as u64, "v@:"),
            me("isGeocoding", cl::shim_clgeocoder_is_geocoding as u64, "B@:"),
        ],
        class_methods: vec![
            me("new", cl::shim_clgeocoder_new as u64, "@@:"),
            me("alloc", cl::shim_clgeocoder_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cl::CLGEOCODER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CLGeocoder");
}

// ── CoreBluetooth ─────────────────────────────────────────────────────────────

pub fn register_cbcentralmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::cbcentralmanager_class_addr();
    let metaclass_addr = nsobj::cbcentralmanager_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CBCentralManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithDelegate:queue:", cl::shim_cbcentralmanager_init_with_delegate as u64, "@@:@@"),
            me("initWithDelegate:queue:options:", cl::shim_cbcentralmanager_init_with_delegate_options as u64, "@@:@@@"),
            me("state", cl::shim_cbcentralmanager_state as u64, "Q@:"),
            me("delegate", cl::shim_cbcentralmanager_delegate as u64, "@@:"),
            me("scanForPeripheralsWithServices:options:", cl::shim_cbcentralmanager_scan_for_peripherals as u64, "v@:@@"),
            me("stopScan", cl::shim_cbcentralmanager_stop_scan as u64, "v@:"),
            me("connectPeripheral:options:", cl::shim_cbcentralmanager_connect as u64, "v@:@@"),
            me("cancelPeripheralConnection:", cl::shim_cbcentralmanager_cancel_connection as u64, "v@:@"),
            me("isScanning", cl::shim_cbcentralmanager_is_scanning as u64, "B@:"),
        ],
        class_methods: vec![
            me("alloc", cl::shim_cbcentralmanager_alloc as u64, "@@:"),
            me("authorization", cl::shim_cbcentralmanager_authorization as u64, "Q@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cl::CBCENTRALMANAGER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CBCentralManager");
}

pub fn register_cbperipheralmanager_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr     = nsobj::cbperipheralmanager_class_addr();
    let metaclass_addr = nsobj::cbperipheralmanager_metaclass_addr();
    rt.register_synthetic_class(ClassInfo {
        name: "CBPeripheralManager".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("initWithDelegate:queue:", cl::shim_cbperipheralmanager_init_with_delegate as u64, "@@:@@"),
            me("state", cl::shim_cbperipheralmanager_state as u64, "Q@:"),
            me("delegate", cl::shim_cbperipheralmanager_delegate as u64, "@@:"),
            me("startAdvertising:", cl::shim_cbperipheralmanager_start_advertising as u64, "v@:@"),
            me("stopAdvertising", cl::shim_cbperipheralmanager_stop_advertising as u64, "v@:"),
            me("isAdvertising", cl::shim_cbperipheralmanager_is_advertising as u64, "B@:"),
            me("addService:", cl::shim_cbperipheralmanager_add as u64, "v@:@"),
            me("removeService:", cl::shim_cbperipheralmanager_remove as u64, "v@:@"),
            me("removeAllServices", cl::shim_cbperipheralmanager_remove_all_services as u64, "v@:"),
        ],
        class_methods: vec![
            me("alloc", cl::shim_cbperipheralmanager_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: cl::CBPERIPHERALMANAGER_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(metaclass_addr, "CBPeripheralManager");
}
