# Chapter 10: The GUI

## Scene 10.1: AppKit Stubs

Can we show a window? That's where Chapter 9 left us — staring at a loader that runs 52 Xcode tools, passes 84 compat fixtures, and executes 39 corpus programs with 100% success. Command-line programs. Text in, text out. stdin, stdout, exit code. The entire validation suite operates in the terminal, because the terminal is where we've been living since Chapter 1.

But macOS is not a terminal operating system. macOS is a GUI operating system. The thing that makes a Mac a Mac is not `stat` or `xcrun` or `xcodebuild`. It's the desktop. Windows, menus, buttons, the Dock, the menu bar, the cursor. The Aqua interface that appeared at Macworld 2000 and has evolved through twenty-five years of releases. Everything we've built so far — the parser, the linker, the interpreter, Foundation, the ObjC runtime, the system shim layer — all of it serves one ultimate purpose: making macOS applications run. And macOS applications, the real ones, the ones that define the platform, are GUI applications.

From CLI to GUI. The final frontier.

Every macOS GUI application starts with three objects. Not two. Not five. Three: an `NSApplication`, at least one `NSWindow`, and at least one `NSView`. The application object manages the event loop, routes events to windows, and coordinates the application lifecycle — launching, activating, terminating. The window object represents a rectangular region on screen with a title bar, a content area, and a position. The view object represents a drawable region within a window — a button, a label, a text field, any visual element. Everything else in AppKit is built on these three.

On macOS, `NSApplication` is a singleton. You don't create it with `alloc init` — you call `[NSApplication sharedApplication]`, and the runtime returns the one instance that exists for the lifetime of the process. The singleton pattern is baked into AppKit at the deepest level. Every window, every view, every menu, every event handler ultimately reports to this one object.

Our `NSApplication` lives in `crates/app-kit-lite/src/app.rs`:

```rust
pub struct NSApplication {
    run_loop: RunLoop,
    delegate: Mutex<Option<Box<dyn AppDelegate>>>,
    backend: Mutex<Option<Box<dyn WindowBackend>>>,
    running: AtomicBool,
    exit_code: AtomicI32,
    event_handlers: Mutex<Vec<EventHandler>>,
    main_menu: Mutex<Option<NSMenu>>,
    appearance: Mutex<Appearance>,
}
```

Eight fields. The `RunLoop` is our timer and closure scheduler from `runloop-core` — it drives the main loop. The `delegate` holds the application delegate, the object that receives lifecycle callbacks: `applicationDidFinishLaunching:`, `applicationWillTerminate:`, `applicationShouldTerminate:`. The `backend` holds whatever windowing backend is currently attached — headless for testing, winit+softbuffer for real rendering. The `running` flag and `exit_code` control the loop's termination. The `event_handlers` vector allows headless tests to inject event observers without a window backend. The `main_menu` holds the menu bar. The `appearance` tracks light mode versus dark mode.

The delegate pattern deserves attention because it's AppKit's fundamental coordination mechanism. On macOS, `NSApplication` doesn't do the interesting work — the delegate does. The application object coordinates; the delegate decides. When should the application terminate? Ask the delegate. What should happen when launching finishes? Ask the delegate. The delegate is the application's personality.

```rust
pub trait AppDelegate: Send {
    fn application_did_finish_launching(&self) {}
    fn application_will_terminate(&self) {}
    fn application_should_terminate(&self) -> bool { true }
}
```

Three methods, all with default implementations. A delegate that overrides nothing gets default behavior: launch silently, terminate when asked, clean up nothing. A delegate that needs to create windows at launch overrides `application_did_finish_launching`. A delegate that needs to save state before quitting overrides `application_will_terminate`. A delegate that needs to prevent accidental closure overrides `application_should_terminate` and returns `false`. The pattern scales from trivial apps to complex ones without changing the protocol.

The run loop itself is deliberately simple. Enter `run()`. Notify the delegate that launching finished. Then loop: poll the window backend for events, run one cycle of the run loop (fire timers, execute enqueued closures), sleep 1ms to avoid busy-waiting, repeat. When `terminate()` is called, set `running` to false, and the loop exits. Notify the delegate that termination is happening. Done.

```rust
pub fn run(&self) {
    self.running.store(true, Ordering::SeqCst);
    if let Some(delegate) = self.delegate.lock().unwrap().as_ref() {
        delegate.application_did_finish_launching();
    }
    while self.running.load(Ordering::SeqCst) {
        self.poll_window_events();
        self.run_loop.run_once();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    if let Some(delegate) = self.delegate.lock().unwrap().as_ref() {
        delegate.application_will_terminate();
    }
}
```

That's the entire event loop. Twenty lines. On real macOS, `[NSApplication run]` is hundreds of thousands of lines of implementation involving Mach ports, Core Animation render servers, WindowServer IPC, accessibility bridges, and input method editors. Our version captures the *contract* — the delegate lifecycle, the event dispatch, the run loop cadence — without the implementation weight. The macOS binary calls `[NSApplication run]`, our shim calls this method, and the delegate fires.

The window object is simpler. `NSWindow` on macOS is a complex beast — coordinate systems, backing stores, display management, sheet presentation, full-screen transitions, tab groups. Our stub captures what a minimal GUI program actually needs: a frame rectangle, a content view, a title, and the ability to appear on screen.

The key method is `makeKeyAndOrderFront:`. On macOS, this is the call that makes a window visible and brings it to the front. In our stub, it signals to the window backend that the window should be created and displayed. A macOS binary that calls `[window makeKeyAndOrderFront:nil]` expects a window to appear. Our job is making one appear — not through AppKit's render server protocol, but through our own windowing backend.

The view stub is the third pillar. `NSView` on macOS manages a rectangular region within a window. It has subviews (child views drawn in front of it), a frame (position in superview coordinates), bounds (local coordinate space), and a `drawRect:` method that the system calls when the view needs to redraw. Our `View` in `crates/ui-view/` captures this hierarchy:

```rust
pub struct View {
    pub frame: Rect,
    pub bounds: Rect,
    pub background_color: Color,
    pub hidden: bool,
    pub tag: usize,
    pub layout_id: Option<ViewId>,
    subviews: Vec<View>,
    needs_display: bool,
}
```

Frame, bounds, background color, visibility, subviews, dirty flag. The essentials. The `addSubview:` call pushes a child onto the `subviews` vector. The `drawRect:` dispatch walks the tree and renders each view's background, then its children, back to front. Hit testing walks the tree in reverse — front to back — to find the deepest view containing a point.

These three stubs — NSApplication, NSWindow, NSView — are registered as synthetic ObjC classes in the runtime, just like the Foundation classes in Chapter 5. When the macOS binary calls `[NSApplication sharedApplication]`, our ObjC dispatch finds the synthetic `NSApplication` class, hits the `sharedApplication` method shim, and returns our Rust singleton. When it calls `[[NSWindow alloc] initWithContentRect:styleMask:backing:defer:]`, the dispatch creates a window entry in our backend registry. When it calls `[view addSubview:child]`, the view tree gets a new child.

The binary thinks it's talking to AppKit. It's talking to us.

---

## Scene 10.2: Core Animation

Below NSView, there is another layer. Literally.

Every NSView on macOS is backed by a `CALayer` — a Core Animation layer object that manages the view's visual content. The layer is what actually gets rendered. Views are the programmer's interface; layers are the render engine's interface. When you set a view's background color, you're setting its layer's background color. When the system composites the window, it reads the layer tree, not the view tree.

Core Animation is Apple's compositing engine. It runs in a separate process (the render server), communicates over Mach IPC, and drives the GPU to composite layers at 60fps (or 120fps on ProMotion displays). Every animation you've ever seen on macOS — a window fading in, a button highlighting, a sheet sliding down — is a Core Animation transaction. The API is deceptively simple: set a property, and Core Animation interpolates between the old value and the new value over a duration. Change a layer's position from (0, 0) to (100, 100), and Core Animation animates the movement smoothly.

We are not building a GPU compositor. Let's be clear about that right now.

Real Core Animation involves Metal shader pipelines, IOSurface shared memory, a render server daemon, display link timing, and a CADisplayLink that synchronizes frame presentation with the display's refresh rate. Reimplementing any of that is a multi-year project unto itself. The question is: what do we actually need?

A `CALayer` has properties: `backgroundColor`, `bounds`, `position`, `opacity`, `cornerRadius`, `borderWidth`, `borderColor`, `shadowColor`, `shadowOffset`, `shadowRadius`. It has sublayers — a tree structure that mirrors the view tree. And it has `CATransaction`, the implicit animation context that batches property changes and commits them as a group.

Our stub captures the tree. Not the rendering, not the animation, not the compositing — the tree. When a macOS binary creates a `CALayer`, sets its background color to blue, adds it as a sublayer of another layer, and commits the transaction, our stub records: "Layer A has background color blue, is a sublayer of layer B, transaction committed." We capture *what the app wants to draw*, not *how to draw it*.

This is the pragmatic approach that defines the entire project. We don't implement `stat` by reimplementing the filesystem — we translate the struct layout and call the host's `stat`. We don't implement `NSString` by reimplementing Unicode algorithms — we wrap Rust's `String` and expose ObjC method selectors. We don't implement Core Animation by writing a GPU compositor — we capture the layer tree and translate it into our own software renderer.

The `CATransaction` stub follows the same pattern. On macOS, `[CATransaction begin]` creates an implicit animation context. Property changes between `begin` and `commit` are batched and animated. Our stub tracks the begin/commit state so that binaries that use the transactional API don't hit a missing-selector crash. The animation duration is recorded but not acted upon — we're rendering static frames, not interpolating animations. A test can inspect the transaction state and verify that the binary called the API correctly, even though no animation occurs.

The layer tree that our stub builds is structurally identical to what Core Animation would build. Same parent-child relationships. Same property values. Same ordering. What's different is what happens with that tree. On macOS, the render server reads it and drives the GPU. In our loader, the software renderer reads it and writes pixels to a `Surface` — a CPU-side pixel buffer:

```rust
pub struct Surface {
    pixels: Vec<u32>,
    width: u32,
    height: u32,
}
```

A vector of ARGB32 pixels. Width times height elements. No GPU, no shaders, no display link. Just a flat array of 32-bit color values that we fill in by walking the view tree back to front, painting each view's background rectangle, then each view's children, then any text labels, then any controls. The software renderer in `crates/graphics-core/` handles anti-aliased text, rounded rectangles, gradients, and shadows — enough to produce a recognizable GUI, not enough to match Quartz 2D's full rendering fidelity.

Stubbing the render pipeline captures what the app wants to draw, not how it wants it drawn. The distinction matters for validation. If a macOS binary creates a window with a blue background and a "Hello" label, we can verify that the window exists, the background is blue, and the label text is "Hello" — all without rendering a single pixel. The pixel rendering is our addition, layered on top of the AppKit stubs, so that the user can *see* the result. But the correctness validation doesn't depend on it. The stubs are the compatibility layer. The renderer is the bonus.

This separation between stubbing and rendering runs through the entire GUI stack. The stubs make the binary run. The renderer makes it visible. They're independent concerns. You could swap the software renderer for a GPU-accelerated one (Metal, Vulkan, wgpu) and the stubs wouldn't change. You could disable rendering entirely and the binary would still execute, create windows, set properties, and exit cleanly — you just wouldn't see anything.

Core Animation, in our loader, is a bookkeeping exercise. Record the tree. Record the properties. Let someone else worry about pixels.

---

## Scene 10.3: The Event Loop

A window that doesn't respond to input is a picture. A window that responds to input is an application. The difference is the event loop and the responder chain.

On macOS, when you click a window, the WindowServer process detects the click, determines which window and which region of that window received it, packages the click into an `NSEvent`, and delivers it to the application through a Mach port. The application's run loop receives the event, determines which window it belongs to, and sends it to that window. The window determines which view was clicked (hit testing), and sends the event to that view. If the view doesn't handle the event, it passes it up to its superview. If the superview doesn't handle it, it passes it to the window. If the window doesn't handle it, it passes it to the application. This is the responder chain.

The responder chain is AppKit's deepest design pattern. It is not merely a way of handling events — it is the fundamental communication pathway between user input and application logic. Every key press, every mouse click, every menu selection, every drag operation travels through the responder chain. A text field handles character input by being first responder. A window handles Escape by implementing `cancelOperation:`. The application handles Cmd+Q through a menu key equivalent that the responder chain evaluates.

Our `Event` type lives in `crates/app-kit-lite/src/event.rs`:

```rust
pub enum Event {
    KeyDown { key: KeyCode, modifiers: ModifierFlags },
    KeyUp { key: KeyCode, modifiers: ModifierFlags },
    MouseDown { location: Point, button: MouseButton },
    MouseUp { location: Point, button: MouseButton },
    MouseMoved { location: Point },
    ScrollWheel { delta_x: f64, delta_y: f64 },
    CharacterInput { text: String },
}
```

Seven variants. The real `NSEvent` has dozens — tablet pressure, gesture magnification, touch bar input, smart magnify, directional pressure — but these seven cover what GUI programs actually rely on for basic interaction. Key presses, mouse actions, scroll input, and text input. Each carries the data the responder chain needs to dispatch it: location for mouse events, key code and modifiers for keyboard events, delta values for scroll events.

The `Responder` trait defines the chain:

```rust
pub trait Responder {
    fn next_responder(&self) -> Option<&dyn Responder> { None }
    fn accepts_first_responder(&self) -> bool { false }
    fn key_down(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.key_down(event);
        }
    }
    fn mouse_down(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.mouse_down(event);
        }
    }
    // ... key_up, mouse_up
}
```

Every method has a default implementation that forwards to the next responder. This is the chain in action: if a responder doesn't override `key_down`, the default implementation walks up to the next responder and tries again. The walk continues until someone handles the event or the chain ends at `NSApplication`, which has no next responder.

The implementation mirrors macOS's architecture precisely. NSView implements `Responder` with `next_responder` returning the superview (or the window, if the view has no superview). NSWindow implements `Responder` with `next_responder` returning the application. NSApplication implements `Responder` with `next_responder` returning `None` — the end of the chain. A key press starts at the first responder (typically the focused text field), walks up through the view hierarchy, through the window, and terminates at the application.

Menu key equivalents add a twist. Before the standard responder chain runs, `NSApplication` checks its menu bar. If the pressed key matches a menu item's key equivalent — Cmd+Q for Quit, Cmd+C for Copy, Cmd+V for Paste — the menu item fires and the event is consumed. Our implementation mirrors this:

```rust
pub fn perform_key_equivalent(&self, key: KeyCode, modifiers: ModifierFlags) -> bool {
    let mut menu = self.main_menu.lock().unwrap();
    if let Some(ref mut m) = *menu {
        m.perform_key_equivalent(key, modifiers)
    } else {
        false
    }
}
```

Try the menu first. If it handles the key, return true and stop. If not, dispatch through the responder chain normally. This is exactly what real AppKit does, and getting the order wrong breaks fundamental keyboard shortcuts.

The event dispatch in `NSApplication::poll_window_events()` translates backend events into our `Event` type and feeds them through this pipeline. The backend provides raw input — key codes, mouse positions, modifier state. The application translates those into AppKit-level events and dispatches them through the chain. A `WindowEvent::KeyDown` from the backend becomes an `Event::KeyDown` dispatched to event handlers. A `WindowEvent::CloseRequested` triggers `terminate(0)`. A `WindowEvent::ModifiersChanged` updates the tracked modifier state so that subsequent key events carry the correct flags.

This is the last major design pattern in the book. Chapter 4 introduced the ObjC message dispatch pattern — the runtime mechanism that makes every method call dynamic. Chapter 5 introduced the class cluster pattern — Foundation's strategy of hiding implementation behind abstract interfaces. Chapter 8 introduced the shim delegation pattern — mapping Darwin APIs to Linux equivalents through thin translators. Now Chapter 10 introduces the responder chain — AppKit's mechanism for routing user input through a hierarchy of objects, with each object choosing to handle or forward.

Four patterns. Four chapters. Message dispatch, class clusters, shim delegation, responder chains. Together they form the architectural core of macOS application development. Every macOS app you've ever used relies on all four, simultaneously, thousands of times per second.

---

## Scene 10.4: The GUI Bridge

A window appears.

Not metaphorically. Not in a test assertion. Not as a JSON field in a CI report. A real window, rendered by a real windowing system, with a real title bar and a real pixel buffer, opened by a macOS Mach-O binary running on a Linux host through the loader we built from scratch across nine chapters.

Here is how it happens.

The `WindowBackend` trait in `crates/window-backend/src/lib.rs` is the abstraction that makes GUI output possible:

```rust
pub trait WindowBackend: Send + Sync {
    fn create_window(&self, config: WindowConfig) -> WindowId;
    fn destroy_window(&self, id: WindowId);
    fn set_title(&self, id: WindowId, title: &str);
    fn set_size(&self, id: WindowId, width: f64, height: f64);
    fn present_surface(&self, id: WindowId, surface: &Surface);
    fn poll_events(&self) -> Vec<WindowEvent>;
    fn request_redraw(&self, id: WindowId);
}
```

Seven required methods. Create a window, destroy it, set its title and size, present a rendered surface, poll for events, request a redraw. This trait has two implementations. The first is `HeadlessBackend` — a no-op backend for testing that tracks window state in memory but never renders anything. The second is the real one: `GuiApp` in `crates/window-backend-darwin/`, backed by winit and softbuffer.

winit is a cross-platform window creation library. It talks to Wayland on Linux, Win32 on Windows, Cocoa on macOS. It handles the platform-specific ceremony of creating a native window, setting up an event loop, and translating platform events into a common event type. softbuffer is a companion library that provides a CPU-rendered pixel buffer — a software surface that maps directly to the window's content. No GPU. No OpenGL. No Vulkan. Just a flat array of pixels that gets blitted to the screen.

The `GuiApp` wraps both:

```rust
pub struct GuiApp {
    config: WindowConfig,
    scene: AppScene,
}
```

`WindowConfig` specifies the title, size, and resizability. `AppScene` is the scene graph from `ui-controls` — it manages labels, buttons, text fields, menu bars, and layout. When the application enters the event loop, `GuiApp` creates a winit `Window`, initializes a softbuffer `Context` and `Surface`, and starts the render loop.

Each frame follows a straightforward pipeline. First, poll winit for events — resizes, key presses, mouse clicks, close requests. Second, dispatch those events through the scene graph for hit testing and interaction. Third, render the scene to a `graphics_core::Surface` — the CPU pixel buffer. Fourth, copy the pixel buffer to the softbuffer surface. Fifth, present.

The render step is where the view tree becomes pixels:

```rust
fn redraw(&mut self) {
    let phys = window.inner_size();
    let scale = window.scale_factor();
    let lw = (phys.width as f64 / scale) as u32;
    let lh = (phys.height as f64 / scale) as u32;
    let surface = self.scene.render(lw, lh);
    // ... copy to softbuffer and present
}
```

`scene.render()` walks the scene graph — backgrounds, labels, buttons, text fields — and paints them into a `Surface`. The HiDPI scaling is handled by rendering at logical resolution and then scaling up during the copy to the physical-sized softbuffer. The result: a complete frame of the application's UI, rendered in software, presented through the platform's native windowing system.

Now consider what happens when the `cocoa_window` fixture runs through our loader.

The fixture lives at `fixtures/cocoa_window/main.m`. It's a minimal Cocoa application written in raw Objective-C — no nib files, no storyboards, no Interface Builder. Pure programmatic AppKit:

```objc
id app = objc_msgSend(objc_getClass("NSApplication"),
                      sel_registerName("sharedApplication"));

Class delegateCls = objc_allocateClassPair(
    objc_getClass("NSObject"), "CocoaWindowDelegate", 0);
class_addMethod(delegateCls,
                sel_registerName("applicationDidFinishLaunching:"),
                (IMP)AppDelegate_didFinishLaunching, "v@:@");
objc_registerClassPair(delegateCls);

id delegate = objc_msgSend(objc_msgSend(delegateCls, sel_registerName("alloc")),
                           sel_registerName("init"));
objc_msgSend(app, sel_registerName("setDelegate:"), delegate);
objc_msgSend(app, sel_registerName("run"));
```

Follow the flow. `[NSApplication sharedApplication]` — our ObjC runtime dispatches to the synthetic NSApplication class, returns the singleton. `objc_allocateClassPair` — creates a new ObjC class at runtime, `CocoaWindowDelegate`, inheriting from NSObject. `class_addMethod` — registers `applicationDidFinishLaunching:` as an instance method pointing to the fixture's C function. `objc_registerClassPair` — finalizes the class. The delegate is allocated, initialized, and set on the application. Then `[app run]` enters the main loop.

The delegate's `applicationDidFinishLaunching:` callback fires. Inside it:

```objc
id window = initW(windowAlloc,
                  sel_registerName("initWithContentRect:styleMask:backing:defer:"),
                  NSMakeRect(100, 100, 400, 300), style, 2, NO);
objc_msgSend(window, sel_registerName("setTitle:"), @"Darwin Loader C7");
objc_msgSend(window, sel_registerName("makeKeyAndOrderFront:"), nil);
```

Create a 400×300 window. Set its title. Show it. Every one of these calls dispatches through our ObjC runtime, hits our synthetic method implementations, and translates into window backend operations. `initWithContentRect:styleMask:backing:defer:` creates an entry in the backend's window registry. `setTitle:` calls `backend.set_title()`. `makeKeyAndOrderFront:` calls `backend.create_window()` with the configured size and title.

And if the backend is `GuiApp` — if there's a real display attached — a window appears.

Four hundred pixels wide. Three hundred pixels tall. Title bar reads "Darwin Loader C7". The pixel buffer contains whatever the view tree renders — a white background, or whatever background color the content view specifies. The winit event loop runs. The softbuffer presents frames. The window sits there on a Linux desktop, rendered by X11 or Wayland, created by a macOS Mach-O binary that has never been within a thousand miles of a Mac.

This is the moment. From Chapter 1's crash — `SIGSEGV at 0x0000000000000000`, a binary that couldn't even read its own magic number without our help — to Chapter 10's window. The same kind of binary. The same `CF FA ED FE` header. The same `__TEXT` and `__DATA` segments, the same load commands, the same ObjC metadata, the same Foundation class references. But now every layer is in place. The parser reads the headers. The mapper lays out the segments. The linker resolves the symbols. The ObjC runtime dispatches the messages. Foundation provides the vocabulary. The system shims translate the ABI. The interpreter executes the instructions. The test harness validates every step. And the GUI bridge takes the final output — an AppKit application's desire to show a window — and makes it real.

The fixture prints its check lines to stdout as it runs:

```
CHECK 1: sharedApplication obtained
CHECK 2: applicationDidFinishLaunching: received
CHECK 3: NSWindow created
CHECK 4: window title set
CHECK 5: window shown
```

Five checks. Five passes. The binary loaded, the runtime dispatched, Foundation responded, the delegate fired, the window was created, titled, and shown. Then the delegate calls `[app terminate:nil]`, the run loop exits, the fixture verifies that the delegate was called, and it exits 0.

In the CI pipeline, this runs headless — the `HeadlessBackend` records the window creation without rendering pixels, and the check lines verify the lifecycle. On a developer's machine with a display, it runs live — winit opens a native window and presents it until the fixture terminates. Same code path. Same binary. Same loader. The only difference is which backend is plugged in.

That's the bridge. The macOS binary speaks AppKit. Our stubs translate AppKit into the `WindowBackend` trait. The backend speaks winit and softbuffer. The result is a macOS GUI application running on Linux, not through emulation, not through a virtual machine, but through a loader that understands the binary format, the runtime, the frameworks, and the system layer deeply enough to reconstruct the application's intent and realize it on a different platform.

From a crash in Chapter 1 to a window in Chapter 10.

---

## Scene 10.5: What's Next

The window closes. The process exits. The terminal prompt returns.

Take stock of what exists. Forty-one crates, roughly 198,000 lines of Rust. A Mach-O parser that handles fat binaries, code signatures, and chained fixups. A segment mapper with W^X enforcement and ASLR slide. A symbol resolver that walks the two-level namespace. An ObjC runtime with class tables, selector interning, ISA-based dispatch, and category loading. A Foundation layer with 93 classes and over 1,100 selector implementations. A libSystem shim with 57 source files covering errno translation, variadic ABI conversion, struct layout differences, TLS emulation, and 37 stub modules for macOS-only frameworks. An ARM64 interpreter for code that can't run natively. A test harness with 2,682 cf-lite tests, 84 compat fixtures, 39 corpus tools, and 52 passing Xcode tools. An AppKit stub layer with NSApplication, NSWindow, NSView, the responder chain, menus, alerts, panels, and paste boards. A window backend with headless and live rendering modes. A software renderer that turns view trees into pixel buffers.

All of it, from the first byte of the magic number to the last pixel of the window, written to answer one question: what does it take to run a macOS binary on a platform that isn't macOS?

The answer, it turns out, is everything. Not everything as in "a lot" — everything as in every layer of the software stack, from binary parsing to GUI rendering, must be accounted for. Skip the ABI translation and printf produces garbage. Skip the ObjC runtime and Foundation crashes. Skip Foundation and every tool returns nil. Skip the system shims and errno is wrong, stat is wrong, threads are wrong. Skip the responder chain and keyboard input vanishes. Every layer depends on the layers below it, and the binary depends on all of them simultaneously. There is no partial compatibility. The stack is either complete enough to run the binary, or it isn't.

But the stack is never truly complete. That's the nature of building a compatibility layer for a proprietary platform that ships new APIs every June. What we've built handles the binaries we've tested. What about the ones we haven't?

The Swift runtime is the most immediate frontier. Apple has been moving away from Objective-C toward Swift since 2014. Every year, more of Apple's frameworks are written in Swift. More third-party apps are written in Swift. The Swift runtime has its own metadata format — protocol witness tables, value witness tables, type descriptors — that lives alongside ObjC metadata in the binary's `__DATA` segment. Our parser can read it. Our class registrar can index it. What we lack is the runtime that interprets it: protocol conformance checking, generic type instantiation, witness table dispatch. The `swift_stub_gen` crate in the project lays the foundation with type metadata stubs, but a full Swift runtime bridge is a substantial project — comparable in scope to the Foundation layer.

Metal is the second frontier. macOS applications increasingly use Metal for rendering — not just games, but productivity apps, system utilities, even Finder. Metal shader compilation, command buffers, texture management, and GPU synchronization are all deeply platform-specific. A Metal compatibility layer would need to translate Metal shading language to SPIR-V or GLSL, map Metal's command buffer model to Vulkan's, and handle Apple's silicon-specific GPU features. It's possible — MoltenVK does the reverse direction (Vulkan to Metal) — but it's a rendering engine project, not a loader project.

Rosetta 2 is the comparison point that will define how people judge this work. Apple's own binary translator runs x86_64 macOS apps on ARM64 Macs with near-native performance. It has advantages we don't: access to Apple's private frameworks, kernel-level support for translated binaries, ahead-of-time translation with optimized code caches, and a team of Apple engineers maintaining it. What our loader has that Rosetta doesn't is cross-platform portability. Rosetta runs x86_64 binaries on ARM64 macOS. Our loader runs ARM64 macOS binaries on ARM64 Linux. Different direction. Different trade-offs. The comparison is inevitable and instructive — Rosetta shows what's possible with full platform access, and our loader shows what's achievable without it.

JIT compilation is the performance frontier. The ARM64 interpreter in `crates/arm64-translate/` currently decodes and executes instructions one at a time. For guest code that can't run natively — code with pointer authentication instructions that the host doesn't support, or code that must be traced for debugging — the interpreter is correct but slow. A JIT compiler that translates hot basic blocks into native ARM64 code (stripping PAC instructions and inlining shim calls) would bring guest code performance within striking distance of native. The infrastructure for this — basic block identification, register allocation, code emission — is well-understood compiler technology. The application to binary translation is specific but not novel.

But those are future directions. Future chapters of a project that this book closes but doesn't end.

Look at what we started with. Chapter 1. A hex dump. `CF FA ED FE`. Four bytes at offset zero that meant nothing to us except that someone at Apple liked clever hex constants. We stared at those bytes and asked: "What is this?" Not "what does the documentation say it is" — documentation describes intent, and we wanted reality. What is this binary, actually? How does it work? What does the operating system do with it?

Ten chapters later, we can answer. The magic number identifies the format. The header declares the architecture and file type. Load commands describe the segments, the libraries, the entry point, the code signature, the encryption info, the source version. Segments map into virtual memory with specific permissions. Sections within segments hold code, data, ObjC metadata, string literals, symbol tables, indirect symbol stubs, lazy binding opcodes, and chained fixup chains. The dynamic linker resolves symbols across a two-level namespace. The ObjC runtime reads class lists, selector references, category attachments, and protocol lists, then builds the dispatch tables that `objc_msgSend` walks at runtime. Foundation provides the class implementations behind every `NSString`, every `NSArray`, every `NSDictionary`. The system layer translates POSIX, Mach, and GCD APIs to the host platform. The GUI layer translates AppKit's window, view, and event model to a cross-platform windowing backend.

Every one of those layers exists in the codebase. Every one has tests. Every one works.

`0xFEEDFACF`. The magic number hasn't changed. It's the same four bytes at offset zero of every 64-bit Mach-O binary, the same greeting we first encountered in a hex dump nine chapters ago. The bytes haven't changed. The reader has. You started this book knowing that macOS binaries were black boxes. You end it knowing what's inside — not from reading Apple's documentation, but from building a loader that opens them, maps them, links them, runs them, and shows their windows on a platform those binaries were never designed to run on.

The opening image of this book was a crash. `SIGSEGV`. A binary that tried to run and couldn't, because the platform beneath it didn't exist. The closing image is a window. Four hundred pixels wide, three hundred pixels tall, title bar reading "Darwin Loader C7", running on Linux, rendered through a software pipeline we wrote in Rust, driven by a binary whose first four bytes still read `CF FA ED FE`.

Same binary. Different understanding.
