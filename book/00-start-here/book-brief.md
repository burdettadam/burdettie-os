---
schema: schemas/00-start-here/book-brief.schema.json
---

# Book Brief

**Phase:** 00 - Start Here
**Purpose:** Master context document for all AI sessions
**Status:** Filled

---

## Basic Information

**Working Title:** Loading Darwin: Building a macOS Binary Loader from Scratch

**Subtitle (working):** A Hands-On Guide to Mach-O Parsing, Dynamic Linking,
and Objective-C Runtime Emulation in Rust

**Genre:** Technical nonfiction / Systems programming education

**Target Word Count:** 80,000–100,000 words (~95K estimated across 10 chapters +
2 appendices)

**Target Audience:** Systems programmers, reverse engineers, OS internals enthusiasts,
Rust developers interested in low-level work. Readers who enjoyed
*Crafting Interpreters*, *Programming Rust*, or *The Linux Programming Interface*.

**Started:** 2024

**Project:** darwin-loader-lab (~198K LOC, 41 crates + 5 apps, 84/84 compat gate)

---

## The Story

**Logline (One Sentence):**
A systems programmer sets out to run macOS Xcode command-line tools on Linux
by building, from scratch, a Mach-O binary loader, Objective-C runtime, and
Foundation framework shim layer—and discovers that the real challenge isn't
parsing binaries, it's faithfully reproducing an entire platform's assumptions.

**Premise (2–3 Sentences):**
What happens when you try to take a macOS binary—compiled for Apple silicon,
linked against Apple's private frameworks, relying on Objective-C message dispatch
and pointer authentication—and run it on a bare Linux ARM64 system? This book
follows the journey of building darwin-loader-lab: from parsing the first Mach-O
header to passing 51+ Xcode tools in a Docker container. Each chapter peels back
a layer of the Apple platform stack, teaching binary formats, dynamic linking,
runtime systems, and CPU architecture through the concrete act of making
real software work where it was never meant to.

**Egri's Dramatic Premise:**
*Relentless curiosity about a closed system leads to mastery through systematic
deconstruction and faithful reconstruction.*

The book proves this proposition: every chapter is a cycle of crash → investigate
→ understand → implement → verify. The protagonist (reader) starts with
curiosity, proceeds through deconstruction of Apple's platform, and arrives at
genuine understanding by rebuilding each layer.

**Setting:**
The intersection of macOS and Linux on ARM64 hardware. Apple Silicon Macs
running Docker containers. The internal architecture of Xcode, Foundation,
and the Objective-C runtime. The world of Mach-O binaries, dyld shared caches,
and PAC-signed pointers.

---

## The Protagonist

**Name:** The Builder (the reader, guided by the author's journey)

**Core Desire:** To understand how platforms actually work—not at the API level,
but at the binary, runtime, and memory level.

**Greatest Fear:** That the stack is too deep, the undocumented behaviors too
numerous, and the platform too proprietary to ever truly understand.

**Key Trait:** Relentless curiosity combined with engineering pragmatism.

**One-Sentence Arc:** Moves from seeing a binary as an opaque black box to
understanding every byte of its structure, every pointer it dereferences, and
every message it sends.

**The Lie:** "Platforms are impenetrable black boxes designed to lock you
in—you can only use them, never understand them."

**The Truth:** "Every platform is a collection of discoverable contracts, and
by rebuilding, you can understand anything."

---

## The Core Conflict

**External Conflict:** Each new tool that crashes reveals another undocumented
assumption baked into Apple's platform—variadic calling conventions, PAC signing,
tagged pointers, toll-free bridging, locale category numbering. The loader must
handle all of them.

**Internal Conflict:** The tension between "good enough" compatibility (stub it
and move on) and "truly correct" emulation (understand and reproduce the real
behavior). When do you fake it? When must you get it right?

**Central Question:** How much of an operating system is actually defined by its
binaries, and how much lives in the invisible contracts between them?

**Stakes (What Happens if We Fail):** The binary crashes. The tool produces
wrong output. The Docker container fails its compatibility gate. But more
importantly, we remain on the surface—users of a system instead of
comprehenders of one.

**Stakes (What We Gain if We Succeed):** A working loader that runs 51+ real
Apple tools on Linux, validated by an 84/84 compat fixture gate. And the
transferable skill to analyze, deconstruct, and rebuild any binary format,
runtime, or platform on any architecture.

---

## Tone and Style

**Tone:** Technical but conversational. Earnest discovery mixed with hard-won
debugging war stories. The satisfaction of watching a tool go from SIGSEGV to
clean exit.

**Narrative Style:** Second-person instructional ("you'll see that...") mixed with
first-person retrospective ("we hit a wall when..."). Code-heavy, with annotated
Rust listings and hex dumps.

**Prose Style:** Direct and concrete. Every concept introduced through a real crash,
a real binary, or a real hex dump. Theory follows practice.

**What the Prose Should Feel Like:**
- The clarity of *Crafting Interpreters* (every line of code explained)
- The depth of *The Linux Programming Interface* (authoritative, thorough)
- The joy of *The Soul of a New Machine* (real people, real engineering, real stakes)
- The concreteness of *Computer Systems: A Programmer's Perspective* (hex dumps, not hand-waving)

---

## Themes

| Theme | Expression |
|-------|-----------|
| Platforms Are Contracts | Every crash is a violated assumption |
| Archaeology of Binaries | Reading structure reveals intent |
| Compatibility as Empathy | Adopt another platform's worldview |
| Building to Understand | The crashes are the curriculum |
| Open vs. Closed | Making the implicit explicit |
| Rust as Systems Language | Safety and FFI for the brave |

**Primary Theme:** Platforms Are Contracts
**Controlling Idea:** "Understanding comes from rebuilding: you cannot claim to
know a system until you can reproduce its behavior."

---

## Comparable Titles

| Title | What We Share | What We Do Differently |
|-------|-------------|----------------------|
| *Crafting Interpreters* (Nystrom 2021) | Build-from-scratch pedagogy, code-heavy chapters | OS/runtime focus, not language design; real binaries, not toy programs |
| *The Linux Programming Interface* (Kerrisk 2010) | Systems depth, authoritative reference | Narrative-driven journey; macOS focus; cross-platform angle |
| *Mac OS X Internals* (Singh 2006) | macOS internals coverage | Builder's perspective, modern ARM64, Rust not C |
| *Programming Rust* (Blandy et al. 2021) | Rust systems code, serious projects | Binary/runtime focus, not language tutorial |
| *Linkers and Loaders* (Levine 1999) | Dynamic linking theory | Practical implementation, modern chained fixups, ARM64 PAC |
| *Computer Systems: A Programmer's Perspective* (Bryant/O'Hallaron 2015) | Systems education, hex/binary literacy | Single narrative project, not textbook; macOS stack |

**High-concept pitch:** "*Crafting Interpreters* meets *Mac OS X Internals*—
build a complete macOS compatibility layer from scratch, one crash at a time."

---

## Unique Elements

This book offers things no existing title covers:
- Running real Apple ARM64 binaries on Linux (not a hypothetical—51+ Xcode tools passing, 84/84 compat fixtures)
- Mach-O format with modern chained fixups and auth pointers (post-2020 binaries)
- Objective-C runtime reconstruction from the binary level up
- ARM64 PAC (Pointer Authentication Codes) demystified through implementation
- Apple's undocumented behaviors documented for the first time (17+ discoveries)
- Foundation/CoreFoundation shim architecture as a teaching vehicle
- Docker-based reproducible environment (every example runs in `burdettie-os`)

---

## Structure Overview

**4 Parts + 2 Appendices, 10 Chapters, 75 Scenes**

| Part | Chapters | Focus |
|------|----------|-------|
| I: The Artifact | Ch 1–2 | Parse the binary, map it to memory |
| II: The Linker | Ch 3–4 | Resolve symbols, build the runtime |
| III: The Platform | Ch 5–7 | Foundation, CoreFoundation, ARM64 |
| IV: The System | Ch 8–10 | OS shims, tool validation, future |
| Appendix A | — | macOS ↔ burdettieOS Stack Map |
| Appendix B | — | Apple's Undocumented Behaviors |

---

## For AI Sessions

Always paste this book brief as context. Supplement with:
- Story architecture (04-plot-and-structure/architecture/story-architecture.md)
- Relevant scene cards (04-plot-and-structure/scene-design/)
- Entity registry excerpt for the chapter's crates/concepts
- The chapter blueprint entry for source file references

---

## Themes

**Primary Theme:** Platforms are contracts—understanding software means understanding
the promises made between layers of abstraction.

**Secondary Themes:**
- The archaeology of binaries (reading intent from compiled artifacts)
- Compatibility as empathy (reproducing another system's worldview)
- The power of building from scratch as a learning method
- Open systems vs. closed ecosystems
- Rust as a systems language for new kinds of systems work

**Thematic Statement:**
The best way to understand a platform is to rebuild it—not to replace it,
but to see it clearly.

---

## Comparable Titles

**Comp 1:** *Crafting Interpreters* by Robert Nystrom — Shares the "build it from
scratch to understand it" pedagogy and the satisfying progression from simple
to complex. Differs by covering OS/runtime internals rather than language design.

**Comp 2:** *The Linux Programming Interface* by Michael Kerrisk — Shares the
exhaustive systems-level depth. Differs by being narrative-driven rather than
reference-oriented.

**High-Concept Comp:** "*Crafting Interpreters* meets *The Art of Mac OS X* by way
of a cross-platform heist movie."

---

## Unique Elements

**What Makes This Book Special:**
No other book teaches macOS internals, ARM64 architecture, Objective-C runtime
mechanics, and dynamic linking through the lens of building a working compatibility
layer. Readers don't just learn what these systems do—they learn by watching
(and building) the code that reproduces them. The project is open source and
the reader can run every example.

**Personal Connection:**
The author built darwin-loader-lab as a real project, encountering every crash
and debugging every SIGSEGV documented in these pages. The discoveries are
genuine, the mistakes are real, and the solutions emerged from actual investigation.
