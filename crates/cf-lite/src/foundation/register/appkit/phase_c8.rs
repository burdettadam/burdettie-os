//! AppKit GUI class method registrations (Phase C8).

use super::me;
use crate::{
    nscontrol, nsdocument, nsdocumentcontroller, nsobj, nsstoryboard, nstextfield,
    nsviewcontroller, nswindowcontroller,
};
use objc_runtime::ClassInfo;
use std::collections::HashMap;

// ── Phase C8 AppKit GUI classes ────────────────────────────────

/// Register `NSStoryboard` - compiled storyboard scene loader.
pub fn register_nsstoryboard_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsstoryboard_class_addr();
    let metaclass_addr = nsobj::nsstoryboard_metaclass_addr();
    let info = ClassInfo {
        name: "NSStoryboard".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithName:bundle:",
                nsstoryboard::shim_nsstoryboard_init_with_name_bundle as *const () as u64,
                "@@:@@",
            ),
            me(
                "instantiateInitialController",
                nsstoryboard::shim_nsstoryboard_instantiate_initial_controller as *const () as u64,
                "@@:",
            ),
            me(
                "instantiateInitialControllerWithCreator:",
                nsstoryboard::shim_nsstoryboard_instantiate_initial_controller_with_creator
                    as *const () as u64,
                "@@:@?",
            ),
            me(
                "instantiateControllerWithIdentifier:",
                nsstoryboard::shim_nsstoryboard_instantiate_controller_with_identifier as *const ()
                    as u64,
                "@@:@",
            ),
            me(
                "instantiateControllerWithIdentifier:creator:",
                nsstoryboard::shim_nsstoryboard_instantiate_controller_with_identifier_creator
                    as *const () as u64,
                "@@:@@?",
            ),
        ],
        class_methods: vec![
            me(
                "alloc",
                nsstoryboard::shim_nsstoryboard_alloc as *const () as u64,
                "@@:",
            ),
            me(
                "storyboardWithName:bundle:",
                nsstoryboard::shim_nsstoryboard_with_name_bundle as *const () as u64,
                "@@:@@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsstoryboard::STORYBOARD_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSStoryboard");
}

/// Register `NSViewController` — view controller for Cocoa GUI apps.
pub fn register_nsviewcontroller_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsviewcontroller_class_addr();
    let metaclass_addr = nsobj::nsviewcontroller_metaclass_addr();
    let info = ClassInfo {
        name: "NSViewController".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithNibName:bundle:",
                nsviewcontroller::shim_nsviewcontroller_init_with_nib_name_bundle as *const ()
                    as u64,
                "@@:@@",
            ),
            me(
                "initWithCoder:",
                nsviewcontroller::shim_nsviewcontroller_init_with_coder as *const () as u64,
                "@@:@",
            ),
            me(
                "view",
                nsviewcontroller::shim_nsviewcontroller_view as *const () as u64,
                "@@:",
            ),
            me(
                "setView:",
                nsviewcontroller::shim_nsviewcontroller_set_view as *const () as u64,
                "v@:@",
            ),
            me(
                "title",
                nsviewcontroller::shim_nsviewcontroller_title as *const () as u64,
                "@@:",
            ),
            me(
                "setTitle:",
                nsviewcontroller::shim_nsviewcontroller_set_title as *const () as u64,
                "v@:@",
            ),
            me(
                "representedObject",
                nsviewcontroller::shim_nsviewcontroller_represented_object as *const () as u64,
                "@@:",
            ),
            me(
                "setRepresentedObject:",
                nsviewcontroller::shim_nsviewcontroller_set_represented_object as *const () as u64,
                "v@:@",
            ),
            me(
                "loadView",
                nsviewcontroller::shim_nsviewcontroller_load_view as *const () as u64,
                "v@:",
            ),
            me(
                "viewDidLoad",
                nsviewcontroller::shim_nsviewcontroller_view_did_load as *const () as u64,
                "v@:",
            ),
            me(
                "viewWillAppear",
                nsviewcontroller::shim_nsviewcontroller_view_will_appear as *const () as u64,
                "v@:",
            ),
            me(
                "viewDidAppear",
                nsviewcontroller::shim_nsviewcontroller_view_did_appear as *const () as u64,
                "v@:",
            ),
            me(
                "viewWillDisappear",
                nsviewcontroller::shim_nsviewcontroller_view_will_disappear as *const () as u64,
                "v@:",
            ),
            me(
                "viewDidDisappear",
                nsviewcontroller::shim_nsviewcontroller_view_did_disappear as *const () as u64,
                "v@:",
            ),
            me(
                "isViewLoaded",
                nsviewcontroller::shim_nsviewcontroller_is_view_loaded as *const () as u64,
                "B@:",
            ),
            me(
                "nibName",
                nsviewcontroller::shim_nsviewcontroller_nib_name as *const () as u64,
                "@@:",
            ),
            me(
                "nibBundle",
                nsviewcontroller::shim_nsviewcontroller_nib_bundle as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsviewcontroller::VIEWCONTROLLER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSViewController");
}

/// Register `NSWindowController` — window management controller.
pub fn register_nswindowcontroller_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nswindowcontroller_class_addr();
    let metaclass_addr = nsobj::nswindowcontroller_metaclass_addr();
    let info = ClassInfo {
        name: "NSWindowController".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "initWithWindow:",
                nswindowcontroller::shim_nswindowcontroller_init_with_window as *const () as u64,
                "@@:@",
            ),
            me(
                "initWithWindowNibName:",
                nswindowcontroller::shim_nswindowcontroller_init_with_window_nib_name as *const ()
                    as u64,
                "@@:@",
            ),
            me(
                "initWithWindowNibName:owner:",
                nswindowcontroller::shim_nswindowcontroller_init_with_window_nib_name_owner
                    as *const () as u64,
                "@@:@@",
            ),
            me(
                "initWithCoder:",
                nswindowcontroller::shim_nswindowcontroller_init_with_coder as *const () as u64,
                "@@:@",
            ),
            me(
                "window",
                nswindowcontroller::shim_nswindowcontroller_window as *const () as u64,
                "@@:",
            ),
            me(
                "setWindow:",
                nswindowcontroller::shim_nswindowcontroller_set_window as *const () as u64,
                "v@:@",
            ),
            me(
                "showWindow:",
                nswindowcontroller::shim_nswindowcontroller_show_window as *const () as u64,
                "v@:@",
            ),
            me(
                "close",
                nswindowcontroller::shim_nswindowcontroller_close as *const () as u64,
                "v@:",
            ),
            me(
                "loadWindow",
                nswindowcontroller::shim_nswindowcontroller_load_window as *const () as u64,
                "v@:",
            ),
            me(
                "isWindowLoaded",
                nswindowcontroller::shim_nswindowcontroller_is_window_loaded as *const () as u64,
                "B@:",
            ),
            me(
                "windowNibName",
                nswindowcontroller::shim_nswindowcontroller_window_nib_name as *const () as u64,
                "@@:",
            ),
            me(
                "windowNibPath",
                nswindowcontroller::shim_nswindowcontroller_window_nib_path as *const () as u64,
                "@@:",
            ),
            me(
                "owner",
                nswindowcontroller::shim_nswindowcontroller_owner as *const () as u64,
                "@@:",
            ),
            me(
                "shouldCascadeWindows",
                nswindowcontroller::shim_nswindowcontroller_should_cascade_windows as *const ()
                    as u64,
                "B@:",
            ),
            me(
                "setShouldCascadeWindows:",
                nswindowcontroller::shim_nswindowcontroller_set_should_cascade_windows as *const ()
                    as u64,
                "v@:B",
            ),
            me(
                "windowWillLoad",
                nswindowcontroller::shim_nswindowcontroller_window_will_load as *const () as u64,
                "v@:",
            ),
            me(
                "windowDidLoad",
                nswindowcontroller::shim_nswindowcontroller_window_did_load as *const () as u64,
                "v@:",
            ),
            me(
                "document",
                nswindowcontroller::shim_nswindowcontroller_document as *const () as u64,
                "@@:",
            ),
            me(
                "setDocument:",
                nswindowcontroller::shim_nswindowcontroller_set_document as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nswindowcontroller::WINDOWCONTROLLER_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsresponder_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSWindowController");
}

/// Register `NSDocument` - base class for document-based Cocoa apps.
pub fn register_nsdocument_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdocument_class_addr();
    let metaclass_addr = nsobj::nsdocument_metaclass_addr();
    let info = ClassInfo {
        name: "NSDocument".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nsdocument::shim_nsdocument_init as *const () as u64,
                "@@:",
            ),
            me(
                "initWithType:error:",
                nsdocument::shim_nsdocument_init_with_type_error as *const () as u64,
                "@@:@^@",
            ),
            me(
                "initWithContentsOfURL:ofType:error:",
                nsdocument::shim_nsdocument_init_with_contents_of_url_of_type_error as *const ()
                    as u64,
                "@@:@@^@",
            ),
            me(
                "initForURL:withContentsOfURL:ofType:error:",
                nsdocument::shim_nsdocument_init_for_url_with_contents_of_url_of_type_error
                    as *const () as u64,
                "@@:@@@^@",
            ),
            me(
                "fileURL",
                nsdocument::shim_nsdocument_file_url as *const () as u64,
                "@@:",
            ),
            me(
                "setFileURL:",
                nsdocument::shim_nsdocument_set_file_url as *const () as u64,
                "v@:@",
            ),
            me(
                "fileName",
                nsdocument::shim_nsdocument_file_name as *const () as u64,
                "@@:",
            ),
            me(
                "setFileName:",
                nsdocument::shim_nsdocument_set_file_name as *const () as u64,
                "v@:@",
            ),
            me(
                "fileType",
                nsdocument::shim_nsdocument_file_type as *const () as u64,
                "@@:",
            ),
            me(
                "setFileType:",
                nsdocument::shim_nsdocument_set_file_type as *const () as u64,
                "v@:@",
            ),
            me(
                "displayName",
                nsdocument::shim_nsdocument_display_name as *const () as u64,
                "@@:",
            ),
            me(
                "setDisplayName:",
                nsdocument::shim_nsdocument_set_display_name as *const () as u64,
                "v@:@",
            ),
            me(
                "windowControllers",
                nsdocument::shim_nsdocument_window_controllers as *const () as u64,
                "@@:",
            ),
            me(
                "addWindowController:",
                nsdocument::shim_nsdocument_add_window_controller as *const () as u64,
                "v@:@",
            ),
            me(
                "removeWindowController:",
                nsdocument::shim_nsdocument_remove_window_controller as *const () as u64,
                "v@:@",
            ),
            me(
                "makeWindowControllers",
                nsdocument::shim_nsdocument_make_window_controllers as *const () as u64,
                "v@:",
            ),
            me(
                "showWindows",
                nsdocument::shim_nsdocument_show_windows as *const () as u64,
                "v@:",
            ),
            me(
                "windowForSheet",
                nsdocument::shim_nsdocument_window_for_sheet as *const () as u64,
                "@@:",
            ),
            me(
                "windowNibName",
                nsdocument::shim_nsdocument_window_nib_name as *const () as u64,
                "@@:",
            ),
            me(
                "readFromURL:ofType:error:",
                nsdocument::shim_nsdocument_read_from_url_of_type_error as *const () as u64,
                "B@:@@^@",
            ),
            me(
                "readFromFile:ofType:",
                nsdocument::shim_nsdocument_read_from_file_of_type as *const () as u64,
                "B@:@@",
            ),
            me(
                "readFromData:ofType:error:",
                nsdocument::shim_nsdocument_read_from_data_of_type_error as *const () as u64,
                "B@:@@^@",
            ),
            me(
                "writeToURL:ofType:error:",
                nsdocument::shim_nsdocument_write_to_url_of_type_error as *const () as u64,
                "B@:@@^@",
            ),
            me(
                "writeToFile:ofType:",
                nsdocument::shim_nsdocument_write_to_file_of_type as *const () as u64,
                "B@:@@",
            ),
            me(
                "dataOfType:error:",
                nsdocument::shim_nsdocument_data_of_type_error as *const () as u64,
                "@@:@^@",
            ),
            me(
                "saveToURL:ofType:forSaveOperation:error:",
                nsdocument::shim_nsdocument_save_to_url_of_type_for_save_operation_error
                    as *const () as u64,
                "B@:@@Q^@",
            ),
            me(
                "saveDocument:",
                nsdocument::shim_nsdocument_save_document as *const () as u64,
                "v@:@",
            ),
            me(
                "saveDocumentAs:",
                nsdocument::shim_nsdocument_save_document_as as *const () as u64,
                "v@:@",
            ),
            me(
                "saveDocumentTo:",
                nsdocument::shim_nsdocument_save_document_to as *const () as u64,
                "v@:@",
            ),
            me(
                "saveDocumentWithDelegate:didSaveSelector:contextInfo:",
                nsdocument::shim_nsdocument_save_document_with_delegate as *const () as u64,
                "v@:@:^v",
            ),
            me(
                "revertDocumentToSaved:",
                nsdocument::shim_nsdocument_revert_document_to_saved as *const () as u64,
                "v@:@",
            ),
            me(
                "close",
                nsdocument::shim_nsdocument_close as *const () as u64,
                "v@:",
            ),
            me(
                "canCloseDocumentWithDelegate:shouldCloseSelector:contextInfo:",
                nsdocument::shim_nsdocument_can_close_document_with_delegate as *const () as u64,
                "v@:@:^v",
            ),
            me(
                "shouldCloseWindowController:delegate:shouldCloseSelector:contextInfo:",
                nsdocument::shim_nsdocument_should_close_window_controller as *const () as u64,
                "v@:@@:^v",
            ),
            me(
                "isDocumentEdited",
                nsdocument::shim_nsdocument_is_document_edited as *const () as u64,
                "B@:",
            ),
            me(
                "setDocumentEdited:",
                nsdocument::shim_nsdocument_set_document_edited as *const () as u64,
                "v@:B",
            ),
            me(
                "hasUnautosavedChanges",
                nsdocument::shim_nsdocument_has_unautosaved_changes as *const () as u64,
                "B@:",
            ),
            me(
                "updateChangeCount:",
                nsdocument::shim_nsdocument_update_change_count as *const () as u64,
                "v@:Q",
            ),
            me(
                "changeCount",
                nsdocument::shim_nsdocument_change_count as *const () as u64,
                "Q@:",
            ),
            me(
                "hasUndoManager",
                nsdocument::shim_nsdocument_has_undo_manager as *const () as u64,
                "B@:",
            ),
            me(
                "setHasUndoManager:",
                nsdocument::shim_nsdocument_set_has_undo_manager as *const () as u64,
                "v@:B",
            ),
            me(
                "undoManager",
                nsdocument::shim_nsdocument_undo_manager as *const () as u64,
                "@@:",
            ),
            me(
                "autosavedContentsFileURL",
                nsdocument::shim_nsdocument_autosaved_contents_file_url as *const () as u64,
                "@@:",
            ),
            me(
                "setAutosavedContentsFileURL:",
                nsdocument::shim_nsdocument_set_autosaved_contents_file_url as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsdocument::DOCUMENT_SIZE as u32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDocument");
}

/// Register `NSDocumentController` - document registry and opener.
pub fn register_nsdocumentcontroller_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdocumentcontroller_class_addr();
    let metaclass_addr = nsobj::nsdocumentcontroller_metaclass_addr();
    let info = ClassInfo {
        name: "NSDocumentController".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("documents", nsdocumentcontroller::shim_nsdocumentcontroller_documents as *const () as u64, "@@:"),
            me("currentDocument", nsdocumentcontroller::shim_nsdocumentcontroller_current_document as *const () as u64, "@@:"),
            me("addDocument:", nsdocumentcontroller::shim_nsdocumentcontroller_add_document as *const () as u64, "v@:@"),
            me("removeDocument:", nsdocumentcontroller::shim_nsdocumentcontroller_remove_document as *const () as u64, "v@:@"),
            me("documentForURL:", nsdocumentcontroller::shim_nsdocumentcontroller_document_for_url as *const () as u64, "@@:@"),
            me("documentForWindow:", nsdocumentcontroller::shim_nsdocumentcontroller_document_for_window as *const () as u64, "@@:@"),
            me("documentClassForType:", nsdocumentcontroller::shim_nsdocumentcontroller_document_class_for_type as *const () as u64, "#@:@"),
            me("typeForContentsOfURL:error:", nsdocumentcontroller::shim_nsdocumentcontroller_type_for_contents_of_url_error as *const () as u64, "@@:@^@"),
            me("defaultType", nsdocumentcontroller::shim_nsdocumentcontroller_default_type as *const () as u64, "@@:"),
            me("makeUntitledDocumentOfType:error:", nsdocumentcontroller::shim_nsdocumentcontroller_make_untitled_document_of_type_error as *const () as u64, "@@:@^@"),
            me("openUntitledDocumentAndDisplay:error:", nsdocumentcontroller::shim_nsdocumentcontroller_open_untitled_document_and_display_error as *const () as u64, "@@:B^@"),
            me("makeDocumentWithContentsOfURL:ofType:error:", nsdocumentcontroller::shim_nsdocumentcontroller_make_document_with_contents_of_url_of_type_error as *const () as u64, "@@:@@^@"),
            me("openDocumentWithContentsOfURL:display:error:", nsdocumentcontroller::shim_nsdocumentcontroller_open_document_with_contents_of_url_display_error as *const () as u64, "@@:@B^@"),
            me("openDocumentWithContentsOfURL:display:completionHandler:", nsdocumentcontroller::shim_nsdocumentcontroller_open_document_with_contents_of_url_display_completion as *const () as u64, "v@:@B@?"),
            me("newDocument:", nsdocumentcontroller::shim_nsdocumentcontroller_new_document as *const () as u64, "v@:@"),
            me("openDocument:", nsdocumentcontroller::shim_nsdocumentcontroller_open_document as *const () as u64, "v@:@"),
            me("saveAllDocuments:", nsdocumentcontroller::shim_nsdocumentcontroller_save_all_documents as *const () as u64, "v@:@"),
            me("closeAllDocumentsWithDelegate:didCloseAllSelector:contextInfo:", nsdocumentcontroller::shim_nsdocumentcontroller_close_all_documents_with_delegate as *const () as u64, "v@:@:^v"),
            me("clearRecentDocuments:", nsdocumentcontroller::shim_nsdocumentcontroller_clear_recent_documents as *const () as u64, "v@:@"),
            me("noteNewRecentDocument:", nsdocumentcontroller::shim_nsdocumentcontroller_note_new_recent_document as *const () as u64, "v@:@"),
            me("noteNewRecentDocumentURL:", nsdocumentcontroller::shim_nsdocumentcontroller_note_new_recent_document_url as *const () as u64, "v@:@"),
            me("recentDocumentURLs", nsdocumentcontroller::shim_nsdocumentcontroller_recent_document_urls as *const () as u64, "@@:"),
            me("currentDirectory", nsdocumentcontroller::shim_nsdocumentcontroller_current_directory as *const () as u64, "@@:"),
            me("setAutosavingDelay:", nsdocumentcontroller::shim_nsdocumentcontroller_set_autosaving_delay as *const () as u64, "v@:d"),
            me("autosavingDelay", nsdocumentcontroller::shim_nsdocumentcontroller_autosaving_delay as *const () as u64, "d@:"),
            me("runModalOpenPanel:forTypes:", nsdocumentcontroller::shim_nsdocumentcontroller_run_modal_open_panel_for_types as *const () as u64, "q@:@@"),
        ],
        class_methods: vec![
            me("sharedDocumentController", nsdocumentcontroller::shim_nsdocumentcontroller_shared_document_controller as *const () as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: 8, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDocumentController");
}

/// Register `NSControl` — base class for interactive controls.
pub fn register_nscontrol_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscontrol_class_addr();
    let metaclass_addr = nsobj::nscontrol_metaclass_addr();
    let info = ClassInfo {
        name: "NSControl".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "isEnabled",
                nscontrol::shim_nscontrol_is_enabled as *const () as u64,
                "B@:",
            ),
            me(
                "setEnabled:",
                nscontrol::shim_nscontrol_set_enabled as *const () as u64,
                "v@:B",
            ),
            me(
                "target",
                nscontrol::shim_nscontrol_target as *const () as u64,
                "@@:",
            ),
            me(
                "setTarget:",
                nscontrol::shim_nscontrol_set_target as *const () as u64,
                "v@:@",
            ),
            me(
                "action",
                nscontrol::shim_nscontrol_action as *const () as u64,
                ":@:",
            ),
            me(
                "setAction:",
                nscontrol::shim_nscontrol_set_action as *const () as u64,
                "v@::",
            ),
            me(
                "stringValue",
                nscontrol::shim_nscontrol_string_value as *const () as u64,
                "@@:",
            ),
            me(
                "setStringValue:",
                nscontrol::shim_nscontrol_set_string_value as *const () as u64,
                "v@:@",
            ),
            me(
                "integerValue",
                nscontrol::shim_nscontrol_integer_value as *const () as u64,
                "q@:",
            ),
            me(
                "setIntegerValue:",
                nscontrol::shim_nscontrol_set_integer_value as *const () as u64,
                "v@:q",
            ),
            me(
                "doubleValue",
                nscontrol::shim_nscontrol_double_value as *const () as u64,
                "d@:",
            ),
            me(
                "setDoubleValue:",
                nscontrol::shim_nscontrol_set_double_value as *const () as u64,
                "v@:d",
            ),
            me(
                "objectValue",
                nscontrol::shim_nscontrol_object_value as *const () as u64,
                "@@:",
            ),
            me(
                "setObjectValue:",
                nscontrol::shim_nscontrol_set_object_value as *const () as u64,
                "v@:@",
            ),
            me(
                "isContinuous",
                nscontrol::shim_nscontrol_is_continuous as *const () as u64,
                "B@:",
            ),
            me(
                "setContinuous:",
                nscontrol::shim_nscontrol_set_continuous as *const () as u64,
                "v@:B",
            ),
            me(
                "sendAction:to:",
                nscontrol::shim_nscontrol_send_action as *const () as u64,
                "B@::@",
            ),
            me(
                "sizeToFit",
                nscontrol::shim_nscontrol_size_to_fit as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 120,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsview_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSControl");
}

/// Register `NSTextField` — text label and input control.
pub fn register_nstextfield_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstextfield_class_addr();
    let metaclass_addr = nsobj::nstextfield_metaclass_addr();
    let info = ClassInfo {
        name: "NSTextField".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "stringValue",
                nstextfield::shim_nstextfield_string_value as *const () as u64,
                "@@:",
            ),
            me(
                "setStringValue:",
                nstextfield::shim_nstextfield_set_string_value as *const () as u64,
                "v@:@",
            ),
            me(
                "isEditable",
                nstextfield::shim_nstextfield_is_editable as *const () as u64,
                "B@:",
            ),
            me(
                "setEditable:",
                nstextfield::shim_nstextfield_set_editable as *const () as u64,
                "v@:B",
            ),
            me(
                "isBezeled",
                nstextfield::shim_nstextfield_is_bezeled as *const () as u64,
                "B@:",
            ),
            me(
                "setBezeled:",
                nstextfield::shim_nstextfield_set_bezeled as *const () as u64,
                "v@:B",
            ),
            me(
                "drawsBackground",
                nstextfield::shim_nstextfield_draws_background as *const () as u64,
                "B@:",
            ),
            me(
                "setDrawsBackground:",
                nstextfield::shim_nstextfield_set_draws_background as *const () as u64,
                "v@:B",
            ),
            me(
                "isSelectable",
                nstextfield::shim_nstextfield_is_selectable as *const () as u64,
                "B@:",
            ),
            me(
                "setSelectable:",
                nstextfield::shim_nstextfield_set_selectable as *const () as u64,
                "v@:B",
            ),
            me(
                "isEnabled",
                nstextfield::shim_nstextfield_is_enabled as *const () as u64,
                "B@:",
            ),
            me(
                "setEnabled:",
                nstextfield::shim_nstextfield_set_enabled as *const () as u64,
                "v@:B",
            ),
            me(
                "placeholderString",
                nstextfield::shim_nstextfield_placeholder_string as *const () as u64,
                "@@:",
            ),
            me(
                "setPlaceholderString:",
                nstextfield::shim_nstextfield_set_placeholder_string as *const () as u64,
                "v@:@",
            ),
            me(
                "target",
                nstextfield::shim_nstextfield_target as *const () as u64,
                "@@:",
            ),
            me(
                "setTarget:",
                nstextfield::shim_nstextfield_set_target as *const () as u64,
                "v@:@",
            ),
            me(
                "action",
                nstextfield::shim_nstextfield_action as *const () as u64,
                ":@:",
            ),
            me(
                "setAction:",
                nstextfield::shim_nstextfield_set_action as *const () as u64,
                "v@::",
            ),
            me(
                "drawRect:",
                nstextfield::shim_nstextfield_draw_rect as *const () as u64,
                "v@:{CGRect=dddd}",
            ),
            me(
                "acceptsFirstResponder",
                nstextfield::shim_nstextfield_accepts_first_responder as *const () as u64,
                "B@:",
            ),
            me(
                "keyDown:",
                nstextfield::shim_nstextfield_key_down as *const () as u64,
                "v@:@",
            ),
            me(
                "mouseDown:",
                nstextfield::shim_nstextfield_mouse_down as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![
            me(
                "labelWithString:",
                nstextfield::shim_nstextfield_label_with_string as *const () as u64,
                "@@:@",
            ),
            me(
                "wrappingLabelWithString:",
                nstextfield::shim_nstextfield_wrapping_label_with_string as *const () as u64,
                "@@:@",
            ),
            me(
                "textFieldWithString:",
                nstextfield::shim_nstextfield_text_field_with_string as *const () as u64,
                "@@:@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 120,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nscontrol_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTextField");
}
