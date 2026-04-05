# Genre and Audience

**Phase:** 01 - Concept
**Purpose:** Identify genre, understand conventions, and define target audience
**Status:** Filled

---

## Identifying Your Genre

### Primary Genre

**What genre is your book?**
Technical nonfiction — systems programming education.

**Why this genre?**
The material is deeply technical (binary formats, memory management, CPU
architecture) and the purpose is educational. The reader learns by building.
This is not a reference manual, a memoir, or popular science.

### Subgenre or Blend

**Subgenre:** "Build it to understand it" project-based learning (same lineage
as *Crafting Interpreters*, *Build Your Own Redis*, *Writing an OS in Rust*)

**Blended genres:** Technical tutorial (60%) + engineering narrative (30%) +
reverse engineering guide (10%)

**Primary vs. secondary:** Tutorial dominates. The narrative thread (crash →
investigate → fix → validate) serves the education, not the other way around.

**Shelf placement:** Computer science / systems programming / OS internals

---

## Genre Conventions

### Core Conventions of Technical Nonfiction

**Structural conventions:**
- Progressive difficulty (parse before link, link before execute)
- Each chapter is self-contained enough to be a reference but builds on previous
- Code listings are real, complete, and compile
- Exercises or validation steps at section boundaries

**Tonal conventions:**
- Authoritative but approachable
- "Show, don't tell" applies to code: show the hex dump, don't describe it abstractly
- Honest about complexity ("this is hard" when it is)
- Generous with explanation, not with padding

**Character conventions:**
- The reader is the protagonist (second-person "you" or collective "we")
- The author is a guide, not a lecturer
- The system under study is revealed gradually (not front-loaded)

**Pacing conventions:**
- Each chapter: crash/problem → investigation → explanation → implementation → verification
- ~8,000–12,000 words per chapter
- Code listings broken into digestible chunks with interleaved explanation

**Setting conventions:**
- Real tools, real binaries, real environments (Docker container, ARM64)
- Reproducible: every example the reader can run

### Conventions We're Embracing

1. **Progressive build-up** — Each chapter adds one layer to the loader. The
   reader can see the project grow.
2. **Real artifacts** — Every hex dump, crash log, and code listing is from the
   actual project, not synthesized examples.
3. **Show the failure first** — Each concept is motivated by a crash or wrong
   output before the explanation arrives.
4. **Reproducible environment** — Docker image ships with all examples.
5. **Code-first pedagogy** — Concepts taught through implementation, not slides.

### Conventions We're Subverting or Bending

1. **No exercises at chapter end** — Instead, the entire chapter IS the exercise.
   The reader builds the loader alongside the author. Validation is built into
   the narrative (did the tool pass? yes/no).
2. **Narrative thread** — Unlike most technical books, this one has a story arc.
   The crash-to-success progression gives it dramatic structure that pure
   reference books lack.
3. **Undocumented discoveries** — Technical books cite documentation. This one
   documents behaviors never publicly documented, discovered through reverse
   engineering. The authority comes from building, not citing.

---

## Reader Expectations

**What emotional experience does the genre promise?**
Intellectual satisfaction: the "aha" of understanding something complex and
hidden. The specific pleasure of making something work that shouldn't be
possible. Pride in new capability.

**What will readers expect from:**

**Plot:** A clear progression from simple (parse a header) to complex (run a
full tool pipeline). Each chapter builds on the last.

**Characters:** An expert guide who respects the reader's intelligence and
doesn't waste their time. Real war stories, not contrived examples.

**Ending:** A working system. 84/84 tools passing. The Docker container works.
The journey was worth it.

---

## Audience Tiers

### Primary Audience
- **Systems programmers** who write Rust, C, or C++ and want to understand
  binary formats, dynamic linking, and runtime internals
- **Reverse engineers** who analyze binaries and want to see the builder's
  perspective
- **OS enthusiasts** who read about kernel internals and want to understand
  the userspace runtime stack

### Secondary Audience
- **Rust developers** looking for a substantial real-world project to learn from
- **macOS/iOS developers** who use Apple frameworks daily and want to understand
  what happens beneath the API surface
- **Computer science students** studying operating systems, compilers, or
  computer architecture

### Tertiary Audience
- **Security researchers** interested in binary exploitation, PAC, and
  compatibility layer attack surfaces
- **Emulator/VM developers** building cross-platform execution environments
- **Technical managers** who want to understand what "platform compatibility"
  actually means at the engineering level

### Reader Values and Preferences

- [x] Character-driven (the reader is the character, learning through building)
- [x] Fast-paced within chapters (crash → fix loop is tight)
- [x] Realism (real binaries, not toy examples)
- [x] Complex prose welcome (code listings, hex dumps, register traces)
- [x] High concept (run macOS on Linux!)

**The reader prioritizes:** Depth over breadth. They want to understand *how*
something works, not just *that* it works. They will read a 15-page chapter on
fixup chains if every paragraph earns its place.

### Audience Specificity

**Target reader:** Systems programmers in their 25s–45s who write Rust or C,
are comfortable with a terminal, have some OS concepts (processes, memory,
files), and are intrigued by the question "how does a binary actually run?"
They've read *Crafting Interpreters* or *Programming Rust* and want something
equally rigorous but focused on OS internals and binary formats. They'd
subscribe to Julia Evans' blog, follow the Rust community, and appreciate a
well-commented hex dump.

---

## Comparable Titles and Positioning

| Title | Overlap | Differentiation |
|-------|---------|----------------|
| *Crafting Interpreters* (Nystrom) | Build-from-scratch pedagogy | OS/runtime focus, not language design |
| *The Linux Programming Interface* (Kerrisk) | Systems depth | Narrative-driven, macOS focus |
| *Mac OS X Internals* (Singh) | macOS internals | Builder's perspective, not reference |
| *Programming Rust* (Blandy et al.) | Rust systems code | Binary/runtime focus, not language tutorial |
| *Linkers and Loaders* (Levine) | Dynamic linking theory | Practical implementation, modern ARM64 |
| *Computer Systems: A Programmer's Perspective* (Bryant/O'Hallaron) | Systems education | Single narrative project, not textbook |

**High-concept pitch:** "*Crafting Interpreters* meets *Mac OS X Internals*—
build a complete macOS compatibility layer from scratch, one crash at a time."

See [comparable-titles.md](comparable-titles.md) for detailed comp analysis.

---

## Genre and Audience Alignment Check

- [x] **Delivers the core promise of the genre?** Yes — the reader builds a real
  thing and learns deeply from the process.
- [x] **Appeals to readers who loved comp titles?** Yes — CI fans get the
  build-it pedagogy; TLPI fans get the systems depth.
- [x] **Matches the tone and pacing the audience prefers?** Yes — technical,
  respectful, code-heavy, no fluff.
- [x] **Fits a clear category for shelving?** Yes — systems programming /
  OS internals. Bookstores know where this goes.

---

## Genre vs. Mode (Frow)

**Genre:** Technical nonfiction (project-based systems education)

**Mode:**
- [x] **Investigative** — Each chapter investigates a crash, discovers a system
- [ ] Comic
- [ ] Tragic
- [ ] Elegiac
- [ ] Satiric
- [ ] Ironic

**Genre × Mode combination:** "Technical nonfiction in an investigative mode" —
the methodology is detective work. Each crash is a crime scene. The hex dump is
the evidence. The fix is the resolution.

**Mode-genre tension:** Investigative mode creates narrative momentum that pure
tutorial mode lacks. The risk is readers who want a reference find the narrative
distracting. Mitigation: clear section headings that let reference-users skip
the narrative scaffolding.

---

## Genre Hybridization Map (Fowler)

| Genre or Subgenre | % of DNA | Reader Expectations | Meeting / Subverting |
|------------------|----------|--------------------|--------------------|
| Technical tutorial | 60% | Progressive build-up, code listings, explanations | Embracing fully |
| Engineering narrative | 30% | War stories, debugging drama, real-world stakes | Embracing — this is the differentiator |
| Reverse engineering guide | 10% | Binary analysis, undocumented behaviors | Embracing — Appendix B delivers |

**Stage in genre evolution (Fowler's model):**
Build-from-scratch technical books are in a **Consolidated** stage — the convention
is well-established (*Crafting Interpreters*, *Build Your Own X*). This book
operates within the convention but modifies it with narrative structure and
real-world tooling (not toy projects).

---

## Genre Theory: Reading Contract

### Rosenblatt: Aesthetic vs. Efferent

**Reading contract:**
- [x] **Primarily efferent** — readers extract knowledge about binary formats,
  linking, runtime systems. They carry away skills.
- [ ] Mixed — some passages (the debugging war stories, the "aha" moments) invite
  aesthetic reading: attending to the experience of discovery itself.

**Where the work asks readers to slow down aesthetically:**
- The first hex dump of a Mach-O header (attend to the structure, not just the
  information)
- The moment a tool goes from SIGSEGV to clean exit (feel the satisfaction)
- The undocumented behaviors list (the strangeness of Apple's hidden contracts)

### Fish: Interpretive Community

**Target interpretive community:** Systems programmers who read the Rust blog,
follow OS development, appreciate well-documented open source. They naturalize
code listings as narrative content, hex dumps as evidence, and `cargo test`
output as resolution.

**What a reader from outside would miss:** Someone without systems programming
background would not know why a SIGSEGV is dramatic, why 84/84 is meaningful,
or why running `cut` on Linux is impressive. The book does not try to serve
this reader.

---

## Reader Prerequisites

**Required:**
- Comfortable reading Rust (or C) code
- Basic understanding of memory (stack, heap, pointers)
- Command-line fluency (terminal, shell, make/cargo)

**Helpful but not required:**
- Prior exposure to assembly language
- Experience with macOS development tools
- Familiarity with Docker

**Not required:**
- Objective-C experience (taught from scratch)
- ARM64 assembly knowledge (taught from scratch)
- Operating system development experience

---

## Related

- [premise-and-logline.md](premise-and-logline.md) — Genre shapes how you frame the premise
- [comparable-titles.md](comparable-titles.md) — Deep dive into comp analysis
- [../04-plot-and-structure/architecture/story-architecture.md](../04-plot-and-structure/architecture/story-architecture.md) — Genre influences structure
