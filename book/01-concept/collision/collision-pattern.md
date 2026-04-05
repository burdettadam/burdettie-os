# Collision Pattern Selection

<!-- collision_pattern:world_crossing -->

**Collision Pattern Slug:** `platform_collision`
**Collision Pattern Name:** "Platform Collision (macOS binary meets Linux host)"
**Collision Type:** `world_crossing`

**Phase:** 01 - Concept / Collision
**Purpose:** Define the inciting incident, power dynamics, and escalation engine
**Status:** Filled

---

## Power Asymmetry

**Who holds structural power:** Circle A (the macOS platform) — the binary
dictates what the environment must provide. Every unresolved symbol, every
expected memory layout, every calling convention is a demand from Circle A.

**Who holds social or emotional power:** Circle B (the reader/builder) — the
reader has agency, creativity, and tools. The platform makes demands; the
builder decides how to meet them.

**Nature of the asymmetry:**
At the start, the power is overwhelmingly with Circle A: the binary crashes
because the platform isn't there, and the builder has no idea what's missing.
As the book progresses, power shifts: each implemented layer reduces the binary's
demands and increases the builder's understanding. By Chapter 9, the builder
has met every demand — the power dynamic is equalized.

---

## Arc Polarity

**Starting polarity:** `hostile` — the binary and the host system are completely
incompatible. Running the binary crashes immediately.

**Target polarity:** `integrated` — 84/84 Xcode tools run successfully. The two
worlds coexist functionally.

**How the arc shifts:**
Each chapter adds one layer of compatibility. The polarity shifts incrementally:
hostile → recognized (can parse) → mapped (can load) → linked (can resolve
symbols) → dispatching (can call ObjC methods) → shimmed (can call Foundation)
→ executing (can run ARM64 code) → integrated (all tools pass).

---

## Circle Constraints

**Circle A:** The macOS ecosystem — Mach-O format, dyld, ObjC runtime,
Foundation, CoreFoundation, libSystem

**Circle B:** Linux ARM64 with Rust — ELF format, glibc, no ObjC, no
Foundation, different calling conventions

**Physical or institutional barrier between them:**
The Linux kernel does not recognize the Mach-O binary format. The `execve()`
syscall fails on `0xfeedfacf`. There is no Objective-C runtime installed. There
are no Apple frameworks. The barrier is absolute — not a single instruction
of guest code can execute without the loader.

**What dissolves the barrier:**
The loader itself. Specifically: `macho-parse` dissolves the format barrier.
`macho-exec` dissolves the memory barrier. `macho-runtime` dissolves the linking
barrier. `objc-runtime` dissolves the dispatch barrier. `cf-lite` dissolves the
framework barrier. Each crate removes one layer of the wall.

---

## Cast Slot Manifest

| Slot | Role | Circle | Notes |
|------|------|--------|-------|
| `protagonist` | The Builder (Reader/Author) | Bridge | Lives in B, penetrates A |
| `antagonist_world` | The Apple Platform | A | Demands are implicit; every crash is an unspoken requirement |
| `guide` | The Author's debugging experience | Bridge | Past tense voice; "we discovered that..." |
| `translator` | objc-runtime crate | Bridge | Speaks both worlds' languages |
| `adapter` | libsystem-shims crate | Bridge | Makes B look like A at the syscall/libc level |
| `artifact` | The Mach-O binary | A | The object that carries Circle A's expectations into Circle B |
| `validator` | test-xcode-tools | B | 84/84 compat gate — the measurable proof of collision resolution |

---

## The Inciting Incident

**What happens:** You have a macOS binary. You have a Linux ARM64 machine.
You run the binary. It segfaults immediately.

**Why it matters:** That SIGSEGV is not a failure—it's a question. The binary
expected a platform that isn't there. Every layer of the missing platform
becomes a chapter of the book.

See [inciting-incident-selection.md](inciting-incident-selection.md) for the
full scene design.

---

## The Collision Worlds

**World A:** The macOS platform — dyld, Objective-C runtime, Foundation,
CoreFoundation, libSystem, PAC, tagged pointers, shared caches. A fully
integrated, tightly coupled ecosystem where everything knows about everything
else.

**World B:** A bare Linux ARM64 system — ELF binaries, glibc, no Objective-C,
no Foundation, different calling conventions, no pointer authentication, no
shared cache.

**The Collision:** Place a World A binary into World B and watch every
assumption shatter. Each shattered assumption becomes a lesson.

See [social-circles.md](social-circles.md) for the full social circle analysis.

---

## Escalation Pattern

| Stage | What Breaks | What It Teaches | Crate That Fixes It |
|-------|------------|----------------|-------------------|
| 1. Parse | Can't even read the file format | Mach-O structure | macho-parse |
| 2. Map | Wrong memory layout, bad permissions | Virtual memory, ASLR | macho-exec |
| 3. Link | Unresolved symbols, wrong addresses | Dynamic linking, fixups | macho-runtime |
| 4. Dispatch | objc_msgSend crashes on first call | ObjC runtime internals | objc-runtime |
| 5. Shim | Foundation method returns garbage | Framework architecture | cf-lite |
| 6. ABI | printf("%s") reads wrong register | Calling convention diffs | arm64-translate |
| 7. Execute | Instructions decode but wrong results | ARM64 ISA subtleties | arm64-translate |
| 8. Integrate | Individual pieces work but pipeline fails | System design | libsystem-shims |
| 9. Validate | Tools run but produce wrong output | End-to-end correctness | test-harness |
| 10. Extend | CLI works but GUI crashes | The never-ending platform | app-kit-lite |

Each collision stage maps to a chapter. The escalation is the book's engine.

---

## How This Collision Serves the Plot Type

**Plot type selected:** Discovery / Construction narrative

**Connection:**
The platform collision generates a natural 10-stage escalation because Apple's
platform is layered. Each layer depends on the one below it. You can't shim
Foundation until you have ObjC dispatch. You can't dispatch until you've linked
symbols. You can't link until you've mapped memory. You can't map until you've
parsed headers. The dependency chain IS the plot structure.

---

## Model Selection Rationale

**What other collision patterns were considered?**
- *Infiltration:* The loader infiltrates the Apple ecosystem from outside.
  Rejected — too adversarial. The book respects Apple's engineering.
- *Translation:* Translating one platform's language to another. Partially
  used — but translation implies equivalence. The collision pattern captures
  the asymmetry and the drama better.
- *Archaeology:* Excavating a lost civilization. Used as a secondary metaphor
  but too passive as the primary engine.

**What made this pattern the right fit?**
The logline itself describes a collision: a macOS binary placed into a Linux
environment. The pattern is not metaphorical — it's literal. The collision IS
the premise.

**Does this pattern create the necessary engine?**
[x] Yes — each stage of the collision generates a chapter's worth of material.
The escalation from "can't parse" to "everything passes" is the narrative arc.

---

## Notes

The escalation pattern has a property that maps well to technical nonfiction:
each stage's resolution enables the next stage's failure. Solving parsing makes
linking failures visible. Solving linking makes dispatch failures visible. This
is exactly how layered systems work — and exactly how the book should read.

---

## Related

- [social-circles.md](social-circles.md) — The two worlds in full detail
- [inciting-incident-selection.md](inciting-incident-selection.md) — The first SIGSEGV
- [../../04-plot-and-structure/architecture/story-architecture.md](../../04-plot-and-structure/architecture/story-architecture.md) — The story structure this collision generates
- [../../02-characters/cast-overview.md](../../02-characters/cast-overview.md) — Cast derived from collision slots
