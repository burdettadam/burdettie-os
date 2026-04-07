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
- 130+ Foundation classes, 1650+ shims, 2718 cf-lite tests

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
| `NSStackView` Auto Layout integration | ❌ Not wired |
| `safeAreaInsets` / `safeAreaLayoutGuide` | ❌ Not wired |

### P0 — CALayer Compositing (`tiny-skia`)

**Status: Not started.**

[tiny-skia](https://github.com/nickel-org/tiny-skia) — CPU Skia subset in pure Rust.
`Pixmap` alpha compositing, affine transforms, clipping, blend modes.

| Component | Status |
|---|---|
| Add `tiny-skia` to workspace deps | ❌ |
| CALayer backing store as `tiny_skia::Pixmap` | ❌ |
| Layer tree compositor (z-order, opacity, transform, cornerRadius, masks) | ❌ |
| Wire into `display_view_recursive` / softbuffer blit | ❌ |
| Replace or augment `graphics_core::SoftwareRenderer` internals | ❌ |

**Alternative**: [vello](https://github.com/linebender/vello) for GPU compositing
(pairs with existing `wgpu` feature gate).

### P1 — Text System (`cosmic-text`)

**Status: Not started.**

[cosmic-text](https://github.com/pop-os/cosmic-text) — full text layout engine from Pop!\_OS.
Text shaping (rustybuzz/swash), line breaking, bidi, selection, cursor, rich styling, editing.

| Component | Status |
|---|---|
| Add `cosmic-text` to workspace deps | ❌ |
| NSLayoutManager backed by cosmic-text buffer | ❌ |
| NSTextStorage ↔ cosmic-text `Buffer` bridge | ❌ |
| NSTextContainer size constraint wiring | ❌ |
| NSTextView selection, cursor, editing | ❌ |
| NSTextField rich text rendering upgrade | ❌ |
| CGContext text drawing via cosmic-text glyphs | ❌ |

**Alternative**: [parley](https://github.com/linebender/parley) (linebender, built on fontique+skrifa).

### P1 — NSImage Rendering (`image` — already in deps)

**Status: `image` crate is optional dep in graphics-core. NSImage is stubbed.**

| Component | Status |
|---|---|
| `image` crate in workspace deps | ✅ Done |
| `initWithContentsOfFile:` → decode to RGBA pixels | ❌ |
| Pixel buffer storage on NSImage object | ❌ |
| `CGContextDrawImage` → blit through CTM | ❌ |
| `initWithData:` / `initWithContentsOfURL:` | ❌ |
| `NSImageRep` / `NSBitmapImageRep` | ❌ |

### P1 — NIB/XIB Loading (`plist` — already in deps)

**Status: `plist` crate used for binary plist. NSKeyedUnarchiver exists. NIB loader missing.**

| Component | Status |
|---|---|
| Binary plist parsing | ✅ Done |
| `NSKeyedUnarchiver` basic decode | ✅ Done |
| `$class` → ObjC runtime class table lookup | ❌ |
| Object graph instantiation from keyed archive | ❌ |
| `IBOutlet` / `IBAction` connection wiring | ❌ |
| `NSBundle loadNibNamed:owner:topLevelObjects:` | ❌ Stub |

### P2 — Accessibility (`accesskit`)

**Status: Not started.**

[AccessKit](https://github.com/AccessKit/accesskit) — cross-platform accessibility with
`accesskit_winit` integration. Maps onto `NSAccessibility` protocol.

| Component | Status |
|---|---|
| Add `accesskit` + `accesskit_winit` | ❌ |
| NSAccessibility protocol bridge | ❌ |
| View tree → AccessKit node tree | ❌ |

### P2 — Font Discovery (`fontique`)

**Status: Using `ab_glyph` for glyph rasterization only. No font enumeration.**

[fontique](https://github.com/linebender/fontique) — cross-platform font enumeration and matching.

| Component | Status |
|---|---|
| `NSFontManager` font enumeration | ❌ |
| `CTFontCreateWithName` / font matching | ❌ |
| Font fallback chains | ❌ |

### P3 — GNUstep / libobjc2 (Linux ObjC runtime)

**Status: Feature-gated in objc-runtime, unimplemented.**

Two strategies:
- **Option A**: Use `libobjc2` as real ObjC runtime backend on Linux
- **Option B**: Hybrid — selective GNUstep Foundation for complex classes

| Component | Status |
|---|---|
| `libobjc2` feature in objc-runtime Cargo.toml | ✅ Exists |
| FFI bindings to libobjc2 | ❌ 11× `unimplemented!()` |
| GNUstep Foundation selective loading | ❌ |

## Architecture Notes

### Two Layout Systems (not bridged)
- **cf-lite `layout_solver.rs`**: Cassowary-based, wired to guest NSView objects. Used by Mach-O guest binaries.
- **ui-view `layout/engine.rs`**: Custom iterative relaxation solver, used by native `ui-controls`/`app-kit-lite`. These are independent — no bridge needed (they serve different purposes).

### Rendering Pipeline
```
Guest drawRect: → cg-shims → graphics-core::SoftwareRenderer → pixel buffer
                                                                    ↓
                               winit window ← softbuffer ← blit from buffer
```
With tiny-skia, CALayer compositing would insert between the per-view pixel buffers
and the final softbuffer surface blit.

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
