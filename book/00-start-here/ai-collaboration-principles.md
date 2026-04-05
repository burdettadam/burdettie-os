# AI Collaboration Principles

**Phase:** 00 - Start Here
**Purpose:** Establish effective patterns for human-AI creative partnership on this book
**Status:** Filled

---

## Core Tenet: You Are the Author

This is your book. AI is your research assistant, brainstorm partner, and draft
reader. AI doesn't write your book for you—it helps you think, drafts from your
instructions, and offers alternatives you might not have considered alone.

For *Loading Darwin*, this means: the author's debugging journey, the
engineering decisions, and the "aha" moments are real and personal. AI can help
structure, draft, and check consistency—but the voice, the war stories, and the
technical judgments are yours.

---

## The Five Collaboration Modes (Adapted for Technical Nonfiction)

### 1. Ideate (AI Generates, You Select)

**When to use:** Exploring chapter structures, brainstorming analogies for hard
concepts, generating exercise ideas.

**Example for this book:** "Generate 10 ways to open Chapter 5 (The Foundation)
that start with a real crash and teach the reader something immediately."

**You lead:** Selection and refinement.

### 2. Build (You Lead, AI Clarifies)

**When to use:** Filling out Grimoire templates, developing scene cards,
structuring chapter outlines.

**Example for this book:** You draft the scene card for "First objc_msgSend
Crash." AI asks: "You mention the ISA pointer—should the reader already know
about tagged pointers from the previous scene, or is this the introduction?"

**You lead:** Creation and decision-making.

### 3. Draft (AI Writes, You Steer)

**When to use:** Generating prose for narrative sections, writing transitions
between code listings, producing explanatory paragraphs.

**Example for this book:** You provide the scene card + entity registry + code
listing. AI drafts the explanatory prose around the code. You mark what works
and what sounds too generic or imprecise.

**Critical rule for this book:** All code examples come from the real codebase.
AI drafts the *prose around* the code, never the code itself.

**AI leads:** Initial execution. **You lead:** Direction and revision.

### 4. Critique (AI Evaluates, You Decide)

**When to use:** Reviewing chapter drafts for pacing, technical accuracy,
consistency with the entity registry, prerequisite ordering.

**Example for this book:** You paste a chapter draft and the story architecture.
AI says: "This scene introduces PAC signing before the reader has seen fixup
chains—either reorder or add a forward reference."

**AI leads:** Analysis. **You lead:** Prioritization and implementation.

### 5. Polish (Tight Back-and-Forth)

**When to use:** Sentence-level editing, tightening explanations, improving
analogies, fixing transitions.

**Example for this book:** You: "This explanation of tagged pointers is too
abstract." AI offers three concrete alternatives using hex values from the
actual codebase.

**Equal partnership:** Fast, iterative refinement.

---

## Guidelines for Effective Prompting (This Book)

### Always Provide Context

Every AI session for this book should include:
- `book-brief.md` (this file's sibling)
- The relevant scene card from `04-plot-and-structure/scene-design/`
- Relevant entity registry entries
- The chapter blueprint entry for source file references

### Be Specific About Technical Claims

**Weak:** "Explain how the Objective-C runtime works."
**Strong:** "Explain how our `objc-runtime` crate resolves a selector for
`-[NSString UTF8String]`, referencing the method list walk in
`crates/objc-runtime/src/method.rs`, assuming the reader already understands
class structures from Scene 4.1."

### Ask for Options, Not Answers

**Weak:** "How should I explain fixup chains?"
**Strong:** "Generate 5 different opening paragraphs for the fixup chains scene
that start with a concrete hex dump from `corpus/hello-darwin/`, ranging from
deep-dive to high-level overview."

### Iterate in Public

Don't just regenerate. Tell AI why it missed: "This explanation is correct but
too dense—the reader hasn't seen their first Mach-O yet. Simplify and use the
`uname` binary as the running example."

---

## What AI Does Well in Technical Nonfiction

- Generating structural outlines and chapter organization
- Spotting prerequisite violations (concept used before introduced)
- Drafting explanatory prose around code listings you provide
- Checking consistency across entity references and cross-references
- Suggesting analogies for complex concepts
- Reformatting notes into structured templates

## What AI Does Poorly in Technical Nonfiction

- **Code correctness** — Never trust AI-generated code for this book. All code
  comes from the real, tested codebase.
- **Undocumented behavior claims** — AI can't know what Apple's platform actually
  does. Our 17+ documented behaviors were discovered through real debugging.
- **Hex dump interpretation** — AI often hallucinates byte-level details.
  Always verify against real binary data.
- **Emotional authenticity** — The "we hit a wall" moments and "aha" discoveries
  are real experiences. AI mimics the pattern but not the feeling.
- **Judgment calls** — When to stub vs. fully implement, what to skip, what
  deserves a deep dive. These are authorial decisions.

---

## Red Flags

### The Prose Sounds Like a Textbook
If the prose reads like Wikipedia with code samples, AI has homogenized your
voice. Fix: Write a passage yourself, use it as a style example in prompts.

### Technical Hedging
If AI writes "this may" or "this could potentially" about things you've verified
through thousands of tests, strengthen the language. This book is authoritative.

### Analogies Without Anchors
If explanations use metaphors without grounding them in real code or hex dumps,
the book loses its power. Fix: Every analogy must be tied to a concrete artifact.

---

## Calibrating Collaboration for This Book

**Target calibration: Moderate Use**
- AI for outlines, templates, scene card development, consistency checking
- AI drafts prose you heavily revise (60–80% of narrative prose is author-written)
- All code, hex dumps, and debugging stories are author-sourced
- AI never makes engineering decisions about the project

**The test:** Does the final prose reflect your debugging journey? Does it sound
like someone who actually built this thing? If yes, the calibration is right.

---

## Related

- [book-brief.md](book-brief.md) — The master context document
- [how-to-use-this-book.md](how-to-use-this-book.md) — Directory orientation
- [../01-concept/premise-and-logline.md](../01-concept/premise-and-logline.md) — What the book is about
