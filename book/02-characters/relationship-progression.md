# Relationship Progression

**Phase:** 02 - Characters
**Purpose:** Track how key relationships evolve through defined stages, beat by beat
**Status:** Filled

---

## Relationship Selection

The 4 most story-critical relationships:

1. **The Reader** & **The Platform** — The central antagonist relationship
2. **The Reader** & **cf-lite** — The longest creative partnership
3. **The Reader** & **libsystem-shims** — The crisis relationship (Ch 6 dark night)
4. **objc-runtime** & **cf-lite** — The foundational alliance that enables all upper layers

---

## Relationship Stage Map

### Relationship: The Reader & The Platform

#### Arc Archetype

**Which pattern best fits this relationship?**
[x] Rivals to Allies

**How does your version deviate from the archetype?**
The antagonist never changes — the platform is a fixed system. All transformation happens in the reader. The "alliance" is one-sided: the reader comes to understand and respect the platform, but the platform simply *is*. This makes the arc more about perception shift than mutual change.

#### Stage Progression

| # | Stage | Trigger Event | Chapter | Emotional State | External Marker |
|---|-------|--------------|---------|----------------|-----------------|
| 1 | Strangers | Reader opens hex editor | Ch 1 | Reader: curiosity (4). Platform: indifferent. | `hexdump -C /usr/bin/true` |
| 2 | First Contact | Magic bytes `0xfeedfacf` decoded | Ch 1 | Reader: excitement (4). Platform: indifferent. | `ParsedMachO` successfully created |
| 3 | Testing | First memory mapping, first crash | Ch 2 | Reader: frustrated (3). Platform: unyielding. | SIGSEGV on first execution attempt |
| 4 | Bonding | Symbol resolution works, first tool runs | Ch 3 | Reader: cautious hope (3). Platform: slightly yielding. | `nm --help` outputs usage text |
| 5 | Deepening | ObjC runtime dispatch works | Ch 4 | Reader: growing confidence (4). Platform: layered revelation. | `objc_msgSend` dispatches correctly |
| 6 | Strain | Foundation calls fail in new ways | Ch 5 | Reader: frustrated (4). Platform: demanding more. | `-[NSString UTF8String]` returns garbage |
| 7 | Rupture | Linux ABI mismatch — crashes with no useful error | Ch 6 | Reader: despair (5). Platform: silent refusal. | SIGSEGV at `0x40000000000000` (PAC trap) |
| 8 | Repair (internal) | Reader realizes platform ABI *is* documented (in the binary) | Ch 6 | Reader: clarity (4). Platform: consistent. | Variadic shim assembly working |
| 9 | Repair (external) | 84/84 tools pass | Ch 9 | Reader: mastery (5). Platform: fully characterized. | `test-xcode-tools` all green |
| 10 | Transformed | GUI window opens | Ch 10 | Reader: earned respect (5). Platform: understood. | `cocoa_window` fixture launches |

#### Relationship Beat Inventory

**Trust-Building Beats Used:**
- [x] Reliability proof — The platform always crashes the same way for the same bug (deterministic)
- [x] Secret shared — Each undocumented behavior discovered is a secret the platform "reveals"

**Tension & Conflict Beats Used:**
- [x] Boundary violation — Reader tries to run a binary that needs Foundation before Foundation is shimmed
- [x] Competing needs — Platform needs full contract compliance; reader needs incremental progress
- [x] Power shift — Shared cache (platform's ultimate opacity) vs. cache extraction (reader's ultimate tool)

**Rupture Beats Used:**
- [x] Confrontation — The variadic ABI crisis (Ch 6): platform provides zero error information
- [x] Reveal — Auth fixup bit positions aren't where Apple's headers say they are

**Repair Beats Used:**
- [x] Changed behavior — Reader stops guessing and starts reading the binary systematically
- [x] Difficult conversation — The realization that "undocumented" doesn't mean "unknowable"

---

### Relationship: The Reader & cf-lite

#### Arc Archetype

**Which pattern best fit?**
[x] Mentor & Student (but reader is both mentor and student — they teach cf-lite what Foundation does by learning it themselves)

#### Stage Progression

| # | Stage | Trigger Event | Chapter | Emotional State | External Marker |
|---|-------|--------------|---------|----------------|-----------------|
| 1 | Aware | Reader sees Foundation import in first real binary | Ch 3 | Reader: intimidated. | `-[NSString UTF8String]` in symbol table |
| 2 | First Contact | First synthetic class registration | Ch 5 | Reader: cautious. | `overlay_from("NSString")` compiles |
| 3 | Testing | First shim function, first test | Ch 5 | Reader: hopeful. | 1 cf-lite test passes |
| 4 | Bonding | NSString, NSArray, NSDictionary all work | Ch 5 | Reader: productive. | 100+ tests pass |
| 5 | Deepening | NSFileManager, NSTask, real I/O | Ch 5 | Reader: ambitious. | Corpus tools start passing |
| 6 | Strain | NSURLSession — real HTTP needed | Ch 5 | Reader: overwhelmed. | Integration complexity spikes |
| 7 | Committed | 2,682 tests, 193 classes, 867 selectors | Ch 9 | Reader: proud, exhausted. | `cargo test -p cf-lite` all green |

---

### Relationship: The Reader & libsystem-shims

#### Arc Archetype

**Which pattern best fits?**
[x] Rivals to Allies — initially hostile (the hardest bugs), eventually essential

#### Stage Progression

| # | Stage | Trigger Event | Chapter | Emotional State | External Marker |
|---|-------|--------------|---------|----------------|-----------------|
| 1 | Strangers | Reader doesn't know Linux ABI differs | Ch 1–5 | Reader: unaware. | Everything works on macOS host |
| 2 | First Contact | First Docker ARM64 run crashes | Ch 6 | Reader: confused (4). | SIGSEGV in `cut` binary |
| 3 | Testing (adversarial) | Three days debugging variadic crash | Ch 6 | Reader: frustrated (5). | `printf("%s", str)` reads wrong register |
| 4 | Rupture | Locale categories numbered differently | Ch 6 | Reader: despair (4). | `setlocale(0, "")` means different things |
| 5 | Bonding | Assembly variadic shim works | Ch 6 | Reader: triumph (5). | `cut` runs correctly on Linux |
| 6 | Deepening | struct stat, data symbols, err/errx | Ch 6–8 | Reader: methodical (3). | Each ABI mismatch resolved systematically |
| 7 | Committed | 443 shim functions across 37 modules | Ch 9 | Reader: respect (4). | Docker compat gate: 51+ PASS |

---

### Relationship: objc-runtime & cf-lite

#### Arc Archetype

**Which pattern best fits?**
[x] Found Family — two systems that become inseparable

#### Stage Progression

| # | Stage | Trigger Event | Chapter | Emotional State | External Marker |
|---|-------|--------------|---------|----------------|-----------------|
| 1 | Strangers | objc-runtime exists (Ch 4); cf-lite doesn't yet | Ch 4 | Independent | Runtime has no consumer |
| 2 | First Contact | cf-lite calls `objc_allocateClassPair` | Ch 5 | Tentative coupling | First synthetic Foundation class |
| 3 | Deepening | cf-lite needs KVO, ARC, exceptions | Ch 5–6 | Demand-driven growth | objc-runtime features added for cf-lite |
| 4 | Strain | Tagged pointer dispatch needs runtime changes | Ch 5 | Friction at boundary | ISA dispatch vs. tag dispatch paths |
| 5 | Committed | Toll-free bridging — CF and NS share ISAs | Ch 5 | Inseparable | `CFStringRef == NSString*` |

---

## Relationship State at Major Beats

| Plot Beat | Reader & Platform | Reader & cf-lite | Reader & libsystem-shims | objc-runtime & cf-lite |
|-----------|------------------|-----------------|--------------------------|----------------------|
| Opening (Ch 1) | Strangers | N/A | N/A | N/A |
| Inciting Incident (Ch 1 first crash) | First Contact | N/A | N/A | N/A |
| Plot Point 1 (Ch 3 linking) | Testing | Aware | N/A | N/A |
| Midpoint (Ch 5 Foundation) | Deepening | Bonding | N/A | Deepening |
| All Is Lost (Ch 6 ABI crisis) | Rupture | Strain | Rupture | Strain |
| Climax (Ch 9 tools pass) | Repair | Committed | Committed | Committed |
| Resolution (Ch 10 GUI) | Transformed | Committed | Committed | Committed |

---

## Testing Your Relationship Progressions

- [x] Does every stage transition have a causal trigger event?
- [x] Are there no skipped stages without narrative justification?
- [x] Does the pacing match genre expectations? (Technical nonfiction: progressive mastery)
- [x] Is there variety in beat types?
- [x] Do physical/external markers mirror emotional states?
- [x] Can you point to the specific scene where each stage transition happens?
- [x] Does the relationship progression support the character arc?

---

## Related

- [character-relationships.md](character-relationships.md) — Static relationship web
- [the-reader.md](the-reader.md) — Character arc that relationships support
- [cast-overview.md](cast-overview.md) — Full ensemble
- [../04-plot-and-structure/architecture/story-architecture.md](../04-plot-and-structure/architecture/story-architecture.md) — Beats to align with
