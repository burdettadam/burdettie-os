# Beat Sheet

**Phase:** 04 - Plot and Structure / Architecture
**Purpose:** Map the emotional beats of the entire narrative
**Status:** Filled

---

## Story-Level Context

**Genre:** Technical nonfiction / Builder's narrative
**Structural Model:** Fichtean Curve + Hero's Journey Hybrid (see [story-structure.md](story-structure.md))
**Theme:** Understanding comes not from permission but from persistence.

---

## The Fifteen Beats

Adapted from Save the Cat for technical nonfiction. Beat timing adjusted for a 10-chapter / ~95,000-word book.

### Beat 1: Opening Image (Ch 1, Scene 1)

**What Happens:** The reader opens a macOS binary in a hex editor. Sees `CF FA ED FE` at offset 0. The file is opaque — a wall of hex digits.

**Emotional Tone:** Curiosity tinged with intimidation. "This is foreign."

**What It Establishes:** The "before" snapshot. The reader cannot yet read a binary. The platform is a black box.

**Thematic Statement (Implicit):** There is meaning in these bytes if you learn to see it.

---

### Beat 2: Theme Stated (Ch 1, Scene 2)

**What Happens:** First attempt to run the binary on Linux. Immediate `SIGSEGV`. The author states the premise: "The crash isn't a failure — it's the platform telling you what it expects. Every crash teaches you something specific."

**Emotional Tone:** Challenge issued.

**What It Establishes:** The book's philosophy. Crashes are information. The binary tells the truth.

---

### Beat 3: Set-Up (Ch 1, Scenes 3–6)

**What Happens:** Introduction to Mach-O format. Header, load commands, segments, sections, symbols. The macho-parse crate makes the binary readable for the first time.

**Emotional Tone:** Structured learning. Foundation-laying.

**What It Establishes:** The reader can now parse any Mach-O binary. They have their first tool.

---

### Beat 4: Catalyst (Ch 2, Scenes 1–3)

**What Happens:** "Parsing isn't enough. We need to load the binary into memory." Virtual memory, mmap, segment permissions. The binary moves from data-on-heap to live-in-memory.

**Emotional Tone:** Momentum building. "We're really doing this."

**What It Establishes:** The step from analysis to execution. A parsed binary is academic; a mapped binary is alive.

---

### Beat 5: Debate (Ch 2, Scenes 4–5 → Ch 3, Scene 1)

**What Happens:** The mapped binary is in memory but every function pointer is zero. External symbols aren't resolved. "Do we need to build an entire dynamic linker?" Yes, we do.

**Emotional Tone:** Hesitation. The scope of the project becomes clear.

**What It Establishes:** The reader commits to building a full runtime. There's no shortcut.

---

### Beat 6: Break into Two (Ch 3, Scenes 1–3)

**What Happens:** The reader enters the "special world" of dynamic linking. Bind opcodes, chained fixups, import/export tries. The linker code is the most complex data structure in the entire book.

**Emotional Tone:** Crossing the threshold. From reader to builder.

**What It Establishes:** The reader is now operating in territory Apple doesn't document. They're reading the binary's instructions, not the platform's documentation.

---

### Beat 7: B Story / Fun and Games (Ch 3, Scenes 4–7 → Ch 4)

**What Happens:** The "B story" IS the Objective-C runtime — an entirely different layer of complexity that's been hiding inside the binary the whole time. While the linker resolves C-level symbols (A story), the ObjC runtime reveals message dispatch, class hierarchies, tagged pointers (B story).

**Emotional Tone:** Discovery, intellectual delight. "ObjC message dispatch is ELEGANT."

**What It Establishes:** The binary contains not just code but an entire object system. The platform's complexity is also its beauty.

**Fun and Games moments:**
- Building objc_msgSend from scratch in ARM64 assembly
- Seeing ISA pointer chains for the first time
- Tagged pointers: "the number isn't a pointer, it IS the pointer"
- Dynamic class creation: KVO ISA swizzling "that's how it works?!"

---

### Beat 8: Midpoint (Ch 5, Scene 2–3)

**What Happens:** NSString works. The reader calls `[NSString stringWithFormat:@"Hello, %@", name]` and gets the right output from a synthetic Foundation built in Rust. A/B stories merge: the linker (A) and the runtime (B) combine to make a real framework function.

**Emotional Tone:** False victory (raised stakes version). "It works!" But only for simple cases. The full Foundation has 93+ classes that each need shimming.

**What It Establishes:** Proof of concept. The approach works. But the scale of the remaining work becomes visible.

**Midpoint Shift:** From "can we make this work?" to "how do we make 2,682 tests pass?"

---

### Beat 9: Bad Guys Close In (Ch 5, Scenes 4–9 → Ch 6, Scenes 1–2)

**What Happens:** Foundation shimming reveals increasingly difficult edge cases: NSFileManager path translation, NSURLSession real HTTP, NSTask subprocess spawning. Then the ABI bridge — everything that works on macOS crashes in Docker because Apple's ARM64 ABI differs from standard AAPCS64.

**Emotional Tone:** Escalating difficulty. Each fix reveals two new problems.

**What Gets Worse:**
- Foundation classes have hidden internal state expectations
- macOS puts ALL variadic args on stack; Linux doesn't
- struct stat is a different size on macOS vs. Linux
- locale category numbers are different

---

### Beat 10: All Is Lost (Ch 6, Scenes 3–5)

**What Happens:** The variadic argument crisis. printf, NSLog, stringWithFormat — they all crash on Linux because the calling convention is different and there's no ABI documentation for arm64-apple-macosx. The reader must write naked assembly shims to translate between ABIs.

**Emotional Tone:** Despair → grim determination. "This might actually be impossible."

**The "Whiff of Death":** The realization that Apple's ABI isn't just undocumented — it's DIFFERENT from what the CPU vendor specifies. The platform's opacity isn't incidental; it's structural.

---

### Beat 11: Dark Night of the Soul (Ch 6, Scenes 5–7)

**What Happens:** The Docker container build. Putting everything together for Linux: library preloading, RTLD_GLOBAL chains, environment bridging. The full weight of the cross-platform problem.

**Emotional Tone:** Lowest point. "Why is this so hard?"

**What It Teaches:** The ABI divergences aren't bugs — they're the deepest layer of platform identity. Understanding them is understanding the platform at its most fundamental.

---

### Beat 12: Break into Three (Ch 7, Scene 1)

**What Happens:** "What if we can't run the code natively?" The ARM64 interpreter idea. Instead of fighting the ABI, interpret the instructions. A completely different approach to the central problem.

**Emotional Tone:** Renewed energy. "There's another way."

**What Enables It:** All the understanding built in Acts I and II makes the interpreter possible. The reader knows the binary format (Act I) and the runtime semantics (Act II) well enough to interpret rather than execute.

---

### Beat 13: Finale (Ch 7–9)

**What Happens:**
1. **Gather the team:** The ARM64 decode/execute engine. All instruction classes. (~Ch 7)
2. **Execute the plan:** The integrated runtime — typestate loader, performance optimization, signal handling. (~Ch 8)
3. **High tower surprise:** Real Xcode tools work. Not just test fixtures — production Apple software running on Linux. (~Ch 9)
4. **Dig deep down:** 84/84 tools passing. The compat gate achieved.
5. **The new world:** GUI applications — NSWindow opens via winit backend. (~Ch 10)

**Emotional Tone:** Crescendo of capability. Each scene adds power.

---

### Beat 14: Final Image (Ch 10, Scene 5)

**What Happens:** The reader has built a complete macOS compatibility runtime. They can read binaries, resolve symbols, dispatch messages, shim frameworks, bridge ABIs, interpret instructions, and open windows. The platform that was a black box in Beat 1 is now transparent.

**Emotional Tone:** Mastery + humility. "I understand this now. And I know what I still don't understand."

**Mirror of Opening Image:**
| Opening Image | Final Image |
|---|---|
| Hex dump: `CF FA ED FE` — opaque | Same hex dump — now the reader sees structure, intent, history |
| First crash: SIGSEGV — failure | SIGSEGV handled: crash decoded, diagnosed, fixed |
| Platform: black box | Platform: understood, reproduced, respected |
| Reader: outsider | Reader: builder |

---

### Beat 15: Stinger / What's Next (Ch 10, Scene 5 continued)

**What Happens:** Brief glimpse of remaining frontiers: Swift metadata parsing, private framework shimming, Metal GPU shims, full Rosetta-level translation. The world is larger than one book.

**Emotional Tone:** Possibility. The door is open.

---

## Beat Timing

| Beat | Chapter | Word Count (approx) | % of Book |
|---|---|---|---|
| 1–3: Opening → Set-Up | Ch 1 | ~12,000 | 12% |
| 4–5: Catalyst → Debate | Ch 2 | ~10,000 | 10% |
| 6: Break into Two | Ch 3 | ~14,000 | 15% |
| 7: B Story / Fun and Games | Ch 4 | ~12,000 | 13% |
| 8: Midpoint | Ch 5 | ~12,000 | 13% |
| 9–11: Bad Guys → Dark Night | Ch 6 | ~10,000 | 10% |
| 12: Break into Three | Ch 7 (start) | ~5,000 | 5% |
| 13: Finale | Ch 7–9 | ~15,000 | 16% |
| 14–15: Final Image | Ch 10 | ~5,000 | 5% |
| **Total** | | **~95,000** | **100%** |

---

## Causality Chain

Each beat causes the next. No beat can be removed without breaking the chain:

```
Magic number → Parse → Load → Crash (no symbols) →
Link → Crash (no runtime) → ObjC dispatch → Foundation →
Crash (ABI) → Bridge → Performance → Real tools →
GUI → Complete
```

**Reversibility test:** Can any chapter be skipped?
- Ch 1: No — you can't parse what you can't read
- Ch 2: No — you can't link what you can't map
- Ch 3: No — you can't dispatch what you can't resolve
- Ch 4: No — you can't shim what you can't dispatch to
- Ch 5: No — you can't bridge what you haven't shimmed
- Ch 6: No — you can't interpret what you don't understand
- Ch 7: No — you can't optimize what isn't complete
- Ch 8: No — you can't test what isn't integrated
- Ch 9: No — validated tools prove the system works
- Ch 10: Optional — but emotionally necessary for the "return with elixir"

---

## Related

- [story-structure.md](story-structure.md) — Framework selection rationale
- [story-architecture.md](story-architecture.md) — Scene-level breakdown
- [../blueprint/chapter-blueprint.md](../blueprint/chapter-blueprint.md) — Crate and file mapping per chapter
- [../../03-world-building/motif-and-symbol-tracker.md](../../03-world-building/motif-and-symbol-tracker.md) — Symbols that appear at each beat
