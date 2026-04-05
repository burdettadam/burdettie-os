# Sensory Palette

**Phase:** 03 - World-Building
**Purpose:** Define the sensory vocabulary for each setting and emotional beat
**Status:** Filled

---

## Setting #1: The Hex Editor

### Visual Details

| Layer | Description |
|---|---|
| **Dominant Color** | Blue-white on black. Monospaced columns. |
| **Texture** | Gridded, aligned, architectural. Every byte has an address. |
| **Lighting** | Backlit screen glow in a dim room. |
| **Movement** | Scrolling through thousands of rows. Cursor blinking at offset 0. |
| **Focal Point** | The magic number at offset 0x00: `CF FA ED FE` (little-endian `0xFEEDFACF`). |
| **Negative Space** | Null bytes (`00 00 00 00`) — empty padding between sections. |

### Sounds (Metaphorical)

| Quality | Description |
|---|---|
| **Dominant Sound** | Silence. The hex editor is the quietest place in the book. |
| **Rhythm** | The steady beat of 16-byte rows. Metronomic. |
| **Signal Sound** | A non-null byte appearing in a sea of zeros — "something lives here." |
| **Dissonance** | Corrupted bytes, unexpected patterns, ASCII strings in executable sections. |

### Textures & Tactile

| Quality | Description |
|---|---|
| **What Things Feel Like** | Smooth, crystalline, exact. Every value is unambiguous. |
| **Reader Experience** | Archaeological — brushing away layers to reveal structure. |
| **Contrast** | The structured binary vs. the messy human interpretation of it. |

### Signature Scent (Metaphorical)

Old stone and dust. An archive where things have been preserved unchanged for years. The binary was compiled once and hasn't changed since.

---

## Setting #2: The Terminal / Docker Container

### Visual Details

| Layer | Description |
|---|---|
| **Dominant Color** | Green text on black. Syntax-colored trace output. |
| **Texture** | Scrolling logs — fast, dense, vertical. |
| **Lighting** | Harsh fluorescent — the Docker build is always running. |
| **Movement** | Rapid vertical scroll of PASS/FAIL lines. Output buffering. |
| **Focal Point** | The final count: `84 of 84 tools passed`. Or: `SIGSEGV at 0x...` |
| **Negative Space** | The empty pause between command and output — "did it work?" |

### Sounds (Metaphorical)

| Quality | Description |
|---|---|
| **Dominant Sound** | Machine-gun staccato of test output. `ok` `ok` `ok`. |
| **Rhythm** | The Docker build: long, steady compilation → sudden burst of test results. |
| **Signal Sound** | `FAIL` — the one red line among fifty green lines. A gunshot in a library. |
| **Dissonance** | Stack traces. Panic messages. Core dumps. |

### Textures & Tactile

| Quality | Description |
|---|---|
| **What Things Feel Like** | Industrial. Cinderblock walls, metal shelving. Functional, not pretty. |
| **Reader Experience** | The factory floor — where the work actually happens. |
| **Contrast** | The polished macOS experience vs. the raw Docker execution environment. |

### Signature Scent (Metaphorical)

Machine oil and ozone. A running engine room. Things are hot, spinning, active.

---

## Setting #3: The Source Code / IDE

### Visual Details

| Layer | Description |
|---|---|
| **Dominant Color** | Syntax-highlighted Rust — orange keywords, blue types, green strings, white identifiers. |
| **Texture** | Indented, nested, tree-shaped. Match arms fan out like branches. |
| **Lighting** | Warm. This is the most comfortable setting in the book. |
| **Movement** | Jumping between files. Ctrl+click navigation. Expanding function chains. |
| **Focal Point** | The current function being written. `fn resolve_chained_fixups(...)` |
| **Negative Space** | Comments explaining intent: `// Apple's header says BIND_TYPE_POINTER but the actual bit-field is...` |

### Sounds (Metaphorical)

| Quality | Description |
|---|---|
| **Dominant Sound** | Quiet construction. Hammering and planing, not explosions. |
| **Rhythm** | Write-test-refactor. A three-beat waltz. |
| **Signal Sound** | `cargo test` passing — the satisfying click of a lock tumbler falling into place. |
| **Dissonance** | Type errors, borrow checker complaints — the Rust compiler as demanding teacher. |

### Textures & Tactile

| Quality | Description |
|---|---|
| **What Things Feel Like** | Crafted wood and dovetail joints. Precise, warm, solid. |
| **Reader Experience** | The workshop — building something with your hands. |
| **Contrast** | Clean source code vs. the messy binary it processes. |

### Signature Scent (Metaphorical)

Fresh sawdust and varnish. Creation in progress. Something being built.

---

## Setting #4: The Crash Scene / Debugger

### Visual Details

| Layer | Description |
|---|---|
| **Dominant Color** | Red. Fault addresses in alarming highlight. |
| **Texture** | Fractured — register dumps, stack backtraces, signal handlers. |
| **Lighting** | Harsh interrogation spotlight. Everything exposed. |
| **Movement** | Frozen. The crash is a snapshot — nothing is running anymore. |
| **Focal Point** | The PC value. The fault address. `pc=0x102a4b8c0, fault_addr=0x0000000000000000` |
| **Negative Space** | The registers that have normal values — they're fine, ignore them. It's the ONE register that's wrong. |

### Sounds (Metaphorical)

| Quality | Description |
|---|---|
| **Dominant Sound** | The ringing silence after an explosion. |
| **Rhythm** | Staccato register dump, then long silence while the reader thinks. |
| **Signal Sound** | The `aha` — "the fault address is in the PAC range!" — a bell ringing in fog. |
| **Dissonance** | When the crash doesn't make sense yet. Signal without meaning. |

### Textures & Tactile

| Quality | Description |
|---|---|
| **What Things Feel Like** | Broken glass. Sharp, dangerous, informative. Each fragment tells a story. |
| **Reader Experience** | Crime scene investigation. The body is the core dump. |
| **Contrast** | The violence of the crash vs. the order it reveals once decoded. |

### Signature Scent (Metaphorical)

Cordite and cold air. Something happened here. The evidence is still warm.

---

## Setting #5: The Architecture Diagram

### Visual Details

| Layer | Description |
|---|---|
| **Dominant Color** | Black and white with color accents. Boxes, arrows, layers. |
| **Texture** | Geometric. Clean lines, labeled connections. |
| **Lighting** | Even, clinical. Everything visible at once. |
| **Movement** | The eye tracing arrows: "this depends on this depends on this." |
| **Focal Point** | The layer boundary being discussed. The dotted line between "our implementation" and "host system." |
| **Negative Space** | Things deliberately omitted from the diagram — "not our problem." |

### Sounds (Metaphorical)

| Quality | Description |
|---|---|
| **Dominant Sound** | Whiteboard markers squeaking. The sound of explanation. |
| **Rhythm** | Top-down or bottom-up traversal. Systematic. |
| **Signal Sound** | The moment a new connection appears: "oh, THIS depends on THAT." |
| **Dissonance** | Circular dependencies. Lines that shouldn't cross but do. |

### Textures & Tactile

| Quality | Description |
|---|---|
| **What Things Feel Like** | Paper and pencil. Structural, abstract, clean. |
| **Reader Experience** | The map room — planning before the expedition. |
| **Contrast** | The simplicity of the diagram vs. the complexity of what it represents. |

### Signature Scent (Metaphorical)

Dry-erase markers and coffee. Strategy session. Planning the approach.

---

## Sensory Signatures by Emotional Beat

| Emotional Beat | Visual | Sound | Texture | Scent |
|---|---|---|---|---|
| **Discovery** | A pattern emerging in hex | Quiet "oh" | Puzzle piece clicking | Fresh air |
| **Frustration** | Red crash output | Static/noise | Sandpaper | Burnt coffee |
| **Breakthrough** | Green PASS line | Clear bell | Smooth stone | After the rain |
| **Wonder** | Full architecture diagram | Symphony | Silk | Old library |
| **Dread** | `SIGSEGV` at unknown address | Silence | Ice | Ozone before storm |
| **Triumph** | `84 of 84 tools passed` | Applause | Warm wood | Sawdust + varnish |
| **Mastery** | Reading hex without a reference | Conversation | Familiar tools | Home |

---

## Transition Sensory Cues

**Hex Editor → Terminal:** The static hex becomes living execution. Colors go from blue-white to green. Static grid becomes flowing scroll. Archive becomes engine room.

**Terminal → Crash Scene:** The green scroll suddenly stops. A red line. Sound cuts to silence. Movement freezes. Engine room becomes crime scene.

**Crash Scene → Source Code:** From broken glass to warm workshop. From red to multi-color syntax. From frozen to flowing. From investigation to construction.

**Source Code → Terminal:** From workshop back to factory floor. From creation to testing. The moment of truth.

---

## Related

- [setting-profile.md](setting-profile.md) — Detailed setting descriptions these palettes enhance
- [../04-plot-and-structure/scene-design/scene-card-index.md](../04-plot-and-structure/scene-design/scene-card-index.md) — Scenes where these palettes are applied
- [technical-landscape.md](technical-landscape.md) — Technical context underlying the sensory details
