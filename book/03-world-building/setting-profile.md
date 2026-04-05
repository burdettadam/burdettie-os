# Setting Profile

**Phase:** 03 - World-Building
**Purpose:** Define the physical environments, geography, and key locations of the story world
**Status:** Filled

---

## World-Level Context

**Genre/Type:** Contemporary technical nonfiction. The "world" is a set of digital environments where all action takes place.

**Time Period:** 2023–2025. The code is written against macOS 14/15 on Apple Silicon and runs in Docker on ARM64 Linux.

**Geography Scale:** Five primary "locations," all virtual. No physical geography — the terrain is abstraction layers, memory maps, and data structures.

**Climate & Weather Patterns:** The "weather" is the state of the build and test pipeline. Sunny: all tests pass, Docker builds clean. Stormy: SIGSEGV at an unknown address, 3 AM debugging session.

---

## Primary Settings

### Location #1: The Hex Editor

**Name:** The Hex Editor View
**Type:** Inspection environment — where the reader first encounters raw binary data
**Size:** One screen. 16 bytes per row. Infinite scroll.

**Physical Description:**
Columns of hex bytes, 16 per row. ASCII representation on the right. Offsets on the left in gray. The occasional recognizable string leaps out — `@(#)PROGRAM:lipo`, `__TEXT`, `_objc_msgSend`. Everything else is numbers.

**Defining Physical Features:**
- The magic number `FE ED FA CF` at offset 0 — always the first thing you see
- Load commands cascading down from offset 32 — repeating patterns of command type, size, data
- The `__LINKEDIT` segment near the end — the densest region, where fixups and symbols live

**Lighting:** High contrast. Black background, green or white hex on dark. The stereotypical hacker aesthetic — but it's actually how these tools look.

**Story Function:** This is the "archaeological dig site" — the place where the reader first makes contact with the platform's encoded expectations. Every chapter begins or ends with a hex view showing the concept just learned.

**Sensory Signature:**
1. Visual: Rows of aligned hex digits with occasional ASCII fragments emerging from noise
2. Sound: Silence — hex editors are contemplative spaces
3. Texture: The pattern-recognition itch when you spot a repeating structure in raw bytes

---

### Location #2: The Terminal / Docker Container

**Name:** The Terminal (and the Docker ARM64 container)
**Type:** Execution environment — where binaries run and crash

**Physical Description:**
A dark terminal window. Command prompt. Output scrolling by. Sometimes a clean result: `Darwin 23.0 arm64`. Sometimes a crash dump: register state, stack trace, signal number.

**Key Districts/Areas:**
- **macOS host terminal** — where `cargo test` and `macho-inspect --info` run natively
- **Docker ARM64 container** — `docker run --platform linux/arm64 burdettie-os` — the foreign territory where macOS binaries must work
- **The SIGSEGV zone** — the register dump screen that appears when a binary crashes. PC, LR, x0-x28, fault_addr.

**Defining Physical Features:**
- The Docker build output scrolling by — `Compiling macho-exec v0.1.0`
- The test-xcode-tools output — `PASS: lipo`, `PASS: nm`, 51+ green lines
- The crash report — `SIGSEGV at PC=0xffffb3e1e808`

**Story Function:** The battlefield. Every hypothesis tested here. Success (tool output matches expected) and failure (crash, wrong output) both happen in the terminal. The Docker container is specifically the "hostile territory" — a Linux machine where macOS binaries have no right to work.

**Sensory Signature:**
1. Visual: Green PASS lines scrolling by (43, 44... done) or a red crash splat
2. Sound: The fan spinning up during a Docker build; the silence after a crash
3. Texture: The satisfaction of a clean test run vs. the dread of "Signal 11"

---

### Location #3: The Source Code (Rust IDE)

**Name:** The Source Code Editor
**Type:** Construction site — where the reader builds understanding through writing Rust

**Physical Description:**
Syntax-highlighted Rust code. Types in blue, functions in yellow, strings in green, comments in gray. The file tree on the left showing 41 crates. `cargo check` output in the terminal pane below.

**Key Districts/Areas:**
- **crates/macho-parse/** — the orderly parsing quarter
- **crates/cf-lite/src/** — the sprawling Foundation district (130+ files)
- **crates/objc-runtime/src/shims/** — the dispatch center
- **crates/libsystem-shims/** — the border crossing between macOS and Linux

**Defining Physical Features:**
- Rust's type system visible everywhere — `ParsedMachO<'a>`, `MappedImage`, `ObjcRuntime`
- The `#[cfg(test)]` blocks at the bottom of every file — the validation infrastructure
- Pattern: every shim file has the same structure — imports, struct, impl, tests

**Story Function:** The workshop. Where understanding crystallizes into code. The reader's transformation manifests here — each successfully compiling function is proof of comprehension.

**Sensory Signature:**
1. Visual: Syntax-highlighted Rust with types cascading from simple (u32) to complex (Arc<ObjcRuntime>)
2. Sound: The keystroke rhythm of writing code; the terminal chime on successful build
3. Texture: The flow state of implementation — knowing what to build and building it

---

### Location #4: The Crash Scene (Debugger View)

**Name:** The Register Dump / Crash Diagnostic
**Type:** Crime scene investigation — where the reader interprets crash data

**Physical Description:**
A block of register state: `x0=0x0000000000000000, x1=0xffff91234abc, ... PC=0xffffb3e1e808, LR=0xffffb3dd6d20, fault_addr=0x0`. Sometimes annotated with symbol names if available.

**Defining Physical Features:**
- PC (Program Counter) — where the crash happened
- LR (Link Register) — where we were called from
- fault_addr — what address we tried to access
- x16 — often holds the function pointer being dispatched

**Story Function:** The autopsy table. Every chapter's central mystery is solved here — reading the register state to determine which platform contract was violated. The reader learns to "think in registers."

**Sensory Signature:**
1. Visual: Aligned register values, patterns in the addresses (high bits = stack, low bits = code)
2. Sound: The internal "click" when a register value suddenly makes sense
3. Texture: The detective's frustration → breakthrough → satisfaction cycle

---

### Location #5: The Architecture Diagram

**Name:** Memory Maps and Stack Diagrams
**Type:** Conceptual space — where relationships between components become visible

**Physical Description:**
ASCII art or diagrams showing memory layout: `__TEXT` at 0x100000000, `__DATA` at 0x100010000, `__LINKEDIT` at 0x100020000. Or stack comparison diagrams: macOS ↔ burdettieOS.

**Story Function:** The map room. Where the reader zooms out from individual bytes to see the whole architecture. The full-page stack comparison diagram in Chapter 3's world-building section is the book's central reference image.

---

## Travel & Distance

**How do people move between locations?**
Keystrokes. Tab between hex editor, terminal, and IDE. The speed of switching is instant — but the *cognitive* distance between "reading hex" and "writing Rust" is enormous.

**Typical travel times:**
- Hex editor → Terminal: 1 keystroke, but a conceptual leap from observation to action
- Terminal → Crash scene: 0 time (crash happens in terminal), but hours of interpretation
- Crash scene → Source code: The longest journey — translating a register dump into a code fix
- Source code → Terminal: `cargo build && docker run` — seconds to minutes

**Communication speed:** Instant. The compile-test-crash cycle is the "communication" between reader and platform. On fast machines: seconds. On Docker ARM64 QEMU: minutes.

---

## Seasonal & Temporal Variation

**How does this setting change across the book?**

| Phase | Dominant Location | Mood | What Changed |
|-------|------------------|------|-------------|
| Ch 1–2 (Artifact) | Hex editor | Exploratory, archaeological | Reader can now "see" inside binaries |
| Ch 3–4 (Linker) | Terminal + crash scene | Tense, investigative | Each crash reveals a new layer |
| Ch 5 (Foundation) | Source code (cf-lite) | Productive, ambitious | Building is the primary activity |
| Ch 6 (Bridge) | Docker terminal + crash scene | Dark, frustrated | The hardest debugging in the book |
| Ch 7 (Interpreter) | Source code (arm64-translate) | Technical, precise | Building a CPU emulator |
| Ch 8–9 (System) | Terminal + test output | Triumphant, methodical | Green test results accumulate |
| Ch 10 (GUI) | Architecture diagrams | Reflective, forward-looking | Zooming out to see the whole |

---

## Setting and Theme

**How does your setting reflect or reinforce your theme?**
The settings ARE the theme. The hex editor embodies "platforms encode their assumptions in the binary." The crash scene embodies "crashes are information." The source code embodies "understanding comes from rebuilding." The Docker container embodies "compatibility is empathy across contexts."

**Setting as obstacle:**
The Docker ARM64 container is the primary environmental obstacle — a Linux machine where macOS binaries have no native support. Every ABI mismatch is a feature of this hostile territory.

**Setting as resource:**
Rust's type system (visible in the source code setting) is the reader's greatest tool. Typestate patterns, comprehensive enums, and the borrow checker catch errors that would be invisible in C.

---

## Related

- [technical-landscape.md](technical-landscape.md) — The two worlds (macOS vs. Linux) this setting spans
- [sensory-palette.md](sensory-palette.md) — Sensory details for each location
- [world-rules.md](world-rules.md) — Constraints that govern behavior in these settings
- [../02-characters/the-reader.md](../02-characters/the-reader.md) — Character shaped by place
- [../01-concept/theme-exploration.md](../01-concept/theme-exploration.md) — Setting serves theme
