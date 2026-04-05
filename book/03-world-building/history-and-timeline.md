# History and Timeline

**Phase:** 03 - World-Building
**Purpose:** Map the historical events and backstory that shaped the story's present moment
**Status:** Filled

---

## Present Moment (Story's "Now")

**Current Date:** 2024–2025. macOS 14 Sonoma / macOS 15 Sequoia. Apple Silicon M1–M4.

**Current Technical Situation:** Apple's platform is at peak vertical integration: custom CPU + custom OS + custom runtime + custom frameworks + custom IDE + custom App Store. No other tech company controls this much of the stack.

**Current Social Climate:** Growing tension between Apple's walled-garden approach and developer desire for transparency. Open-source alternatives (Linux, Rust ecosystem) offer an escape route but lack platform-specific capabilities.

**Defining Characteristics of This Era:** "The Integration Era" — Apple Silicon made hardware-software integration tighter than ever. PAC, AME, shared cache optimizations, and custom GPU instructions are all evidence.

**What's About to Change:** This book is the disruption: someone builds a compatibility layer that makes Apple's binaries run outside Apple's walled garden for the first time.

---

## Historical Eras

### Era #1: The NeXT Era (1985–1997)

**Time Period:** 12 years. Steve Jobs left Apple and founded NeXT.

**Defining Event(s):** Creation of NeXTSTEP — the Mach kernel + Objective-C runtime + Foundation + Interface Builder. The most advanced development platform of its era.

**Key Figures:** Steve Jobs (vision), Avie Tevanian (Mach kernel from CMU), Bertrand Serlet (frameworks).

**How It Changed the World:** Created every technology that defines modern macOS: Objective-C message dispatch, Foundation classes, NIB files, dynamic linking via dyld. The Web was invented on a NeXT machine.

**Long-Term Consequences:** Every binary this book loads still contains NeXT-era conventions — selector strings, class_ro_t structures, Foundation class hierarchies. The past is literally encoded in the binaries.

**Cultural Memory:** Developers who remember NeXTSTEP recall it as a golden age of platform transparency. Full documentation, complete headers, a philosophy that developers should understand the system. Apple killed this openness.

---

### Era #2: The Mac OS X Transition (1997–2007)

**Time Period:** 10 years. Apple acquires NeXT. Mac OS X ships. Intel transition.

**Defining Event(s):**
- 1997: Apple acquires NeXT for $427M (really: NeXT acquires Apple)
- 2001: Mac OS X 10.0 ships — NeXTSTEP rebuilt as a consumer OS
- 2006: Mac transitions from PowerPC to Intel (first architecture migration)

**Key Figures:** Steve Jobs (CEO), Bertrand Serlet (VP Software Eng), Craig Federighi (successor).

**How It Changed the World:** Merged NeXT's elegance with Apple's consumer market. Made Objective-C mainstream. Introduced Cocoa frameworks. The fat binary format was invented for PPC→Intel transition.

**Long-Term Consequences:** The Mach-O format gained fat binary support (multiple architectures in one file). dyld became the standard dynamic linker. The concept of "universal binaries" was born — and used again for ARM64 transition.

**Cultural Memory:** Remembered as the "rebirth" of Apple. Long-time Mac users mourned Classic mode. Developers learned Objective-C or left the ecosystem.

---

### Era #3: The Mobile Revolution (2007–2020)

**Time Period:** 13 years. iPhone through the last Intel Mac.

**Defining Event(s):**
- 2007: iPhone ships — Objective-C + Foundation + UIKit on ARM
- 2014: Swift introduced (new language, but ObjC interop required)
- 2017: dyld 3 and chained fixups — performance optimizations for mobile
- 2019: dyld shared cache becomes mandatory on macOS

**Key Figures:** Craig Federighi (Software), Johny Srouji (Hardware Silicon), Chris Lattner (Swift/LLVM).

**How It Changed the World:** ARM became Apple's primary architecture. The shared cache (prelinked library image) was created to optimize iOS launch times. Chained fixups replaced classic relocation. The platform optimized for scale (billions of devices) at the cost of developer transparency.

**Long-Term Consequences:**
- Shared cache means most dylibs don't exist as individual files (our Rule #17)
- Chained fixups have undocumented bit-field positions (our Secrets #4–6)
- ARM64 ABI has Apple-specific extensions (variadic convention, PAC)
- Swift metadata adds a new layer of binary complexity

**Cultural Memory:** Developers remember the "golden age of iOS development" — Objective-C, Interface Builder, small apps, simple deployment. By 2019, the complexity had grown enormously.

---

### Era #4: The Silicon Era (2020–Present)

**Time Period:** 4 years and counting. Apple Silicon.

**Defining Event(s):**
- 2020: M1 chip announced — ARM64 on desktop/laptop
- 2020: Rosetta 2 ships — x86 → ARM64 binary translation
- 2022: arm64e (pointer authentication) becomes default for system binaries
- 2024: macOS 15 — further shared cache optimization, library validation tightening

**Key Figures:** Johny Srouji (silicon), Craig Federighi (software integration).

**How It Changed the World:** Total vertical integration. Apple now controls: CPU design, instruction set extensions (PAC, AMX), kernel, linker, runtime, frameworks, compiler, IDE. No other company has this depth.

**Long-Term Consequences:** PAC makes even function pointers into authenticated secrets. The platform is simultaneously faster, more secure, and more opaque than ever. This is the world our project navigates.

**Cultural Memory:** Developers are impressed by performance but increasingly aware of lock-in. The "it just works" promise requires accepting opacity as a feature.

---

## Key Historical Events

### Event #1: The Invention of Objective-C Message Dispatch (1983)

**Date:** 1983, Stepstone (Brad Cox and Tom Love)
**What Happened:** Created Objective-C by adding Smalltalk-style message passing to C. Every method call becomes `objc_msgSend(receiver, selector, args...)` — a runtime dispatch, not a compile-time call.
**Connection to Story:** Chapter 4's central concept. Every ObjC binary in the world still uses this dispatch model. Our trampoline in `trampoline.s` reproduces this 40-year-old design.

### Event #2: The Creation of dyld (1997)

**Date:** 1997, NeXT/Apple merger
**What Happened:** dyld (dynamic linker/loader) replaced the classic Mac OS code fragment manager. Introduced Mach-O format as the standard binary format for the platform.
**Connection to Story:** Chapters 1–3. Our entire loader is a reimplementation of dyld's functionality.

### Event #3: The ARM64 Transition (2013–2020)

**Date:** 2013 (A7 chip, first 64-bit ARM in a phone), culminating in M1 (2020)
**What Happened:** Apple moved from x86 to ARM64, first on mobile, then desktop. Introduced arm64e (pointer authentication) for system binaries.
**Connection to Story:** Chapter 6 (ABI mismatches), Chapter 7 (ARM64 interpreter). The ARM64 transition created the specific ABI divergences this project must handle.

### Event #4: The Shared Cache Mandate (2019)

**Date:** macOS 10.15 Catalina
**What Happened:** Apple moved nearly all system libraries into the shared cache. Individual `.dylib` files ceased to exist on disk. dyld_shared_cache_arm64e (~2GB) became the only source for system library code.
**Connection to Story:** Chapter 3 (shared cache parsing). Without understanding the cache, you can't examine or load any system framework.

### Event #5: Pointer Authentication Becomes Default (2022)

**Date:** macOS 13 Ventura
**What Happened:** System binaries began using arm64e with pointer authentication by default. Function pointers, return addresses, and vtable entries are now PAC-signed.
**Connection to Story:** Chapter 3 (auth fixups). PAC signatures mean our loader must either sign pointers correctly or strip PAC bits — getting it wrong triggers traps at `0x40000000000000`.

---

## Backstory That Intrudes

**Old Secrets:**
The chained fixup bit-field positions don't match Apple's header comments. This was discovered when our loader produced garbage fixup chains using the documented positions. The truth had to be reverse-engineered from working binaries.

**Unresolved Conflicts:**
Apple's documentation philosophy — "we document the API, not the implementation" — directly conflicts with the needs of compatibility layer builders. This tension drives the entire book.

**Lost Knowledge:**
NeXTSTEP era documentation was comprehensive and detailed. Much of that knowledge was deprecated, deleted, or superseded without replacement. The binary format has evolved but the documentation hasn't kept pace.

---

## History and Theme

**What patterns repeat in your world's history?**
Every platform transition (PPC→Intel→ARM64) follows the same pattern: new binary format features, new linking strategy, backward compatibility layer (Rosetta, fat binaries), gradual deprecation of the old.

**What does history reveal about your theme?**
The platform's history shows that opacity is a *choice*, not a necessity. NeXTSTEP was transparent by design. Apple chose opacity for commercial reasons. Understanding comes from recognizing that the opacity is artificial — the underlying engineering is rational and discoverable.

**How will your story add to this world's history?**
This is the first time someone has rebuilt the macOS runtime from scratch on Linux. If historians of computing look back, this project represents the moment the walled garden became permeable — not through breaking down the wall, but through understanding what the wall is made of.

---

## Timeline Visualization

**1983:** Objective-C invented (Stepstone)
**1985:** NeXT founded / Mach kernel at CMU
**1988:** NeXTSTEP 0.8 — Mach + ObjC + Foundation + IB
**1996:** Apple announces NeXT acquisition
**2001:** Mac OS X 10.0 (Cheetah) — NeXTSTEP goes mainstream
**2006:** Intel transition, fat binaries, Rosetta 1
**2007:** iPhone — ARM + ObjC + Foundation goes mobile
**2013:** A7 chip — first 64-bit ARM in a consumer device
**2014:** Swift announced — new language layer atop ObjC runtime
**2017:** dyld 3, chained fixups — binary loading optimized for mobile
**2019:** macOS Catalina — shared cache becomes mandatory
**2020:** Apple M1 — ARM64 on desktop, Rosetta 2
**2022:** arm64e + PAC default for system binaries
**2024:** darwin-loader-lab reaches 84/84 compat gate ← **STORY NOW**

---

## Related

- [technical-landscape.md](technical-landscape.md) — Current state of both worlds
- [world-rules.md](world-rules.md) — Rules that history produced
- [culture-and-society.md](culture-and-society.md) — Developer cultures shaped by this history
- [setting-profile.md](setting-profile.md) — Settings where this history is visible
- [../02-characters/the-platform.md](../02-characters/the-platform.md) — The antagonist shaped by this history
