//! NSImage shim: image factory and instance methods for AppKit.
//!
//! Maintains a backing store of image metadata (file path, dimensions)
//! keyed by object pointer so that NSImage objects can be created,
//! passed around, and queried for size.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Per-object metadata for allocated NSImage instances.
struct ImageInfo {
    /// Original file path (if loaded from file).
    #[allow(dead_code)]
    file_path: Option<String>,
    name: Option<String>,
    template: bool,
    width: f64,
    height: f64,
    /// CGImage handle (from cg-shims bridge). 0 = no pixel data.
    cg_handle: u64,
}

static IMAGE_STORE: Mutex<Option<HashMap<u64, ImageInfo>>> = Mutex::new(None);
const NAMED_IMAGE_EXTENSIONS: &[&str] =
    &["png", "jpg", "jpeg", "tiff", "tif", "gif", "bmp", "icns"];

fn store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, ImageInfo>>> {
    IMAGE_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

fn ensure_store(guard: &mut Option<HashMap<u64, ImageInfo>>) -> &mut HashMap<u64, ImageInfo> {
    guard.get_or_insert_with(HashMap::new)
}

fn make_nsstring_from_str(value: &str) -> u64 {
    let cstr = std::ffi::CString::new(value).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

fn release_image_info(info: ImageInfo) {
    if info.cg_handle != 0 {
        cg_shims::bridge::image_release(info.cg_handle);
    }
}

fn insert_image_info(obj: u64, info: ImageInfo) {
    let mut guard = store();
    let map = ensure_store(&mut guard);
    if let Some(old) = map.insert(obj, info) {
        release_image_info(old);
    }
}

fn decode_image_file(path: &Path) -> (f64, f64, u64) {
    if let Some(img) = graphics_core::ImageBuffer::from_file(path) {
        let w = img.width() as f64;
        let h = img.height() as f64;
        let handle = cg_shims::bridge::image_create_from_buffer(img);
        (w, h, handle)
    } else {
        (0.0, 0.0, 0)
    }
}

fn init_image_from_buffer(
    obj: u64,
    img: graphics_core::ImageBuffer,
    file_path: Option<String>,
    name: Option<&str>,
    template: bool,
) -> u64 {
    let w = img.width() as f64;
    let h = img.height() as f64;
    let cg_handle = cg_shims::bridge::image_create_from_buffer(img);
    insert_image_info(
        obj,
        ImageInfo {
            file_path,
            name: name.map(str::to_string),
            template,
            width: w,
            height: h,
            cg_handle,
        },
    );
    obj
}

fn template_hint_for_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    if lower.ends_with("template") || lower.contains("template.") {
        return true;
    }

    Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.to_ascii_lowercase().ends_with("template"))
}

fn init_image_from_path_with_template(
    obj: u64,
    path: &Path,
    name: Option<&str>,
    template: bool,
) -> u64 {
    let path_str = path.to_string_lossy().to_string();
    let (w, h, cg_handle) = decode_image_file(path);
    insert_image_info(
        obj,
        ImageInfo {
            file_path: Some(path_str),
            name: name.map(str::to_string),
            template,
            width: w,
            height: h,
            cg_handle,
        },
    );
    obj
}

fn init_image_from_path(obj: u64, path: &Path, name: Option<&str>) -> u64 {
    let template = name.map(template_hint_for_name).unwrap_or_else(|| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(template_hint_for_name)
    });
    init_image_from_path_with_template(obj, path, name, template)
}

fn init_empty_image(obj: u64, name: Option<&str>, template: bool) -> u64 {
    insert_image_info(
        obj,
        ImageInfo {
            file_path: None,
            name: name.map(str::to_string),
            template,
            width: 0.0,
            height: 0.0,
            cg_handle: 0,
        },
    );
    obj
}

fn alloc_image_object() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 8) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsimage_class_addr();
        }
    }
    obj
}

fn has_known_image_extension(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            NAMED_IMAGE_EXTENSIONS
                .iter()
                .any(|candidate| *candidate == ext)
        })
        .unwrap_or(false)
}

fn resolve_named_image_file(bundle: u64, name: &str) -> Option<PathBuf> {
    if let Some(path) =
        crate::nsbundle::resolve_bundle_resource_path_parts(bundle, name, None, None, None)
    {
        return Some(path);
    }

    if has_known_image_extension(name) {
        return None;
    }

    for ext in NAMED_IMAGE_EXTENSIONS {
        if let Some(path) =
            crate::nsbundle::resolve_bundle_resource_path_parts(bundle, name, Some(ext), None, None)
        {
            return Some(path);
        }
    }

    None
}

fn push_unique_string(values: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    let normalized = value.replace('\\', "/");
    if !values.iter().any(|existing| existing == &normalized) {
        values.push(normalized);
    }
}

fn strip_known_image_extension(name: &str) -> Option<String> {
    let ext = Path::new(name).extension().and_then(|ext| ext.to_str())?;
    let ext = ext.to_ascii_lowercase();
    if !NAMED_IMAGE_EXTENSIONS
        .iter()
        .any(|candidate| *candidate == ext)
    {
        return None;
    }
    let suffix_len = ext.len() + 1;
    (name.len() > suffix_len).then(|| name[..name.len() - suffix_len].to_string())
}

fn asset_catalog_name_candidates(name: &str) -> Vec<String> {
    let mut names = Vec::new();
    push_unique_string(&mut names, name);
    if let Some(without_ext) = strip_known_image_extension(name) {
        push_unique_string(&mut names, without_ext);
        if let Some(stem) = Path::new(name).file_stem().and_then(|stem| stem.to_str()) {
            push_unique_string(&mut names, stem);
        }
    }
    names
}

fn path_extension_eq(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension.trim_start_matches('.')))
}

fn collect_paths_with_extension(
    root: &Path,
    extension: &str,
    max_depth: usize,
    want_dir: bool,
) -> Vec<PathBuf> {
    fn visit(
        dir: &Path,
        extension: &str,
        max_depth: usize,
        depth: usize,
        want_dir: bool,
        paths: &mut Vec<PathBuf>,
    ) {
        if depth > max_depth {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries = entries
            .flatten()
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        entries.sort();

        for path in entries {
            let is_dir = path.is_dir();
            if is_dir == want_dir && path_extension_eq(&path, extension) {
                paths.push(path.clone());
            }
            if is_dir && depth < max_depth {
                visit(&path, extension, max_depth, depth + 1, want_dir, paths);
            }
        }
    }

    let mut paths = Vec::new();
    visit(root, extension, max_depth, 0, want_dir, &mut paths);
    paths
}

fn collect_files_with_extension(root: &Path, extension: &str, max_depth: usize) -> Vec<PathBuf> {
    collect_paths_with_extension(root, extension, max_depth, false)
}

fn collect_dirs_with_extension(root: &Path, extension: &str, max_depth: usize) -> Vec<PathBuf> {
    collect_paths_with_extension(root, extension, max_depth, true)
}

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !paths.iter().any(|path| path == &candidate) {
        paths.push(candidate);
    }
}

fn compiled_asset_catalog_paths(bundle: u64) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = crate::nsbundle::resolve_bundle_resource_path_parts(
        bundle,
        "Assets",
        Some("car"),
        None,
        None,
    ) {
        push_unique_path(&mut paths, path);
    }
    if let Some(root) = crate::nsbundle::bundle_resource_root(bundle) {
        for path in collect_files_with_extension(&root, "car", 4) {
            push_unique_path(&mut paths, path);
        }
    }
    paths
}

fn decode_asset_catalog_image(
    bundle: u64,
    name: &str,
) -> Option<(graphics_core::ImageBuffer, bool)> {
    if bundle == 0 {
        return None;
    }

    let template = template_hint_for_name(name);
    for catalog_path in compiled_asset_catalog_paths(bundle) {
        let Ok(archive) = car::Car::new(&catalog_path) else {
            continue;
        };
        for candidate in asset_catalog_name_candidates(name) {
            let Ok(variant) = archive.best_variant_for_name(&candidate, &car::VariantQuery::new())
            else {
                continue;
            };
            let Some(item) = archive.item_with_key_values(&variant.key_values) else {
                continue;
            };
            let Ok(payload) = car::image::rgba_bytes(item) else {
                continue;
            };
            if let Some(image) =
                graphics_core::ImageBuffer::from_rgba(payload.width, payload.height, payload.rgba)
            {
                return Some((image, template));
            }
        }
    }
    None
}

fn source_asset_catalog_paths(bundle: u64) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = crate::nsbundle::resolve_bundle_resource_path_parts(
        bundle,
        "Assets",
        Some("xcassets"),
        None,
        None,
    ) {
        push_unique_path(&mut paths, path);
    }
    if let Some(root) = crate::nsbundle::bundle_resource_root(bundle) {
        for path in collect_dirs_with_extension(&root, "xcassets", 6) {
            push_unique_path(&mut paths, path);
        }
    }
    paths
}

fn path_to_asset_name(path: &Path) -> Option<String> {
    let mut path = path.to_path_buf();
    path.set_extension("");
    let name = path.to_string_lossy().replace('\\', "/");
    (!name.is_empty()).then_some(name)
}

fn image_set_asset_names(catalog: &Path, image_set: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(stem) = image_set.file_stem().and_then(|stem| stem.to_str()) {
        push_unique_string(&mut names, stem);
    }
    if let Ok(relative) = image_set.strip_prefix(catalog) {
        if let Some(name) = path_to_asset_name(relative) {
            push_unique_string(&mut names, name);
        }
    }
    names
}

fn image_set_matches(catalog: &Path, image_set: &Path, candidate: &str) -> bool {
    let candidate = candidate.trim().replace('\\', "/");
    image_set_asset_names(catalog, image_set)
        .iter()
        .any(|name| name == &candidate || name.eq_ignore_ascii_case(&candidate))
}

fn contents_template_hint(contents: &serde_json::Value) -> bool {
    let Some(properties) = contents.get("properties") else {
        return false;
    };
    ["template-rendering-intent", "rendering-intent"]
        .iter()
        .any(|key| {
            properties
                .get(key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case("template"))
        })
}

fn image_set_idiom_score(image: &serde_json::Value) -> i32 {
    match image.get("idiom").and_then(serde_json::Value::as_str) {
        Some("mac") | Some("macos") => 0,
        Some("universal") => 1,
        None => 2,
        Some("iphone") | Some("ipad") | Some("watch") | Some("tv") => 4,
        Some(_) => 3,
    }
}

fn image_set_scale_score(image: &serde_json::Value) -> i32 {
    match image.get("scale").and_then(serde_json::Value::as_str) {
        Some("1x") => 0,
        None => 1,
        Some("2x") => 2,
        Some("3x") => 3,
        Some(_) => 4,
    }
}

fn image_set_extension_score(filename: &str) -> i32 {
    if has_known_image_extension(filename) {
        0
    } else {
        1
    }
}

fn select_image_set_filename(contents: &serde_json::Value) -> Option<String> {
    let images = contents.get("images")?.as_array()?;
    let mut candidates = images
        .iter()
        .filter_map(|image| {
            let filename = image.get("filename")?.as_str()?.trim();
            (!filename.is_empty()).then(|| {
                (
                    image_set_idiom_score(image),
                    image_set_scale_score(image),
                    image_set_extension_score(filename),
                    filename.to_string(),
                )
            })
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .into_iter()
        .map(|(_, _, _, filename)| filename)
        .next()
}

fn resolve_xcassets_image_path(bundle: u64, name: &str) -> Option<(PathBuf, bool)> {
    if bundle == 0 {
        return None;
    }

    let candidates = asset_catalog_name_candidates(name);
    for catalog in source_asset_catalog_paths(bundle) {
        let image_sets = collect_dirs_with_extension(&catalog, "imageset", 8);
        for candidate in &candidates {
            for image_set in &image_sets {
                if !image_set_matches(&catalog, image_set, candidate) {
                    continue;
                }
                let contents_path = image_set.join("Contents.json");
                let Ok(contents) = std::fs::read(&contents_path) else {
                    continue;
                };
                let Ok(contents) = serde_json::from_slice::<serde_json::Value>(&contents) else {
                    continue;
                };
                let Some(filename) = select_image_set_filename(&contents) else {
                    continue;
                };
                let image_path = image_set.join(filename);
                if image_path.is_file() {
                    let template =
                        template_hint_for_name(name) || contents_template_hint(&contents);
                    return Some((image_path, template));
                }
            }
        }
    }
    None
}

fn bundle_info_string(bundle: u64, key: &str) -> Option<String> {
    let key = make_nsstring_from_str(key);
    let value = crate::nsbundle::shim_nsbundle_object_for_info_key(bundle, 0, key);
    crate::cfstring::extract_cfstring_cstr(value).map(str::to_string)
}

fn resolve_application_icon_file(bundle: u64) -> Option<PathBuf> {
    for key in ["CFBundleIconFile", "CFBundleIconName"] {
        let Some(icon_name) = bundle_info_string(bundle, key) else {
            continue;
        };
        if let Some(path) = resolve_named_image_file(bundle, &icon_name) {
            return Some(path);
        }
    }
    None
}

fn resolve_application_icon_asset(bundle: u64) -> Option<(graphics_core::ImageBuffer, bool)> {
    for key in ["CFBundleIconFile", "CFBundleIconName"] {
        let Some(icon_name) = bundle_info_string(bundle, key) else {
            continue;
        };
        if let Some(image) = decode_asset_catalog_image(bundle, &icon_name) {
            return Some(image);
        }
    }
    None
}

fn resolve_application_icon_source_asset(bundle: u64) -> Option<(PathBuf, bool)> {
    for key in ["CFBundleIconFile", "CFBundleIconName"] {
        let Some(icon_name) = bundle_info_string(bundle, key) else {
            continue;
        };
        if let Some(path) = resolve_xcassets_image_path(bundle, &icon_name) {
            return Some(path);
        }
    }
    None
}

fn resolve_named_image_path(bundle: u64, name: &str) -> Option<PathBuf> {
    if name == "NSApplicationIcon" {
        if let Some(path) = resolve_application_icon_file(bundle) {
            return Some(path);
        }
    }
    resolve_named_image_file(bundle, name)
}

#[derive(Clone, Copy)]
enum SystemImageGlyph {
    ArrowLeft,
    ArrowRight,
    Caution,
    Check,
    Folder,
    Gear,
    Generic,
    Info,
    Lock,
    Minus,
    Plus,
    Refresh,
    Share,
    Stop,
    Trash,
    Unlock,
    User,
    View,
}

struct SystemImageSpec {
    glyph: SystemImageGlyph,
    template: bool,
}

fn system_image_spec(name: &str, allow_symbol_name: bool) -> Option<SystemImageSpec> {
    let lower = name.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    let template = lower.ends_with("template")
        || lower.contains("template.")
        || lower.contains("symbol")
        || allow_symbol_name;

    let glyph = if lower.contains("add") || lower == "plus" || lower.ends_with(".plus") {
        SystemImageGlyph::Plus
    } else if lower.contains("remove")
        || lower == "minus"
        || lower.ends_with(".minus")
        || lower.contains("menumixedstate")
    {
        SystemImageGlyph::Minus
    } else if lower.contains("menuonstate") || lower.contains("checkmark") {
        SystemImageGlyph::Check
    } else if lower.contains("caution")
        || lower.contains("warning")
        || lower.contains("exclamationmark.triangle")
    {
        SystemImageGlyph::Caution
    } else if lower.contains("folder") {
        SystemImageGlyph::Folder
    } else if lower.contains("preferences") || lower.contains("gear") || lower.contains("advanced")
    {
        SystemImageGlyph::Gear
    } else if lower.contains("trash") {
        SystemImageGlyph::Trash
    } else if lower.contains("lockunlocked") || lower.contains("lock.open") {
        SystemImageGlyph::Unlock
    } else if lower.contains("lock") {
        SystemImageGlyph::Lock
    } else if lower.contains("goleft")
        || lower.contains("left")
        || lower.contains("chevron.left")
        || lower.contains("arrow.left")
    {
        SystemImageGlyph::ArrowLeft
    } else if lower.contains("goright")
        || lower.contains("right")
        || lower.contains("chevron.right")
        || lower.contains("arrow.right")
    {
        SystemImageGlyph::ArrowRight
    } else if lower.contains("refresh") || lower.contains("arrow.clockwise") {
        SystemImageGlyph::Refresh
    } else if lower.contains("share")
        || lower.contains("action")
        || lower.contains("square.and.arrow")
    {
        SystemImageGlyph::Share
    } else if lower.contains("info") {
        SystemImageGlyph::Info
    } else if lower.contains("stop") || lower.contains("invalid") || lower.contains("xmark") {
        SystemImageGlyph::Stop
    } else if lower.contains("user") || lower.contains("person") || lower.contains("account") {
        SystemImageGlyph::User
    } else if lower.contains("quicklook")
        || lower.contains("reveal")
        || lower.contains("view")
        || lower.contains("eye")
    {
        SystemImageGlyph::View
    } else if lower.starts_with("nsimagename")
        || lower.starts_with("nstouchbar")
        || lower.ends_with("template")
    {
        SystemImageGlyph::Generic
    } else if allow_symbol_name
        && lower
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        SystemImageGlyph::Generic
    } else {
        return None;
    };

    Some(SystemImageSpec { glyph, template })
}

fn put_pixel(data: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let idx = ((y as u32 * width + x as u32) * 4) as usize;
    data[idx..idx + 4].copy_from_slice(&color);
}

fn fill_rect(
    data: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: [u8; 4],
) {
    for yy in y..(y + h) {
        for xx in x..(x + w) {
            put_pixel(data, width, height, xx, yy, color);
        }
    }
}

fn draw_line(
    data: &mut [u8],
    width: u32,
    height: u32,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 4],
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        for oy in -1..=1 {
            for ox in -1..=1 {
                put_pixel(data, width, height, x0 + ox, y0 + oy, color);
            }
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn draw_circle_outline(
    data: &mut [u8],
    width: u32,
    height: u32,
    cx: i32,
    cy: i32,
    r: i32,
    color: [u8; 4],
) {
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            let d = (x - cx) * (x - cx) + (y - cy) * (y - cy);
            if d >= (r - 1) * (r - 1) && d <= (r + 1) * (r + 1) {
                put_pixel(data, width, height, x, y, color);
            }
        }
    }
}

fn fill_circle(data: &mut [u8], width: u32, height: u32, cx: i32, cy: i32, r: i32, color: [u8; 4]) {
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r {
                put_pixel(data, width, height, x, y, color);
            }
        }
    }
}

fn synthetic_system_image_buffer(
    name: &str,
    allow_symbol_name: bool,
) -> Option<(graphics_core::ImageBuffer, bool)> {
    let spec = system_image_spec(name, allow_symbol_name)?;
    let width = 18;
    let height = 18;
    let mut data = vec![0; (width * height * 4) as usize];
    let ink = if spec.template {
        [24, 24, 27, 230]
    } else {
        [32, 96, 180, 240]
    };
    let accent = [255, 204, 58, 245];
    let danger = [220, 64, 54, 245];

    match spec.glyph {
        SystemImageGlyph::Plus => {
            fill_rect(&mut data, width, height, 8, 4, 3, 10, ink);
            fill_rect(&mut data, width, height, 4, 8, 10, 3, ink);
        }
        SystemImageGlyph::Minus => fill_rect(&mut data, width, height, 4, 8, 10, 3, ink),
        SystemImageGlyph::Check => {
            draw_line(&mut data, width, height, 4, 9, 8, 13, ink);
            draw_line(&mut data, width, height, 8, 13, 14, 5, ink);
        }
        SystemImageGlyph::Caution => {
            for y in 3..15 {
                let inset = (y - 3) / 2;
                fill_rect(
                    &mut data,
                    width,
                    height,
                    8 - inset,
                    y,
                    inset * 2 + 2,
                    1,
                    accent,
                );
            }
            fill_rect(&mut data, width, height, 8, 7, 2, 5, danger);
            fill_rect(&mut data, width, height, 8, 13, 2, 2, danger);
        }
        SystemImageGlyph::Folder => {
            fill_rect(&mut data, width, height, 2, 6, 14, 9, [232, 178, 64, 245]);
            fill_rect(&mut data, width, height, 3, 4, 6, 3, [245, 204, 92, 245]);
            draw_line(&mut data, width, height, 2, 6, 15, 6, [170, 120, 44, 245]);
        }
        SystemImageGlyph::Gear => {
            draw_circle_outline(&mut data, width, height, 9, 9, 5, ink);
            fill_circle(&mut data, width, height, 9, 9, 2, ink);
            fill_rect(&mut data, width, height, 8, 2, 2, 4, ink);
            fill_rect(&mut data, width, height, 8, 12, 2, 4, ink);
            fill_rect(&mut data, width, height, 2, 8, 4, 2, ink);
            fill_rect(&mut data, width, height, 12, 8, 4, 2, ink);
        }
        SystemImageGlyph::Trash => {
            fill_rect(&mut data, width, height, 5, 7, 9, 8, ink);
            fill_rect(&mut data, width, height, 4, 5, 11, 2, ink);
            fill_rect(&mut data, width, height, 7, 3, 5, 2, ink);
        }
        SystemImageGlyph::Lock | SystemImageGlyph::Unlock => {
            if matches!(spec.glyph, SystemImageGlyph::Lock) {
                draw_circle_outline(&mut data, width, height, 9, 8, 4, ink);
            } else {
                draw_circle_outline(&mut data, width, height, 12, 8, 4, ink);
            }
            fill_rect(&mut data, width, height, 5, 8, 9, 7, ink);
        }
        SystemImageGlyph::ArrowLeft => {
            draw_line(&mut data, width, height, 4, 9, 14, 9, ink);
            draw_line(&mut data, width, height, 4, 9, 9, 4, ink);
            draw_line(&mut data, width, height, 4, 9, 9, 14, ink);
        }
        SystemImageGlyph::ArrowRight => {
            draw_line(&mut data, width, height, 4, 9, 14, 9, ink);
            draw_line(&mut data, width, height, 14, 9, 9, 4, ink);
            draw_line(&mut data, width, height, 14, 9, 9, 14, ink);
        }
        SystemImageGlyph::Refresh => {
            draw_circle_outline(&mut data, width, height, 9, 9, 5, ink);
            draw_line(&mut data, width, height, 13, 4, 15, 4, ink);
            draw_line(&mut data, width, height, 13, 4, 13, 7, ink);
        }
        SystemImageGlyph::Share => {
            draw_line(&mut data, width, height, 9, 3, 9, 12, ink);
            draw_line(&mut data, width, height, 9, 3, 5, 7, ink);
            draw_line(&mut data, width, height, 9, 3, 13, 7, ink);
            fill_rect(&mut data, width, height, 4, 11, 10, 4, ink);
        }
        SystemImageGlyph::Info => {
            draw_circle_outline(&mut data, width, height, 9, 9, 6, ink);
            fill_rect(&mut data, width, height, 8, 8, 2, 6, ink);
            fill_rect(&mut data, width, height, 8, 5, 2, 2, ink);
        }
        SystemImageGlyph::Stop => {
            draw_line(&mut data, width, height, 5, 5, 13, 13, danger);
            draw_line(&mut data, width, height, 13, 5, 5, 13, danger);
        }
        SystemImageGlyph::User => {
            fill_circle(&mut data, width, height, 9, 6, 3, ink);
            fill_circle(&mut data, width, height, 9, 14, 6, ink);
        }
        SystemImageGlyph::View => {
            draw_line(&mut data, width, height, 3, 9, 7, 5, ink);
            draw_line(&mut data, width, height, 7, 5, 11, 5, ink);
            draw_line(&mut data, width, height, 11, 5, 15, 9, ink);
            draw_line(&mut data, width, height, 15, 9, 11, 13, ink);
            draw_line(&mut data, width, height, 11, 13, 7, 13, ink);
            draw_line(&mut data, width, height, 7, 13, 3, 9, ink);
            fill_circle(&mut data, width, height, 9, 9, 2, ink);
        }
        SystemImageGlyph::Generic => {
            draw_circle_outline(&mut data, width, height, 9, 9, 6, ink);
            fill_rect(&mut data, width, height, 8, 4, 2, 10, ink);
            fill_rect(&mut data, width, height, 4, 8, 10, 2, ink);
        }
    }

    graphics_core::ImageBuffer::from_rgba(width, height, data).map(|image| (image, spec.template))
}

fn alloc_image_from_buffer(
    name: &str,
    img: graphics_core::ImageBuffer,
    file_path: Option<String>,
    template: bool,
) -> u64 {
    let obj = alloc_image_object();
    if obj == 0 {
        return 0;
    }
    init_image_from_buffer(obj, img, file_path, Some(name), template)
}

fn image_named_in_bundle(bundle: u64, name: &str) -> u64 {
    let name = name.trim();
    if name.is_empty() {
        return 0;
    }

    if bundle != 0 {
        if let Some(path) = resolve_named_image_path(bundle, name) {
            let obj = alloc_image_object();
            if obj == 0 {
                return 0;
            }
            return init_image_from_path(obj, &path, Some(name));
        }

        if name == "NSApplicationIcon" {
            if let Some((image, template)) = resolve_application_icon_asset(bundle) {
                return alloc_image_from_buffer(name, image, None, template);
            }
            if let Some((path, template)) = resolve_application_icon_source_asset(bundle) {
                let obj = alloc_image_object();
                if obj == 0 {
                    return 0;
                }
                return init_image_from_path_with_template(obj, &path, Some(name), template);
            }
        }

        if let Some((image, template)) = decode_asset_catalog_image(bundle, name) {
            return alloc_image_from_buffer(name, image, None, template);
        }

        if let Some((path, template)) = resolve_xcassets_image_path(bundle, name) {
            let obj = alloc_image_object();
            if obj == 0 {
                return 0;
            }
            return init_image_from_path_with_template(obj, &path, Some(name), template);
        }
    }

    if let Some((image, template)) = synthetic_system_image_buffer(name, false) {
        return alloc_image_from_buffer(name, image, None, template);
    }

    0
}

/// `+[NSImage imageNamed:]` — load a named image from the main bundle.
pub extern "C" fn shim_nsimage_image_named(_cls: u64, _cmd: u64, name: u64) -> u64 {
    let Some(name) = crate::cfstring::extract_cfstring_cstr(name) else {
        return 0;
    };
    let image = image_named_in_bundle(crate::nsbundle::get_or_create_main_bundle(), name);
    if image == 0 {
        log::debug!("NSImage imageNamed: '{}' not found", name);
    }
    image
}

/// `+[NSImage imageWithSystemSymbolName:accessibilityDescription:]`.
pub extern "C" fn shim_nsimage_image_with_system_symbol_name(
    _cls: u64,
    _cmd: u64,
    name: u64,
    _accessibility_description: u64,
) -> u64 {
    let Some(name) = crate::cfstring::extract_cfstring_cstr(name) else {
        return 0;
    };
    let Some((image, template)) = synthetic_system_image_buffer(name, true) else {
        return 0;
    };
    alloc_image_from_buffer(name, image, None, template)
}

/// `-[NSImage initWithCoder:]`.
///
/// Compiled nibs often archive named images rather than file paths. Resolve
/// those names through the same bundle/system-image path used by
/// `+[NSImage imageNamed:]` so image-backed controls get drawable pixels after
/// unarchiving.
pub extern "C" fn shim_nsimage_init_with_coder(obj: u64, _cmd: u64, coder: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let template = crate::nskeyedarchiver::decode_bool_for_any_key(
        coder,
        &[
            "NSTemplate",
            "NSIsTemplate",
            "template",
            "isTemplate",
            "_template",
        ],
    );

    let data = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSData", "NS.data", "data", "_data", "NSImageData"],
    );
    if data != 0 {
        let image = shim_nsimage_init_with_data(obj, 0, data);
        if let Some(template) = template {
            shim_nsimage_set_template(image, 0, u8::from(template));
        }
        return image;
    }

    if let Some(path) = crate::nskeyedarchiver::decode_string_for_any_key(
        coder,
        &["NSPath", "NSFilePath", "filePath", "path", "_path"],
    ) {
        let path_obj = make_nsstring_from_str(&path);
        let image = shim_nsimage_init_with_contents_of_file(obj, 0, path_obj);
        if let Some(template) = template {
            shim_nsimage_set_template(image, 0, u8::from(template));
        }
        return image;
    }

    if let Some(symbol) = crate::nskeyedarchiver::decode_string_for_any_key(
        coder,
        &[
            "NSSymbolName",
            "NSSystemSymbolName",
            "systemSymbolName",
            "symbolName",
            "_symbolName",
        ],
    ) {
        let symbol_obj = make_nsstring_from_str(&symbol);
        let image = shim_nsimage_image_with_system_symbol_name(0, 0, symbol_obj, 0);
        if image != 0 {
            if let Some(template) = template {
                shim_nsimage_set_template(image, 0, u8::from(template));
            }
            return image;
        }
        return init_empty_image(obj, Some(&symbol), template.unwrap_or(false));
    }

    if let Some(name) = crate::nskeyedarchiver::decode_string_for_any_key(
        coder,
        &[
            "NSName",
            "NS.name",
            "name",
            "_name",
            "NSImageName",
            "NSResourceName",
            "NSImageResourceName",
            "resourceName",
            "imageName",
        ],
    ) {
        let name_obj = make_nsstring_from_str(&name);
        let image = shim_nsimage_image_named(0, 0, name_obj);
        if image != 0 {
            if let Some(template) = template {
                shim_nsimage_set_template(image, 0, u8::from(template));
            }
            return image;
        }
        return init_empty_image(obj, Some(&name), template.unwrap_or(false));
    }

    init_empty_image(obj, None, template.unwrap_or(false))
}

/// `-[NSImage initWithContentsOfFile:]` decodes the image and stores a CGImage handle.
pub extern "C" fn shim_nsimage_init_with_contents_of_file(
    obj: u64,
    _cmd: u64,
    path_ptr: u64,
) -> u64 {
    // Extract NSString content from the runtime if possible.
    let path_str = crate::cfstring::extract_cfstring_cstr(path_ptr).map(|s| s.to_string());
    log::debug!("NSImage initWithContentsOfFile: {:?}", path_str);

    if let Some(ref path) = path_str {
        return init_image_from_path(obj, Path::new(path), None);
    }

    insert_image_info(
        obj,
        ImageInfo {
            file_path: None,
            name: None,
            template: false,
            width: 0.0,
            height: 0.0,
            cg_handle: 0,
        },
    );
    obj
}

/// `-[NSImage initWithData:]` — decodes image data and stores a CGImage handle.
pub extern "C" fn shim_nsimage_init_with_data(obj: u64, _cmd: u64, data_ptr: u64) -> u64 {
    log::debug!("NSImage initWithData:");

    let (w, h, cg_handle) = if data_ptr != 0 && data_ptr % 8 == 0 {
        // Try to read NSData bytes (only if pointer is aligned, i.e. plausibly valid).
        let bytes = crate::nsdata::nsdata_bytes_slice(data_ptr);
        if !bytes.is_empty() {
            if let Some(img) = graphics_core::ImageBuffer::from_encoded(bytes) {
                let w = img.width() as f64;
                let h = img.height() as f64;
                let handle = cg_shims::bridge::image_create_from_buffer(img);
                (w, h, handle)
            } else {
                (0.0, 0.0, 0)
            }
        } else {
            (0.0, 0.0, 0)
        }
    } else {
        (0.0, 0.0, 0)
    };

    insert_image_info(
        obj,
        ImageInfo {
            file_path: None,
            name: None,
            template: false,
            width: w,
            height: h,
            cg_handle,
        },
    );
    obj
}

/// `-[NSImage size]` — returns stored width (compact struct return).
pub extern "C" fn shim_nsimage_size(_obj: u64, _cmd: u64) -> f64 {
    let guard = store();
    if let Some(map) = guard.as_ref() {
        if let Some(info) = map.get(&_obj) {
            return info.width;
        }
    }
    0.0
}

/// `-[NSImage setSize:]` — stores dimensions.
pub extern "C" fn shim_nsimage_set_size(obj: u64, _cmd: u64, w: f64, h: f64) {
    log::debug!("NSImage setSize: {}×{}", w, h);
    let mut guard = store();
    let map = ensure_store(&mut guard);
    if let Some(info) = map.get_mut(&obj) {
        info.width = w;
        info.height = h;
    } else {
        map.insert(
            obj,
            ImageInfo {
                file_path: None,
                name: None,
                template: false,
                width: w,
                height: h,
                cg_handle: 0,
            },
        );
    }
}

/// `-[NSImage isTemplate]`.
pub extern "C" fn shim_nsimage_is_template(obj: u64, _cmd: u64) -> u8 {
    let guard = store();
    guard
        .as_ref()
        .and_then(|map| map.get(&obj))
        .map(|info| u8::from(info.template))
        .unwrap_or(0)
}

/// `-[NSImage setTemplate:]`.
pub extern "C" fn shim_nsimage_set_template(obj: u64, _cmd: u64, value: u8) {
    let mut guard = store();
    let map = ensure_store(&mut guard);
    if let Some(info) = map.get_mut(&obj) {
        info.template = value != 0;
    } else {
        map.insert(
            obj,
            ImageInfo {
                file_path: None,
                name: None,
                template: value != 0,
                width: 0.0,
                height: 0.0,
                cg_handle: 0,
            },
        );
    }
}

/// `-[NSImage name]` — returns the stored image name when present.
pub extern "C" fn shim_nsimage_name(obj: u64, _cmd: u64) -> u64 {
    let guard = store();
    guard
        .as_ref()
        .and_then(|map| map.get(&obj))
        .and_then(|info| info.name.as_deref())
        .map(make_nsstring_from_str)
        .unwrap_or(0)
}

/// Get a CGImage handle for an NSImage (for NSImageView rendering).
/// Returns 0 if no pixel data is available.
pub fn image_to_cg_handle(image: u64) -> u64 {
    let guard = store();
    guard
        .as_ref()
        .and_then(|map| map.get(&image))
        .map(|info| info.cg_handle)
        .unwrap_or(0)
}

/// `-[NSImage drawInRect:fromRect:operation:fraction:]` — renders image into current CG context.
pub extern "C" fn shim_nsimage_draw_in_rect(
    obj: u64,
    _cmd: u64,
    dx: f64,
    dy: f64,
    dw: f64,
    dh: f64,
    _sx: f64,
    _sy: f64,
    _sw: f64,
    _sh: f64,
    _op: u64,
    _fraction: f64,
) {
    if obj == 0 {
        return;
    }
    let cg_handle = image_to_cg_handle(obj);
    if cg_handle == 0 {
        log::debug!("NSImage drawInRect: no CGImage handle, skipping");
        return;
    }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }
    cg_shims::bridge::draw_image_in_context(cg_ctx, cg_handle, dx, dy, dw, dh);
}

/// Remove the metadata entry for an NSImage (cleanup helper).
pub fn nsimage_remove(obj: u64) {
    let mut guard = store();
    if let Some(map) = guard.as_mut() {
        if let Some(info) = map.remove(&obj) {
            release_image_info(info);
        }
    }
}

/// Query the stored file path for an NSImage object (used by rendering layer).
#[allow(dead_code)]
pub fn nsimage_file_path(obj: u64) -> Option<String> {
    let guard = store();
    guard
        .as_ref()
        .and_then(|map| map.get(&obj))
        .and_then(|info| info.file_path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // NSImage uses a global HashMap (IMAGE_STORE) keyed by object address,
    // so object layout/ISA doesn't matter for most shims. We use alloc_obj_fake
    // with a minimal 8-byte allocation just to get a unique, valid address.
    const OBJ_SIZE: usize = 8;
    const ONE_PIXEL_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 1, 115, 82, 71, 66, 0, 174, 206, 28, 233, 0, 0, 0, 4,
        103, 65, 77, 65, 0, 0, 177, 143, 11, 252, 97, 5, 0, 0, 0, 9, 112, 72, 89, 115, 0, 0, 14,
        195, 0, 0, 14, 195, 1, 199, 111, 168, 100, 0, 0, 0, 13, 73, 68, 65, 84, 24, 87, 99, 248,
        207, 192, 240, 31, 0, 5, 0, 1, 255, 166, 92, 155, 93, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
        96, 130,
    ];

    fn unique_temp_root(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cf-lite-nsimage-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn make_temp_app_bundle(label: &str) -> (std::path::PathBuf, u64, u64) {
        let root = unique_temp_root(label).join("Sample.app");
        std::fs::create_dir_all(root.join("Contents").join("Resources"))
            .expect("create app resource tree");
        let path = make_string(root.to_string_lossy().as_ref());
        let bundle = crate::nsbundle::shim_nsbundle_bundle_with_path(
            crate::nsobj::nsbundle_class_addr(),
            0,
            path,
        );
        (root, bundle, path)
    }

    fn write_png_imageset(
        image_set: &std::path::Path,
        filename: &str,
        template: bool,
    ) -> std::path::PathBuf {
        std::fs::create_dir_all(image_set).expect("create image set");
        let image_path = image_set.join(filename);
        std::fs::write(&image_path, ONE_PIXEL_PNG).expect("write image set png");
        let contents = if template {
            format!(
                r#"{{
  "images": [
    {{ "idiom": "universal", "filename": "{filename}", "scale": "1x" }}
  ],
  "properties": {{ "template-rendering-intent": "template" }},
  "info": {{ "version": 1, "author": "xcode" }}
}}"#
            )
        } else {
            format!(
                r#"{{
  "images": [
    {{ "idiom": "universal", "filename": "{filename}", "scale": "1x" }}
  ],
  "info": {{ "version": 1, "author": "xcode" }}
}}"#
            )
        };
        std::fs::write(image_set.join("Contents.json"), contents).expect("write Contents.json");
        image_path
    }

    // ── shim_nsimage_image_named ──

    #[test]
    fn image_named_missing_resource_returns_nil() {
        let cls = alloc_obj_fake(OBJ_SIZE);
        let name = make_string("DefinitelyMissingCfLiteImage");
        let result = shim_nsimage_image_named(cls, 0, name);
        assert_eq!(result, 0);
        free_obj(name);
        free_obj(cls);
    }

    #[test]
    fn image_named_nil_name() {
        let result = shim_nsimage_image_named(0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn image_named_system_template_returns_pixel_backed_image() {
        let image = image_named_in_bundle(0, "NSImageNameAddTemplate");
        assert_ne!(image, 0);
        assert_eq!(shim_nsimage_size(image, 0), 18.0);
        assert_eq!(shim_nsimage_is_template(image, 0), 1);
        assert_ne!(image_to_cg_handle(image), 0);
        let name = shim_nsimage_name(image, 0);
        assert_string_eq(name, "NSImageNameAddTemplate");

        free_obj(name);
        nsimage_remove(image);
        free_obj(image);
    }

    #[test]
    fn image_with_system_symbol_name_returns_template_image() {
        let symbol = make_string("square.and.arrow.up");
        let image = shim_nsimage_image_with_system_symbol_name(0, 0, symbol, 0);
        assert_ne!(image, 0);
        assert_eq!(shim_nsimage_size(image, 0), 18.0);
        assert_eq!(shim_nsimage_is_template(image, 0), 1);
        assert_ne!(image_to_cg_handle(image), 0);
        let name = shim_nsimage_name(image, 0);
        assert_string_eq(name, "square.and.arrow.up");

        free_obj(name);
        nsimage_remove(image);
        free_obj(image);
        free_obj(symbol);
    }

    #[test]
    fn image_named_in_bundle_loads_png_resource() {
        let (root, bundle, path) = make_temp_app_bundle("named");
        let image_path = root
            .join("Contents")
            .join("Resources")
            .join("ToolbarIcon.png");
        std::fs::write(&image_path, ONE_PIXEL_PNG).expect("write test image");

        let image = image_named_in_bundle(bundle, "ToolbarIcon");
        assert_ne!(image, 0);
        assert_eq!(
            nsimage_file_path(image),
            Some(image_path.to_string_lossy().to_string())
        );
        assert_eq!(shim_nsimage_size(image, 0), 1.0);
        let name = shim_nsimage_name(image, 0);
        assert_string_eq(name, "ToolbarIcon");
        assert_eq!(shim_nsimage_is_template(image, 0), 0);

        free_obj(name);
        nsimage_remove(image);
        free_obj(image);
        free_obj(bundle);
        free_obj(path);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn image_named_in_bundle_marks_template_named_png_resource() {
        let (root, bundle, path) = make_temp_app_bundle("named-template");
        let image_path = root
            .join("Contents")
            .join("Resources")
            .join("ToolbarTemplate.png");
        std::fs::write(&image_path, ONE_PIXEL_PNG).expect("write test image");

        let image = image_named_in_bundle(bundle, "ToolbarTemplate");
        assert_ne!(image, 0);
        assert_eq!(
            nsimage_file_path(image),
            Some(image_path.to_string_lossy().to_string())
        );
        assert_eq!(shim_nsimage_size(image, 0), 1.0);
        assert_eq!(shim_nsimage_is_template(image, 0), 1);

        nsimage_remove(image);
        free_obj(image);
        free_obj(bundle);
        free_obj(path);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn image_named_in_bundle_loads_xcassets_imageset_resource() {
        let (root, bundle, path) = make_temp_app_bundle("xcassets");
        let image_set = root
            .join("Contents")
            .join("Resources")
            .join("Assets.xcassets")
            .join("ToolbarTemplate.imageset");
        let image_path = write_png_imageset(&image_set, "ToolbarTemplate.png", true);

        let image = image_named_in_bundle(bundle, "ToolbarTemplate");
        assert_ne!(image, 0);
        assert_eq!(
            nsimage_file_path(image),
            Some(image_path.to_string_lossy().to_string())
        );
        assert_eq!(shim_nsimage_size(image, 0), 1.0);
        assert_eq!(shim_nsimage_is_template(image, 0), 1);
        assert_ne!(image_to_cg_handle(image), 0);
        let name = shim_nsimage_name(image, 0);
        assert_string_eq(name, "ToolbarTemplate");

        free_obj(name);
        nsimage_remove(image);
        free_obj(image);
        free_obj(bundle);
        free_obj(path);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn image_named_in_bundle_finds_nested_source_asset_catalog() {
        let (root, bundle, path) = make_temp_app_bundle("nested-xcassets");
        let image_set = root
            .join("Contents")
            .join("Resources")
            .join("Extra.xcassets")
            .join("Toolbar")
            .join("Refresh.imageset");
        let image_path = write_png_imageset(&image_set, "Refresh.png", false);

        let image = image_named_in_bundle(bundle, "Toolbar/Refresh");
        assert_ne!(image, 0);
        assert_eq!(
            nsimage_file_path(image),
            Some(image_path.to_string_lossy().to_string())
        );
        assert_eq!(shim_nsimage_size(image, 0), 1.0);
        assert_eq!(shim_nsimage_is_template(image, 0), 0);

        nsimage_remove(image);
        free_obj(image);
        free_obj(bundle);
        free_obj(path);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn image_named_application_icon_uses_info_plist() {
        let (root, bundle, path) = make_temp_app_bundle("app-icon");
        let image_path = root.join("Contents").join("Resources").join("AppIcon.png");
        std::fs::write(&image_path, ONE_PIXEL_PNG).expect("write test image");
        std::fs::write(
            root.join("Contents").join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIconFile</key><string>AppIcon</string>
</dict></plist>"#,
        )
        .expect("write Info.plist");

        let image = image_named_in_bundle(bundle, "NSApplicationIcon");
        assert_ne!(image, 0);
        assert_eq!(
            nsimage_file_path(image),
            Some(image_path.to_string_lossy().to_string())
        );
        let name = shim_nsimage_name(image, 0);
        assert_string_eq(name, "NSApplicationIcon");

        free_obj(name);
        nsimage_remove(image);
        free_obj(image);
        free_obj(bundle);
        free_obj(path);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn image_named_application_icon_uses_xcassets_icon_name() {
        let (root, bundle, path) = make_temp_app_bundle("app-icon-xcassets");
        let image_set = root
            .join("Contents")
            .join("Resources")
            .join("Assets.xcassets")
            .join("AppIcon.imageset");
        let image_path = write_png_imageset(&image_set, "AppIcon.png", false);
        std::fs::write(
            root.join("Contents").join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIconName</key><string>AppIcon</string>
</dict></plist>"#,
        )
        .expect("write Info.plist");

        let image = image_named_in_bundle(bundle, "NSApplicationIcon");
        assert_ne!(image, 0);
        assert_eq!(
            nsimage_file_path(image),
            Some(image_path.to_string_lossy().to_string())
        );
        assert_eq!(shim_nsimage_size(image, 0), 1.0);
        let name = shim_nsimage_name(image, 0);
        assert_string_eq(name, "NSApplicationIcon");

        free_obj(name);
        nsimage_remove(image);
        free_obj(image);
        free_obj(bundle);
        free_obj(path);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    // ── shim_nsimage_init_with_contents_of_file ──

    #[test]
    fn init_with_contents_of_file_returns_self() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let path = make_string("/tmp/test.png");
        let result = shim_nsimage_init_with_contents_of_file(obj, 0, path);
        assert_eq!(result, obj, "initWithContentsOfFile: should return self");
        // Verify the path was stored.
        assert_eq!(nsimage_file_path(obj), Some("/tmp/test.png".to_string()));
        free_obj(path);
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn init_with_contents_of_file_nil_path() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let result = shim_nsimage_init_with_contents_of_file(obj, 0, 0);
        assert_eq!(result, obj, "should still return self even with nil path");
        // Path should be None since extract_cfstring_cstr(0) returns None.
        assert_eq!(nsimage_file_path(obj), None);
        nsimage_remove(obj);
        free_obj(obj);
    }

    // ── shim_nsimage_init_with_data ──

    #[test]
    fn init_with_data_returns_self() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let result = shim_nsimage_init_with_data(obj, 0, 0x1234);
        assert_eq!(result, obj, "initWithData: should return self");
        // No file path for data-based init.
        assert_eq!(nsimage_file_path(obj), None);
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn init_with_data_nil_data() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let result = shim_nsimage_init_with_data(obj, 0, 0);
        assert_eq!(result, obj);
        nsimage_remove(obj);
        free_obj(obj);
    }

    // ── shim_nsimage_size / shim_nsimage_set_size ──

    #[test]
    fn size_default_zero() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        shim_nsimage_init_with_data(obj, 0, 0);
        let w = shim_nsimage_size(obj, 0);
        assert_eq!(w, 0.0, "default size should be 0");
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn set_size_and_read_back() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        shim_nsimage_init_with_data(obj, 0, 0);
        shim_nsimage_set_size(obj, 0, 128.0, 256.0);
        let w = shim_nsimage_size(obj, 0);
        assert_eq!(w, 128.0, "size should return stored width");
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn set_size_creates_entry_if_missing() {
        // set_size on an obj that was never init'd — should create the entry.
        let obj = alloc_obj_fake(OBJ_SIZE);
        shim_nsimage_set_size(obj, 0, 64.0, 32.0);
        let w = shim_nsimage_size(obj, 0);
        assert_eq!(w, 64.0);
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn template_flag_round_trips() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        shim_nsimage_init_with_data(obj, 0, 0);
        assert_eq!(shim_nsimage_is_template(obj, 0), 0);
        shim_nsimage_set_template(obj, 0, 1);
        assert_eq!(shim_nsimage_is_template(obj, 0), 1);
        shim_nsimage_set_template(obj, 0, 0);
        assert_eq!(shim_nsimage_is_template(obj, 0), 0);
        nsimage_remove(obj);
        free_obj(obj);
    }

    #[test]
    fn size_unknown_object_returns_zero() {
        // An obj pointer that was never registered in the store.
        let obj = alloc_obj_fake(OBJ_SIZE);
        let w = shim_nsimage_size(obj, 0);
        assert_eq!(w, 0.0, "unknown object should return 0.0");
        free_obj(obj);
    }

    // ── shim_nsimage_name ──

    #[test]
    fn name_returns_nil() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        let result = shim_nsimage_name(obj, 0);
        assert_eq!(result, 0, "name should return nil");
        free_obj(obj);
    }

    #[test]
    fn name_nil_obj() {
        let result = shim_nsimage_name(0, 0);
        assert_eq!(result, 0);
    }

    // ── shim_nsimage_draw_in_rect ──

    #[test]
    fn draw_in_rect_does_not_crash() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        // Just verify no panic.
        shim_nsimage_draw_in_rect(
            obj, 0, 0.0, 0.0, 100.0, 100.0, 0.0, 0.0, 100.0, 100.0, 0, 1.0,
        );
        free_obj(obj);
    }

    #[test]
    fn draw_in_rect_nil_obj() {
        // Should not crash with nil obj.
        shim_nsimage_draw_in_rect(0, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0, 0.0);
    }

    // ── nsimage_file_path ──

    #[test]
    fn file_path_none_for_unknown() {
        let obj = alloc_obj_fake(OBJ_SIZE);
        assert_eq!(nsimage_file_path(obj), None);
        free_obj(obj);
    }

    #[test]
    fn file_path_none_for_zero() {
        assert_eq!(nsimage_file_path(0), None);
    }
}
