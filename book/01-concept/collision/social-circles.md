---
schema: schemas/01-concept/collision/social-circles.schema.json
---

# Social Circles

**Phase:** 01 - Concept & Collision
**Purpose:** Define the social/technical groups that create the book's collision
**Status:** Filled

---

## Collision Pattern

**Selected Plot Type:** Discovery / Construction narrative
**Required Collision Pattern:** World crossing (two incompatible worlds forced into contact)
**Pattern Name:** Platform Collision
**Power Asymmetry:** dominant_a (macOS controls the binary format and runtime expectations)

---

## Circle A: The Apple Platform

**Circle Name:** The macOS Ecosystem
**Type:** `technology_ecosystem`
**Role in Collision:** Circle A / The dominant world / The system being reproduced

### Identity

| Field | Value |
|---|---|
| **Ideology** | "Control the full stack, optimize the integrated experience" |
| **Core Values** | 1. Tight integration across layers 2. Binary compatibility across versions 3. User experience over developer transparency |
| **Taboos** | 1. Exposing internal implementation details 2. Breaking binary compatibility |
| **Power Level** | dominant — controls the binary format, runtime expectations, and framework APIs |

### Capital Profile (Bourdieu)

| Capital Type | Level (1–5) | Notes |
|---|---|---|
| **Economic** | 5 | Trillion-dollar company; controls hardware + software |
| **Cultural** | 5 | Design authority; "the Apple way" is self-reinforcing |
| **Social** | 4 | Massive developer community, but one-directional (Apple dictates) |
| **Symbolic** | 5 | "It just works" brand; implicit trust in platform correctness |

### Boundaries

| Field | Value |
|---|---|
| **Membership Criteria** | Buy the hardware, use the SDK, accept the Terms of Service |
| **Entry Ritual** | Download Xcode, sign the developer agreement, learn Objective-C/Swift |
| **Exit Cost** | Rewrite everything; lose framework access; lose hardware ecosystem |

### Polarity (Weiland)

| Field | Value |
|---|---|
| **Lie This Circle Enforces** | "The platform is too complex for outsiders to understand; trust us" |
| **Truth This Circle Resists** | "The platform's behavior is discoverable if you're willing to rebuild it" |

### Key Members

| Character | Membership Type | Role in Circle | Notes |
|---|---|---|---|
| Mach-O Format | core_insider | The binary language | Every tool speaks it |
| dyld | core_insider | The bootstrap engine | Loads everything before main() |
| ObjC Runtime | core_insider | The dispatch authority | Controls message passing |
| Foundation | core_insider | The API surface | What applications talk to |
| libSystem | core_insider | The OS abstraction | Wraps kernel + libc + dispatch |

---

## Circle B: The Linux ARM64 World

**Circle Name:** The Linux/Rust Builder's Workshop
**Type:** `open_source_ecosystem`
**Role in Collision:** Circle B / The host system / Where we rebuild

### Identity

| Field | Value |
|---|---|
| **Ideology** | "Everything is open, documented, and hackable" |
| **Core Values** | 1. Transparency of implementation 2. POSIX compatibility 3. User freedom over user experience |
| **Taboos** | 1. Proprietary lock-in 2. Undocumented behavior |
| **Power Level** | established — dominant in servers/containers, but has no Apple framework knowledge |

### Capital Profile (Bourdieu)

| Capital Type | Level (1–5) | Notes |
|---|---|---|
| **Economic** | 3 | Free but with infrastructure costs |
| **Cultural** | 4 | Strong open-source culture; Rust community |
| **Social** | 5 | Massive global community; bidirectional contribution |
| **Symbolic** | 3 | Respected in engineering; less consumer prestige |

### Boundaries

| Field | Value |
|---|---|
| **Membership Criteria** | Install it; contribute if you want |
| **Entry Ritual** | `apt install`, `cargo init`, submit a PR |
| **Exit Cost** | Minimal — portable by design |

### Polarity (Weiland)

| Field | Value |
|---|---|
| **Lie This Circle Enforces** | "POSIX is sufficient; Apple's proprietary APIs are unnecessary complexity" |
| **Truth This Circle Resists** | "Apple's integration produces genuine quality that open alternatives don't match" |

### Key Members

| Character | Membership Type | Role in Circle | Notes |
|---|---|---|---|
| ELF Format | core_insider | The native binary language | What Linux expects |
| glibc | core_insider | The C runtime | POSIX compliance anchor |
| Docker | core_insider | The containment system | Where the collision happens |
| Rust / Cargo | core_insider | The builder's tools | What we construct with |
| The Author/Reader | bridge | The person crossing worlds | Lives in both circles |

---

## The Collision

### Collision Dynamics

| Field | Value |
|---|---|
| **Collision Type** | world_crossing — a macOS binary placed into a Linux system |
| **What Forces Contact?** | Running `./uname` (a Mach-O binary) on Linux ARM64 triggers SIGSEGV |
| **What's at Stake for Circle A?** | Its opacity. Every successful shim reveals a previously hidden contract |
| **What's at Stake for Circle B?** | Its assumptions. POSIX is not enough; the binary demands more |
| **Power Dynamic** | Circle A dominates at start (the binary dictates requirements); shifts as the loader grows powerful enough to satisfy them |

### Bridge Characters

| Character | Circle A Role | Circle B Role | Why They Bridge | Cost of Bridging |
|---|---|---|---|---|
| The Builder (Reader/Author) | Consumer/Analyst of Apple binaries | Rust developer on Linux | Curiosity about both worlds | Must learn Apple's worldview to succeed in Linux's |
| macho-inspect | Speaks Mach-O, understands Apple conventions | Runs on Linux, built with Cargo | The binary executer that lives in both worlds | Carries complexity of both stacks |
| libsystem-shims | Presents Apple's libSystem face | Delegates to Linux's glibc | Translates syscalls and C library between worlds | Every undocumented difference is a potential crash |

---

## Collision Escalation

| Story Phase | Collision State | What Happens |
|---|---|---|
| **Setup (Ch 1)** | Total incompatibility | Linux can't even read a Mach-O file |
| **Inciting Incident (Ch 1)** | First contact (parse) | `macho-parse` reads the header; the artifact speaks |
| **Rising Action (Ch 2–4)** | Escalating friction | Each layer (memory, symbols, runtime) reveals new incompatibilities |
| **Midpoint (Ch 5)** | Foundation breakthrough | First Foundation method call succeeds — Circle B starts speaking Circle A's language |
| **Crisis (Ch 7–8)** | ABI and system-level collisions | Variadic args, locale numbering, PAC signing — the deepest incompatibilities |
| **Climax (Ch 9)** | Full convergence | 84/84 tools pass. The two worlds can coexist |
| **Resolution (Ch 10)** | New equilibrium | GUI framework (AppKit) reveals the collision never truly ends — there's always another layer |

---

## Related

- [collision-pattern.md](collision-pattern.md) — The escalation pattern in detail
- [inciting-incident-selection.md](inciting-incident-selection.md) — The SIGSEGV that starts everything
- [../genre-and-audience.md](../genre-and-audience.md) — Genre conventions this collision serves
- [../../02-characters/cast-overview.md](../../02-characters/cast-overview.md) — Full cast derived from collision slots
