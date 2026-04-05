# Scene Card Index

**Phase:** 04 - Plot and Structure / Scene Design
**Purpose:** CAP-compliant scene cards (Units) for each chapter, ready for drafting
**Status:** Filled — 75 scene cards created as CAP Units

---

## CAP Corpus Layout

Scene cards are stored as [Canonical Artifact Protocol](https://github.com/Origin-Draft/cap-protocol) v1.0.0 Units using the narrative profile. Each card is a `Unit` with observables, structure, transitions, interpretations, and participant states — derived from the writing guides, not crafted as prose.

```
book/cap/
  artifact.json                    # Top-level artifact: entities, relationships, views
  units/
    ch01_the_binary.json           # 6 units  (seq 1–6)
    ch02_the_map.json              # 5 units  (seq 7–11)
    ch03_the_links.json            # 7 units  (seq 12–18)
    ch04_the_runtime.json          # 8 units  (seq 19–26)
    ch05_the_foundation.json       # 9 units  (seq 27–35)
    ch06_the_bridge.json           # 7 units  (seq 36–42)
    ch07_the_interpreter.json      # 8 units  (seq 43–50)
    ch08_the_system.json           # 6 units  (seq 51–56)
    ch09_the_tools.json            # 6 units  (seq 57–62)
    ch10_the_gui.json              # 5 units  (seq 63–67)
    appendices.json                # 8 units  (seq 68–75)
```

### Per-Unit Fields (CAP Narrative Profile)

| Field | Section | Description |
|-------|---------|-------------|
| `unit_id` | core | Scene slug: `ch{NN}_s{NN}` |
| `sequence_index` | core | Global position (1–75) |
| `observables.participants` | observables | Entity refs (crates, reader, platform) |
| `observables.context` | observables | Setting, chapter, estimated words |
| `observables.event_type` | observables | discovery / implementation / failure / resolution / theory |
| `structure.position` | structure | Normalized position 0.0–1.0 |
| `structure.causal_role` | structure | setup / trigger / complication / resolution |
| `structure.steps` | structure | Ordered sub-actions within the scene |
| `structure.transition` | structure | Polarity shift: before → after with trigger |
| `structure.grouping.beat` | structure | Beat sheet alignment |
| `interpretations.subtext` | interpretations | Deeper lesson beneath the technical content |
| `interpretations.motifs` | interpretations | Recurring symbols (0xFEEDFACF, SIGSEGV, etc.) |
| `participant_states` | participant_states | Reader's knowledge/capability trajectory |

---

## Part I: The Artifact

### Chapter 1 — The Binary (6 scenes) → `ch01_the_binary.json`

- [x] `ch01_s01` — The Black Box
- [x] `ch01_s02` — The Header
- [x] `ch01_s03` — The Load Commands
- [x] `ch01_s04` — The Sections
- [x] `ch01_s05` — The Symbol Table
- [x] `ch01_s06` — Building the Parser

### Chapter 2 — The Map (5 scenes) → `ch02_the_map.json`

- [x] `ch02_s01` — Virtual Memory Primer
- [x] `ch02_s02` — Segments to Memory
- [x] `ch02_s03` — ASLR and the Slide
- [x] `ch02_s04` — The MappedImage
- [x] `ch02_s05` — Code Signing and Verification

---

## Part II: The Linker

### Chapter 3 — The Links (7 scenes) → `ch03_the_links.json`

- [x] `ch03_s01` — Why Dynamic Linking
- [x] `ch03_s02` — Import Binds
- [x] `ch03_s03` — Chained Fixups
- [x] `ch03_s04` — Auth Fixups and PAC
- [x] `ch03_s05` — The Stub Resolver
- [x] `ch03_s06` — Dependency Loading
- [x] `ch03_s07` — The Shared Cache

### Chapter 4 — The Runtime (8 scenes) → `ch04_the_runtime.json`

- [x] `ch04_s01` — The ObjC Object Model
- [x] `ch04_s02` — Selector Interning
- [x] `ch04_s03` — Message Dispatch
- [x] `ch04_s04` — Building objc_msgSend
- [x] `ch04_s05` — Class Registration
- [x] `ch04_s06` — Protocols and Categories
- [x] `ch04_s07` — Dynamic Class Creation
- [x] `ch04_s08` — Tagged Pointers

---

## Part III: The Platform

### Chapter 5 — The Foundation (9 scenes) → `ch05_the_foundation.json`

- [x] `ch05_s01` — What Foundation Provides
- [x] `ch05_s02` — Synthetic Class Registration
- [x] `ch05_s03` — NSString Internals
- [x] `ch05_s04` — Collections
- [x] `ch05_s05` — NSFileManager and Friends
- [x] `ch05_s06` — NSURLSession
- [x] `ch05_s07` — NSTask and Process Control
- [x] `ch05_s08` — Toll-Free Bridging
- [x] `ch05_s09` — The 2,682 Tests

### Chapter 6 — The Bridge (7 scenes) → `ch06_the_bridge.json`

- [x] `ch06_s01` — Two ARM64 Worlds
- [x] `ch06_s02` — The Variadic Crisis
- [x] `ch06_s03` — setlocale and Category Numbers
- [x] `ch06_s04` — struct stat Translation
- [x] `ch06_s05` — Data Symbols
- [x] `ch06_s06` — Library Preloading
- [x] `ch06_s07` — The Docker Container

### Chapter 7 — The Interpreter (8 scenes) → `ch07_the_interpreter.json`

- [x] `ch07_s01` — Why Interpret
- [x] `ch07_s02` — ARM64 Decode
- [x] `ch07_s03` — Integer Instructions
- [x] `ch07_s04` — Memory Instructions
- [x] `ch07_s05` — Branches and Calls
- [x] `ch07_s06` — Floating Point and SIMD
- [x] `ch07_s07` — The FFI Boundary
- [x] `ch07_s08` — The Overhead Report

---

## Part IV: The System

### Chapter 8 — The System (6 scenes) → `ch08_the_system.json`

- [x] `ch08_s01` — libSystem and the Shim Layer
- [x] `ch08_s02` — Thread-Local Variables
- [x] `ch08_s03` — Exception Handling
- [x] `ch08_s04` — Signal Handling
- [x] `ch08_s05` — The Typestate Loader
- [x] `ch08_s06` — Performance

### Chapter 9 — The Tools (6 scenes) → `ch09_the_tools.json`

- [x] `ch09_s01` — The Test Harness
- [x] `ch09_s02` — The Easy Wins
- [x] `ch09_s03` — The Hard Cases
- [x] `ch09_s04` — The Crash Autopsy
- [x] `ch09_s05` — 51+ Tools Passing
- [x] `ch09_s06` — The Corpus

### Chapter 10 — The GUI (5 scenes) → `ch10_the_gui.json`

- [x] `ch10_s01` — AppKit Stubs
- [x] `ch10_s02` — Core Animation
- [x] `ch10_s03` — The Event Loop
- [x] `ch10_s04` — The GUI Bridge
- [x] `ch10_s05` — What's Next

---

## Appendices → `appendices.json`

### Appendix A — The Stack Map (4 scenes)

- [x] `appa_s01` — The Full Stack Diagram
- [x] `appa_s02` — The Parity Scoreboard
- [x] `appa_s03` — Where We Align
- [x] `appa_s04` — Where We Diverge

### Appendix B — Apple's Undocumented Behaviors (4 scenes)

- [x] `appb_s01` — ABI Secrets
- [x] `appb_s02` — Binary Format Secrets
- [x] `appb_s03` — Runtime Secrets
- [x] `appb_s04` — Framework Secrets

---

**Total: 75 scene cards created (67 chapters + 8 appendix) — see `book/cap/`**
