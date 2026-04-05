---
schema: schemas/01-concept/comparable-titles.schema.json
---

# Comparable Titles Analysis

**Phase:** 01 - Concept
**Purpose:** Identify and analyze comp titles to position the book and understand the market
**Status:** Filled

---

## Comp 1: *Crafting Interpreters* by Robert Nystrom (2021)

**Why it's similar:**
The gold standard for "build it from scratch" technical pedagogy. Progressive
construction of a real system (two complete interpreters). Code-heavy, every
line explained. Joy of building is the emotional core.

**Why ours is different:**
We build at a lower level — binary formats, memory mapping, calling conventions,
not language design. Our artifacts are real macOS ARM64 binaries, not toy
programs. The "from scratch" means from-the-first-byte, including undocumented
platform behaviors no reference covers. CI teaches you to build a language;
*Loading Darwin* teaches you to understand a platform.

**What we can learn from this comp:**
The pacing is masterful — every concept is motivated by what the reader needs
*right now*. Nystrom never explains something before the reader needs it. We
need this discipline. Also: the challenge exercises at the end of each chapter
that extend the core material.

**Reader reviews — common praise and criticism:**
- Praise: "Best technical book I've ever read." "I actually finished the whole thing."
- Criticism: "The two-language approach (Java + C) can be jarring." "Some
  chapters feel long." We avoid both — single language (Rust), chapters are
  crisis-resolution arcs with natural endpoints.

---

## Comp 2: *The Linux Programming Interface* by Michael Kerrisk (2010)

**Why it's similar:**
Definitive systems programming reference. Deeply authoritative. Covers the full
POSIX/Linux API surface with real-world examples. The reader trusts it completely.

**Why ours is different:**
TLPI is a reference organized by system call. We're a narrative organized by
what breaks. TLPI explains Linux; we explain macOS-on-Linux. TLPI is comprehensive;
we're focused on one project from start to finish. TLPI readers look things up;
our readers read cover-to-cover.

**What we can learn from this comp:**
The absolute precision of technical claims. Kerrisk never hedges when he knows
the answer, and every claim is backed by concrete code and output. Our prose
should aspire to this level of authority.

---

## Comp 3: *Mac OS X Internals: A Systems Approach* by Amit Singh (2006)

**Why it's similar:**
The closest existing work on macOS internals. Covers Mach-O format, dynamic
linking, Mach kernel, frameworks. Singh went deep.

**Why ours is different:**
Singh's book is a reference from the insider perspective — how macOS works on
macOS. Ours is from the outsider perspective — how macOS works when you have to
rebuild it. Singh covers the kernel; we stay in userspace. Singh uses C/Obj-C;
we use Rust. Singh's book is 20 years old and covers PowerPC/Intel; ours covers
modern ARM64 with PAC, chained fixups, and Apple Silicon.

---

## Comp 4: *Linkers and Loaders* by John R. Levine (1999)

**Why it's similar:**
The definitive academic treatment of dynamic linking. Explains symbol resolution,
relocation, and loading across formats (ELF, COFF, PE, a.out).

**Why ours is different:**
Levine is format-agnostic theory. We're format-specific practice: Mach-O with
modern chained fixups, auth pointers, and shared cache extraction — none of
which existed when Levine wrote. We don't just explain linking; we implement it
against real Apple binaries and test against 51+ real tools.

---

## Comp 5: *Programming Rust* by Jim Blandy, Jason Orendorff, and Leonora Tindall (2021)

**Why it's similar:**
The serious Rust systems book. Uses real-world examples, explains ownership and
FFI in depth. Our readers are likely Rust developers.

**Why ours is different:**
*Programming Rust* teaches the language. We use the language to teach systems
concepts. A reader finishes PR knowing Rust; a reader finishes *Loading Darwin*
knowing platforms, binaries, and runtimes — with Rust as the vehicle.

---

## Comp 6: *Computer Systems: A Programmer's Perspective* by Bryant and O'Hallaron (2015)

**Why it's similar:**
The canonical CS:APP textbook. Covers binary representation, assembly, linking,
virtual memory, processes. Used in hundreds of university courses.

**Why ours is different:**
CS:APP is a textbook with problem sets. We're a narrative with a single project.
CS:APP teaches x86; we teach ARM64. CS:APP covers Linux exclusively; we cover
the macOS stack. CS:APP is encyclopedic; we're focused.

---

## Comp Mashup / High-Concept Pitch

**Variations:**

1. *Crafting Interpreters* meets *Mac OS X Internals* — build a complete macOS
   compatibility layer from scratch, one crash at a time
2. *Crafting Interpreters* for binary loaders — progressive construction of a
   real system, but instead of an interpreter, it's an OS compatibility layer
3. *The Linux Programming Interface* but from the builder's side — instead of
   documenting what the API does, show what happens when the API isn't there

**Best version:** "*Crafting Interpreters* meets *Mac OS X Internals*—build a
complete macOS compatibility layer from scratch, one crash at a time."

---

## Testing the Comps

### The Reader Test

**What itch does CI scratch that our book also scratches?**
"I want to build something real and understand how it works — not just read about it."

**What additional itch does our book scratch?**
"I want to understand the hidden platform contracts that no API documentation
reveals — the undocumented behaviors, the binary-level assumptions, the runtime
secrets."

### The Agent Test (Future)

- [x] Recent and relevant (CI 2021, PR 2021, CS:APP 2015)
- [x] Successful (CI: universal praise; TLPI: definitive reference; PR: standard Rust text)
- [x] Clear positioning (readers immediately understand what kind of book this is)
- [x] Not mega-bestsellers (none of these are Harry Potter-scale outliers)

---

## Related

- [premise-and-logline.md](premise-and-logline.md) — Comps help sharpen the premise
- [genre-and-audience.md](genre-and-audience.md) — Comps define the audience
- [../00-start-here/book-brief.md](../00-start-here/book-brief.md) — Comps go in the book brief
