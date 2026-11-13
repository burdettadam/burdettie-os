//! NSStoryboard shim for compiled AppKit storyboards.
//!
//! Object layout (24 bytes):
//!   offset 0:  ISA    (u64)
//!   offset 8:  name   (u64, NSString storyboard name)
//!   offset 16: bundle (u64, NSBundle, 0 = mainBundle)

use std::path::{Path, PathBuf};

const NAME_OFF: usize = 8;
const BUNDLE_OFF: usize = 16;
pub(crate) const STORYBOARD_SIZE: usize = 24;

const INITIAL_IDENTIFIER_KEYS: &[&str] = &[
    "UIStoryboardDesignatedEntryPointIdentifier",
    "NSStoryboardDesignatedEntryPointIdentifier",
    "NSStoryboardInitialControllerIdentifier",
    "NSInitialControllerIdentifier",
    "InitialViewControllerIdentifier",
    "initialViewControllerIdentifier",
];

const INITIAL_NIB_KEYS: &[&str] = &[
    "UIStoryboardDesignatedEntryPointNibName",
    "NSStoryboardInitialControllerNibName",
    "NSInitialControllerNibName",
    "InitialViewControllerNibName",
    "initialViewControllerNibName",
];

const IDENTIFIER_TO_NIB_MAP_KEYS: &[&str] = &[
    "UIViewControllerIdentifiersToNibNames",
    "NSViewControllerIdentifiersToNibNames",
    "NSStoryboardSceneIdentifiersToNibNames",
    "NSSceneIdentifiersToNibNames",
    "NSControllerIdentifiersToNibNames",
    "UIStoryboardIdentifiersToNibNames",
    "StoryboardIdentifiersToNibNames",
];

const NIB_NAME_KEYS: &[&str] = &[
    "NibName",
    "nibName",
    "NSNibName",
    "UINibName",
    "UIViewControllerNibName",
    "NSViewControllerNibName",
];

const COMMON_INITIAL_SCENE_NAMES: &[&str] = &[
    "InitialViewController",
    "InitialController",
    "MainWindowController",
    "WindowController",
    "ViewController",
    "Main",
];

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

fn nsstring_to_string(obj: u64) -> Option<String> {
    crate::cfstring::extract_cfstring_cstr(obj).map(ToOwned::to_owned)
}

fn normalize_storyboard_name(name: &str) -> &str {
    name.strip_suffix(".storyboardc")
        .or_else(|| name.strip_suffix(".storyboard"))
        .unwrap_or(name)
}

fn object_is_kind_of(obj: u64, class_addr: u64) -> bool {
    if obj == 0 || class_addr == 0 {
        return false;
    }

    (unsafe { *(obj as *const u64) == class_addr })
        || crate::nsobj::shim_nsobject_is_kind_of_class(obj, 0, class_addr) != 0
}

fn array_count(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let isa = unsafe { *(obj as *const u64) };
    if isa != crate::nsobj::nsarray_class_addr() && isa != crate::nsobj::nsmutablearray_class_addr()
    {
        return 0;
    }
    crate::nsarray::shim_nsarray_count(obj, 0)
}

fn array_item(obj: u64, index: u64) -> u64 {
    if array_count(obj) == 0 {
        return 0;
    }
    crate::nsarray::shim_nsarray_object_at_index(obj, 0, index)
}

fn first_top_level_object_kind_of(top_level: u64, class_addr: u64) -> u64 {
    if top_level == 0 {
        return 0;
    }

    if object_is_kind_of(top_level, class_addr) {
        return top_level;
    }

    let count = array_count(top_level);
    for index in 0..count {
        let item = crate::nsarray::shim_nsarray_object_at_index(top_level, 0, index);
        if object_is_kind_of(item, class_addr) {
            return item;
        }
    }
    0
}

fn select_storyboard_controller(top_level: u64) -> u64 {
    for class_addr in [
        crate::nsobj::nswindowcontroller_class_addr(),
        crate::nsobj::nsviewcontroller_class_addr(),
        crate::nsobj::nswindow_class_addr(),
        crate::nsobj::nsview_class_addr(),
    ] {
        let item = first_top_level_object_kind_of(top_level, class_addr);
        if item != 0 {
            return item;
        }
    }

    if array_count(top_level) == 1 {
        return array_item(top_level, 0);
    }

    top_level
}

fn push_nsstring_path(paths: &mut Vec<PathBuf>, value: u64) {
    let Some(path) = nsstring_to_string(value) else {
        return;
    };
    let path = PathBuf::from(path);
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn storyboard_bundle(obj: u64) -> u64 {
    let bundle = read_u64(obj, BUNDLE_OFF);
    if bundle != 0 {
        bundle
    } else {
        crate::nsbundle::get_or_create_main_bundle()
    }
}

fn bundle_search_roots(bundle: u64) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if bundle == 0 {
        return roots;
    }

    push_nsstring_path(
        &mut roots,
        crate::nsbundle::shim_nsbundle_resource_path(bundle, 0),
    );
    push_nsstring_path(
        &mut roots,
        crate::nsbundle::shim_nsbundle_bundle_path(bundle, 0),
    );
    roots
}

fn storyboard_directory(obj: u64) -> Option<PathBuf> {
    let name = nsstring_to_string(read_u64(obj, NAME_OFF))?;
    let name = normalize_storyboard_name(&name);
    if name.is_empty() {
        return None;
    }

    let storyboard_dir_name = format!("{name}.storyboardc");
    for root in bundle_search_roots(storyboard_bundle(obj)) {
        for candidate in [
            root.join(&storyboard_dir_name),
            root.join("Base.lproj").join(&storyboard_dir_name),
            root.join("en.lproj").join(&storyboard_dir_name),
        ] {
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }

    None
}

fn load_storyboard_info(storyboard_dir: &Path) -> Option<plist::Dictionary> {
    let info_path = storyboard_dir.join("Info.plist");
    let bytes = std::fs::read(info_path).ok()?;
    match plist::Value::from_reader(std::io::Cursor::new(bytes)).ok()? {
        plist::Value::Dictionary(dict) => Some(dict),
        _ => None,
    }
}

fn plist_string_for_keys(dict: &plist::Dictionary, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| dict.get(key).and_then(plist::Value::as_string))
        .map(ToOwned::to_owned)
}

fn nib_name_from_plist_value(value: &plist::Value) -> Option<String> {
    if let Some(value) = value.as_string() {
        return Some(value.to_owned());
    }

    let dict = value.as_dictionary()?;
    if let Some(value) = plist_string_for_keys(dict, NIB_NAME_KEYS) {
        return Some(value);
    }

    dict.values()
        .find_map(plist::Value::as_string)
        .map(ToOwned::to_owned)
}

fn mapped_nib_name(info: &plist::Dictionary, identifier: &str) -> Option<String> {
    for key in IDENTIFIER_TO_NIB_MAP_KEYS {
        let Some(map) = info.get(key).and_then(plist::Value::as_dictionary) else {
            continue;
        };
        let Some(value) = map.get(identifier) else {
            continue;
        };
        if let Some(nib_name) = nib_name_from_plist_value(value) {
            return Some(nib_name);
        }
    }
    None
}

fn nib_path_for_scene_name(storyboard_dir: &Path, scene_name: &str) -> Option<PathBuf> {
    let scene_name = scene_name.trim();
    if scene_name.is_empty() {
        return None;
    }

    let scene_path = Path::new(scene_name);
    if scene_path.is_absolute() {
        return None;
    }

    let mut candidates = Vec::new();
    candidates.push(storyboard_dir.join(scene_path));
    if !scene_name.ends_with(".nib") {
        candidates.push(storyboard_dir.join(format!("{scene_name}.nib")));
    }

    for candidate in candidates {
        if let Some(path) = crate::nsbundle::resolve_nib_file_path(&candidate) {
            return Some(path);
        }
    }

    None
}

fn first_storyboard_nib(storyboard_dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(storyboard_dir).ok()?;
    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("nib") {
            continue;
        }
        if let Some(path) = crate::nsbundle::resolve_nib_file_path(&path) {
            candidates.push(path);
        }
    }

    candidates.sort();
    candidates.into_iter().next()
}

fn initial_nib_path(
    storyboard_dir: &Path,
    info: Option<&plist::Dictionary>,
    storyboard_name: &str,
) -> Option<PathBuf> {
    if let Some(info) = info {
        if let Some(nib_name) = plist_string_for_keys(info, INITIAL_NIB_KEYS) {
            if let Some(path) = nib_path_for_scene_name(storyboard_dir, &nib_name) {
                return Some(path);
            }
        }

        if let Some(identifier) = plist_string_for_keys(info, INITIAL_IDENTIFIER_KEYS) {
            if let Some(nib_name) = mapped_nib_name(info, &identifier) {
                if let Some(path) = nib_path_for_scene_name(storyboard_dir, &nib_name) {
                    return Some(path);
                }
            }
            if let Some(path) = nib_path_for_scene_name(storyboard_dir, &identifier) {
                return Some(path);
            }
        }
    }

    for scene_name in COMMON_INITIAL_SCENE_NAMES {
        if let Some(path) = nib_path_for_scene_name(storyboard_dir, scene_name) {
            return Some(path);
        }
    }

    nib_path_for_scene_name(storyboard_dir, storyboard_name)
        .or_else(|| first_storyboard_nib(storyboard_dir))
}

fn identified_nib_path(
    storyboard_dir: &Path,
    info: Option<&plist::Dictionary>,
    identifier: &str,
) -> Option<PathBuf> {
    if let Some(info) = info {
        if let Some(nib_name) = mapped_nib_name(info, identifier) {
            if let Some(path) = nib_path_for_scene_name(storyboard_dir, &nib_name) {
                return Some(path);
            }
        }
    }

    nib_path_for_scene_name(storyboard_dir, identifier)
}

fn instantiate_nib_at_path(nib_path: &Path) -> u64 {
    let mut top_level = 0_u64;
    let ok =
        crate::nsbundle::load_nib_file_at_path(nib_path, 0, (&mut top_level as *mut u64) as u64);
    if ok == 0 || top_level == 0 {
        return 0;
    }
    select_storyboard_controller(top_level)
}

fn instantiate_scene(obj: u64, identifier: Option<&str>) -> u64 {
    if obj == 0 {
        return 0;
    }

    let Some(storyboard_dir) = storyboard_directory(obj) else {
        log::debug!("NSStoryboard: storyboard directory not found");
        return 0;
    };
    let info = load_storyboard_info(&storyboard_dir);
    let storyboard_name = nsstring_to_string(read_u64(obj, NAME_OFF))
        .map(|name| normalize_storyboard_name(&name).to_owned())
        .unwrap_or_default();

    let nib_path = match identifier {
        Some(identifier) => identified_nib_path(&storyboard_dir, info.as_ref(), identifier),
        None => initial_nib_path(&storyboard_dir, info.as_ref(), &storyboard_name),
    };

    let Some(nib_path) = nib_path else {
        log::debug!(
            "NSStoryboard: no scene nib found in '{}'",
            storyboard_dir.display()
        );
        return 0;
    };

    log::debug!("NSStoryboard: loading scene '{}'", nib_path.display());
    instantiate_nib_at_path(&nib_path)
}

pub(crate) fn present_storyboard_controller(controller: u64, sender: u64) -> bool {
    if object_is_kind_of(controller, crate::nsobj::nswindowcontroller_class_addr()) {
        crate::nswindowcontroller::shim_nswindowcontroller_show_window(controller, 0, sender);
        return true;
    }

    if object_is_kind_of(controller, crate::nsobj::nswindow_class_addr()) {
        crate::nswindow::shim_nswindow_make_key_and_order_front(controller, 0, sender);
        return true;
    }

    if object_is_kind_of(controller, crate::nsobj::nsviewcontroller_class_addr()) {
        return crate::nsviewcontroller::shim_nsviewcontroller_view(controller, 0) != 0;
    }

    object_is_kind_of(controller, crate::nsobj::nsview_class_addr())
}

/// `+[NSStoryboard alloc]`.
pub extern "C" fn shim_nsstoryboard_alloc(cls: u64, _cmd: u64) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, STORYBOARD_SIZE) as u64;
    if ptr == 0 {
        return 0;
    }
    let cls = if cls != 0 {
        cls
    } else {
        crate::nsobj::nsstoryboard_class_addr()
    };
    unsafe {
        *(ptr as *mut u64) = cls;
    }
    ptr
}

/// `+[NSStoryboard storyboardWithName:bundle:]`.
pub extern "C" fn shim_nsstoryboard_with_name_bundle(
    cls: u64,
    _cmd: u64,
    name: u64,
    bundle: u64,
) -> u64 {
    if name == 0 {
        return 0;
    }
    let obj = shim_nsstoryboard_alloc(cls, 0);
    shim_nsstoryboard_init_with_name_bundle(obj, 0, name, bundle)
}

/// `-[NSStoryboard initWithName:bundle:]`.
pub extern "C" fn shim_nsstoryboard_init_with_name_bundle(
    obj: u64,
    _cmd: u64,
    name: u64,
    bundle: u64,
) -> u64 {
    if obj == 0 || name == 0 {
        return 0;
    }
    write_u64(obj, NAME_OFF, name);
    write_u64(obj, BUNDLE_OFF, bundle);
    obj
}

/// `-[NSStoryboard instantiateInitialController]`.
pub extern "C" fn shim_nsstoryboard_instantiate_initial_controller(obj: u64, _cmd: u64) -> u64 {
    instantiate_scene(obj, None)
}

/// `-[NSStoryboard instantiateInitialControllerWithCreator:]`.
pub extern "C" fn shim_nsstoryboard_instantiate_initial_controller_with_creator(
    obj: u64,
    _cmd: u64,
    _creator: u64,
) -> u64 {
    instantiate_scene(obj, None)
}

/// `-[NSStoryboard instantiateControllerWithIdentifier:]`.
pub extern "C" fn shim_nsstoryboard_instantiate_controller_with_identifier(
    obj: u64,
    _cmd: u64,
    identifier: u64,
) -> u64 {
    let Some(identifier) = nsstring_to_string(identifier) else {
        return 0;
    };
    instantiate_scene(obj, Some(&identifier))
}

/// `-[NSStoryboard instantiateControllerWithIdentifier:creator:]`.
pub extern "C" fn shim_nsstoryboard_instantiate_controller_with_identifier_creator(
    obj: u64,
    _cmd: u64,
    identifier: u64,
    _creator: u64,
) -> u64 {
    shim_nsstoryboard_instantiate_controller_with_identifier(obj, 0, identifier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::FoundationShims;
    use crate::shim_test_helpers::*;
    use objc_runtime::ObjcRuntime;
    use plist::{Dictionary, Value};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static STORYBOARD_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_storyboard_runtime<R>(f: impl FnOnce() -> R) -> R {
        let saved_runtime = {
            let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap();
            let saved = guard.take();
            let mut runtime = ObjcRuntime::new();

            FoundationShims::register_nsobject_class(&mut runtime);
            FoundationShims::register_nsstring_class(&mut runtime);
            FoundationShims::register_nsarray_class(&mut runtime);
            FoundationShims::register_nsmutablearray_class(&mut runtime);
            FoundationShims::register_nsnumber_class(&mut runtime);
            FoundationShims::register_nsbundle_class(&mut runtime);
            FoundationShims::register_nsnotification_class(&mut runtime);
            FoundationShims::register_nsnotificationcenter_class(&mut runtime);
            FoundationShims::register_nsresponder_class(&mut runtime);
            FoundationShims::register_nsapplication_class(&mut runtime);
            FoundationShims::register_nswindow_class(&mut runtime);
            FoundationShims::register_nsview_class(&mut runtime);
            FoundationShims::register_nsviewcontroller_class(&mut runtime);
            FoundationShims::register_nswindowcontroller_class(&mut runtime);
            FoundationShims::register_nsstoryboard_class(&mut runtime);

            *guard = Some(runtime);
            saved
        };

        let result = f();

        let mut guard = objc_runtime::OBJC_RUNTIME.write().unwrap();
        *guard = saved_runtime;
        result
    }

    fn unique_temp_root(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "burdettie-nsstoryboard-{name}-{}-{stamp}",
            std::process::id()
        ))
    }

    fn make_temp_app_bundle(name: &str) -> (PathBuf, PathBuf, u64) {
        let root = unique_temp_root(name);
        let app = root.join("Fixture.app");
        std::fs::create_dir_all(app.join("Contents").join("Resources")).unwrap();

        let mut info = Dictionary::new();
        info.insert(
            "CFBundleIdentifier".to_string(),
            Value::String(format!("dev.burdettie.tests.{name}")),
        );
        let mut bytes = Vec::new();
        plist::to_writer_xml(&mut bytes, &Value::Dictionary(info)).unwrap();
        std::fs::write(app.join("Contents").join("Info.plist"), bytes).unwrap();

        let bundle_path = make_string(app.to_string_lossy().as_ref());
        let bundle = crate::nsbundle::shim_nsbundle_bundle_with_path(0, 0, bundle_path);
        (root, app, bundle)
    }

    fn uid(id: u64) -> Value {
        Value::Uid(plist::Uid::new(id))
    }

    fn class_descriptor(name: &str, classes: &[&str]) -> Value {
        let mut dict = Dictionary::new();
        dict.insert("$classname".to_string(), Value::String(name.to_string()));
        dict.insert(
            "$classes".to_string(),
            Value::Array(
                classes
                    .iter()
                    .map(|class| Value::String((*class).to_string()))
                    .collect(),
            ),
        );
        Value::Dictionary(dict)
    }

    fn keyed_object(class_uid: u64, entries: Vec<(&str, Value)>) -> Value {
        let mut dict = Dictionary::new();
        dict.insert("$class".to_string(), uid(class_uid));
        for (key, value) in entries {
            dict.insert(key.to_string(), value);
        }
        Value::Dictionary(dict)
    }

    fn build_apple_archive(objects: Vec<Value>, root_uid: u64) -> Vec<u8> {
        let mut top = Dictionary::new();
        top.insert("root".to_string(), uid(root_uid));

        let mut root = Dictionary::new();
        root.insert(
            "$archiver".to_string(),
            Value::String("NSKeyedArchiver".to_string()),
        );
        root.insert("$version".to_string(), Value::Integer(100000.into()));
        root.insert("$top".to_string(), Value::Dictionary(top));
        root.insert("$objects".to_string(), Value::Array(objects));

        let mut bytes = Vec::new();
        plist::to_writer_binary(&mut bytes, &Value::Dictionary(root)).unwrap();
        bytes
    }

    fn storyboard_info_bytes() -> Vec<u8> {
        let mut scene_map = Dictionary::new();
        scene_map.insert(
            "main-window".to_string(),
            Value::String("MainWindowController".to_string()),
        );

        let mut info = Dictionary::new();
        info.insert(
            "NSStoryboardInitialControllerIdentifier".to_string(),
            Value::String("main-window".to_string()),
        );
        info.insert(
            "NSStoryboardSceneIdentifiersToNibNames".to_string(),
            Value::Dictionary(scene_map),
        );

        let mut bytes = Vec::new();
        plist::to_writer_xml(&mut bytes, &Value::Dictionary(info)).unwrap();
        bytes
    }

    fn scene_nib_bytes() -> Vec<u8> {
        build_apple_archive(
            vec![
                Value::String("$null".to_string()),
                keyed_object(10, vec![("NS.objects", Value::Array(vec![uid(2)]))]),
                keyed_object(
                    11,
                    vec![
                        ("NSWindow", uid(3)),
                        (
                            "NSWindowNibName",
                            Value::String("MainWindowController".to_string()),
                        ),
                        ("NSShouldCascadeWindows", Value::Boolean(false)),
                    ],
                ),
                keyed_object(
                    12,
                    vec![
                        (
                            "NSWindowFrame",
                            Value::String("{{44, 55}, {640, 480}}".to_string()),
                        ),
                        ("NSTitle", Value::String("Storyboard Window".to_string())),
                        ("NSWindowStyleMask", Value::Integer(15.into())),
                        ("NSContentView", uid(6)),
                    ],
                ),
                Value::String("{{44, 55}, {640, 480}}".to_string()),
                Value::String("Storyboard Window".to_string()),
                keyed_object(
                    13,
                    vec![
                        ("NSFrame", Value::String("{{0, 0}, {640, 480}}".to_string())),
                        ("NSSubviews", uid(14)),
                    ],
                ),
                Value::String("MainWindowController".to_string()),
                Value::Boolean(false),
                Value::String("{{0, 0}, {640, 480}}".to_string()),
                class_descriptor("NSArray", &["NSArray", "NSObject"]),
                class_descriptor(
                    "NSWindowController",
                    &["NSWindowController", "NSResponder", "NSObject"],
                ),
                class_descriptor("NSWindow", &["NSWindow", "NSResponder", "NSObject"]),
                class_descriptor("NSView", &["NSView", "NSResponder", "NSObject"]),
                keyed_object(10, vec![("NS.objects", Value::Array(vec![uid(15)]))]),
                keyed_object(
                    13,
                    vec![(
                        "NSFrame",
                        Value::String("{{12, 18}, {120, 40}}".to_string()),
                    )],
                ),
                Value::String("{{12, 18}, {120, 40}}".to_string()),
            ],
            1,
        )
    }

    fn write_storyboard_fixture(app: &Path) {
        let storyboard_dir = app
            .join("Contents")
            .join("Resources")
            .join("Base.lproj")
            .join("Main.storyboardc");
        let nib_dir = storyboard_dir.join("MainWindowController.nib");
        std::fs::create_dir_all(&nib_dir).unwrap();
        std::fs::write(storyboard_dir.join("Info.plist"), storyboard_info_bytes()).unwrap();
        std::fs::write(nib_dir.join("keyedobjects.nib"), scene_nib_bytes()).unwrap();
    }

    #[test]
    fn storyboard_initial_controller_loads_bundle_scene_nib_and_presents_window() {
        let _guard = STORYBOARD_TEST_LOCK.lock().unwrap();

        with_storyboard_runtime(|| {
            let (root, app, bundle) = make_temp_app_bundle("initial-controller");
            write_storyboard_fixture(&app);

            let storyboard_name = make_string("Main");
            let storyboard = shim_nsstoryboard_with_name_bundle(
                crate::nsobj::nsstoryboard_class_addr(),
                0,
                storyboard_name,
                bundle,
            );
            assert_ne!(storyboard, 0);

            let controller = shim_nsstoryboard_instantiate_initial_controller(storyboard, 0);
            assert_ne!(controller, 0);
            assert_eq!(
                unsafe { *(controller as *const u64) },
                crate::nsobj::nswindowcontroller_class_addr()
            );

            let window = crate::nswindowcontroller::shim_nswindowcontroller_window(controller, 0);
            assert_ne!(window, 0);
            assert_eq!(crate::nswindow::shim_nswindow_is_visible(window, 0), 0);

            let title = crate::nswindow::shim_nswindow_title(window, 0);
            assert_string_eq(title, "Storyboard Window");

            let frame = crate::nswindow::shim_nswindow_frame(window, 0);
            assert_eq!(frame.origin.x, 44.0);
            assert_eq!(frame.origin.y, 55.0);
            assert_eq!(frame.size.width, 640.0);
            assert_eq!(frame.size.height, 480.0);

            let content_view = crate::nswindow::shim_nswindow_content_view(window, 0);
            assert_ne!(content_view, 0);
            let content_frame = crate::nsview::shim_nsview_frame(content_view, 0);
            assert_eq!(content_frame.size.width, 640.0);
            assert_eq!(content_frame.size.height, 480.0);

            let subviews = crate::nsview::shim_nsview_subviews(content_view, 0);
            assert_eq!(crate::nsarray::shim_nsarray_count(subviews, 0), 1);
            let child = crate::nsarray::shim_nsarray_object_at_index(subviews, 0, 0);
            assert_ne!(child, 0);
            assert_eq!(crate::nsview::shim_nsview_superview(child, 0), content_view);

            let app_object = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
            assert!(present_storyboard_controller(controller, app_object));
            assert_eq!(crate::nswindow::shim_nswindow_is_visible(window, 0), 1);
            assert_eq!(
                crate::nsapplication::shim_nsapplication_key_window(app_object, 0),
                window
            );
            assert_eq!(
                crate::nsapplication::shim_nsapplication_main_window(app_object, 0),
                window
            );

            std::fs::remove_dir_all(root).unwrap();
        });
    }
}
