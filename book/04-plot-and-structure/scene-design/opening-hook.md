# Opening Hook

**Phase:** 04 - Plot and Structure / Scene Design
**Purpose:** Design the book's first scene to capture and hold the reader
**Status:** Filled

---

## The Hook Strategy

**Target Reader:** A systems programmer who has written application-level code but never looked inside a binary. They know Rust or C. They're curious about how things work "below the API."

**Promise of the Hook:** "I'm going to show you something you've never seen inside the files you use every day."

**Hook Type:** The Artifact Reveal — open a familiar object (a macOS binary) and show what's actually inside it.

---

## Scene 1.1: The Black Box

### Opening Lines (Draft)

> Every macOS binary begins with a greeting.
>
> Open `/usr/bin/true` in a hex editor. At offset zero, four bytes:
>
> ```
> CF FA ED FE
> ```
>
> Little-endian for `0xFEEDFACF`. The Mach-O 64-bit magic number.
>
> That's the file introducing itself to the operating system's loader. It has been doing this since 1997, when NeXT's technology became Apple's foundation. Every binary you've ever run on a Mac starts with this handshake.
>
> Most developers never see it. They write Swift or Objective-C, press ⌘R, and the binary appears in `/Build/Products/Debug/`. A black box that enters the system and does its work unseen.
>
> We're going to open the black box.

### Why This Works

| Element | Technique | Purpose |
|---|---|---|
| `CF FA ED FE` | Concrete sensory detail | Immediately grounds the reader in something REAL |
| "Every macOS binary begins with a greeting" | Anthropomorphization | Makes the binary relatable — it's speaking to someone |
| "Since 1997" | Historical depth | Signals this isn't surface-level tutorial content |
| "Most developers never see it" | Exclusivity | "I'm about to show you something most people miss" |
| "We're going to open the black box" | Direct promise | States the chapter's (and book's) mission in one sentence |

---

## The Opening Crash (Scene 1.2 preview)

The hook has two phases:

**Phase 1 (Scene 1.1):** Wonder — "look at this beautiful structure inside the binary."

**Phase 2 (Scene 1.2):** Challenge — "now try to run it on Linux."

> Let's try something naive. Copy `/usr/bin/true` into a Docker container running Ubuntu on ARM64. Run it.
>
> ```
> $ ./true
> Segmentation fault (core dumped)
> ```
>
> Signal 11. The binary loaded, started to execute, and immediately touched memory it shouldn't have. It expected macOS. It got Linux. The ground it stood on was wrong.
>
> That crash is the starting point of this book. Not the problem — the information. Every detail the crash reveals is a lesson about what the binary expects from its operating system.
>
> By the time you finish this book, that same binary will run on Linux. And you'll understand every byte that makes it possible.

### Why This Works

| Element | Technique | Purpose |
|---|---|---|
| "Copy... Docker... Run it" | Reproducible action | Reader could do this right now on their machine |
| `Segmentation fault` | Concrete failure | The crash is visceral, immediate, undeniable |
| "It expected macOS. It got Linux." | Simple framing | The entire project's premise in one sentence |
| "Not the problem — the information" | Reframe | Establishes the book's relationship to failure |
| "You'll understand every byte" | Scope promise | Reader knows what "done" looks like |

---

## Hook Checklist

- [x] **Sensory entry point** — hex bytes on screen, specific offset, specific values
- [x] **Emotional hook** — curiosity (wonder at the hidden structure) + challenge (the crash)
- [x] **Scope signal** — reader knows this is thorough (binary format → running on Linux)
- [x] **Voice established** — direct, concrete, no hedging, the author has done this
- [x] **Implicit promise** — "I will teach you to see what most people can't"
- [x] **First motif introduced** — `0xFEEDFACF` (appears in every chapter)
- [x] **Second motif introduced** — `SIGSEGV` (opens every chapter)
- [x] **Pacing** — Fast. Hook → wonder → crash in under 500 words
- [x] **No prerequisites** — Reader needs zero prior binary knowledge
- [x] **Reproducible** — Reader with Docker + macOS can reproduce both scenes

---

## What the Opening Does NOT Do

- Does NOT explain Mach-O format (that's Scenes 1.3–1.6)
- Does NOT explain why the crash happens (that's the chapter's journey)
- Does NOT introduce Rust or the codebase (that's Scene 1.6)
- Does NOT use jargon without immediate context
- Does NOT apologize for difficulty or promise it will be "easy"

The opening trusts the reader. It shows them something real and says "let's figure this out."

---

## First-Page Architecture

```
Line 1   → Concrete claim ("Every macOS binary begins with a greeting")
Lines 2-4 → Evidence (hex dump, magic number, the bytes)
Lines 5-6 → Context (history, what this means)
Lines 7-8 → Contrast (most people never see this)
Line 9    → Promise ("We're going to open the black box")
---
Lines 10-12 → Action (copy binary, run it, crash)
Lines 13-14 → Reframe (crash = information)
Lines 15-16 → Full promise (by the end, it'll run + you'll understand)
```

**Total opening:** ~350 words to establish premise, demonstrate with real artifact, create the central question, and promise the answer.

---

## Alternative Hooks Considered and Rejected

### Option B: Start with the crash
> "Segmentation fault (core dumped). That's what macOS says when you try to run its software somewhere else."

**Rejected:** Too aggressive. Reader doesn't understand what failed or why they should care yet. The hex dump provides grounding first.

### Option C: Start with the project scope
> "This book builds a ~198,000-line compatibility layer that runs Apple's macOS binaries on Linux."

**Rejected:** Impressive but abstract. The reader hasn't seen anything concrete. Numbers alone don't create emotional investment.

### Option D: Start with a question
> "What does your operating system actually do when you double-click an application?"

**Rejected:** Too generic. Could be a blog post. Doesn't signal the depth or specificity of this project.

### Option E: Start with history
> "In 1988, Steve Jobs's company NeXT designed a binary format..."

**Rejected:** History is important but doesn't create urgency. The binary in front of you right now is more compelling than the binary from 1988.

**The chosen hook (Option A) wins because it combines concrete artifact (hex bytes) + wonder (the greeting) + challenge (the crash) in the shortest space.**

---

## Related

- [scene-card-index.md](scene-card-index.md) — Scene 1.1 and 1.2 slot into here
- [../architecture/beat-sheet.md](../architecture/beat-sheet.md) — Beat 1 (Opening Image) and Beat 2 (Theme Stated)
- [../../03-world-building/sensory-palette.md](../../03-world-building/sensory-palette.md) — Hex Editor sensory palette for this scene
- [../../03-world-building/motif-and-symbol-tracker.md](../../03-world-building/motif-and-symbol-tracker.md) — `0xFEEDFACF` and `SIGSEGV` first appearances
- [../../02-characters/character-voice.md](../../02-characters/character-voice.md) — Author's voice established here
