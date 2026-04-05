# Story Architecture — Chapter and Scene Outline

**Phase:** 04 - Plot and Structure
**Purpose:** Define the book's chapter progression and scene breakdown
**Status:** Outline

---

## Part I: The Artifact

### Chapter 1 — The Binary
*What is a Mach-O file, and how do you read one?*

<!-- beat:opening_image characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 1.1 | The Black Box | Open a macOS binary in a hex editor. See the magic bytes `0xFEEDFACF`. | Mach-O magic, fat binaries, byte order |
| 1.2 | The Header | Parse the mach_header_64 struct. CPU type, file type, flags. | Header fields, CPU subtypes, MH_PIE |
| 1.3 | The Load Commands | Walk the load command array. LC_SEGMENT_64, LC_DYLIB, LC_MAIN. | Load command taxonomy, segment→section hierarchy |
| 1.4 | The Sections | Dive into __TEXT/__text, __DATA/__objc_classlist, __LINKEDIT. | Section roles, content types, alignment |
| 1.5 | The Symbol Table | nlist_64 entries, string table, indirect symbol table. | Symbol types (N_EXT, N_UNDF), binding |
| 1.6 | Building the Parser | Introduce macho-parse crate. Zero-copy parsing in Rust. | ParsedMachO struct, error handling |

**Chapter Arc:** Opaque file → structured understanding. The reader can now "see" inside any Mach-O binary.

---

### Chapter 2 — The Map
*How do you load a binary into memory?*

<!-- beat:rising_action characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 2.1 | Virtual Memory Primer | Pages, mappings, mmap(), protection bits. | VM basics for binary loading |
| 2.2 | Segments to Memory | Map __TEXT (r-x), __DATA (rw-), __LINKEDIT (r--). | Segment permissions, page alignment, slide |
| 2.3 | ASLR and the Slide | PIE binaries, random base address, preferred load address. | Address Space Layout Randomization |
| 2.4 | The MappedImage | Introduce macho-exec crate. MappedImage struct, mmap lifecycle. | Memory-mapped binary representation |
| 2.5 | Code Signing and Verification | LC_CODE_SIGNATURE, code directory, page hashes. | Integrity checking (light treatment) |

**Chapter Arc:** File on disk → live memory image. The binary now exists at a real address.

---

## Part II: The Linker

### Chapter 3 — The Links
*How does a binary find the functions it calls?*

<!-- beat:first_pinch characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 3.1 | Why Dynamic Linking | Static vs. dynamic. Shared libraries. Install names. | Motivation for dynamic linking |
| 3.2 | Import Binds | LC_DYLD_INFO_ONLY, bind opcodes, lazy binding. | Classic bind/lazy bind protocol |
| 3.3 | Chained Fixups | LC_DYLD_CHAINED_FIXUPS, DYLD_CHAINED_PTR_64_OFFSET format. | Modern fixup chains, rebase vs. bind |
| 3.4 | Auth Fixups and PAC | arm64e, DYLD_CHAINED_PTR_ARM64E, pointer authentication codes. | PAC keys, blend discriminators, signing |
| 3.5 | The Stub Resolver | Symbol name → host dlsym(). Safe list, prefix matching. | Our resolution strategy, StubResolver |
| 3.6 | Dependency Loading | BFS walk of LC_LOAD_DYLIB commands. ImageRegistry. | Recursive library loading, dependency graphs |
| 3.7 | The Shared Cache | dyld_shared_cache format, extraction, image lookup. | Apple's prelinked library optimization |

**Chapter Arc:** Dangling symbol references → resolved function pointers. The binary can now call external functions.

---

### Chapter 4 — The Runtime
*How does Objective-C message dispatch actually work?*

<!-- beat:midpoint characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 4.1 | The ObjC Object Model | ISA pointers, class_t, class_ro_t, method lists. | Objective-C binary representation |
| 4.2 | Selector Interning | __objc_selrefs, SEL as interned C strings, uniquing. | How selectors work at the binary level |
| 4.3 | Message Dispatch | objc_msgSend: ISA → cache → method list → superclass chain. | The hot path of every ObjC call |
| 4.4 | Building objc_msgSend | ARM64 assembly trampoline. Register save/restore. Lookup. | Our trampoline.s, LookupResult struct |
| 4.5 | Class Registration | Parse __objc_classlist, build runtime class table. | ObjcRuntime struct, classes_by_name/addr |
| 4.6 | Protocols and Categories | __objc_protolist, __objc_catlist, method injection. | Protocol conformance, category loading |
| 4.7 | Dynamic Class Creation | objc_allocateClassPair, class_addMethod, KVO ISA swizzling. | Dynamic runtime, synthetic address allocator |
| 4.8 | Tagged Pointers | Bit 63 tagging, NSNumber/NSDate/NSString small values. | Pointer tagging scheme, tag dispatch |

**Chapter Arc:** Raw binary class data → working message dispatch. The reader has built an Objective-C runtime.

---

## Part III: The Platform

### Chapter 5 — The Foundation
*How do you shim an entire framework?*

<!-- beat:second_pinch characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 5.1 | What Foundation Provides | NSString, NSArray, NSDictionary, NSData—the substrate. | Foundation's role in every macOS binary |
| 5.2 | Synthetic Class Registration | Building 93+ Foundation classes without Apple's code. | Template caching, overlay_from() |
| 5.3 | NSString Internals | UTF-8 storage, CFString layout, toll-free bridging. | String representation, __CFConstantString |
| 5.4 | Collections | NSArray/NSDictionary/NSSet: real storage, real enumeration. | How guest code iterates our fake collections |
| 5.5 | NSFileManager and Friends | File I/O shims, NSBundle, NSPropertyListSerialization. | Shimming system interactions |
| 5.6 | NSURLSession | Real HTTP via ureq. Synchronous request on resume. | Networking shim with actual functionality |
| 5.7 | NSTask and Process Control | posix_spawn, pipe wiring, environment bridging. | Subprocess shims, the hardest Foundation class |
| 5.8 | Toll-Free Bridging | CF and NS share ISAs. CFStringRef == NSString*. | CoreFoundation/Foundation duality |
| 5.9 | The 2,682 Tests | Test-driven shim development. Regression gates. | Testing strategy for compatibility work |

**Chapter Arc:** Bare runtime → living framework. Code that calls [NSString stringWithFormat:] now works.

---

### Chapter 6 — The Bridge
*What changes when you cross from macOS to Linux?*

<!-- beat:dark_night characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 6.1 | Two ARM64 Worlds | Apple ARM64 vs. AAPCS64. Same ISA, different ABIs. | ABI divergence on same architecture |
| 6.2 | The Variadic Crisis | Apple puts ALL variadic args on stack. Linux uses registers. | va_list struct differences, naked assembly shims |
| 6.3 | setlocale and Category Numbers | macOS LC_ALL=0, Linux LC_ALL=6. | Enum translation, subtle ABI mismatches |
| 6.4 | struct stat Translation | macOS stat (144 bytes) → Linux stat (128 bytes). | Structure layout conversion |
| 6.5 | Data Symbols | __mb_cur_max, mach_task_self_, NSApp. Not functions. | Data symbols vs. function symbols |
| 6.6 | Library Preloading | RTLD_GLOBAL, dependency chain, safe-list expansion. | Linux shared library loading strategy |
| 6.7 | The Docker Container | Dockerfile.burdettie, the full runtime environment. | Putting it all together for Linux |

**Chapter Arc:** Works on macOS → works on Linux. The hardest bugs in the project.

---

### Chapter 7 — The Interpreter
*What if you can't run the code natively?*

<!-- beat:climax_prep characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 7.1 | Why Interpret | Cross-architecture execution, sandboxing, debugging. | Use cases for interpretation vs. native |
| 7.2 | ARM64 Decode | Instruction encoding, bit field extraction, decode tables. | ARM64 encoding format, instruction classes |
| 7.3 | Integer Instructions | ADD/SUB/MOV/logical/shift/bitfield/conditional. | The integer core of ARM64 |
| 7.4 | Memory Instructions | LDR/STR/LDP/STP, addressing modes, atomics. | Load/store architecture |
| 7.5 | Branches and Calls | B/BL/BR/BLR/RET, condition codes, comparison. | Control flow at machine level |
| 7.6 | Floating Point and SIMD | V registers, FADD/FMUL/FCVT, Q/D/S/H accessors. | 128-bit SIMD on ARM64 |
| 7.7 | The FFI Boundary | Unmapped BR/BLR → host function call. FfiCallArgs. | Interpreter ↔ native transition |
| 7.8 | The Overhead Report | Instruction count, wall clock, per-fixture benchmarks. | Performance characteristics of interpretation |

**Chapter Arc:** Opaque instruction bytes → executing program. The reader has built a CPU emulator.

---

## Part IV: The System

### Chapter 8 — The System
*What does an operating system look like from a binary's perspective?*

<!-- beat:climax characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 8.1 | libSystem and the Shim Layer | _exit, malloc, pthread, stack guard, atexit. | The OS interface binaries actually use |
| 8.2 | Thread-Local Variables | __thread_vars, TLV thunks, pthread_key allocation. | TLS on macOS vs. Linux |
| 8.3 | Exception Handling | __unwind_info, compact unwind, LSDA, personality. | C++ / ObjC exception unwinding |
| 8.4 | Signal Handling | SIGSEGV handler, crash diagnostics, register dump. | Diagnostic infrastructure |
| 8.5 | The Typestate Loader | Parsed → Checked → Mapped → FixedUp → Registered → Protected → Initialized. | Compile-time phase correctness |
| 8.6 | Performance | OnceLock caches, Arc<ClassInfo>, parse caching. 300µs → 100µs. | Optimization of loader overhead |

**Chapter Arc:** Individual pieces → integrated system. The loader is now a complete runtime.

---

### Chapter 9 — The Tools
*Running real Xcode CLI tools on Linux*

<!-- beat:resolution characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 9.1 | The Test Harness | 39 corpus tools, compat gate, regression testing. | How to validate a compatibility layer |
| 9.2 | The Easy Wins | lipo, strip, strings, nm—tools that "just work." | What simple tools need from a loader |
| 9.3 | The Hard Cases | xcodebuild, swift-demangle, notarytool. | Deep dependency chains, Swift interop |
| 9.4 | The Crash Autopsy | Picking apart a SIGSEGV: PC, LR, registers, stack. | Real debugging methodology |
| 9.5 | 51+ Tools Passing | The Docker e2e run. test-xcode-tools. | The full compatibility picture |
| 9.6 | The Corpus | 39 Foundation test programs. Structured validation. | Systematic behavioral testing |

**Chapter Arc:** Theory → practice. Real Apple software runs on Linux.

---

### Chapter 10 — The GUI
*From headless tools to windowed applications*

<!-- beat:denouement characters:reader -->

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| 10.1 | AppKit Stubs | NSApplication, NSWindow, NSView, NSResponder. | GUI framework shimming |
| 10.2 | Core Animation | CALayer, CATransaction. Minimal compositing. | Animation framework stubs |
| 10.3 | The Event Loop | NSEvent dispatch, responder chain, run loop. | macOS event architecture |
| 10.4 | The GUI Bridge | winit + softbuffer backend. set_gui_bridge(). | Pluggable windowing backend |
| 10.5 | What's Next | Remaining gaps, Swift metadata, private frameworks. | Future directions, open problems |

**Chapter Arc:** CLI-only → graphical applications. The platform stack is complete enough to open a window.

---

### Appendix A — The Stack Map
*macOS vs. burdettieOS: layer-by-layer feature parity*

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| A.1 | The Full Stack Diagram | Side-by-side Apple ↔ burdettieOS architecture | Where every crate fits in the real platform |
| A.2 | The Parity Scoreboard | Coverage percentages per layer | What ~92% libSystem or ~43% Foundation means |
| A.3 | Where We Align | Same fixup engine, dispatch, bridging, load order | Why architectural fidelity matters |
| A.4 | Where We Diverge | Shared cache, Mach IPC, Metal, Swift runtime | Intentional trade-offs and their rationale |

**Appendix Arc:** Reference material. The reader can look up any layer and see the exact mapping.

---

### Appendix B — Apple's Undocumented Behaviors
*17 secrets the loader revealed*

| Scene | Title | Core Concept | Reader Learns |
|-------|-------|-------------|---------------|
| B.1 | ABI Secrets | Variadic convention, locale categories, struct stat | Apple ARM64 ABI divergences from AAPCS64 |
| B.2 | Binary Format Secrets | Chained fixup bit fields, rebase offsets, auth metadata | Where Apple's headers lie about their own format |
| B.3 | Runtime Secrets | Tagged pointers, non-pointer ISA, __cxa_throw, EHTYPE | ObjC runtime implementation details |
| B.4 | Framework Secrets | Binary plist, libxo, err/errx, shared cache, +load order | Hidden dependencies in real macOS tools |

**Appendix Arc:** War stories and hard-won knowledge. The reader gains the debugging intuition that only comes from building a compatibility layer.

---

## Structural Notes

**Total chapters:** 10 (across 4 parts) + 2 appendices
**Total scenes:** 75 (67 chapter scenes + 8 appendix scenes)
**Estimated length per scene:** 1,200–2,000 words
**Estimated total:** ~95,000 words

**Progression model:** Each chapter follows a consistent arc:
1. Motivation (why this layer exists — a crash or failure)
2. Theory (what the standard/format/protocol defines)
3. Implementation (how we build it in Rust)
4. Validation (tests, real tools, hex dumps showing success)

**Code integration:** Every scene references real files in the darwin-loader-lab
codebase. Readers can check out the corresponding commit and run the code.
