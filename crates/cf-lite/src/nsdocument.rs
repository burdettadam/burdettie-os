//! NSDocument shim: document lifecycle support for Cocoa document apps.
//!
//! Object layout:
//!
//!   offset  0: isa                    (u64)
//!   offset  8: file_url               (NSURL*, or 0)
//!   offset 16: file_type              (NSString*, or 0)
//!   offset 24: display_name           (NSString*, or 0)
//!   offset 32: window_controllers     (NSMutableArray*, or 0)
//!   offset 40: undo_manager           (NSUndoManager*, or 0)
//!   offset 48: autosaved_file_url     (NSURL*, or 0)
//!   offset 56: flags                  (u64)
//!
//! Total: 64 bytes.

use std::ffi::{c_char, CStr, CString};

const FILE_URL_OFF: usize = 8;
const FILE_TYPE_OFF: usize = 16;
const DISPLAY_NAME_OFF: usize = 24;
const WINDOW_CONTROLLERS_OFF: usize = 32;
const UNDO_MANAGER_OFF: usize = 40;
const AUTOSAVED_FILE_URL_OFF: usize = 48;
const FLAGS_OFF: usize = 56;

pub(crate) const DOCUMENT_SIZE: usize = 64;

const FLAG_EDITED: u64 = 1 << 0;
const FLAG_HAS_UNDO_MANAGER: u64 = 1 << 1;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

fn set_flag(obj: u64, flag: u64, enabled: bool) {
    let mut flags = read_u64(obj, FLAGS_OFF);
    if enabled {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

fn flag_is_set(obj: u64, flag: u64) -> bool {
    read_u64(obj, FLAGS_OFF) & flag != 0
}

fn clear_error(error: *mut u64) {
    if !error.is_null() {
        unsafe {
            *error = 0;
        }
    }
}

fn make_nsstring(value: &str) -> u64 {
    let cstr = CString::new(value).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

fn fallback_selector_ptr(selector: &str) -> u64 {
    CString::new(selector)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut()) as *const c_char as u64
}

fn selector_name(sel: u64) -> Option<&'static str> {
    if sel == 0 {
        return None;
    }
    unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .ok()
        .map(str::to_owned)
        .map(String::into_boxed_str)
        .map(Box::leak)
        .map(|s| &*s)
}

fn lookup_instance_imp(obj: u64, selector: &str) -> Option<(u64, u64)> {
    if obj == 0 || selector.is_empty() {
        return None;
    }
    let isa = unsafe { *(obj as *const u64) };
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let rt = guard.as_ref()?;
    let imp = rt.msg_lookup(isa, selector)?;
    let sel = rt
        .selector_ptr(selector)
        .map(|ptr| ptr as u64)
        .unwrap_or_else(|| fallback_selector_ptr(selector));
    Some((imp, sel))
}

pub(crate) fn dispatch_void(obj: u64, selector: &str) -> bool {
    let Some((imp, sel)) = lookup_instance_imp(obj, selector) else {
        return false;
    };
    let f: extern "C" fn(u64, u64) = unsafe { std::mem::transmute(imp) };
    f(obj, sel);
    true
}

fn dispatch_delegate_bool_context(
    delegate: u64,
    selector: u64,
    document: u64,
    flag: bool,
    context: u64,
) {
    let Some(selector) = selector_name(selector) else {
        return;
    };
    let Some((imp, sel)) = lookup_instance_imp(delegate, selector) else {
        return;
    };
    let f: extern "C" fn(u64, u64, u64, u8, u64) = unsafe { std::mem::transmute(imp) };
    f(delegate, sel, document, u8::from(flag), context);
}

fn ensure_window_controllers_array(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let existing = read_u64(obj, WINDOW_CONTROLLERS_OFF);
    if existing != 0 {
        return existing;
    }
    let array = crate::nsmutablearray::shim_nsmutablearray_array_with_capacity(
        crate::nsobj::nsmutablearray_class_addr(),
        0,
        2,
    );
    write_u64(obj, WINDOW_CONTROLLERS_OFF, array);
    array
}

pub(crate) fn initialize_document(obj: u64) {
    if obj == 0 {
        return;
    }
    ensure_window_controllers_array(obj);
    set_flag(obj, FLAG_HAS_UNDO_MANAGER, true);
    set_flag(obj, FLAG_EDITED, false);
}

fn set_file_name(obj: u64, path: u64) {
    if obj == 0 || path == 0 {
        return;
    }
    let url =
        crate::nsurl::shim_nsurl_file_url_with_path(crate::nsobj::nsurl_class_addr(), 0, path);
    write_u64(obj, FILE_URL_OFF, url);
}

fn first_window_controller(obj: u64) -> u64 {
    let controllers = ensure_window_controllers_array(obj);
    if controllers == 0 || crate::nsarray::shim_nsarray_count(controllers, 0) == 0 {
        return 0;
    }
    crate::nsarray::shim_nsarray_object_at_index(controllers, 0, 0)
}

/// `-[NSDocument init]`.
pub extern "C" fn shim_nsdocument_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    initialize_document(obj);
    obj
}

/// `-[NSDocument initWithType:error:]`.
pub extern "C" fn shim_nsdocument_init_with_type_error(
    obj: u64,
    _cmd: u64,
    file_type: u64,
    error: *mut u64,
) -> u64 {
    clear_error(error);
    let obj = shim_nsdocument_init(obj, 0);
    if obj != 0 {
        write_u64(obj, FILE_TYPE_OFF, file_type);
    }
    obj
}

/// `-[NSDocument initWithContentsOfURL:ofType:error:]`.
pub extern "C" fn shim_nsdocument_init_with_contents_of_url_of_type_error(
    obj: u64,
    _cmd: u64,
    url: u64,
    file_type: u64,
    error: *mut u64,
) -> u64 {
    clear_error(error);
    let obj = shim_nsdocument_init(obj, 0);
    if obj == 0 {
        return 0;
    }
    write_u64(obj, FILE_URL_OFF, url);
    write_u64(obj, FILE_TYPE_OFF, file_type);
    if url != 0 && shim_nsdocument_read_from_url_of_type_error(obj, 0, url, file_type, error) == 0 {
        return 0;
    }
    obj
}

/// `-[NSDocument initForURL:withContentsOfURL:ofType:error:]`.
pub extern "C" fn shim_nsdocument_init_for_url_with_contents_of_url_of_type_error(
    obj: u64,
    _cmd: u64,
    file_url: u64,
    contents_url: u64,
    file_type: u64,
    error: *mut u64,
) -> u64 {
    let obj = shim_nsdocument_init_with_contents_of_url_of_type_error(
        obj,
        0,
        contents_url,
        file_type,
        error,
    );
    if obj != 0 {
        write_u64(
            obj,
            FILE_URL_OFF,
            if file_url != 0 {
                file_url
            } else {
                contents_url
            },
        );
    }
    obj
}

/// `-[NSDocument fileURL]`.
pub extern "C" fn shim_nsdocument_file_url(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, FILE_URL_OFF)
}

/// `-[NSDocument setFileURL:]`.
pub extern "C" fn shim_nsdocument_set_file_url(obj: u64, _cmd: u64, url: u64) {
    write_u64(obj, FILE_URL_OFF, url);
}

/// `-[NSDocument fileName]` legacy path accessor.
pub extern "C" fn shim_nsdocument_file_name(obj: u64, _cmd: u64) -> u64 {
    let url = read_u64(obj, FILE_URL_OFF);
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_path(url, 0)
}

/// `-[NSDocument setFileName:]` legacy path setter.
pub extern "C" fn shim_nsdocument_set_file_name(obj: u64, _cmd: u64, path: u64) {
    set_file_name(obj, path);
}

/// `-[NSDocument fileType]`.
pub extern "C" fn shim_nsdocument_file_type(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, FILE_TYPE_OFF)
}

/// `-[NSDocument setFileType:]`.
pub extern "C" fn shim_nsdocument_set_file_type(obj: u64, _cmd: u64, file_type: u64) {
    write_u64(obj, FILE_TYPE_OFF, file_type);
}

/// `-[NSDocument displayName]`.
pub extern "C" fn shim_nsdocument_display_name(obj: u64, _cmd: u64) -> u64 {
    let explicit = read_u64(obj, DISPLAY_NAME_OFF);
    if explicit != 0 {
        return explicit;
    }
    let url = read_u64(obj, FILE_URL_OFF);
    if url != 0 {
        let name = crate::nsurl::shim_nsurl_last_path_component(url, 0);
        if name != 0 {
            return name;
        }
    }
    make_nsstring("Untitled")
}

/// Non-public but common enough setter used by some document subclasses.
pub extern "C" fn shim_nsdocument_set_display_name(obj: u64, _cmd: u64, name: u64) {
    write_u64(obj, DISPLAY_NAME_OFF, name);
}

/// `-[NSDocument windowControllers]`.
pub extern "C" fn shim_nsdocument_window_controllers(obj: u64, _cmd: u64) -> u64 {
    ensure_window_controllers_array(obj)
}

/// `-[NSDocument addWindowController:]`.
pub extern "C" fn shim_nsdocument_add_window_controller(obj: u64, _cmd: u64, controller: u64) {
    if obj == 0 || controller == 0 {
        return;
    }
    let controllers = ensure_window_controllers_array(obj);
    if controllers == 0
        || crate::nsarray::shim_nsarray_contains_object(controllers, 0, controller) != 0
    {
        return;
    }
    crate::nsmutablearray::shim_nsmutablearray_add_object(controllers, 0, controller);
    crate::nswindowcontroller::shim_nswindowcontroller_set_document(controller, 0, obj);
}

/// `-[NSDocument removeWindowController:]`.
pub extern "C" fn shim_nsdocument_remove_window_controller(obj: u64, _cmd: u64, controller: u64) {
    if obj == 0 || controller == 0 {
        return;
    }
    let controllers = ensure_window_controllers_array(obj);
    crate::nsmutablearray::ops::shim_nsmutablearray_remove_object(controllers, 0, controller);
    crate::nswindowcontroller::shim_nswindowcontroller_set_document(controller, 0, 0);
}

/// `-[NSDocument makeWindowControllers]` default implementation.
pub extern "C" fn shim_nsdocument_make_window_controllers(_obj: u64, _cmd: u64) {}

/// `-[NSDocument showWindows]`.
pub extern "C" fn shim_nsdocument_show_windows(obj: u64, _cmd: u64) {
    let controllers = ensure_window_controllers_array(obj);
    let count = crate::nsarray::shim_nsarray_count(controllers, 0);
    for index in 0..count {
        let controller = crate::nsarray::shim_nsarray_object_at_index(controllers, 0, index);
        crate::nswindowcontroller::shim_nswindowcontroller_show_window(controller, 0, obj);
    }
}

/// `-[NSDocument windowForSheet]`.
pub extern "C" fn shim_nsdocument_window_for_sheet(obj: u64, _cmd: u64) -> u64 {
    let controller = first_window_controller(obj);
    if controller == 0 {
        return 0;
    }
    crate::nswindowcontroller::shim_nswindowcontroller_window(controller, 0)
}

/// `-[NSDocument readFromURL:ofType:error:]`.
pub extern "C" fn shim_nsdocument_read_from_url_of_type_error(
    obj: u64,
    _cmd: u64,
    url: u64,
    file_type: u64,
    error: *mut u64,
) -> u8 {
    clear_error(error);
    if obj == 0 {
        return 0;
    }
    write_u64(obj, FILE_URL_OFF, url);
    write_u64(obj, FILE_TYPE_OFF, file_type);
    1
}

/// `-[NSDocument readFromFile:ofType:]`.
pub extern "C" fn shim_nsdocument_read_from_file_of_type(
    obj: u64,
    _cmd: u64,
    path: u64,
    file_type: u64,
) -> u8 {
    if obj == 0 {
        return 0;
    }
    set_file_name(obj, path);
    write_u64(obj, FILE_TYPE_OFF, file_type);
    1
}

/// `-[NSDocument readFromData:ofType:error:]`.
pub extern "C" fn shim_nsdocument_read_from_data_of_type_error(
    obj: u64,
    _cmd: u64,
    _data: u64,
    file_type: u64,
    error: *mut u64,
) -> u8 {
    clear_error(error);
    if obj == 0 {
        return 0;
    }
    write_u64(obj, FILE_TYPE_OFF, file_type);
    1
}

/// `-[NSDocument writeToURL:ofType:error:]`.
pub extern "C" fn shim_nsdocument_write_to_url_of_type_error(
    obj: u64,
    _cmd: u64,
    url: u64,
    file_type: u64,
    error: *mut u64,
) -> u8 {
    clear_error(error);
    if obj == 0 {
        return 0;
    }
    write_u64(obj, FILE_URL_OFF, url);
    write_u64(obj, FILE_TYPE_OFF, file_type);
    set_flag(obj, FLAG_EDITED, false);
    1
}

/// `-[NSDocument writeToFile:ofType:]`.
pub extern "C" fn shim_nsdocument_write_to_file_of_type(
    obj: u64,
    _cmd: u64,
    path: u64,
    file_type: u64,
) -> u8 {
    if obj == 0 {
        return 0;
    }
    set_file_name(obj, path);
    write_u64(obj, FILE_TYPE_OFF, file_type);
    set_flag(obj, FLAG_EDITED, false);
    1
}

/// `-[NSDocument dataOfType:error:]`.
pub extern "C" fn shim_nsdocument_data_of_type_error(
    _obj: u64,
    _cmd: u64,
    _file_type: u64,
    error: *mut u64,
) -> u64 {
    clear_error(error);
    0
}

/// `-[NSDocument saveToURL:ofType:forSaveOperation:error:]`.
pub extern "C" fn shim_nsdocument_save_to_url_of_type_for_save_operation_error(
    obj: u64,
    _cmd: u64,
    url: u64,
    file_type: u64,
    _operation: u64,
    error: *mut u64,
) -> u8 {
    shim_nsdocument_write_to_url_of_type_error(obj, 0, url, file_type, error)
}

/// `-[NSDocument saveDocument:]`.
pub extern "C" fn shim_nsdocument_save_document(obj: u64, _cmd: u64, _sender: u64) {
    set_flag(obj, FLAG_EDITED, false);
}

/// `-[NSDocument saveDocumentAs:]`.
pub extern "C" fn shim_nsdocument_save_document_as(obj: u64, _cmd: u64, _sender: u64) {
    set_flag(obj, FLAG_EDITED, false);
}

/// `-[NSDocument saveDocumentTo:]`.
pub extern "C" fn shim_nsdocument_save_document_to(obj: u64, _cmd: u64, _sender: u64) {
    set_flag(obj, FLAG_EDITED, false);
}

/// `-[NSDocument saveDocumentWithDelegate:didSaveSelector:contextInfo:]`.
pub extern "C" fn shim_nsdocument_save_document_with_delegate(
    obj: u64,
    _cmd: u64,
    delegate: u64,
    selector: u64,
    context: u64,
) {
    set_flag(obj, FLAG_EDITED, false);
    dispatch_delegate_bool_context(delegate, selector, obj, true, context);
}

/// `-[NSDocument revertDocumentToSaved:]`.
pub extern "C" fn shim_nsdocument_revert_document_to_saved(obj: u64, _cmd: u64, _sender: u64) {
    set_flag(obj, FLAG_EDITED, false);
}

/// `-[NSDocument close]`.
pub extern "C" fn shim_nsdocument_close(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let controllers = ensure_window_controllers_array(obj);
    let count = crate::nsarray::shim_nsarray_count(controllers, 0);
    for index in 0..count {
        let controller = crate::nsarray::shim_nsarray_object_at_index(controllers, 0, index);
        crate::nswindowcontroller::shim_nswindowcontroller_close(controller, 0);
    }
    crate::nsdocumentcontroller::remove_document_from_shared_controller(obj);
}

/// `-[NSDocument canCloseDocumentWithDelegate:shouldCloseSelector:contextInfo:]`.
pub extern "C" fn shim_nsdocument_can_close_document_with_delegate(
    obj: u64,
    _cmd: u64,
    delegate: u64,
    selector: u64,
    context: u64,
) {
    dispatch_delegate_bool_context(delegate, selector, obj, true, context);
}

/// `-[NSDocument shouldCloseWindowController:delegate:shouldCloseSelector:contextInfo:]`.
pub extern "C" fn shim_nsdocument_should_close_window_controller(
    obj: u64,
    _cmd: u64,
    _controller: u64,
    delegate: u64,
    selector: u64,
    context: u64,
) {
    dispatch_delegate_bool_context(delegate, selector, obj, true, context);
}

/// `-[NSDocument isDocumentEdited]`.
pub extern "C" fn shim_nsdocument_is_document_edited(obj: u64, _cmd: u64) -> u8 {
    u8::from(flag_is_set(obj, FLAG_EDITED))
}

/// `-[NSDocument setDocumentEdited:]`.
pub extern "C" fn shim_nsdocument_set_document_edited(obj: u64, _cmd: u64, edited: u8) {
    set_flag(obj, FLAG_EDITED, edited != 0);
}

/// `-[NSDocument hasUnautosavedChanges]`.
pub extern "C" fn shim_nsdocument_has_unautosaved_changes(obj: u64, _cmd: u64) -> u8 {
    shim_nsdocument_is_document_edited(obj, _cmd)
}

/// `-[NSDocument updateChangeCount:]`.
pub extern "C" fn shim_nsdocument_update_change_count(obj: u64, _cmd: u64, change: u64) {
    match change {
        2 | 4 => set_flag(obj, FLAG_EDITED, false),
        _ => set_flag(obj, FLAG_EDITED, true),
    }
}

/// `-[NSDocument changeCount]`.
pub extern "C" fn shim_nsdocument_change_count(obj: u64, _cmd: u64) -> u64 {
    u64::from(flag_is_set(obj, FLAG_EDITED))
}

/// `-[NSDocument hasUndoManager]`.
pub extern "C" fn shim_nsdocument_has_undo_manager(obj: u64, _cmd: u64) -> u8 {
    u8::from(flag_is_set(obj, FLAG_HAS_UNDO_MANAGER))
}

/// `-[NSDocument setHasUndoManager:]`.
pub extern "C" fn shim_nsdocument_set_has_undo_manager(obj: u64, _cmd: u64, enabled: u8) {
    set_flag(obj, FLAG_HAS_UNDO_MANAGER, enabled != 0);
}

/// `-[NSDocument undoManager]`.
pub extern "C" fn shim_nsdocument_undo_manager(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 || !flag_is_set(obj, FLAG_HAS_UNDO_MANAGER) {
        return 0;
    }
    let existing = read_u64(obj, UNDO_MANAGER_OFF);
    if existing != 0 {
        return existing;
    }
    let manager = crate::nsundomanager::alloc_undo_manager();
    write_u64(obj, UNDO_MANAGER_OFF, manager);
    manager
}

/// `-[NSDocument autosavedContentsFileURL]`.
pub extern "C" fn shim_nsdocument_autosaved_contents_file_url(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, AUTOSAVED_FILE_URL_OFF)
}

/// `-[NSDocument setAutosavedContentsFileURL:]`.
pub extern "C" fn shim_nsdocument_set_autosaved_contents_file_url(obj: u64, _cmd: u64, url: u64) {
    write_u64(obj, AUTOSAVED_FILE_URL_OFF, url);
}

/// `-[NSDocument windowNibName]` default implementation.
pub extern "C" fn shim_nsdocument_window_nib_name(_obj: u64, _cmd: u64) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn init_sets_defaults() {
        let doc = alloc_obj(DOCUMENT_SIZE, crate::nsobj::nsdocument_class_addr());
        assert_eq!(shim_nsdocument_init(doc, 0), doc);
        assert_ne!(shim_nsdocument_window_controllers(doc, 0), 0);
        assert_eq!(shim_nsdocument_has_undo_manager(doc, 0), 1);
        assert_eq!(shim_nsdocument_is_document_edited(doc, 0), 0);
        free_obj(doc);
    }

    #[test]
    fn file_url_and_type_round_trip() {
        let doc = alloc_obj(DOCUMENT_SIZE, crate::nsobj::nsdocument_class_addr());
        shim_nsdocument_init(doc, 0);
        shim_nsdocument_set_file_url(doc, 0, 0xCAFE);
        shim_nsdocument_set_file_type(doc, 0, 0xBEEF);
        assert_eq!(shim_nsdocument_file_url(doc, 0), 0xCAFE);
        assert_eq!(shim_nsdocument_file_type(doc, 0), 0xBEEF);
        free_obj(doc);
    }

    #[test]
    fn edited_flag_tracks_changes() {
        let doc = alloc_obj(DOCUMENT_SIZE, crate::nsobj::nsdocument_class_addr());
        shim_nsdocument_init(doc, 0);
        shim_nsdocument_set_document_edited(doc, 0, 1);
        assert_eq!(shim_nsdocument_is_document_edited(doc, 0), 1);
        shim_nsdocument_save_document(doc, 0, 0);
        assert_eq!(shim_nsdocument_is_document_edited(doc, 0), 0);
        free_obj(doc);
    }
}
