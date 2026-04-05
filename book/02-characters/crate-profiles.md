# Crate Profiles — Supporting Cast

**Phase:** 02 - Characters
**Purpose:** Character profiles for each major crate in the supporting cast
**Status:** Filled

---

## Overview

Each crate is a "character" in the book's technical narrative. They are the
protagonist's tools, each with a specific role in resolving the collision
between Circle A (macOS) and Circle B (Linux).

---

## macho-parse — The Archaeologist

| Field | Value |
|---|---|
| **Role** | Herald — announces what the binary contains |
| **First Appearance** | Chapter 1: The Binary |
| **What it does** | Parses Mach-O headers, load commands, segments, sections |
| **Source** | `crates/macho-parse/` |
| **Platform layer it addresses** | Layer 1: Binary format |
| **Personality** | Calm, methodical, observant. "Let me read you what this header says." |
| **Strength** | Can read any Mach-O binary: fat binaries, arm64e, chained fixups |
| **Flaw** | Read-only — understands the artifact but can't act on it |
| **Arc** | Introduced as the first tool; becomes the foundation everything else depends on |
| **Signature moment** | First successful parse of a real Xcode binary's header |

**Voice in the narrative:** When the book explains parsing, the prose is
deliberate and observational: "The first four bytes are `cf fa ed fe`. This is
the Mach-O 64-bit magic number, little-endian..."

---

## macho-exec — The Engineer

| Field | Value |
|---|---|
| **Role** | Ally — maps the binary into memory |
| **First Appearance** | Chapter 2: The Map |
| **What it does** | Memory-maps segments with correct permissions, handles ASLR slide |
| **Source** | `crates/macho-exec/` |
| **Platform layer** | Layer 2: Virtual memory layout |
| **Personality** | Precise, literal, hands-on. "I put bytes where they belong." |
| **Strength** | Gets memory layout exactly right — segments, permissions, addresses |
| **Flaw** | Operates purely at the physical level; doesn't understand what the bytes mean |
| **Arc** | From mapping a single binary to mapping an entire dependency tree |
| **Signature moment** | First clean `mmap` of all segments with correct R/W/X permissions |

---

## macho-runtime — The Conductor

| Field | Value |
|---|---|
| **Role** | Mentor — orchestrates the full loading pipeline |
| **First Appearance** | Chapter 3: The Links |
| **What it does** | Coordinates parse → map → fixup → ObjC registration → initialize → execute |
| **Source** | `crates/macho-runtime/` |
| **Platform layer** | All layers (orchestration) |
| **Personality** | Authoritative, organized, deliberate. "First parse, then map, then link, then execute." |
| **Strength** | Knows the correct order. Ensures prerequisites are met before each phase |
| **Flaw** | Rigid — if any phase fails, the whole pipeline fails |
| **Arc** | From executing a single binary to managing a full dependency graph of 20+ dylibs |
| **Signature moment** | First successful end-to-end pipeline: parse → map → link → execute → clean exit |

---

## objc-runtime — The Translator

| Field | Value |
|---|---|
| **Role** | Shapeshifter — speaks both worlds' languages |
| **First Appearance** | Chapter 4: The Runtime |
| **What it does** | Implements Objective-C message dispatch: class registration, selector uniquing, method resolution, ISA pointer handling |
| **Source** | `crates/objc-runtime/` |
| **Platform layer** | Layer 4: Objective-C runtime |
| **Personality** | Adaptable, complex, dynamic. "Tell me the selector and I'll find the method." |
| **Strength** | Can dispatch any Objective-C message, handle categories, resolve method conflicts |
| **Flaw** | The most complex crate — edge cases in non-pointer ISA, tagged pointers, +load ordering |
| **Arc** | From handling a single `objc_msgSend` to supporting the full ObjC class hierarchy |
| **Signature moment** | First successful `objc_msgSend` on a real binary's method |

---

## cf-lite — The Diplomat

| Field | Value |
|---|---|
| **Role** | Ally — presents a familiar face to foreign code |
| **First Appearance** | Chapter 5: The Foundation |
| **What it does** | Implements Foundation class shims: NSString, NSArray, NSDictionary, NSNumber, NSData, etc. |
| **Source** | `crates/cf-lite/` |
| **Platform layer** | Layer 5: Foundation framework |
| **Personality** | Polished, accommodating, broad. "You called -[NSString UTF8String]? I've got you." |
| **Strength** | Covers the most-used Foundation methods; toll-free bridging with core-foundation-lite |
| **Flaw** | ~43% selector coverage means many methods are stubs; must prioritize what real tools actually call |
| **Arc** | From stubbing a single method to supporting the full Foundation surface area that 51+ Xcode tools need |
| **Signature moment** | First `nm` binary that exercises Foundation and produces correct output |

---

## core-foundation-lite — The Twin

| Field | Value |
|---|---|
| **Role** | Ally — the C-language twin of cf-lite |
| **First Appearance** | Chapter 5: The Foundation |
| **What it does** | Implements CoreFoundation C API: CFString, CFArray, CFDictionary, CFRelease, toll-free bridging |
| **Source** | `crates/core-foundation-lite/` |
| **Platform layer** | Layer 5–6: CoreFoundation + toll-free bridging |
| **Personality** | Precise, paired, complementary. "I'm the C face; she's the ObjC face. We're the same object." |
| **Strength** | Toll-free bridging — a CFString IS an NSString at the pointer level |
| **Flaw** | The bridging contract is extremely specific; any layout mismatch cascades |
| **Arc** | From simple CF types to full toll-free bridge with Foundation |
| **Signature moment** | First toll-free bridge: a CFStringRef passed to an ObjC method and handled correctly |

---

## arm64-translate — The Interpreter

| Field | Value |
|---|---|
| **Role** | Ally — understands every word of machine code |
| **First Appearance** | Chapter 7: The Interpreter |
| **What it does** | Decodes and (optionally) translates ARM64 instructions; handles PAC, variadic ABI |
| **Source** | `crates/arm64-translate/` |
| **Platform layer** | Layer 6: ARM64 ABI |
| **Personality** | Literal, exhaustive, precise. "That's an LDR with post-index, signed immediate, writing to x19." |
| **Strength** | Can decode any ARM64 instruction; understands PAC variants (PACIA, PACIB, AUTIA, etc.) |
| **Flaw** | Operates at the instruction level — can miss higher-level patterns |
| **Arc** | From decoding individual instructions to understanding ABI-level patterns (variadic calls, struct returns) |
| **Signature moment** | Diagnosing a variadic printf crash by tracing register usage through the calling convention |

---

## libsystem-shims — The Adapter

| Field | Value |
|---|---|
| **Role** | Threshold Guardian — guards the boundary between worlds |
| **First Appearance** | Chapter 6: The Bridge |
| **What it does** | Implements Apple-specific C library functions: locale handling, err/errx, _NSGetArgc, etc. |
| **Source** | `crates/libsystem-shims/` |
| **Platform layer** | Layer 7: libSystem / OS interface |
| **Personality** | Pragmatic, occasionally deceptive. "You asked for Apple's setlocale? Here's glibc's, with locale category numbers remapped." |
| **Strength** | Bridges the deepest gap: Apple's C library vs. glibc |
| **Flaw** | "Close enough" is dangerous at this layer; struct layout differences can corrupt data silently |
| **Arc** | From simple passthrough to careful translation of undocumented differences |
| **Signature moment** | Fixing the `struct stat` layout difference that caused `stat` to return wrong file sizes |

---

## test-harness — The Judge

| Field | Value |
|---|---|
| **Role** | Mentor — validates correctness |
| **First Appearance** | Chapter 9: The Tools |
| **What it does** | Runs all 51+ Xcode tools inside Docker, compares output against reference, reports pass/fail |
| **Source** | `crates/test-harness/` |
| **Platform layer** | All layers (validation) |
| **Personality** | Impartial, binary, absolute. "84/84. All pass." or "FAIL: cut output differs at line 3." |
| **Strength** | No ambiguity. Either the tool produces correct output or it doesn't |
| **Flaw** | Can't diagnose *why* something failed — only that it did |
| **Arc** | From testing individual tools to the full gate sweep |
| **Signature moment** | "84/84. All pass." — the climax of the book |

---

## macho-inspect — The Driver

| Field | Value |
|---|---|
| **Role** | Herald — the user's entry point |
| **First Appearance** | Chapter 1 (as the tool the reader runs) |
| **What it does** | CLI binary that loads and executes Mach-O files using the full runtime |
| **Source** | `crates/macho-inspect/` |
| **Platform layer** | All layers (orchestration + CLI) |
| **Personality** | Simple, direct, practical. "Give me a binary and I'll run it." |
| **Strength** | The user never touches the internals directly — this is the interface |
| **Flaw** | Hides complexity behind simplicity; when it fails, the error could be in any layer |
| **Arc** | From running `--info` to running `--exec` with full pipeline |
| **Signature moment** | First successful `macho-inspect --exec /opt/xcode-tools/system/uname` → "Darwin" |

---

## Crate Relationships

| Relationship | Nature | Tension |
|---|---|---|
| macho-parse → macho-exec | Parse provides structure; exec maps it | Parse is abstract; exec is physical |
| macho-exec → macho-runtime | Exec provides mapped memory; runtime orchestrates | Exec is a single binary; runtime manages the graph |
| macho-runtime → objc-runtime | Runtime triggers ObjC registration | Runtime owns the pipeline; ObjC owns the dispatch |
| objc-runtime → cf-lite | ObjC dispatch calls Foundation methods | ObjC is generic dispatch; cf-lite is domain knowledge |
| cf-lite ↔ core-foundation-lite | Toll-free bridging — the same object, two faces | Any layout divergence breaks both |
| libsystem-shims → all | Provides the OS-level substrate everyone depends on | The deepest, most dangerous translation layer |
| arm64-translate → macho-exec | Decodes instructions in mapped memory | Decode is read-only; exec is read-write |
| test-harness → macho-inspect | Tests the full system through the CLI | Tests only the output; doesn't test internals |

---

## Related

- [cast-overview.md](cast-overview.md) — Summary of all characters
- [the-reader.md](the-reader.md) — The protagonist who wields these tools
- [the-platform.md](the-platform.md) — The antagonist these tools address
- [../04-plot-and-structure/architecture/entity-registry.md](../04-plot-and-structure/architecture/entity-registry.md) — Entity cross-references
