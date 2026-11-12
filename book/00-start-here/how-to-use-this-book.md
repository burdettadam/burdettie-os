# How to Use This Book Directory

**Phase:** 00 - Start Here
**Purpose:** Orient contributors to the book's Grimoire-based structure

---

## Overview

This directory contains the planning documents for *Loading Darwin*, a
technical nonfiction book about building a macOS binary loader from scratch.

The book structure follows the [Grimoire](https://github.com/Origin-Draft/grimoire)
author-led writing system, adapted for technical nonfiction. Scene cards follow
the [CAP Narrative Profile](https://github.com/Origin-Draft/cap-narrative-profile)
protocol for structured scene representation.

---

## Directory Structure

```
book/
  00-start-here/
    book-brief.md                    ← Master context document for AI sessions
    how-to-use-this-book.md          ← You are here
    ai-collaboration-principles.md   ← How to use AI effectively on this book

  01-concept/
    premise-and-logline.md           ← Core idea, logline, Egri's dramatic premise
    theme-exploration.md             ← Thematic core: controlling idea, motifs, semiotic square
    genre-and-audience.md            ← Genre conventions, audience tiers, Frow mode analysis
    comparable-titles.md             ← Comp analysis: CI, TLPI, Singh, Levine, PR, CS:APP
    collision/
      collision-pattern.md           ← Power asymmetry, arc polarity, escalation engine
      social-circles.md              ← Circle A (macOS) vs Circle B (Linux), Bourdieu capitals
      inciting-incident-selection.md ← The first SIGSEGV as inciting incident

  02-characters/
    cast-overview.md                 ← Full cast: protagonist, antagonist, supporting, crashes
    the-reader.md                    ← Protagonist profile: identity, arc, lie/truth, voice
    the-platform.md                  ← Antagonist profile: layers, demands, revelation arc
    crate-profiles.md                ← Supporting cast: 10 crate character profiles
    antagonist-design.md             ← Grimoire antagonist: thematic mirror, psychology, opposition
    character-relationships.md       ← Relationship mapping, power dynamics, triangles
    relationship-progression.md      ← Stage-by-stage relationship evolution across chapters
    character-voice.md               ← Voice profiles: author prose, platform, crate voices

  03-world-building/
    technical-landscape.md           ← Two worlds, stack comparison, undocumented behaviors
    setting-profile.md               ← 5 primary settings with sensory signatures
    world-rules.md                   ← Core rules, soft rules, costs, knowledge distribution
    history-and-timeline.md          ← NeXT→Apple→Silicon eras, key events, timeline
    culture-and-society.md           ← Apple ecosystem vs systems programming cultures
    sensory-palette.md               ← Full sensory vocabulary for each setting
    motif-and-symbol-tracker.md      ← Recurring symbols, motifs, density map

  04-plot-and-structure/
    architecture/
      story-architecture.md          ← 4 parts, 10 chapters, 2 appendices, 75 scenes
      entity-registry.md             ← Canonical registry: 18 crates, 19 concepts, 8 tools
      story-structure.md             ← Framework: Fichtean Curve + Hero's Journey hybrid
      beat-sheet.md                  ← 15-beat emotional arc mapped to chapters
    scene-design/
      scene-card-index.md            ← 75 scene card slots (unfilled)
      opening-hook.md                ← Scene 1.1–1.2 hook design and alternatives
      chapter-outline.md             ← Expanded scene-level outlines per chapter
    blueprint/
      chapter-blueprint.md           ← Chapter → source file mapping

  05-drafting/            ← Technical chapter and appendix drafts
  06-revision/            ← Structural revision, consistency checks
  07-polish-and-publish/  ← Blurb, pitch, paratext
```

---

## Current Status

Planning documents and technical chapter drafts are present. Begin with the
[chapter index](../README.md) when reading the engineering narrative. Drafts and
planning notes may describe older revisions; the repository README and current
source are the reference for building and running the project.

---

## Workflow

### To draft a chapter:
1. Read the book brief (00-start-here/book-brief.md)
2. Read the story architecture (04-plot-and-structure/architecture/story-architecture.md)
3. Read the chapter's blueprint entry
4. Read the protagonist arc stages relevant to this chapter (02-characters/the-reader.md)
5. Fill the relevant scene cards
6. Draft scenes in order into 05-drafting/

### To use with AI:
Paste the book brief + relevant scene card + entity registry excerpt into
an AI session. Use the scene card's structure to ground the AI's output in
the book's narrative architecture. See ai-collaboration-principles.md for
detailed guidance on collaboration modes.

---

## File Status Summary

| Phase | Files | Status |
|-------|-------|--------|
| 00 - Start Here | 3 files | Planning notes |
| 01 - Concept | 7 files (3 + 3 collision) | Filled |
| 02 - Characters | 8 files | Filled |
| 03 - World Building | 7 files | Filled |
| 04 - Plot & Structure | 8 files | Filled (scene cards unfilled) |
| 05 - Drafting | 10 chapters + 2 appendices | Drafts |
| 06 - Revision | — | Empty |
| 07 - Polish | — | Empty |

---

## Grimoire Phase Gates

Each phase directory will eventually contain a `_gate.md` with readiness
criteria. For now, Phase 00–04 are filled and ready for scene card
development and drafting.
