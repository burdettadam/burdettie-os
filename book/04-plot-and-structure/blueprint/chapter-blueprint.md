# Chapter Blueprint

**Phase:** 04 - Plot and Structure / Blueprint
**Purpose:** Map each chapter to source files, key commits, and validation criteria
**Status:** Outline

---

## Blueprint Format

Each chapter maps to:
- **Primary crate(s)** — the source code the reader studies
- **Key source files** — specific files to reference in listings
- **Opening crash** — the motivating failure that opens the chapter
- **Closing validation** — the test/tool that proves the chapter's work
- **Prerequisites** — which prior chapters must be understood

---

## Chapter Blueprints

### Ch 1 — The Binary
- **Crate:** macho-parse, macho-types
- **Key files:** macho-parse/src/lib.rs, macho-types/src/header.rs, macho-types/src/load_command.rs
- **Opening:** `hexdump -C /usr/bin/true | head` → "What are these bytes?"
- **Closing:** `macho-inspect --info /path/to/binary` → clean parsed output
- **Prerequisites:** None

### Ch 2 — The Map
- **Crate:** macho-exec
- **Key files:** macho-exec/src/mapped.rs, macho-exec/src/plan.rs
- **Opening:** Parsed binary can't execute—it's just data on the heap
- **Closing:** MappedImage with correct segment permissions
- **Prerequisites:** Ch 1

### Ch 3 — The Links
- **Crate:** macho-exec (fixup/), macho-resolve
- **Key files:** fixup/chained/walk.rs, fixup/resolver.rs, macho-shared-cache/
- **Opening:** Mapped binary crashes instantly—all function pointers are zero
- **Closing:** All symbols resolved, tools that only need C functions work (strings, nm)
- **Prerequisites:** Ch 1, Ch 2

### Ch 4 — The Runtime
- **Crate:** objc-runtime, macho-objc
- **Key files:** objc-runtime/src/class/, shims/dispatch.rs, trampoline.s
- **Opening:** Symbol resolution works but first ObjC call crashes
- **Closing:** objc_msgSend dispatches correctly, synthetic classes respond
- **Prerequisites:** Ch 1, Ch 2, Ch 3

### Ch 5 — The Foundation
- **Crate:** cf-lite, core-foundation-lite
- **Key files:** cf-lite/src/foundation/, cf-lite/src/cfstring/
- **Opening:** ObjC dispatch works but [NSString stringWithFormat:] returns garbage
- **Closing:** 2,682 cf-lite tests pass, 39 corpus tools pass
- **Prerequisites:** Ch 4

### Ch 6 — The Bridge
- **Crate:** libsystem-shims, platform-linux
- **Key files:** libsystem-shims/src/darwin_compat.rs, cross_platform/variadic_shims.rs
- **Opening:** Tool works on macOS host but crashes in Docker with identical loader
- **Closing:** 51+ tools pass in linux/arm64 Docker container
- **Prerequisites:** Ch 3, Ch 5

### Ch 7 — The Interpreter
- **Crate:** arm64-translate
- **Key files:** arm64-translate/src/decode.rs, cpu.rs, interp.rs, ffi.rs
- **Opening:** "What if the host can't natively execute the guest instructions?"
- **Closing:** 27 interpreted ObjC tests pass, overhead report
- **Prerequisites:** Ch 3, Ch 4

### Ch 8 — The System
- **Crate:** macho-runtime, runtime-diagnostics
- **Key files:** macho-runtime/src/typestate/, macho-exec/src/tlv.rs, unwind.rs
- **Opening:** Individual components work but the pipeline is fragile and slow
- **Closing:** Typestate loader compiles away invalid phase transitions, 300µs→100µs
- **Prerequisites:** All prior chapters

### Ch 9 — The Tools
- **Crate:** macho-inspect, test-harness
- **Key files:** macho-inspect/src/exec.rs, test-harness/
- **Opening:** "Does it actually work with real software?"
- **Closing:** test-xcode-tools reports 51+ PASS in Docker
- **Prerequisites:** All prior chapters

### Ch 10 — The GUI
- **Crate:** app-kit-lite, ui-view, ui-controls, window-backend-darwin
- **Key files:** cf-lite/src/calayer.rs, gui_bridge.rs, nsapplication.rs
- **Opening:** CLI tools work, but what about windowed apps?
- **Closing:** cocoa_window fixture opens a real window via winit backend
- **Prerequisites:** Ch 5, Ch 8
