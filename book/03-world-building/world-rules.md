# World Rules

**Phase:** 03 - World-Building
**Purpose:** Establish the laws, systems, and hard constraints that govern the story world
**Status:** Filled

---

## Rule System Type

**What kind of rules define your world?**
- [x] Real-world physics and natural laws (computer science fundamentals)
- [x] Advanced technology (platform ABIs, compiler conventions, CPU architecture)
- [x] Social/legal systems (Apple's NDAs, open-source licensing, platform TOS)

**Primary rule type:** Advanced technology — the platform's binary format, ABI, and runtime protocols are the inviolable laws of this world.

---

## Core Rules (The Inviolable Constraints)

### Core Rule #1: The Binary Is the Contract

**The Rule:** A Mach-O binary encodes every assumption the platform makes about the runtime environment. These assumptions are not optional — they must be honored exactly, or the binary crashes.

**Why This Rule Exists:** Apple designed the platform for a controlled environment where all components are built by the same compiler, targeting the same OS. The binary assumes this context is provided.

**What This Rule Prevents:** You cannot "partially" load a binary. You cannot skip chained fixups. You cannot ignore PAC signatures. You cannot register half the ObjC classes. Every shortcut produces a crash at an unpredictable point.

**Consequences of Attempting to Break It:** SIGSEGV (null dereference from unresolved symbol), SIGBUS (wrong memory permissions), PAC trap at `0x40000000000000` (unauthenticated pointer), or silent data corruption (wrong struct layout reads "work" but produce garbage values).

**How This Rule Creates Story Tension:** The reader cannot build incrementally unless they also handle every layer. Each chapter adds a new set of contracts that must be fully honored before any tool will work correctly.

---

### Core Rule #2: Crashes Are Deterministic Information

**The Rule:** Given the same binary, the same environment, and the same input, the platform always crashes at the same address for the same reason. Crashes are reproducible, diagnosable, and informative.

**Why This Rule Exists:** The platform is a deterministic state machine. Virtual memory, register state, and instruction sequencing are all deterministic (excluding ASLR, which shifts everything by a constant offset).

**What This Rule Prevents:** You cannot dismiss a crash as "random" or "intermittent." Every crash has a specific cause — an unresolved symbol, a wrong ABI convention, a missing class registration, a layout mismatch.

**Consequences of Attempting to Break It:** If you assume a crash is non-deterministic and add a retry, you'll get the same crash forever. The only path forward is understanding.

**How This Rule Creates Story Tension:** This rule is the engine of the book's discovery cycle. Each crash is a puzzle with a definite answer. The reader can always make progress, even when the crash initially looks unfixable.

---

### Core Rule #3: Layers Depend on Layers (Bottom-Up Only)

**The Rule:** The platform stack has a strict dependency order: binary format → memory mapping → dynamic linking → ObjC runtime → Foundation → application. Each layer requires all layers below it. You cannot skip a layer.

**Why This Rule Exists:** This is the real architecture of every operating system. High-level abstractions (like `[NSString stringWithFormat:]`) compile down to lower-level operations (selector lookup → `objc_msgSend` → method list walk → C function call → system call).

**What This Rule Prevents:** You cannot shim Foundation without first having an ObjC runtime. You cannot have class registration without first parsing the binary. The book's chapter order is not arbitrary — it mirrors the platform's dependency order.

**Consequences of Attempting to Break It:** Attempting to call a Foundation method without a registered class produces a crash in `objc_msgSend`. Attempting to dispatch without a parsed binary produces a null-pointer SIGSEGV. Each layer's crash tells you which prerequisite is missing.

**How This Rule Creates Story Tension:** The reader must be patient. Chapters 1–4 build infrastructure that doesn't produce visible results until Chapter 5 (first Foundation call). The delayed payoff creates narrative tension: "When will this actually DO something?"

---

### Core Rule #4: The Platform Cannot Be Negotiated With

**The Rule:** The platform does not "try its best." It either gets exactly what it expects, or it crashes. There is no partial success, no degraded mode, no fallback. Binary compatibility means binary: yes or no.

**Why This Rule Exists:** Platform integrity. If the platform tolerated incorrect ISA pointers or wrong PAC signatures, it would become vulnerable to exploitation. Strictness is security.

**What This Rule Prevents:** The reader cannot use approximations. "Close enough" struct layouts produce silent data corruption. "Most" symbols resolved still means one null function pointer, which means one crash.

**How This Rule Creates Story Tension:** Every victory is complete and every failure is total. There's no middle ground. This creates sharp dramatic beats: a tool either runs or it crashes.

---

### Core Rule #5: Documentation Lies (But the Binary Tells the Truth)

**The Rule:** Apple's header files, developer docs, and WWDC talks contain errors, omissions, and outdated information. The only reliable source of truth is the binary itself — what the compiler actually emits and what the runtime actually does.

**Why This Rule Exists:** Documentation is written by humans and maintained separately from code. Over 35 years, the implementation has evolved faster than the docs. Header comments from macOS 10.4 may describe behavior that changed in macOS 12.

**What This Rule Prevents:** You cannot treat Apple's documentation as ground truth. The chained fixup bit-field positions in the headers are wrong. The auth key positions in the comments are wrong.

**How This Rule Creates Story Tension:** The reader learns to distrust authority and verify empirically. This maps to the book's broader theme: understanding comes from direct investigation, not accepting received wisdom.

---

## Soft Rules (Common Patterns, But Breakable)

**Soft Rule #1: Most macOS system tools are FreeBSD derivatives.**
Almost all tools in `/usr/bin` (cut, sort, wc, sed, grep) descend from FreeBSD. They share calling patterns: `setlocale()` → `getopt()` → `err()`/`errx()`. Exceptions: Apple-original tools like `lipo`, `codesign`, `plutil`.

**Usual Effect:** A shim that handles the FreeBSD calling pattern covers ~30 tools at once.
**Exception Conditions:** Apple-original tools like `xcodebuild` have completely different initialization patterns. Swift-based tools add Swift runtime requirements.

**Soft Rule #2: Foundation methods behave like their documentation specifies.**
Apple's Foundation class reference is generally accurate for method behavior (not implementation details).

**Usual Effect:** Shimming `-[NSString UTF8String]` to return a UTF-8 C string suffices.
**Exception Conditions:** Tagged pointer strings store data inline in the pointer — no heap allocation, no standard layout. The documentation doesn't mention this optimization.

**Soft Rule #3: The ABI is AAPCS64 with Apple exceptions.**
ARM64 calling convention on both platforms follows AAPCS64 for most cases.

**Usual Effect:** Non-variadic functions with simple arguments can be called directly across the ABI boundary.
**Exception Conditions:** Variadic functions diverge completely (Apple: all on stack; Linux: first 8 in registers). Struct return conventions also differ for large structs.

---

## Costs & Limitations

**Energy/Fuel Requirements:** CPU time. A Docker ARM64 build on QEMU takes 10–30 minutes. Native ARM64 (M-series Mac): 2–5 minutes. This time cost is real and affects development velocity.

**Knowledge/Skill Requirements:** The reader needs: Rust, basic OS concepts (virtual memory, processes), willingness to stare at hex. ARM64 assembly knowledge is built during the book.

**Material Requirements:** A Mac (for building and testing on macOS host) + Docker (for Linux ARM64 testing). Or a Linux ARM64 machine with Rosetta/QEMU.

**Failure States:** When things go wrong, two paths: (a) clean crash with register dump → diagnosable, (b) silent data corruption (wrong struct layout, wrong ABI) → insidious, discovered only when output is wrong.

---

## Knowledge Distribution

**Common Knowledge:** Programmers know that macOS runs binaries, that Objective-C exists, that dynamic linking is a thing.

**Specialized Knowledge:** How Mach-O load commands work. How chained fixups encode relocation information. How `objc_msgSend` dispatches. This is the knowledge the book transmits.

**Secret/Forbidden Knowledge:** Apple's undocumented behaviors — the 17 secrets listed in Appendix B. These can only be learned by building a compatibility layer and hitting the crashes yourself.

**Misinformation:** Apple's header file comments about chained fixup bit positions. The widespread belief that ARM64 calling convention is the same on all ARM64 platforms.

---

## Edge Cases & Exploits

**Known Exploits:**
- Darwin variant suffixes (`$INODE64`, `$DARWIN_EXTSN`) can be stripped to resolve symbols that would otherwise fail
- Tagged pointers bypass normal allocation — you can create NSNumbers without touching the heap
- The shared cache extraction pipeline lets you inspect libraries that don't exist as files on disk

**Forbidden Experiments:**
- Running code-signed Apple frameworks outside their intended context (legal gray area under DMCA)
- Attempting to load the real libSystem.B.dylib on Linux (ABI mismatch → immediate crash or worse)

---

## Rules and Theme

**How do your world's rules reflect your story's themes?**
Rule #1 (binary is the contract) embodies "platforms are understandable." Rule #2 (crashes are information) embodies "understanding comes from failure." Rule #5 (documentation lies) embodies "authority must be verified empirically."

**What philosophical question do your rules explore?**
Can a system designed for opacity be made transparent through systematic reconstruction? The rules say yes — determinism means every behavior is discoverable — but the cost is enormous.

**What do the rules reveal about power in your world?**
Apple holds the power of opacity (undocumented behaviors, closed source, changing ABI without notice). The reader gains power through empiricism (every behavior is deterministic and reproducible). Knowledge equalizes the power asymmetry.

---

## Related

- [technical-landscape.md](technical-landscape.md) — The two worlds these rules govern
- [setting-profile.md](setting-profile.md) — Physical settings shaped by these rules
- [history-and-timeline.md](history-and-timeline.md) — How these rules evolved over 35 years
- [../02-characters/the-platform.md](../02-characters/the-platform.md) — The antagonist defined by these rules
- [../01-concept/theme-exploration.md](../01-concept/theme-exploration.md) — Rules embody theme
