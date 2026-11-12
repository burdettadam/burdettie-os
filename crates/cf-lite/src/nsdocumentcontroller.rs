//! NSDocumentController shim: tracks NSDocument instances for document apps.

use std::ffi::{c_char, CStr, CString};
use std::sync::{Mutex, OnceLock};

static SHARED_CONTROLLER: OnceLock<u64> = OnceLock::new();
static DOCUMENTS: OnceLock<Mutex<Vec<u64>>> = OnceLock::new();
static AUTOSAVING_DELAY_BITS: OnceLock<Mutex<u64>> = OnceLock::new();

fn documents() -> &'static Mutex<Vec<u64>> {
    DOCUMENTS.get_or_init(|| Mutex::new(Vec::new()))
}

fn autosaving_delay_bits() -> &'static Mutex<u64> {
    AUTOSAVING_DELAY_BITS.get_or_init(|| Mutex::new(0.0f64.to_bits()))
}

pub(crate) fn reset_runtime_state_for_reuse() {
    documents()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    *autosaving_delay_bits()
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = 0.0f64.to_bits();
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

fn nsstring_to_string(obj: u64) -> Option<String> {
    crate::cfstring::extract_cfstring_cstr(obj).map(ToOwned::to_owned)
}

fn fallback_selector_ptr(selector: &str) -> u64 {
    CString::new(selector)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut()) as *const c_char as u64
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

fn lookup_selector_imp(target: u64, selector: u64) -> Option<(u64, u64)> {
    if target == 0 || selector == 0 {
        return None;
    }
    let selector_name = unsafe { CStr::from_ptr(selector as *const c_char) }
        .to_str()
        .ok()?;
    lookup_instance_imp(target, selector_name)
}

fn class_addr_named(name: &str) -> u64 {
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    guard
        .as_ref()
        .and_then(|rt| rt.get_class(name).map(|class| class.mapped_addr))
        .unwrap_or(0)
}

fn runtime_class_is_subclass_of(class_addr: u64, target_addr: u64) -> bool {
    if class_addr == 0 || target_addr == 0 {
        return false;
    }
    if class_addr == target_addr {
        return true;
    }
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return false;
    };
    let mut current = Some(class_addr);
    let mut depth = 0;
    while let Some(addr) = current {
        if addr == target_addr {
            return true;
        }
        if depth > 64 {
            break;
        }
        current = rt
            .get_class_by_addr(addr)
            .and_then(|class| class.superclass_addr);
        depth += 1;
    }
    false
}

fn instance_size_for_class(class_addr: u64, minimum_size: usize) -> usize {
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    guard
        .as_ref()
        .and_then(|rt| {
            rt.get_class_by_addr(class_addr)
                .map(|class| class.instance_size as usize)
        })
        .unwrap_or(minimum_size)
        .max(minimum_size)
}

fn alloc_object_for_class(class_addr: u64, minimum_size: usize) -> u64 {
    if class_addr == 0 {
        return 0;
    }
    let size = instance_size_for_class(class_addr, minimum_size);
    let obj = crate::pal_alloc().calloc(1, size) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = class_addr;
        }
    }
    obj
}

fn allocate_controller(cls: u64) -> u64 {
    let class_addr = if cls != 0 {
        cls
    } else {
        crate::nsobj::nsdocumentcontroller_class_addr()
    };
    alloc_object_for_class(class_addr, 8)
}

fn allocate_document(class_addr: u64) -> u64 {
    let base = crate::nsobj::nsdocument_class_addr();
    let chosen = if runtime_class_is_subclass_of(class_addr, base) {
        class_addr
    } else {
        base
    };
    let doc = alloc_object_for_class(chosen, crate::nsdocument::DOCUMENT_SIZE);
    crate::nsdocument::initialize_document(doc);
    doc
}

fn dispatch_init_with_type_error(doc: u64, file_type: u64, error: *mut u64) -> u64 {
    let Some((imp, sel)) = lookup_instance_imp(doc, "initWithType:error:") else {
        return crate::nsdocument::shim_nsdocument_init_with_type_error(doc, 0, file_type, error);
    };
    let f: extern "C" fn(u64, u64, u64, *mut u64) -> u64 = unsafe { std::mem::transmute(imp) };
    f(doc, sel, file_type, error)
}

fn dispatch_init_with_contents_url(doc: u64, url: u64, file_type: u64, error: *mut u64) -> u64 {
    let Some((imp, sel)) = lookup_instance_imp(doc, "initWithContentsOfURL:ofType:error:") else {
        return crate::nsdocument::shim_nsdocument_init_with_contents_of_url_of_type_error(
            doc, 0, url, file_type, error,
        );
    };
    let f: extern "C" fn(u64, u64, u64, u64, *mut u64) -> u64 = unsafe { std::mem::transmute(imp) };
    f(doc, sel, url, file_type, error)
}

fn dispatch_delegate_bool_context(
    delegate: u64,
    selector: u64,
    controller: u64,
    flag: bool,
    context: u64,
) {
    let Some((imp, sel)) = lookup_selector_imp(delegate, selector) else {
        return;
    };
    let f: extern "C" fn(u64, u64, u64, u8, u64) = unsafe { std::mem::transmute(imp) };
    f(delegate, sel, controller, u8::from(flag), context);
}

fn invoke_open_completion(block: u64, document: u64, already_open: bool, error: u64) {
    if block == 0 {
        return;
    }
    let invoke = unsafe { *((block as usize + 16) as *const u64) };
    if invoke == 0 {
        return;
    }
    let f: extern "C" fn(u64, u64, u8, u64) = unsafe { std::mem::transmute(invoke) };
    f(block, document, u8::from(already_open), error);
}

fn object_isa(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *(obj as *const u64) }
}

fn object_is_array(obj: u64) -> bool {
    let isa = object_isa(obj);
    isa == crate::nsobj::nsarray_class_addr() || isa == crate::nsobj::nsmutablearray_class_addr()
}

fn object_is_dictionary(obj: u64) -> bool {
    let isa = object_isa(obj);
    isa == crate::nsobj::nsdictionary_class_addr()
        || isa == crate::nsobj::nsmutabledictionary_class_addr()
}

fn info_value(key: &str) -> u64 {
    let bundle = crate::nsbundle::get_or_create_main_bundle();
    if bundle == 0 {
        return 0;
    }
    let key = make_nsstring(key);
    crate::nsbundle::shim_nsbundle_object_for_info_key(bundle, 0, key)
}

fn dict_value(dict: u64, key: &str) -> u64 {
    if !object_is_dictionary(dict) {
        return 0;
    }
    let key = make_nsstring(key);
    crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key)
}

fn string_or_first_array_string(value: u64) -> u64 {
    if value == 0 {
        return 0;
    }
    if crate::cfstring::extract_cfstring_cstr(value).is_some() {
        return value;
    }
    if object_is_array(value) && crate::nsarray::shim_nsarray_count(value, 0) != 0 {
        return crate::nsarray::shim_nsarray_object_at_index(value, 0, 0);
    }
    0
}

fn string_value_matches(value: u64, needle: &str, case_insensitive: bool) -> bool {
    if needle.is_empty() || value == 0 {
        return false;
    }
    if object_is_array(value) {
        let count = crate::nsarray::shim_nsarray_count(value, 0);
        return (0..count).any(|index| {
            let item = crate::nsarray::shim_nsarray_object_at_index(value, 0, index);
            string_value_matches(item, needle, case_insensitive)
        });
    }
    let Some(candidate) = nsstring_to_string(value) else {
        return false;
    };
    if case_insensitive {
        candidate.eq_ignore_ascii_case(needle)
    } else {
        candidate == needle
    }
}

fn document_type_dicts() -> Vec<u64> {
    let value = info_value("CFBundleDocumentTypes");
    if value == 0 {
        return Vec::new();
    }
    if object_is_dictionary(value) {
        return vec![value];
    }
    if !object_is_array(value) {
        return Vec::new();
    }
    let count = crate::nsarray::shim_nsarray_count(value, 0);
    let mut result = Vec::with_capacity(count as usize);
    for index in 0..count {
        let item = crate::nsarray::shim_nsarray_object_at_index(value, 0, index);
        if object_is_dictionary(item) {
            result.push(item);
        }
    }
    result
}

fn type_name_from_dict(dict: u64) -> u64 {
    let type_name = dict_value(dict, "CFBundleTypeName");
    if type_name != 0 {
        return type_name;
    }
    let uti = string_or_first_array_string(dict_value(dict, "LSItemContentTypes"));
    if uti != 0 {
        return uti;
    }
    string_or_first_array_string(dict_value(dict, "CFBundleTypeExtensions"))
}

fn dict_matches_type_or_extension(
    dict: u64,
    file_type: Option<&str>,
    extension: Option<&str>,
) -> bool {
    if let Some(file_type) = file_type {
        if string_value_matches(dict_value(dict, "CFBundleTypeName"), file_type, false)
            || string_value_matches(dict_value(dict, "LSItemContentTypes"), file_type, false)
            || string_value_matches(dict_value(dict, "CFBundleTypeExtensions"), file_type, true)
        {
            return true;
        }
    }
    if let Some(extension) = extension {
        if string_value_matches(dict_value(dict, "CFBundleTypeExtensions"), extension, true) {
            return true;
        }
    }
    file_type.is_none() && extension.is_none()
}

fn document_class_name_from_dict(dict: u64) -> Option<String> {
    nsstring_to_string(dict_value(dict, "NSDocumentClass")).filter(|name| !name.is_empty())
}

fn document_type_name_for_url(url: u64) -> u64 {
    let extension_obj = if url != 0 {
        crate::nsurl::shim_nsurl_path_extension(url, 0)
    } else {
        0
    };
    let extension = nsstring_to_string(extension_obj);
    for dict in document_type_dicts() {
        if dict_matches_type_or_extension(dict, None, extension.as_deref()) {
            let name = type_name_from_dict(dict);
            if name != 0 {
                return name;
            }
        }
    }
    default_document_type()
}

fn default_document_type() -> u64 {
    for dict in document_type_dicts() {
        let name = type_name_from_dict(dict);
        if name != 0 {
            return name;
        }
    }
    make_nsstring("public.data")
}

fn document_class_name_for_type(file_type: u64) -> Option<String> {
    let type_string = nsstring_to_string(file_type);
    let mut fallback = None;
    for dict in document_type_dicts() {
        let class_name = document_class_name_from_dict(dict);
        if fallback.is_none() {
            fallback = class_name.clone();
        }
        if dict_matches_type_or_extension(dict, type_string.as_deref(), None) {
            return class_name.or(fallback);
        }
    }
    fallback
}

fn class_addr_for_document_type(file_type: u64) -> u64 {
    let base = crate::nsobj::nsdocument_class_addr();
    let Some(class_name) = document_class_name_for_type(file_type) else {
        return base;
    };
    let class_addr = class_addr_named(&class_name);
    if class_addr != 0 && runtime_class_is_subclass_of(class_addr, base) {
        class_addr
    } else {
        log::debug!(
            "NSDocumentController: document class '{}' unavailable or not an NSDocument subclass",
            class_name
        );
        base
    }
}

fn url_key(url: u64) -> Option<String> {
    if url == 0 {
        return None;
    }
    let absolute = crate::nsurl::shim_nsurl_absolute_string(url, 0);
    if absolute != 0 {
        return nsstring_to_string(absolute);
    }
    let path = crate::nsurl::shim_nsurl_path(url, 0);
    nsstring_to_string(path)
}

fn urls_match(lhs: u64, rhs: u64) -> bool {
    lhs != 0
        && rhs != 0
        && (lhs == rhs
            || url_key(lhs).is_some_and(|left| url_key(rhs).is_some_and(|right| left == right)))
}

fn snapshot_documents() -> Vec<u64> {
    documents()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn add_document(document: u64) {
    if document == 0 {
        return;
    }
    let mut docs = documents().lock().unwrap_or_else(|e| e.into_inner());
    if docs.contains(&document) {
        return;
    }
    crate::nsarray::retain_collection_object(document);
    docs.push(document);
}

fn remove_document(document: u64) {
    let mut docs = documents().lock().unwrap_or_else(|e| e.into_inner());
    let before = docs.len();
    docs.retain(|&candidate| candidate != document);
    if docs.len() != before {
        crate::nsarray::release_collection_object(document);
    }
}

pub(crate) fn remove_document_from_shared_controller(document: u64) {
    remove_document(document);
}

fn document_for_url(url: u64) -> u64 {
    for document in snapshot_documents() {
        let document_url = crate::nsdocument::shim_nsdocument_file_url(document, 0);
        if urls_match(document_url, url) {
            return document;
        }
    }
    0
}

fn make_document_for_url(url: u64, file_type: u64, error: *mut u64) -> u64 {
    let file_type = if file_type != 0 {
        file_type
    } else {
        document_type_name_for_url(url)
    };
    let class_addr = class_addr_for_document_type(file_type);
    let doc = allocate_document(class_addr);
    if doc == 0 {
        return 0;
    }
    let initialized = dispatch_init_with_contents_url(doc, url, file_type, error);
    if initialized == 0 {
        return 0;
    }
    initialized
}

fn make_document_for_type(file_type: u64, error: *mut u64) -> u64 {
    let file_type = if file_type != 0 {
        file_type
    } else {
        default_document_type()
    };
    let class_addr = class_addr_for_document_type(file_type);
    let doc = allocate_document(class_addr);
    if doc == 0 {
        return 0;
    }
    let initialized = dispatch_init_with_type_error(doc, file_type, error);
    if initialized == 0 {
        return 0;
    }
    crate::nsdocument::shim_nsdocument_set_file_type(initialized, 0, file_type);
    initialized
}

fn display_document(document: u64) {
    if document == 0 {
        return;
    }
    if !crate::nsdocument::dispatch_void(document, "makeWindowControllers") {
        crate::nsdocument::shim_nsdocument_make_window_controllers(document, 0);
    }
    if !crate::nsdocument::dispatch_void(document, "showWindows") {
        crate::nsdocument::shim_nsdocument_show_windows(document, 0);
    }
}

fn append_string_objects(value: u64, out: &mut Vec<u64>) {
    if value == 0 {
        return;
    }
    if crate::cfstring::extract_cfstring_cstr(value).is_some() {
        out.push(value);
        return;
    }
    if !object_is_array(value) {
        return;
    }
    let count = crate::nsarray::shim_nsarray_count(value, 0);
    for index in 0..count {
        let item = crate::nsarray::shim_nsarray_object_at_index(value, 0, index);
        if crate::cfstring::extract_cfstring_cstr(item).is_some() {
            out.push(item);
        }
    }
}

fn document_open_panel_types() -> u64 {
    let mut types = Vec::new();
    for dict in document_type_dicts() {
        append_string_objects(dict_value(dict, "CFBundleTypeExtensions"), &mut types);
        append_string_objects(dict_value(dict, "LSItemContentTypes"), &mut types);
    }
    if types.is_empty() {
        0
    } else {
        crate::nsarray::make_nsarray(&types)
    }
}

fn open_documents_from_panel_selection(controller: u64, panel: u64) -> usize {
    if panel == 0 {
        return 0;
    }
    let urls = crate::nsopenpanel::shim_nsopenpanel_urls(panel, 0);
    let count = crate::nsarray::shim_nsarray_count(urls, 0);
    let mut opened = 0;
    for index in 0..count {
        let url = crate::nsarray::shim_nsarray_object_at_index(urls, 0, index);
        let mut error = 0_u64;
        let document = shim_nsdocumentcontroller_open_document_with_contents_of_url_display_error(
            controller, 0, url, 1, &mut error,
        );
        if document != 0 {
            opened += 1;
        }
    }
    opened
}

/// `+[NSDocumentController sharedDocumentController]`.
pub extern "C" fn shim_nsdocumentcontroller_shared_document_controller(cls: u64, _cmd: u64) -> u64 {
    *SHARED_CONTROLLER.get_or_init(|| allocate_controller(cls))
}

/// `-[NSDocumentController documents]`.
pub extern "C" fn shim_nsdocumentcontroller_documents(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&snapshot_documents())
}

/// `-[NSDocumentController currentDocument]`.
pub extern "C" fn shim_nsdocumentcontroller_current_document(_obj: u64, _cmd: u64) -> u64 {
    snapshot_documents().last().copied().unwrap_or(0)
}

/// `-[NSDocumentController addDocument:]`.
pub extern "C" fn shim_nsdocumentcontroller_add_document(_obj: u64, _cmd: u64, document: u64) {
    add_document(document);
}

/// `-[NSDocumentController removeDocument:]`.
pub extern "C" fn shim_nsdocumentcontroller_remove_document(_obj: u64, _cmd: u64, document: u64) {
    remove_document(document);
}

/// `-[NSDocumentController documentForURL:]`.
pub extern "C" fn shim_nsdocumentcontroller_document_for_url(
    _obj: u64,
    _cmd: u64,
    url: u64,
) -> u64 {
    document_for_url(url)
}

/// `-[NSDocumentController documentForWindow:]`.
pub extern "C" fn shim_nsdocumentcontroller_document_for_window(
    _obj: u64,
    _cmd: u64,
    window: u64,
) -> u64 {
    if window == 0 {
        return 0;
    }
    for document in snapshot_documents() {
        let controllers = crate::nsdocument::shim_nsdocument_window_controllers(document, 0);
        let count = crate::nsarray::shim_nsarray_count(controllers, 0);
        for index in 0..count {
            let controller = crate::nsarray::shim_nsarray_object_at_index(controllers, 0, index);
            if crate::nswindowcontroller::shim_nswindowcontroller_window(controller, 0) == window {
                return document;
            }
        }
    }
    0
}

/// `-[NSDocumentController documentClassForType:]`.
pub extern "C" fn shim_nsdocumentcontroller_document_class_for_type(
    _obj: u64,
    _cmd: u64,
    file_type: u64,
) -> u64 {
    class_addr_for_document_type(file_type)
}

/// `-[NSDocumentController typeForContentsOfURL:error:]`.
pub extern "C" fn shim_nsdocumentcontroller_type_for_contents_of_url_error(
    _obj: u64,
    _cmd: u64,
    url: u64,
    error: *mut u64,
) -> u64 {
    clear_error(error);
    document_type_name_for_url(url)
}

/// `-[NSDocumentController defaultType]`.
pub extern "C" fn shim_nsdocumentcontroller_default_type(_obj: u64, _cmd: u64) -> u64 {
    default_document_type()
}

/// `-[NSDocumentController makeUntitledDocumentOfType:error:]`.
pub extern "C" fn shim_nsdocumentcontroller_make_untitled_document_of_type_error(
    _obj: u64,
    _cmd: u64,
    file_type: u64,
    error: *mut u64,
) -> u64 {
    clear_error(error);
    make_document_for_type(file_type, error)
}

/// `-[NSDocumentController openUntitledDocumentAndDisplay:error:]`.
pub extern "C" fn shim_nsdocumentcontroller_open_untitled_document_and_display_error(
    obj: u64,
    _cmd: u64,
    display: u8,
    error: *mut u64,
) -> u64 {
    let file_type = shim_nsdocumentcontroller_default_type(obj, 0);
    let document = make_document_for_type(file_type, error);
    if document == 0 {
        return 0;
    }
    add_document(document);
    if display != 0 {
        display_document(document);
    }
    document
}

/// `-[NSDocumentController makeDocumentWithContentsOfURL:ofType:error:]`.
pub extern "C" fn shim_nsdocumentcontroller_make_document_with_contents_of_url_of_type_error(
    _obj: u64,
    _cmd: u64,
    url: u64,
    file_type: u64,
    error: *mut u64,
) -> u64 {
    clear_error(error);
    make_document_for_url(url, file_type, error)
}

/// `-[NSDocumentController openDocumentWithContentsOfURL:display:error:]`.
pub extern "C" fn shim_nsdocumentcontroller_open_document_with_contents_of_url_display_error(
    _obj: u64,
    _cmd: u64,
    url: u64,
    display: u8,
    error: *mut u64,
) -> u64 {
    clear_error(error);
    if url == 0 {
        return 0;
    }
    let existing = document_for_url(url);
    if existing != 0 {
        if display != 0 {
            display_document(existing);
        }
        return existing;
    }
    let document = make_document_for_url(url, 0, error);
    if document == 0 {
        return 0;
    }
    add_document(document);
    if display != 0 {
        display_document(document);
    }
    document
}

/// `-[NSDocumentController openDocumentWithContentsOfURL:display:completionHandler:]`.
pub extern "C" fn shim_nsdocumentcontroller_open_document_with_contents_of_url_display_completion(
    obj: u64,
    _cmd: u64,
    url: u64,
    display: u8,
    completion: u64,
) {
    let already = document_for_url(url) != 0;
    let mut error = 0_u64;
    let document = shim_nsdocumentcontroller_open_document_with_contents_of_url_display_error(
        obj, 0, url, display, &mut error,
    );
    invoke_open_completion(completion, document, already, error);
}

/// `-[NSDocumentController newDocument:]`.
pub extern "C" fn shim_nsdocumentcontroller_new_document(obj: u64, _cmd: u64, _sender: u64) {
    let mut error = 0_u64;
    let _ =
        shim_nsdocumentcontroller_open_untitled_document_and_display_error(obj, 0, 1, &mut error);
}

/// `-[NSDocumentController openDocument:]`.
pub extern "C" fn shim_nsdocumentcontroller_open_document(obj: u64, _cmd: u64, _sender: u64) {
    let panel =
        crate::nsopenpanel::shim_nsopenpanel_open_panel(crate::nsobj::nsopenpanel_class_addr(), 0);
    if panel == 0 {
        return;
    }
    let types = document_open_panel_types();
    let response = shim_nsdocumentcontroller_run_modal_open_panel_for_types(obj, 0, panel, types);
    if response == crate::nsopenpanel::NS_MODAL_RESPONSE_OK as i64 {
        open_documents_from_panel_selection(obj, panel);
    }
}

/// `-[NSDocumentController saveAllDocuments:]`.
pub extern "C" fn shim_nsdocumentcontroller_save_all_documents(_obj: u64, _cmd: u64, _sender: u64) {
    for document in snapshot_documents() {
        crate::nsdocument::shim_nsdocument_save_document(document, 0, 0);
    }
}

/// `-[NSDocumentController closeAllDocumentsWithDelegate:didCloseAllSelector:contextInfo:]`.
pub extern "C" fn shim_nsdocumentcontroller_close_all_documents_with_delegate(
    obj: u64,
    _cmd: u64,
    delegate: u64,
    selector: u64,
    context: u64,
) {
    for document in snapshot_documents() {
        crate::nsdocument::shim_nsdocument_close(document, 0);
    }
    dispatch_delegate_bool_context(delegate, selector, obj, true, context);
}

/// `-[NSDocumentController clearRecentDocuments:]`.
pub extern "C" fn shim_nsdocumentcontroller_clear_recent_documents(
    _obj: u64,
    _cmd: u64,
    _sender: u64,
) {
}

/// `-[NSDocumentController noteNewRecentDocument:]`.
pub extern "C" fn shim_nsdocumentcontroller_note_new_recent_document(
    _obj: u64,
    _cmd: u64,
    _document: u64,
) {
}

/// `-[NSDocumentController noteNewRecentDocumentURL:]`.
pub extern "C" fn shim_nsdocumentcontroller_note_new_recent_document_url(
    _obj: u64,
    _cmd: u64,
    _url: u64,
) {
}

/// `-[NSDocumentController recentDocumentURLs]`.
pub extern "C" fn shim_nsdocumentcontroller_recent_document_urls(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

/// `-[NSDocumentController currentDirectory]`.
pub extern "C" fn shim_nsdocumentcontroller_current_directory(_obj: u64, _cmd: u64) -> u64 {
    let bundle = crate::nsbundle::get_or_create_main_bundle();
    crate::nsbundle::shim_nsbundle_bundle_path(bundle, 0)
}

/// `-[NSDocumentController setAutosavingDelay:]`.
pub extern "C" fn shim_nsdocumentcontroller_set_autosaving_delay(_obj: u64, _cmd: u64, delay: f64) {
    *autosaving_delay_bits()
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = delay.to_bits();
}

/// `-[NSDocumentController autosavingDelay]`.
pub extern "C" fn shim_nsdocumentcontroller_autosaving_delay(_obj: u64, _cmd: u64) -> f64 {
    f64::from_bits(
        *autosaving_delay_bits()
            .lock()
            .unwrap_or_else(|e| e.into_inner()),
    )
}

/// `-[NSDocumentController runModalOpenPanel:forTypes:]`.
pub extern "C" fn shim_nsdocumentcontroller_run_modal_open_panel_for_types(
    _obj: u64,
    _cmd: u64,
    panel: u64,
    types: u64,
) -> i64 {
    if panel == 0 {
        return crate::nsopenpanel::NS_MODAL_RESPONSE_CANCEL as i64;
    }
    if types != 0 {
        crate::nsopenpanel::shim_nssavepanel_set_allowed_content_types(panel, 0, types);
    }
    crate::nsopenpanel::shim_nsopenpanel_run_modal(panel, 0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn shared_controller_is_singleton() {
        let a = shim_nsdocumentcontroller_shared_document_controller(
            crate::nsobj::nsdocumentcontroller_class_addr(),
            0,
        );
        let b = shim_nsdocumentcontroller_shared_document_controller(
            crate::nsobj::nsdocumentcontroller_class_addr(),
            0,
        );
        assert_ne!(a, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn add_and_remove_document_tracks_documents() {
        reset_runtime_state_for_reuse();
        let doc = alloc_obj(
            crate::nsdocument::DOCUMENT_SIZE,
            crate::nsobj::nsdocument_class_addr(),
        );
        crate::nsdocument::shim_nsdocument_init(doc, 0);

        shim_nsdocumentcontroller_add_document(0, 0, doc);
        let docs = shim_nsdocumentcontroller_documents(0, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(docs, 0), 1);
        assert_eq!(shim_nsdocumentcontroller_current_document(0, 0), doc);

        shim_nsdocumentcontroller_remove_document(0, 0, doc);
        let docs = shim_nsdocumentcontroller_documents(0, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(docs, 0), 0);
        free_obj(doc);
    }

    #[test]
    fn default_type_is_available_without_info_plist() {
        let default_type = shim_nsdocumentcontroller_default_type(0, 0);
        assert_ne!(default_type, 0);
    }

    #[test]
    fn panel_selection_opens_selected_urls_as_documents() {
        reset_runtime_state_for_reuse();
        let panel = crate::nsopenpanel::shim_nsopenpanel_open_panel(0, 0);
        let path = make_string("/tmp/document-controller-panel.txt");
        let url =
            crate::nsurl::shim_nsurl_file_url_with_path(crate::nsobj::nsurl_class_addr(), 0, path);
        crate::nsopenpanel::store_urls_for_tests(panel, vec![url]);

        assert_eq!(open_documents_from_panel_selection(0, panel), 1);
        let docs = shim_nsdocumentcontroller_documents(0, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(docs, 0), 1);
        assert_eq!(
            shim_nsdocumentcontroller_document_for_url(0, 0, url),
            shim_nsdocumentcontroller_current_document(0, 0)
        );

        reset_runtime_state_for_reuse();
        free_obj(panel);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn run_modal_open_panel_applies_types_and_uses_cancel_fallback() {
        let panel = crate::nsopenpanel::shim_nsopenpanel_open_panel(0, 0);
        let text = make_string("txt");
        let data = make_string("public.data");
        let types = crate::nsarray::make_nsarray(&[text, data]);

        let response = shim_nsdocumentcontroller_run_modal_open_panel_for_types(0, 0, panel, types);

        assert_eq!(
            response,
            crate::nsopenpanel::NS_MODAL_RESPONSE_CANCEL as i64
        );
        assert_eq!(
            crate::nsopenpanel::allowed_types_for_tests(panel),
            vec![String::from("txt"), String::from("public.data")]
        );
        free_obj(panel);
    }
}
