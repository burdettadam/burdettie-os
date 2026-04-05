---
schema: schemas/01-concept/collision/inciting-incident-selection.schema.json
---

# Inciting Incident Selection

<!-- beat:inciting_incident -->

**Incident Slug:** `first_sigsegv`
**Incident Name:** "The First SIGSEGV"

---

## Description

The reader runs a real macOS ARM64 binary (`uname`) on a Linux ARM64 system.
The binary is a valid executable — compiled by Apple, shipped with Xcode, signed
and authenticated. But it crashes immediately with SIGSEGV.

**Your story's specific version:**
Chapter 1 opens with the reader in a Docker container on an ARM64 Linux host.
They have a macOS `uname` binary extracted from Apple's Xcode toolchain. They
run it. The kernel rejects it — Linux doesn't recognize the Mach-O magic number
`0xfeedfacf`. The crash is immediate and total.

This is not a subtle bug. It's a fundamental format incompatibility. The binary
and the platform don't even share a common language yet.

---

## Structural Function

**What this incident does narratively:**
It opens the story question: "What would it take to run this binary?" The answer
to that question IS the book. Every chapter peels back one layer of the answer.

**Point of no return:**
Once the reader has seen the Mach-O header in hex — once they've seen that the
binary is not opaque but structured, readable, and full of meaning — they can't
unsee it. The binary becomes an artifact to understand, not a black box to use.

---

## Collision Activation

**Collision pattern it activates:** World crossing (platform collision)

**How the incident introduces the collision:**
The SIGSEGV is the exact moment Circle A (macOS ecosystem) and Circle B (Linux
ARM64 world) make contact. The binary brings macOS's expectations — Mach-O
format, dyld loading, Objective-C runtime, Foundation methods — into a world
that has none of them. The crash is the collision made literal.

---

## Example Scene Sketch

**Scene location:** A Docker container running Ubuntu on ARM64.

**Who is present:** The Reader, the `uname` binary, the Linux kernel.

**What happens beat-by-beat:**

1. **Setup:** The reader has Docker running, the macOS binary in hand. They
   know what `uname` does on macOS — it prints the OS name. Simple tool.
2. **Disruption:** They run `./uname`. SIGSEGV. Linux's kernel sees the Mach-O
   magic number `0xfeedfacf` and doesn't recognize it. The `execve()` syscall
   fails before a single guest instruction executes.
3. **Reaction:** Wait — this is just a binary with bytes. Can we read those
   bytes? The reader opens the file in a hex editor. The first four bytes are
   `0xcf 0xfa 0xed 0xfe`. The magic number. This is the Mach-O signature.
4. **New question:** The story question opens: "If we understand this format —
   if we can parse these headers, map these segments, resolve these symbols —
   can we make this binary run?" The rest of the book answers yes.

---

## Alternatives Considered

| Incident | Why Rejected |
|----------|-------------|
| Start with a successful `nm` execution | Too much works before anything breaks — no collision |
| Start with ELF-vs-Mach-O comparison | Too theoretical — no drama, no crash |
| Start explaining ARM64 architecture | Too bottom-up — the reader has no motivation yet |
| Start with Docker setup instructions | Too procedural — save for a "Getting Started" appendix |

---

## Related

- [social-circles.md](social-circles.md) — The two worlds that collide
- [collision-pattern.md](collision-pattern.md) — The full escalation pattern
- [../../04-plot-and-structure/architecture/story-architecture.md](../../04-plot-and-structure/architecture/story-architecture.md) — Where this incident sits in the structure

---

## Notes

The SIGSEGV as inciting incident serves triple duty:
1. **Narrative:** It's dramatic — a real crash that motivates the entire journey
2. **Pedagogical:** It demonstrates exactly what "platform incompatibility" means
   at the binary level
3. **Reproducible:** The reader can reproduce it in the Docker container,
   establishing the hands-on methodology that runs through every chapter
