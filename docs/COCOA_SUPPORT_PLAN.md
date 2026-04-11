# Cocoa Application Support Plan

Strategy for supporting real Cocoa (AppKit) applications using open-source libraries
to reduce implementation effort and maintenance burden.

## Current State

The loader already runs simple `drawRect:`-based Cocoa apps end-to-end:
- `NSApplication` run loop, event queue, `sendAction:to:from:`, responder chain
- `NSWindow` with frame, visibility, delegate callbacks, fullscreen
- `NSView` hierarchy with superview/subviews, frame, autoresize, layer backing
- `drawRect:` → CGContext → softbuffer pixel pipeline
- Mouse/keyboard events through the full responder chain
- `NSTextField`, `NSButton` rendering with CG text
- `NSMenu/NSMenuItem`, `NSOpenPanel/NSSavePanel`, `NSPasteboard`
- `NSRunLoop` with timers, `CFRunLoop` blocking
- 130+ Foundation classes, 1650+ shims, 2880 cf-lite tests

## Open-Source Library Integration Plan

### P0 — Auto Layout Solver (`cassowary` — already in deps)

**Status: Mostly wired. Key gap: `setActive:` / `activateConstraints:` don't auto-install.**

| Component | Status |
|---|---|
| `cassowary` crate in workspace deps | ✅ Done |
| `layout_solver.rs` — Cassowary bridge | ✅ Done (10 tests) |
| `NSLayoutConstraint` 96-byte object + all accessors | ✅ Done |
| `NSLayoutAnchor` / `NSLayoutDimension` with constraint factories | ✅ Done |
| `NSView` anchor accessors (leading/trailing/top/bottom/width/height/center) | ✅ Done |
| VFL parser (`constraintsWithVisualFormat:options:metrics:views:`) | ✅ Done |
| `NSLayoutGuide` with anchor accessors | ✅ Done |
| `translatesAutoresizingMaskIntoConstraints` on NSView | ✅ Done |
| `layoutIfNeeded` / `layoutSubtreeIfNeeded` → solver dispatch | ✅ Done |
| `flush_pending_layouts()` from NSRunLoop | ✅ Done |
| `setActive:` / `activateConstraints:` auto-install to nearest common ancestor | ✅ Done |
| VFL metric name resolution | ✅ Done |
| Content hugging/compression resistance stored per-view | ✅ Done |
| `NSStackView` Auto Layout integration | ✅ Done |
| `safeAreaInsets` / `safeAreaLayoutGuide` | ✅ Done |

### P0 — CALayer Compositing (`tiny-skia`)

**Status: Done. Sublayer tree, per-layer property storage, and tiny-skia compositor wired into display pipeline.**

[tiny-skia](https://github.com/nickel-org/tiny-skia) — CPU Skia subset in pure Rust.
`Pixmap` alpha compositing, affine transforms, clipping, blend modes.

| Component | Status |
|---|---|
| Add `tiny-skia` to workspace deps | ✅ Done |
| CALayer sublayer tree (`addSublayer:`, `removeFromSuperlayer`, `sublayers`, `superlayer`) | ✅ Done |
| Per-layer property storage (opacity, hidden, cornerRadius, borderWidth, borderColor, masksToBounds, contents, contentsScale) | ✅ Done |
| Layer compositor module (`layer_compositor.rs`) with tiny-skia alpha compositing | ✅ Done |
| Offscreen rendering for views with non-default layer props (opacity, cornerRadius, border) | ✅ Done |
| Layer backgroundColor rendering (rounded rect support via tiny-skia) | ✅ Done |
| Corner-radius clipping (`masksToBounds` + `cornerRadius`) | ✅ Done |
| Border rendering (inset stroke, rounded or straight) | ✅ Done |
| Reset state for runtime reuse | ✅ Done |
| CALayer `contents` (CGImage→layer rendering) | ✅ Done |
| CALayer `transform` / `sublayerTransform` (affine matrix compositing) | ✅ Done |
| Replace `SoftwareRenderer` path fill with tiny-skia AA paths | ✅ Done |

### P1 — Text System (`cosmic-text`)

**Status: Core integration done (2808 tests).**

[cosmic-text](https://github.com/pop-os/cosmic-text) — full text layout engine from Pop!\_OS.
Text shaping (harfrust/swash), line breaking, bidi, selection, cursor, rich styling, editing.

| Component | Status |
|---|---|
| Add `cosmic-text` to workspace deps | ✅ Done (0.18, in graphics-core) |
| `text_system` module (FontSystem + SwashCache globals) | ✅ Done (embedded DejaVu fonts loaded) |
| CGContext text drawing via cosmic-text glyphs | ✅ Done (CgContext::draw_text_with_font) |
| CTLineDraw / CoreText measurement via cosmic-text | ✅ Done (ct_text_measure_impl) |
| NSTextView word-wrap rendering | ✅ Done (draw_shaped_text with max_width) |
| NSTextField vertical centering | ✅ Done (proper baseline via cosmic-text metrics) |
| NSTextStorage stub class | ✅ Done (24B, string backing, 9 methods) |
| NSLayoutManager stub class | ✅ Done (24B, text storage + container refs, 10 methods) |
| NSTextContainer stub class | ✅ Done (32B, size + layout manager, 10 methods) |
| TextKit classes registered (Phase C15) | ✅ Done (shim_classes + objc_setup) |
| NSTextView selection, cursor, editing via cosmic-text | ✅ Done |
| NSTextStorage ↔ cosmic-text `Buffer` live bridge | ✅ Done |

**Alternative**: [parley](https://github.com/linebender/parley) (linebender, built on fontique+skrifa).

### P1 — NSImage Rendering (`image` — already in deps)

**Status: DONE — NSImage decodes PNG/JPEG via `image` crate, stores CGImage handle, renders through NSImageView.**

| Component | Status |
|---|---|
| `image` crate in workspace deps | ✅ Done |
| `imaging` feature enabled in cf-lite | ✅ Done |
| `initWithContentsOfFile:` → decode to RGBA pixels | ✅ Done |
| Pixel buffer storage on NSImage object (CGImage handle) | ✅ Done |
| `CGContextDrawImage` → blit through CTM | ✅ Done (was already working) |
| `initWithData:` → decode from NSData bytes | ✅ Done |
| `image_to_cg_handle` returns stored handle | ✅ Done |
| NSImageView `drawRect:` renders via CGImage | ✅ Done |
| CGImage handle cleanup on `nsimage_remove` | ✅ Done |
| `NSImageRep` / `NSBitmapImageRep` | ✅ Done |

### P1 — NIB/XIB Loading (`plist` — already in deps)

**Status: Apple keyed archive format implemented. NSKeyedUnarchiver decodes real
`$objects`/`$top`/`CF$UID` archives. Foundation types short-circuited.
NSBundle `loadNibNamed:` searches for and loads `.nib` files.**

Implementation:
- `nskeyedarchiver.rs`: Full Apple keyed archive decoder — parses `$objects` array,
  resolves `CF$UID` references, caches decoded objects, handles recursive decode.
  Foundation types (NSString, NSArray, NSDictionary, NSData, NSNumber, NSDate, NSURL,
  NSSet, NSValue) decoded directly without ObjC dispatch. Custom classes go through
  `initWithCoder:` via runtime dispatch. Decode methods (`decodeObjectForKey:`,
  `decodeIntegerForKey:`, etc.) check Apple archive context first.
- `nsbundle.rs`: `loadNibNamed:owner:topLevelObjects:` — searches bundle Resources
  for `.nib` files (directory or flat), reads and unarchives via Apple format decoder.
- `nsnib.rs`: NSNib stores name/bundle (24B), `instantiateWithOwner:` delegates to
  `NSBundle loadNibNamed:`.

| Component | Status |
|---|---|
| Binary plist parsing | ✅ Done |
| `NSKeyedUnarchiver` basic decode | ✅ Done |
| Apple keyed archive format (`$objects`/`$top`/`CF$UID`) | ✅ Done (18 tests) |
| `$class` → ObjC runtime class table lookup | ✅ Done |
| Foundation type decoders (NSString/Array/Dict/Data/Number/Date/URL/Set/Value) | ✅ Done |
| Object graph instantiation from keyed archive | ✅ Done (via `initWithCoder:` dispatch) |
| UID deduplication cache | ✅ Done |
| `NSBundle loadNibNamed:owner:topLevelObjects:` | ✅ Done |
| `NSNib` stores name/bundle, delegates to NSBundle | ✅ Done |
| `IBOutlet` / `IBAction` connection wiring | ✅ Done |

### P2 — Accessibility (`accesskit`)

**Status: Done.** AccessKit 0.17 integration with view-tree-to-node-tree bridge and
NSAccessibility protocol shims. 18 tests (10 bridge + 8 protocol).

[AccessKit](https://github.com/AccessKit/accesskit) — cross-platform accessibility with
`accesskit_winit` integration. Maps onto `NSAccessibility` protocol.

Implementation:
- `cf-lite/accessibility_bridge.rs`: ISA→Role mapping (17 AppKit classes), recursive view
  tree walker, `build_tree_update()` produces `TreeUpdate` for screen readers, label
  extraction from NSButton title / NSTextField stringValue. 10 tests.
- `cf-lite/nsaccessibility.rs`: 7 NSAccessibility informal-protocol shims
  (`accessibilityRole`, `accessibilityRoleDescription`, `accessibilityLabel`,
  `accessibilityValue`, `isAccessibilityElement`, `accessibilityChildren`,
  `accessibilityParent`). Registered on NSView in phase_c5. 8 tests.
- `cf-lite/gui_bridge.rs`: `update_accessibility` trait method, called after
  `RedrawRequested` in `dispatch_gui_event`.
- Dependencies: `accesskit = "0.17.1"`, `accesskit_winit = "0.23.1"` (workspace + cf-lite +
  window-backend-darwin + macho-inspect).

| Component | Status |
|---|---|
| Add `accesskit` + `accesskit_winit` | ✅ |
| NSAccessibility protocol bridge | ✅ |
| View tree → AccessKit node tree | ✅ |

### P2 — Font Discovery (cosmic-text/fontdb)

**Status: Done.** System font loading, enumeration, and matching via cosmic-text's
built-in `fontdb`. No new dependency needed — `fontique` was originally planned but
`fontdb` (transitive via cosmic-text) provides system font loading + enumeration.

Implementation:
- `graphics-core/text_system.rs`: `db.load_system_fonts()` in init, `available_font_families()` API,
  `attrs_for_font()` resolves arbitrary names via `Family::Name`.
- `cg-shims/bridge.rs`: `font_family_names()` bridge function.
- `cf-lite/nsfontmanager.rs`: 8-method NSFontManager shim (shared singleton,
  `availableFontFamilies`, `availableFonts`, `fontWithFamily:traits:weight:size:`, etc.).
- `libsystem-shims/core_text.rs`: `CTFontManagerCopyAvailableFontFamilyNames` now uses
  dynamic callback from cg-shims for real system font enumeration.

| Component | Status |
|---|---|
| `NSFontManager` font enumeration | ✅ 8 methods + tests |
| `CTFontCreateWithName` / font matching | ✅ `Family::Name` fallback |
| Font fallback chains | ✅ fontdb built-in fallback |

### P3 — GNUstep / libobjc2 (Linux ObjC runtime)

**Status: FFI bindings implemented.** `Libobjc2Backend` in `objc-runtime/src/backend.rs`
provides real FFI calls to GNUstep's `libobjc.so` for all 12 `RuntimeBackend` trait methods.
Feature-gated behind `--features libobjc2`. 9 feature-gated tests + 3 built-in backend tests.

Two strategies:
- **Option A**: Use `libobjc2` as real ObjC runtime backend on Linux
- **Option B**: Hybrid — selective GNUstep Foundation for complex classes

Implementation:
- `objc-runtime/src/backend.rs`: `Libobjc2Backend` implements `RuntimeBackend` with FFI
  calls to `objc_getClass`, `objc_allocateClassPair`, `objc_registerClassPair`,
  `sel_registerName`, `sel_getName`, `class_addMethod`, `class_replaceMethod`,
  `class_getMethodImplementation`, `class_createInstance`, `object_dispose`,
  `objc_msgSend` (address). Raw bindings in `ffi` submodule.
- `objc-runtime/build.rs`: Links `-lobjc` when `libobjc2` feature is enabled.
- Build: `apt-get install gnustep-libobjc2-dev && cargo build --features libobjc2`

| Component | Status |
|---|---|
| `libobjc2` feature in objc-runtime Cargo.toml | ✅ |
| FFI bindings to libobjc2 | ✅ 12 methods |
| `build.rs` libobjc linking | ✅ |
| Feature-gated tests | ✅ 9 tests |
| GNUstep Foundation selective loading | ❌ |

## AppKit Control Interaction & Rendering Gaps

**Status: Done.** Data-driven controls, interactive sliders/steppers, and NSImage compositing
all wired beyond the original plan scope.

| Component | Status |
|---|---|
| NSTableView data source dispatch (`numberOfRowsInTableView:`, `tableView:objectValueForTableColumn:row:`) | ✅ Done |
| NSTableView `drawRect:` (header, alternating rows, selection highlight, cell text) | ✅ Done |
| NSTableView `mouseDown:` (row selection, delegate notification) | ✅ Done |
| NSTableView `selectedRow` field-backed (was hardcoded -1) | ✅ Done |
| NSTableView `rowHeight` / `setRowHeight:` | ✅ Done |
| NSSlider `mouseDown:` (click→value update→sendAction:to:) | ✅ Done |
| NSSlider target/action fields at own offsets (was missing) | ✅ Done |
| NSStepper `drawRect:` (up/down arrow rendering) | ✅ Done |
| NSStepper `mouseDown:` (increment/decrement with wrapping/clamping→sendAction) | ✅ Done |
| NSStepper `integerValue`/`doubleValue` accessors | ✅ Done |
| NSImage `drawInRect:fromRect:operation:fraction:` → real CGImage blit (was no-op) | ✅ Done |

## Architecture Notes

### Two Layout Systems (not bridged)
- **cf-lite `layout_solver.rs`**: Cassowary-based, wired to guest NSView objects. Used by Mach-O guest binaries.
- **ui-view `layout/engine.rs`**: Custom iterative relaxation solver, used by native `ui-controls`/`app-kit-lite`. These are independent — no bridge needed (they serve different purposes).

### Rendering Pipeline
```
Guest drawRect: → cg-shims → graphics-core::SoftwareRenderer → pixel buffer
                                                                    ↓
                    layer_compositor (tiny-skia) applies:  ← if layer has non-default props
                      • offscreen render → composite_layer()
                      • opacity scaling, cornerRadius clipping, border stroke
                      • layer backgroundColor (rounded rect via tiny-skia)
                                                                    ↓
                               winit window ← softbuffer ← blit from buffer
```

### Key Crate Versions
| Crate | Version | Purpose |
|---|---|---|
| `cassowary` | 0.3 | Auto Layout constraint solver |
| `image` | 0.25 | Image decode (PNG/JPEG) |
| `ab_glyph` | 0.2 | Glyph rasterization |
| `winit` | 0.30 | Platform windowing |
| `softbuffer` | 0.4 | CPU pixel buffer → screen |
| `wgpu` | 24 | GPU renderer (optional) |
| `plist` | 1 | Binary/XML plist parsing |
