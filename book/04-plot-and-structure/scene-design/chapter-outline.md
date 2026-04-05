# Chapter Outlines — Expanded

**Phase:** 04 - Plot and Structure / Scene Design
**Purpose:** Detailed scene-level outlines for each chapter with emotional arc, pacing, and transitions
**Status:** Filled

---

## How to Read These Outlines

Each chapter outline includes:
- **Opening beat** — the crash or question that motivates the chapter
- **Scene summaries** — one paragraph per scene with purpose and emotional tone
- **Closing beat** — what the reader can now do that they couldn't before
- **Transition** — how this chapter's ending creates the next chapter's opening
- **Estimated length** — word count target

---

## Part I: The Artifact

### Chapter 1 — The Binary (~12,000 words)

**Opening Beat:** Hex editor shows `CF FA ED FE`. The reader sees inside a binary for the first time. Then: `SIGSEGV` when they try to run it on Linux.

**Scene 1.1 — The Black Box** (~1,800 words)
Open `/usr/bin/true` in hex. The magic number introduction. Historical context (NeXT, 1997). Establish the book's voice: concrete, direct, reverent toward the engineering. *Emotional tone: wonder.*

**Scene 1.2 — The Header** (~2,000 words)
Parse `mach_header_64`: cpu_type, cpu_subtype, filetype, ncmds, sizeofcmds, flags. Show how the header is a table of contents for everything that follows. First introduction of Rust struct mapping. *Emotional tone: structured learning.*

**Scene 1.3 — The Load Commands** (~2,200 words)
Walk the load command array. Introduce the taxonomy: segment commands (LC_SEGMENT_64), dependency commands (LC_LOAD_DYLIB), entry point (LC_MAIN). Show that the binary is self-describing — it tells the loader what to do. *Emotional tone: growing confidence.*

**Scene 1.4 — The Sections** (~2,000 words)
Dive into sections within segments. `__TEXT/__text` (executable code), `__DATA/__objc_classlist` (ObjC metadata), `__LINKEDIT` (linking data). Show real section contents with hex side-by-side. *Emotional tone: deepening understanding.*

**Scene 1.5 — The Symbol Table** (~2,000 words)
nlist_64 entries, string table, indirect symbols. External vs. local vs. undefined. Show how a binary declares what it needs from the outside world. *Emotional tone: anticipation — these unresolved symbols are the "links" that Chapter 3 will connect.*

**Scene 1.6 — Building the Parser** (~2,000 words)
Introduce macho-parse crate. Zero-copy parsing with Rust's type system. Show ParsedMachO struct. Run `macho-inspect --info` on a real binary. The reader has their first tool. *Emotional tone: accomplishment.*

**Closing Beat:** The reader can parse any Mach-O binary. But parsing isn't running. The binary is understood but inert.

**Transition → Ch 2:** "We can read the binary. Now we need to give it somewhere to live."

---

### Chapter 2 — The Map (~10,000 words)

**Opening Beat:** The parsed binary is a data structure on the heap. It can't execute because it's not at a real address with correct permissions.

**Scene 2.1 — Virtual Memory Primer** (~2,000 words)
Pages, permissions (RWX), mmap(), MAP_ANONYMOUS. The concept of address space as a canvas. Short, focused — just enough to understand what comes next. *Emotional tone: foundational.*

**Scene 2.2 — Segments to Memory** (~2,200 words)
Map __TEXT as r-x, __DATA as rw-, __LINKEDIT as r--. Show the `mmap()` calls. Explain page alignment and why segment sizes are multiples of 16K. *Emotional tone: mechanical satisfaction — puzzle pieces fitting together.*

**Scene 2.3 — ASLR and the Slide** (~1,800 words)
PIE binaries land at random addresses. The "slide" is the difference between preferred and actual base address. Every pointer in the binary needs adjustment. *Emotional tone: "ah, THAT'S why addresses change."*

**Scene 2.4 — The MappedImage** (~2,200 words)
Introduce macho-exec crate. MappedImage struct wraps the memory mapping. Show the lifecycle: parse → validate → map → ready for linking. *Emotional tone: architectural clarity.*

**Scene 2.5 — Code Signing and Verification** (~1,800 words)
LC_CODE_SIGNATURE, code directory, page hash verification. Light treatment — enough to understand why Apple cares, not enough to implement full verification. *Emotional tone: awareness of the security layer.*

**Closing Beat:** The binary is alive in memory at a real address with correct permissions. But every external function pointer is still `NULL`.

**Transition → Ch 3:** "The binary is loaded. But look at the GOT — every entry is zero. It needs its libraries."

---

## Part II: The Linker

### Chapter 3 — The Links (~14,000 words)

**Opening Beat:** GOT entries are all zero. First function call: `SIGSEGV` — jumped to address 0x0.

**Scene 3.1 — Why Dynamic Linking** (~1,500 words)
Motivation: shared code, version independence, memory savings. Static vs. dynamic comparison. Install names and `@rpath`. *Emotional tone: context-setting.*

**Scene 3.2 — Import Binds** (~2,000 words)
LC_DYLD_INFO_ONLY bind opcodes. The opcode VM — BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS, BIND_OPCODE_DO_BIND. Walk through a real bind opcode stream byte by byte. *Emotional tone: code-breaking, archaeologist vibe.*

**Scene 3.3 — Chained Fixups** (~2,500 words)
The modern format: LC_DYLD_CHAINED_FIXUPS. Pointer chains linking data pages. Rebase vs. bind encoded in the same word. **The moment the documentation lies:** Apple's header says fields are at certain bit positions, but the actual encoding differs. The binary tells the truth. *Emotional tone: revelation + vindication of the empirical approach.*

**Scene 3.4 — Auth Fixups and PAC** (~2,000 words)
arm64e authenticated binds. PAC keys, blend discriminators, the `0x40000000000000` trap address on failure. Show a real PAC crash and how to decode it. *Emotional tone: entering deep territory — this is security at the pointer level.*

**Scene 3.5 — The Stub Resolver** (~1,500 words)
Symbol name → host `dlsym()`. Safe list: which symbols we resolve, which we shim, which we reject. Prefix matching rules. *Emotional tone: engineering decision-making.*

**Scene 3.6 — Dependency Loading** (~1,500 words)
BFS walk of LC_LOAD_DYLIB commands. ImageRegistry. Cycle detection. Show the dependency graph for a real Xcode tool (many libraries deep). *Emotional tone: seeing the scale of the dependency web.*

**Scene 3.7 — The Shared Cache** (~2,500 words)
dyld_shared_cache_arm64e format. 2GB of prelinked libraries in a single file. Image lookup, slide info, local symbol extraction. The reason most `.dylib` files don't exist on disk anymore. *Emotional tone: awe at the engineering + "this makes our job harder."*

**Closing Beat:** All symbols resolved. Simple C-only tools (strings, nm) now execute correctly.

**Transition → Ch 4:** "C functions work. But the first ObjC call — `[NSObject init]` — crashes with a different error. There's another runtime hidden inside these binaries."

---

### Chapter 4 — The Runtime (~12,000 words)

**Opening Beat:** Symbol resolution succeeded but `objc_msgSend(receiver, @selector(init))` crashes. There's no ObjC runtime on Linux.

**Scene 4.1 — The ObjC Object Model** (~1,800 words)
ISA pointers, class_t, class_ro_t, method_list_t. Show the binary representation of a class. The reader sees ObjC objects for the first time as memory layouts, not API abstractions. *Emotional tone: paradigm shift.*

**Scene 4.2 — Selector Interning** (~1,200 words)
__objc_selrefs section, SEL as interned C strings, selref uniquing. Why every "init" string in the binary must resolve to the same pointer. *Emotional tone: technical elegance.*

**Scene 4.3 — Message Dispatch** (~2,000 words)
The hot path: receiver → ISA → cache lookup → method list scan → superclass chain. Show the algorithm. Compare to C++ vtable dispatch. ObjC is LATE-binding: the method isn't determined until the message is sent. *Emotional tone: "this is more elegant than I expected."*

**Scene 4.4 — Building objc_msgSend** (~2,000 words)
ARM64 assembly trampoline. `mrs x16, TPIDR_EL0` for TLS. Register save area. LookupResult struct. The trampoline.s file — the single most critical piece of assembly in the project. *Emotional tone: high-wire act — assembly is precise or it's wrong.*

**Scene 4.5 — Class Registration** (~1,500 words)
Parse __objc_classlist, build runtime class table. classes_by_name, classes_by_addr. Handle metaclasses. Register every class from every loaded image. *Emotional tone: systematic construction.*

**Scene 4.6 — Protocols and Categories** (~1,500 words)
__objc_protolist, __objc_catlist. Category method injection into existing classes. Protocol conformance tables. *Emotional tone: completing the picture.*

**Scene 4.7 — Dynamic Class Creation** (~1,200 words)
objc_allocateClassPair, class_addMethod, KVO ISA swizzling. The runtime isn't just static — it modifies itself at runtime. *Emotional tone: runtime dynamism is beautiful AND dangerous.*

**Scene 4.8 — Tagged Pointers** (~1,000 words)
Bit 63 set means "this isn't a real pointer." NSNumber, NSDate, NSString small values encoded in the pointer itself. Show how to detect and decode them. *Emotional tone: "apple engineers are clever."*

**Closing Beat:** objc_msgSend dispatches correctly. Synthetic classes respond to messages. The ObjC runtime is alive.

**Transition → Ch 5:** "The runtime works. But `[NSString stringWithFormat:]` returns garbage. We need Foundation."

---

## Part III: The Platform

### Chapter 5 — The Foundation (~12,000 words)

**Opening Beat:** ObjC dispatch works but Foundation method calls produce wrong results. We have a runtime with no classes in it.

**Scenes 5.1–5.9:** Each scene adds a Foundation capability: NSString, collections, file management, networking, subprocess control, toll-free bridging, and the massive test suite. The chapter is a montage of shim construction with each scene adding real functionality. *Emotional arc: determination → momentum → confidence.*

**Closing Beat:** 2,682 cf-lite tests pass. 39 corpus tools succeed.

**Transition → Ch 6:** "Everything works on macOS. Build the Docker image. Run the same tests. Crash."

---

### Chapter 6 — The Bridge (~10,000 words)

**Opening Beat:** Identical code, identical binary, different OS. Crashes on Linux that don't happen on macOS. The ABI is different.

**Scenes 6.1–6.7:** The hardest chapter. ABI divergence, variadic calling convention, struct layout differences, data symbol resolution, Docker environment assembly. *Emotional arc: frustration → grim determination → hard-won victory.*

**Closing Beat:** 51+ tools pass in Docker on linux/arm64.

**Transition → Ch 7:** "What if the host machine isn't ARM64 at all?"

---

### Chapter 7 — The Interpreter (~12,000 words)

**Opening Beat:** "Can we run ARM64 code without an ARM64 CPU?" The interpreter concept.

**Scenes 7.1–7.8:** ARM64 instruction decoding, integer/memory/branch/SIMD execution, FFI boundary crossing, performance analysis. *Emotional arc: intellectual excitement → methodical construction → honest performance assessment.*

**Closing Beat:** 27 interpreted ObjC tests pass. The overhead is significant but the capability is real.

**Transition → Ch 8:** "Individual components work. Now we need a system."

---

## Part IV: The System

### Chapter 8 — The System (~8,000 words)

**Opening Beat:** The pipeline is functional but fragile and slow. Time to engineer reliability.

**Scenes 8.1–8.6:** OS interface, TLS, exception handling, diagnostics, typestate loader, performance optimization. *Emotional arc: satisfaction of integration, pride in engineering quality.*

**Closing Beat:** Typestate loader prevents invalid phase transitions at compile time. Load time: 100µs.

**Transition → Ch 9:** "Is it good enough for real software?"

---

### Chapter 9 — The Tools (~8,000 words)

**Opening Beat:** "Does it work with real Apple software, not just test fixtures?"

**Scenes 9.1–9.6:** Test harness, easy tools, hard tools, crash autopsies, the full Docker e2e run, the validation corpus. *Emotional arc: tension → vindication.*

**Closing Beat:** 84/84 tools pass. The compat gate holds.

**Transition → Ch 10:** "CLI tools run. What about a window?"

---

### Chapter 10 — The GUI (~5,000 words)

**Opening Beat:** "Everything so far is headless. Can we open a macOS application window on Linux?"

**Scenes 10.1–10.5:** AppKit stubs, Core Animation, event loop, GUI bridge, future directions. *Emotional arc: expansion → possibility → humility.*

**Closing Beat:** A window opens. The reader has built a macOS runtime. The `0xFEEDFACF` that was foreign in Chapter 1 is now familiar. The journey is complete.

---

## Pacing Summary

| Chapter | Words | Scenes | Pace |
|---|---|---|---|
| 1 — The Binary | 12,000 | 6 | Measured, exploratory |
| 2 — The Map | 10,000 | 5 | Building momentum |
| 3 — The Links | 14,000 | 7 | Dense, technical peak |
| 4 — The Runtime | 12,000 | 8 | Revelatory, exciting |
| 5 — The Foundation | 12,000 | 9 | Montage, productive |
| 6 — The Bridge | 10,000 | 7 | Grueling, adversarial |
| 7 — The Interpreter | 12,000 | 8 | Intellectual, methodical |
| 8 — The System | 8,000 | 6 | Integrative, smooth |
| 9 — The Tools | 8,000 | 6 | Tense → triumphant |
| 10 — The GUI | 5,000 | 5 | Expansive, closing |
| **Total** | **~103,000** | **67** | |

---

## Related

- [scene-card-index.md](scene-card-index.md) — Scene slots for each chapter
- [opening-hook.md](opening-hook.md) — Detailed design of Scene 1.1 and 1.2
- [../architecture/story-architecture.md](../architecture/story-architecture.md) — Master chapter/scene table
- [../blueprint/chapter-blueprint.md](../blueprint/chapter-blueprint.md) — Crate/file mappings per chapter
- [../architecture/beat-sheet.md](../architecture/beat-sheet.md) — Emotional beats mapped to chapters
