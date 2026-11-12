//! CoreAnimation shims: CALayer, CAMetalLayer, CATransaction, CADisplayLink.
//!
//! Modern Cocoa apps expect layer-backed views (`wantsLayer = YES`).
//! CALayer now provides a real sublayer tree and stores per-layer
//! visual properties (opacity, cornerRadius, borderWidth, etc.)
//! that the `layer_compositor` module composites via tiny-skia.
//!
//! ## CALayer object layout (48 bytes)
//!
//!   offset  0: isa                 (u64)
//!   offset  8: bounds_x            (f64)
//!   offset 16: bounds_y            (f64)
//!   offset 24: bounds_w            (f64)
//!   offset 32: bounds_h            (f64)
//!   offset 40: background_color    (u64 — CGColorRef, or 0)
//!
//! Extended properties (opacity, hidden, cornerRadius, borderWidth,
//! borderColor, masksToBounds, contents, contentsScale) are stored
//! in the external `LAYER_PROPS` map to keep the 48-byte layout stable.
//!
//! ## CATransaction — class-only, no instance layout.
//! ## CADisplayLink (24 bytes)
//!
//!   offset  0: isa                 (u64)
//!   offset  8: target              (u64)
//!   offset 16: selector            (u64)

use std::collections::HashMap;
use std::sync::Mutex;

use crate::cg_types::CATransform3D;

const BOUNDS_X_OFF: usize = 8;
const BOUNDS_Y_OFF: usize = 16;
const BOUNDS_W_OFF: usize = 24;
const BOUNDS_H_OFF: usize = 32;
const BG_COLOR_OFF: usize = 40;
const LAYER_SIZE: usize = 48;

// ── Extended property storage ──────────────────────────────────

/// Extended per-layer properties that don't fit in the 48-byte object.
#[derive(Clone, Debug)]
pub(crate) struct LayerProps {
    pub opacity: f32,
    pub hidden: bool,
    pub corner_radius: f64,
    pub border_width: f64,
    pub border_color: u64, // CGColorRef handle
    pub shadow_opacity: f32,
    pub masks_to_bounds: bool,
    pub contents: u64, // CGImageRef handle
    pub contents_scale: f64,
    pub needs_display: bool,
    /// CATransform3D stored as a 4×4 column-major matrix.  Only the 2D affine
    /// subset (m11, m12, m21, m22, m41, m42) is applied during compositing.
    pub transform: [f64; 16],
    pub sublayer_transform: [f64; 16],
}

impl Default for LayerProps {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            hidden: false,
            corner_radius: 0.0,
            border_width: 0.0,
            border_color: 0,
            shadow_opacity: 0.0,
            masks_to_bounds: false,
            contents: 0,
            contents_scale: 1.0,
            needs_display: false,
            transform: IDENTITY_TRANSFORM,
            sublayer_transform: IDENTITY_TRANSFORM,
        }
    }
}

/// 4×4 identity matrix in column-major order.
const IDENTITY_TRANSFORM: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Check whether a 4×4 matrix is the identity.
pub(crate) fn is_identity_transform(m: &[f64; 16]) -> bool {
    m == &IDENTITY_TRANSFORM
}

static LAYER_PROPS: Mutex<Option<HashMap<u64, LayerProps>>> = Mutex::new(None);

#[derive(Clone, Debug)]
struct MetalLayerProps {
    device: u64,
    pixel_format: u64,
    framebuffer_only: bool,
    drawable_width: f64,
    drawable_height: f64,
    opaque: bool,
}

impl Default for MetalLayerProps {
    fn default() -> Self {
        Self {
            device: 0,
            pixel_format: 0,
            framebuffer_only: true,
            drawable_width: 0.0,
            drawable_height: 0.0,
            opaque: true,
        }
    }
}

static METAL_LAYER_PROPS: Mutex<Option<HashMap<u64, MetalLayerProps>>> = Mutex::new(None);

fn with_props<R>(layer: u64, f: impl FnOnce(&LayerProps) -> R) -> R {
    let guard = LAYER_PROPS.lock().unwrap_or_else(|e| e.into_inner());
    let default = LayerProps::default();
    let props = guard
        .as_ref()
        .and_then(|m| m.get(&layer))
        .unwrap_or(&default);
    f(props)
}

fn with_props_mut<R>(layer: u64, f: impl FnOnce(&mut LayerProps) -> R) -> R {
    let mut guard = LAYER_PROPS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let props = map.entry(layer).or_default();
    f(props)
}

fn with_metal_props<R>(layer: u64, f: impl FnOnce(&MetalLayerProps) -> R) -> R {
    let guard = METAL_LAYER_PROPS.lock().unwrap_or_else(|e| e.into_inner());
    let default = MetalLayerProps::default();
    let props = guard
        .as_ref()
        .and_then(|m| m.get(&layer))
        .unwrap_or(&default);
    f(props)
}

fn with_metal_props_mut<R>(layer: u64, f: impl FnOnce(&mut MetalLayerProps) -> R) -> R {
    let mut guard = METAL_LAYER_PROPS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let props = map.entry(layer).or_default();
    f(props)
}

/// Get a snapshot of all layer props for a given layer.
pub(crate) fn get_layer_props(layer: u64) -> LayerProps {
    with_props(layer, |p| p.clone())
}

// ── Sublayer tree ──────────────────────────────────────────────

/// sublayer children: parent → ordered Vec<child>
static LAYER_CHILDREN: Mutex<Option<HashMap<u64, Vec<u64>>>> = Mutex::new(None);
/// sublayer parent: child → parent
static LAYER_PARENT: Mutex<Option<HashMap<u64, u64>>> = Mutex::new(None);

/// Get the children of a layer.
pub(crate) fn layer_children(layer: u64) -> Vec<u64> {
    if layer == 0 {
        return vec![];
    }
    let guard = LAYER_CHILDREN.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&layer))
        .cloned()
        .unwrap_or_default()
}

/// Get the superlayer of a layer.
pub(crate) fn layer_superlayer(layer: u64) -> u64 {
    if layer == 0 {
        return 0;
    }
    let guard = LAYER_PARENT.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&layer))
        .copied()
        .unwrap_or(0)
}

fn add_sublayer_internal(parent: u64, child: u64) {
    if parent == 0 || child == 0 || parent == child {
        return;
    }

    // Remove child from its old parent first.
    remove_from_superlayer_internal(child);

    // Add to new parent's children list.
    {
        let mut guard = LAYER_CHILDREN.lock().unwrap_or_else(|e| e.into_inner());
        let map = guard.get_or_insert_with(HashMap::new);
        map.entry(parent).or_default().push(child);
    }

    // Record parent link.
    {
        let mut guard = LAYER_PARENT.lock().unwrap_or_else(|e| e.into_inner());
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(child, parent);
    }
}

fn remove_from_superlayer_internal(child: u64) {
    if child == 0 {
        return;
    }

    let old_parent = {
        let mut guard = LAYER_PARENT.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_mut().and_then(|m| m.remove(&child)).unwrap_or(0)
    };

    if old_parent != 0 {
        let mut guard = LAYER_CHILDREN.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_mut() {
            if let Some(children) = map.get_mut(&old_parent) {
                children.retain(|&c| c != child);
            }
        }
    }
}

/// Reset all CALayer extended state (for runtime reuse between guest runs).
pub(crate) fn reset_layer_state() {
    {
        let mut guard = LAYER_PROPS.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }
    {
        let mut guard = METAL_LAYER_PROPS.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }
    {
        let mut guard = LAYER_CHILDREN.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }
    {
        let mut guard = LAYER_PARENT.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }
}

const DISPLAYLINK_TARGET_OFF: usize = 8;
const DISPLAYLINK_SEL_OFF: usize = 16;
const DISPLAYLINK_SIZE: usize = 24;

// ── CALayer ────────────────────────────────────────────────────

/// Allocate a new layer with zero bounds and no background color.
pub(crate) fn alloc_layer() -> u64 {
    let obj = crate::pal_alloc().calloc(1, LAYER_SIZE) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = crate::nsobj::calayer_class_addr();
        }
    }
    obj
}

pub(crate) fn alloc_metal_layer() -> u64 {
    let obj = alloc_layer();
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = crate::nsobj::cametallayer_class_addr();
        }
    }
    obj
}

/// `-[CALayer init]`
pub extern "C" fn shim_calayer_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return alloc_layer();
    }
    obj
}

/// `-[CALayer bounds]` — returns origin.x (f64). Full CGRect via x8 in real ABI.
pub extern "C" fn shim_calayer_bounds(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + BOUNDS_X_OFF) as *const f64) }
}

/// `-[CALayer setBounds:]`
pub extern "C" fn shim_calayer_set_bounds(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + BOUNDS_X_OFF) as *mut f64) = x;
        *((obj as usize + BOUNDS_Y_OFF) as *mut f64) = y;
        *((obj as usize + BOUNDS_W_OFF) as *mut f64) = w;
        *((obj as usize + BOUNDS_H_OFF) as *mut f64) = h;
    }
}

/// `-[CALayer frame]` — same as bounds for non-transformed layers.
pub extern "C" fn shim_calayer_frame(obj: u64, _cmd: u64) -> f64 {
    shim_calayer_bounds(obj, _cmd)
}

/// `-[CALayer setFrame:]`
pub extern "C" fn shim_calayer_set_frame(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    shim_calayer_set_bounds(obj, _cmd, x, y, w, h);
}

/// `-[CALayer position]` — returns center point x.
pub extern "C" fn shim_calayer_position(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe {
        let x = *((obj as usize + BOUNDS_X_OFF) as *const f64);
        let w = *((obj as usize + BOUNDS_W_OFF) as *const f64);
        x + w / 2.0
    }
}

/// `-[CALayer setPosition:]` — stub, adjusts bounds origin.
pub extern "C" fn shim_calayer_set_position(_obj: u64, _cmd: u64, _x: f64, _y: f64) {
    // no-op for now
}

/// `-[CALayer backgroundColor]`
pub extern "C" fn shim_calayer_background_color(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + BG_COLOR_OFF) as *const u64) }
}

/// `-[CALayer setBackgroundColor:]`
pub extern "C" fn shim_calayer_set_background_color(obj: u64, _cmd: u64, color: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + BG_COLOR_OFF) as *mut u64) = color;
    }
}

/// `-[CALayer setNeedsDisplay]`
pub extern "C" fn shim_calayer_set_needs_display(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.needs_display = true);
}

/// `-[CALayer displayIfNeeded]`
pub extern "C" fn shim_calayer_display_if_needed(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.needs_display = false);
}

/// `-[CALayer setNeedsLayout]` — no-op stub.
pub extern "C" fn shim_calayer_set_needs_layout(_obj: u64, _cmd: u64) {}

/// `-[CALayer layoutIfNeeded]` — no-op stub.
pub extern "C" fn shim_calayer_layout_if_needed(_obj: u64, _cmd: u64) {}

/// `-[CALayer addSublayer:]`
pub extern "C" fn shim_calayer_add_sublayer(obj: u64, _cmd: u64, layer: u64) {
    add_sublayer_internal(obj, layer);
}

/// `-[CALayer removeFromSuperlayer]`
pub extern "C" fn shim_calayer_remove_from_superlayer(obj: u64, _cmd: u64) {
    remove_from_superlayer_internal(obj);
}

/// `-[CALayer sublayers]` — returns an NSArray of child layers.
pub extern "C" fn shim_calayer_sublayers(obj: u64, _cmd: u64) -> u64 {
    let children = layer_children(obj);
    crate::nsarray::make_nsarray(&children)
}

/// `-[CALayer isHidden]`
pub extern "C" fn shim_calayer_is_hidden(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(with_props(obj, |p| p.hidden))
}

/// `-[CALayer setHidden:]`
pub extern "C" fn shim_calayer_set_hidden(obj: u64, _cmd: u64, hidden: u8) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.hidden = hidden != 0);
}

/// `-[CALayer opacity]`
pub extern "C" fn shim_calayer_opacity(obj: u64, _cmd: u64) -> f32 {
    if obj == 0 {
        return 1.0;
    }
    with_props(obj, |p| p.opacity)
}

/// `-[CALayer setOpacity:]`
pub extern "C" fn shim_calayer_set_opacity(obj: u64, _cmd: u64, opacity: f32) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.opacity = opacity);
}

/// `-[CALayer cornerRadius]`
pub extern "C" fn shim_calayer_corner_radius(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    with_props(obj, |p| p.corner_radius)
}

/// `-[CALayer setCornerRadius:]`
pub extern "C" fn shim_calayer_set_corner_radius(obj: u64, _cmd: u64, r: f64) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.corner_radius = r);
}

/// `-[CALayer borderWidth]`
pub extern "C" fn shim_calayer_border_width(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    with_props(obj, |p| p.border_width)
}

/// `-[CALayer setBorderWidth:]`
pub extern "C" fn shim_calayer_set_border_width(obj: u64, _cmd: u64, w: f64) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.border_width = w);
}

/// `-[CALayer borderColor]`
pub extern "C" fn shim_calayer_border_color(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_props(obj, |p| p.border_color)
}

/// `-[CALayer setBorderColor:]`
pub extern "C" fn shim_calayer_set_border_color(obj: u64, _cmd: u64, color: u64) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.border_color = color);
}

/// `-[CALayer shadowOpacity]`
pub extern "C" fn shim_calayer_shadow_opacity(obj: u64, _cmd: u64) -> f32 {
    if obj == 0 {
        return 0.0;
    }
    with_props(obj, |p| p.shadow_opacity)
}

/// `-[CALayer setShadowOpacity:]`
pub extern "C" fn shim_calayer_set_shadow_opacity(obj: u64, _cmd: u64, opacity: f32) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.shadow_opacity = opacity);
}

/// `-[CALayer contents]`
pub extern "C" fn shim_calayer_contents(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_props(obj, |p| p.contents)
}

/// `-[CALayer setContents:]`
pub extern "C" fn shim_calayer_set_contents(obj: u64, _cmd: u64, contents: u64) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.contents = contents);
}

/// `-[CALayer contentsScale]`
pub extern "C" fn shim_calayer_contents_scale(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 1.0;
    }
    with_props(obj, |p| p.contents_scale)
}

/// `-[CALayer setContentsScale:]`
pub extern "C" fn shim_calayer_set_contents_scale(obj: u64, _cmd: u64, scale: f64) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.contents_scale = scale);
}

/// `-[CALayer transform]` — returns CATransform3D (16 f64 values, 128 bytes).
/// On ARM64 this is returned via x8 indirect-result pointer.
pub extern "C" fn shim_calayer_transform(obj: u64, _cmd: u64) -> CATransform3D {
    if obj == 0 {
        return IDENTITY_TRANSFORM.into();
    }
    with_props(obj, |p| p.transform.into())
}

/// `-[CALayer setTransform:]`
pub extern "C" fn shim_calayer_set_transform(obj: u64, _cmd: u64, transform: CATransform3D) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.transform = transform.into());
}

/// `-[CALayer sublayerTransform]`
pub extern "C" fn shim_calayer_sublayer_transform(obj: u64, _cmd: u64) -> CATransform3D {
    if obj == 0 {
        return IDENTITY_TRANSFORM.into();
    }
    with_props(obj, |p| p.sublayer_transform.into())
}

/// `-[CALayer setSublayerTransform:]`
pub extern "C" fn shim_calayer_set_sublayer_transform(
    obj: u64,
    _cmd: u64,
    transform: CATransform3D,
) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.sublayer_transform = transform.into());
}

/// `-[CALayer masksToBounds]`
pub extern "C" fn shim_calayer_masks_to_bounds(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(with_props(obj, |p| p.masks_to_bounds))
}

/// `-[CALayer setMasksToBounds:]`
pub extern "C" fn shim_calayer_set_masks_to_bounds(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    with_props_mut(obj, |p| p.masks_to_bounds = flag != 0);
}

/// `-[CALayer superlayer]`
pub extern "C" fn shim_calayer_superlayer(obj: u64, _cmd: u64) -> u64 {
    layer_superlayer(obj)
}

// ── CAMetalLayer ───────────────────────────────────────────────

/// `-[CAMetalLayer init]`
pub extern "C" fn shim_cametallayer_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return alloc_metal_layer();
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::cametallayer_class_addr();
    }
    obj
}

/// `-[CAMetalLayer device]`
pub extern "C" fn shim_cametallayer_device(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_metal_props(obj, |props| props.device)
}

/// `-[CAMetalLayer setDevice:]`
pub extern "C" fn shim_cametallayer_set_device(obj: u64, _cmd: u64, device: u64) {
    if obj == 0 {
        return;
    }
    with_metal_props_mut(obj, |props| props.device = device);
}

/// `-[CAMetalLayer pixelFormat]`
pub extern "C" fn shim_cametallayer_pixel_format(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_metal_props(obj, |props| props.pixel_format)
}

/// `-[CAMetalLayer setPixelFormat:]`
pub extern "C" fn shim_cametallayer_set_pixel_format(obj: u64, _cmd: u64, pixel_format: u64) {
    if obj == 0 {
        return;
    }
    with_metal_props_mut(obj, |props| props.pixel_format = pixel_format);
}

/// `-[CAMetalLayer framebufferOnly]`
pub extern "C" fn shim_cametallayer_framebuffer_only(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    with_metal_props(obj, |props| u8::from(props.framebuffer_only))
}

/// `-[CAMetalLayer setFramebufferOnly:]`
pub extern "C" fn shim_cametallayer_set_framebuffer_only(
    obj: u64,
    _cmd: u64,
    framebuffer_only: u8,
) {
    if obj == 0 {
        return;
    }
    with_metal_props_mut(obj, |props| props.framebuffer_only = framebuffer_only != 0);
}

/// `-[CAMetalLayer setDrawableSize:]`
pub extern "C" fn shim_cametallayer_set_drawable_size(
    obj: u64,
    _cmd: u64,
    width: f64,
    height: f64,
) {
    if obj == 0 {
        return;
    }
    with_metal_props_mut(obj, |props| {
        props.drawable_width = width;
        props.drawable_height = height;
    });
}

/// `-[CAMetalLayer opaque]`
pub extern "C" fn shim_cametallayer_opaque(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    with_metal_props(obj, |props| u8::from(props.opaque))
}

/// `-[CAMetalLayer isOpaque]`
pub extern "C" fn shim_cametallayer_is_opaque(obj: u64, _cmd: u64) -> u8 {
    shim_cametallayer_opaque(obj, _cmd)
}

/// `-[CAMetalLayer setOpaque:]`
pub extern "C" fn shim_cametallayer_set_opaque(obj: u64, _cmd: u64, opaque: u8) {
    if obj == 0 {
        return;
    }
    with_metal_props_mut(obj, |props| props.opaque = opaque != 0);
}

/// `-[CAMetalLayer nextDrawable]` — returns nil until Metal drawable plumbing exists.
pub extern "C" fn shim_cametallayer_next_drawable(_obj: u64, _cmd: u64) -> u64 {
    0
}

// ── CATransaction ──────────────────────────────────────────────

/// `+[CATransaction begin]` — no-op stub.
pub extern "C" fn shim_catransaction_begin(_cls: u64, _cmd: u64) {
    log::trace!("CATransaction begin (stub)");
}

/// `+[CATransaction commit]` — no-op stub.
pub extern "C" fn shim_catransaction_commit(_cls: u64, _cmd: u64) {
    log::trace!("CATransaction commit (stub)");
}

/// `+[CATransaction flush]` — no-op stub.
pub extern "C" fn shim_catransaction_flush(_cls: u64, _cmd: u64) {}

/// `+[CATransaction setDisableActions:]` — no-op stub.
pub extern "C" fn shim_catransaction_set_disable_actions(_cls: u64, _cmd: u64, _flag: u8) {}

/// `+[CATransaction disableActions]` — returns NO.
pub extern "C" fn shim_catransaction_disable_actions(_cls: u64, _cmd: u64) -> u8 {
    0
}

/// `+[CATransaction animationDuration]` — returns 0.25.
pub extern "C" fn shim_catransaction_animation_duration(_cls: u64, _cmd: u64) -> f64 {
    0.25
}

/// `+[CATransaction setAnimationDuration:]` — no-op stub.
pub extern "C" fn shim_catransaction_set_animation_duration(_cls: u64, _cmd: u64, _dur: f64) {}

/// `+[CATransaction setCompletionBlock:]` — no-op stub.
pub extern "C" fn shim_catransaction_set_completion_block(_cls: u64, _cmd: u64, _block: u64) {}

// ── CADisplayLink ──────────────────────────────────────────────

/// `+[CADisplayLink displayLinkWithTarget:selector:]`
pub extern "C" fn shim_cadisplaylink_create(
    _cls: u64,
    _cmd: u64,
    target: u64,
    selector: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, DISPLAYLINK_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::cadisplaylink_class_addr();
        *((obj as usize + DISPLAYLINK_TARGET_OFF) as *mut u64) = target;
        *((obj as usize + DISPLAYLINK_SEL_OFF) as *mut u64) = selector;
    }
    obj
}

/// `-[CADisplayLink addToRunLoop:forMode:]` — no-op stub.
pub extern "C" fn shim_cadisplaylink_add_to_run_loop(
    _obj: u64,
    _cmd: u64,
    _run_loop: u64,
    _mode: u64,
) {
}

/// `-[CADisplayLink invalidate]` — no-op stub.
pub extern "C" fn shim_cadisplaylink_invalidate(_obj: u64, _cmd: u64) {}

/// `-[CADisplayLink isPaused]` — returns YES (paused by default).
pub extern "C" fn shim_cadisplaylink_is_paused(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[CADisplayLink setPaused:]` — no-op stub.
pub extern "C" fn shim_cadisplaylink_set_paused(_obj: u64, _cmd: u64, _paused: u8) {}

/// `-[CADisplayLink timestamp]` — returns 0.
pub extern "C" fn shim_cadisplaylink_timestamp(_obj: u64, _cmd: u64) -> f64 {
    0.0
}

/// `-[CADisplayLink duration]` — returns 1/60.
pub extern "C" fn shim_cadisplaylink_duration(_obj: u64, _cmd: u64) -> f64 {
    1.0 / 60.0
}

// ── CAMediaTimingFunction ──────────────────────────────────────

/// `+[CAMediaTimingFunction functionWithName:]` — returns a sentinel.
pub extern "C" fn shim_camediatimingfunction_with_name(_cls: u64, _cmd: u64, _name: u64) -> u64 {
    // Return a non-nil sentinel.
    static SENTINEL: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *SENTINEL.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = crate::nsobj::camediatimingfunction_class_addr();
            }
        }
        obj
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // --- CALayer ---

    #[test]
    fn layer_alloc_has_isa() {
        let layer = alloc_layer();
        assert_ne!(layer, 0);
        let isa = unsafe { *(layer as *const u64) };
        assert_eq!(isa, crate::nsobj::calayer_class_addr());
        free_obj(layer);
    }

    #[test]
    fn layer_set_bounds_roundtrip() {
        let layer = alloc_layer();
        shim_calayer_set_bounds(layer, 0, 10.0, 20.0, 300.0, 400.0);
        let x = unsafe { *((layer as usize + BOUNDS_X_OFF) as *const f64) };
        let h = unsafe { *((layer as usize + BOUNDS_H_OFF) as *const f64) };
        assert!((x - 10.0).abs() < f64::EPSILON);
        assert!((h - 400.0).abs() < f64::EPSILON);
        free_obj(layer);
    }

    #[test]
    fn layer_background_color_roundtrip() {
        let layer = alloc_layer();
        shim_calayer_set_background_color(layer, 0, 0xCAFE);
        assert_eq!(shim_calayer_background_color(layer, 0), 0xCAFE);
        free_obj(layer);
    }

    #[test]
    fn layer_nil_safety() {
        assert!((shim_calayer_bounds(0, 0)).abs() < f64::EPSILON);
        shim_calayer_set_bounds(0, 0, 1.0, 2.0, 3.0, 4.0);
        assert_eq!(shim_calayer_background_color(0, 0), 0);
    }

    #[test]
    fn layer_transform_aggregate_roundtrip() {
        let layer = alloc_layer();
        let transform = CATransform3D {
            m11: 1.0,
            m12: 2.0,
            m13: 3.0,
            m14: 4.0,
            m21: 5.0,
            m22: 6.0,
            m23: 7.0,
            m24: 8.0,
            m31: 9.0,
            m32: 10.0,
            m33: 11.0,
            m34: 12.0,
            m41: 13.0,
            m42: 14.0,
            m43: 15.0,
            m44: 16.0,
        };
        let values = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let set: extern "C" fn(u64, u64, CATransform3D) = shim_calayer_set_transform;
        let get: extern "C" fn(u64, u64) -> CATransform3D = shim_calayer_transform;
        let set_sublayer: extern "C" fn(u64, u64, CATransform3D) =
            shim_calayer_set_sublayer_transform;
        let get_sublayer: extern "C" fn(u64, u64) -> CATransform3D =
            shim_calayer_sublayer_transform;

        // Check the named-field order against the compositor's existing matrix layout.
        set(layer, 0, transform);
        assert_eq!(get_layer_props(layer).transform, values);
        assert_eq!(get(layer, 0), transform);
        let sublayer_values = values.map(|value| -value);
        let sublayer_transform = CATransform3D::from(sublayer_values);
        set_sublayer(layer, 0, sublayer_transform);
        assert_eq!(get_layer_props(layer).sublayer_transform, sublayer_values);
        assert_eq!(get_sublayer(layer, 0), sublayer_transform);
        assert_eq!(get(layer, 0), transform);

        // Do not leave transform state behind if another test reuses the allocation.
        with_props_mut(layer, |props| {
            props.transform = IDENTITY_TRANSFORM;
            props.sublayer_transform = IDENTITY_TRANSFORM;
        });
        free_obj(layer);
    }

    #[test]
    fn nil_transform_aggregates_preserve_identity() {
        let identity = CATransform3D::from(IDENTITY_TRANSFORM);
        shim_calayer_set_transform(0, 0, CATransform3D::from([2.0; 16]));
        shim_calayer_set_sublayer_transform(0, 0, CATransform3D::from([3.0; 16]));
        assert_eq!(shim_calayer_transform(0, 0), identity);
        assert_eq!(shim_calayer_sublayer_transform(0, 0), identity);
    }

    #[test]
    fn layer_opacity_roundtrip() {
        let layer = alloc_layer();
        assert!((shim_calayer_opacity(layer, 0) - 1.0).abs() < f32::EPSILON);
        shim_calayer_set_opacity(layer, 0, 0.5);
        assert!((shim_calayer_opacity(layer, 0) - 0.5).abs() < f32::EPSILON);
        free_obj(layer);
    }

    #[test]
    fn layer_hidden_roundtrip() {
        let layer = alloc_layer();
        assert_eq!(shim_calayer_is_hidden(layer, 0), 0);
        shim_calayer_set_hidden(layer, 0, 1);
        assert_eq!(shim_calayer_is_hidden(layer, 0), 1);
        shim_calayer_set_hidden(layer, 0, 0);
        assert_eq!(shim_calayer_is_hidden(layer, 0), 0);
        free_obj(layer);
    }

    #[test]
    fn layer_corner_radius_roundtrip() {
        let layer = alloc_layer();
        assert!((shim_calayer_corner_radius(layer, 0)).abs() < f64::EPSILON);
        shim_calayer_set_corner_radius(layer, 0, 12.0);
        assert!((shim_calayer_corner_radius(layer, 0) - 12.0).abs() < f64::EPSILON);
        free_obj(layer);
    }

    #[test]
    fn layer_border_roundtrip() {
        let layer = alloc_layer();
        assert!((shim_calayer_border_width(layer, 0)).abs() < f64::EPSILON);
        shim_calayer_set_border_width(layer, 0, 2.0);
        assert!((shim_calayer_border_width(layer, 0) - 2.0).abs() < f64::EPSILON);
        assert_eq!(shim_calayer_border_color(layer, 0), 0);
        shim_calayer_set_border_color(layer, 0, 0xBEEF);
        assert_eq!(shim_calayer_border_color(layer, 0), 0xBEEF);
        free_obj(layer);
    }

    #[test]
    fn layer_masks_to_bounds_roundtrip() {
        let layer = alloc_layer();
        assert_eq!(shim_calayer_masks_to_bounds(layer, 0), 0);
        shim_calayer_set_masks_to_bounds(layer, 0, 1);
        assert_eq!(shim_calayer_masks_to_bounds(layer, 0), 1);
        free_obj(layer);
    }

    #[test]
    fn layer_contents_roundtrip() {
        let layer = alloc_layer();
        assert_eq!(shim_calayer_contents(layer, 0), 0);
        shim_calayer_set_contents(layer, 0, 0xDEAD);
        assert_eq!(shim_calayer_contents(layer, 0), 0xDEAD);
        free_obj(layer);
    }

    #[test]
    fn layer_contents_scale_roundtrip() {
        let layer = alloc_layer();
        assert!((shim_calayer_contents_scale(layer, 0) - 1.0).abs() < f64::EPSILON);
        shim_calayer_set_contents_scale(layer, 0, 2.0);
        assert!((shim_calayer_contents_scale(layer, 0) - 2.0).abs() < f64::EPSILON);
        free_obj(layer);
    }

    // --- Sublayer tree ---

    #[test]
    fn add_sublayer_builds_tree() {
        let parent = alloc_layer();
        let child1 = alloc_layer();
        let child2 = alloc_layer();

        shim_calayer_add_sublayer(parent, 0, child1);
        shim_calayer_add_sublayer(parent, 0, child2);

        let children = layer_children(parent);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0], child1);
        assert_eq!(children[1], child2);
        assert_eq!(layer_superlayer(child1), parent);
        assert_eq!(layer_superlayer(child2), parent);

        // sublayers shim returns NSArray
        let arr = shim_calayer_sublayers(parent, 0);
        assert_ne!(arr, 0);

        // Clean up
        remove_from_superlayer_internal(child1);
        remove_from_superlayer_internal(child2);
        free_obj(parent);
        free_obj(child1);
        free_obj(child2);
    }

    #[test]
    fn remove_from_superlayer_works() {
        let parent = alloc_layer();
        let child = alloc_layer();

        shim_calayer_add_sublayer(parent, 0, child);
        assert_eq!(layer_children(parent).len(), 1);

        shim_calayer_remove_from_superlayer(child, 0);
        assert_eq!(layer_children(parent).len(), 0);
        assert_eq!(layer_superlayer(child), 0);

        free_obj(parent);
        free_obj(child);
    }

    #[test]
    fn add_sublayer_reparents() {
        let parent1 = alloc_layer();
        let parent2 = alloc_layer();
        let child = alloc_layer();

        shim_calayer_add_sublayer(parent1, 0, child);
        assert_eq!(layer_superlayer(child), parent1);

        // Adding to a new parent removes from old parent.
        shim_calayer_add_sublayer(parent2, 0, child);
        assert_eq!(layer_children(parent1).len(), 0);
        assert_eq!(layer_children(parent2).len(), 1);
        assert_eq!(layer_superlayer(child), parent2);

        remove_from_superlayer_internal(child);
        free_obj(parent1);
        free_obj(parent2);
        free_obj(child);
    }

    #[test]
    fn superlayer_shim() {
        let parent = alloc_layer();
        let child = alloc_layer();
        assert_eq!(shim_calayer_superlayer(child, 0), 0);
        shim_calayer_add_sublayer(parent, 0, child);
        assert_eq!(shim_calayer_superlayer(child, 0), parent);
        remove_from_superlayer_internal(child);
        free_obj(parent);
        free_obj(child);
    }

    // --- CAMetalLayer ---

    #[test]
    fn metal_layer_alloc_has_subclass_isa() {
        let layer = alloc_metal_layer();
        assert_ne!(layer, 0);
        let isa = unsafe { *(layer as *const u64) };
        assert_eq!(isa, crate::nsobj::cametallayer_class_addr());
        free_obj(layer);
    }

    #[test]
    fn metal_layer_properties_roundtrip() {
        let layer = alloc_metal_layer();

        shim_cametallayer_set_device(layer, 0, 0xCAFE);
        assert_eq!(shim_cametallayer_device(layer, 0), 0xCAFE);

        shim_cametallayer_set_pixel_format(layer, 0, 80);
        assert_eq!(shim_cametallayer_pixel_format(layer, 0), 80);

        shim_cametallayer_set_framebuffer_only(layer, 0, 0);
        assert_eq!(shim_cametallayer_framebuffer_only(layer, 0), 0);

        shim_cametallayer_set_opaque(layer, 0, 0);
        assert_eq!(shim_cametallayer_is_opaque(layer, 0), 0);
        assert_eq!(shim_cametallayer_opaque(layer, 0), 0);

        shim_cametallayer_set_drawable_size(layer, 0, 640.0, 480.0);
        with_metal_props(layer, |props| {
            assert!((props.drawable_width - 640.0).abs() < f64::EPSILON);
            assert!((props.drawable_height - 480.0).abs() < f64::EPSILON);
        });

        assert_eq!(shim_cametallayer_next_drawable(layer, 0), 0);
        free_obj(layer);
    }

    #[test]
    fn metal_layer_nil_safety() {
        assert_eq!(shim_cametallayer_device(0, 0), 0);
        assert_eq!(shim_cametallayer_pixel_format(0, 0), 0);
        assert_eq!(shim_cametallayer_framebuffer_only(0, 0), 0);
        assert_eq!(shim_cametallayer_is_opaque(0, 0), 0);
        assert_eq!(shim_cametallayer_next_drawable(0, 0), 0);

        shim_cametallayer_set_device(0, 0, 0xCAFE);
        shim_cametallayer_set_pixel_format(0, 0, 80);
        shim_cametallayer_set_framebuffer_only(0, 0, 1);
        shim_cametallayer_set_drawable_size(0, 0, 1.0, 1.0);
        shim_cametallayer_set_opaque(0, 0, 1);
    }

    // --- CATransaction ---

    #[test]
    fn transaction_stubs_dont_crash() {
        shim_catransaction_begin(0, 0);
        shim_catransaction_commit(0, 0);
        shim_catransaction_flush(0, 0);
        shim_catransaction_set_disable_actions(0, 0, 1);
        assert_eq!(shim_catransaction_disable_actions(0, 0), 0);
        let dur = shim_catransaction_animation_duration(0, 0);
        assert!((dur - 0.25).abs() < f64::EPSILON);
    }

    // --- CADisplayLink ---

    #[test]
    fn displaylink_create_has_isa() {
        let link = shim_cadisplaylink_create(0, 0, 0xABC, 0xDEF);
        assert_ne!(link, 0);
        let isa = unsafe { *(link as *const u64) };
        assert_eq!(isa, crate::nsobj::cadisplaylink_class_addr());
        let target = unsafe { *((link as usize + DISPLAYLINK_TARGET_OFF) as *const u64) };
        assert_eq!(target, 0xABC);
        free_obj(link);
    }

    #[test]
    fn displaylink_stubs_dont_crash() {
        shim_cadisplaylink_add_to_run_loop(0, 0, 0, 0);
        shim_cadisplaylink_invalidate(0, 0);
        assert_eq!(shim_cadisplaylink_is_paused(0, 0), 1);
        let dur = shim_cadisplaylink_duration(0, 0);
        assert!((dur - 1.0 / 60.0).abs() < f64::EPSILON);
    }

    // --- CAMediaTimingFunction ---

    #[test]
    fn timing_function_returns_sentinel() {
        let result = shim_camediatimingfunction_with_name(0, 0, 0);
        assert_ne!(result, 0);
        // calling again returns same singleton
        let result2 = shim_camediatimingfunction_with_name(0, 0, 0);
        assert_eq!(result, result2);
    }
}
