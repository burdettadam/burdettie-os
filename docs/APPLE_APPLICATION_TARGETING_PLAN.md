# Apple Application Targeting Plan

## Goal

Push burdettie toward real Apple application execution as aggressively as the
current architecture can support, while staying clean-room, Linux-first, and
independently verifiable.

This is not a license to chase every Apple-private dependency tree. It is a
priority rule: when two pieces of work are both plausible, prefer the one that
unlocks more real Apple application behavior.

## Operational Context

- The primary runtime lane is an external Docker Compose deployment of
  burdettie. The Compose file is not present in this workspace, so this repo can
  define requirements for that lane but cannot verify the service wiring itself.
- The current Linux container workflow already stages a Mac-extracted Xcode tool
  corpus for compatibility testing. That is acceptable as a validation input for
  today, but application targeting should treat imported Apple artifacts as an
  optional validation overlay, not as the definition of the base runtime.
- A Mac host is available as an extraction oracle. Apple tools, frameworks, and
  app bundles may be staged from that host through repo scripts or transferred
  externally, including by wormhole.
- Imported Apple artifacts may be required for compatibility testing in the
  current workflow, but the runtime substrate and build path must not require a
  live Apple host at execution time.

## Artifact Ingestion Rules

When importing Apple binaries or app bundles from a Mac host, keep the process
boring and auditable.

Required provenance for each imported artifact set:

- source path on the Mac host
- macOS version and build
- Xcode version and build, if the artifact comes from Xcode or its toolchains
- extraction date
- architecture slice information
- SHA-256 or equivalent content hash

The concrete Docker-first manifest contract now lives in:

- `docker/imported-artifacts.schema.json`
- `docker/imported-artifacts.example.json`
- `docs/DOCKER_FIRST_DEVELOPMENT.md`

The repo already contains a Mac-side extraction path for Xcode tooling in
`scripts/extract_xcode_tools.sh`. That script is the preferred path when the
target is a tool binary rather than an app bundle.

For the external Compose lane, keep two layers separate:

- base runtime image: burdettie itself and repo-native assets
- imported validation overlay: read-only Apple tools, frameworks, and app
  bundles used for compatibility testing

If the imported overlay is absent, the base runtime should still build and run
its repo-native applications.

## Strategic Bias

### Prefer application-enabling work over tool-only wins

Additional banner or usage-output wins for Apple developer tools are lower
priority than work that makes real app bundles launch, paint, respond to input,
or load resources correctly.

Higher-value application-facing surfaces include:

- `NSBundle` and `Info.plist` behavior
- `NSApplication`, `NSWindow`, `NSView`, responder chain, and event loop
- menu dispatch, pasteboard, file panels, and workspace integration
- TextKit and text input
- `NSImage`, Core Graphics rendering, and layer compositing
- Auto Layout and nib or keyed-archive resource loading

### Treat private-framework apps as a separate tier

Apps or tools that import large private-framework chains are still useful as
evidence, but they should not drive the first compatibility contract.

## Target Tiers

### Tier A — Public-surface Apple applications

Highest priority.

These are Apple-signed or Apple-style macOS app bundles whose dependency graph
stays primarily within public surfaces already represented in the repo:

- AppKit
- Foundation
- CoreFoundation
- CoreGraphics
- CoreText
- Security
- CFNetwork
- libSystem

Success criteria:

- app bundle loads under the runtime
- main executable launches
- window or event-loop behavior is observable
- key interactions work for the chosen scenario
- no private frameworks are required on the execution path

### Tier B — Rich public Cocoa applications

Second priority.

These apps deepen public Cocoa coverage without requiring Xcode-private IDE
frameworks. Typical pressure points include:

- nib or keyed-archive resource loading
- menus, document lifecycle, and panels
- table or outline views
- text editing and selection
- image decoding and compositing
- Auto Layout-heavy interfaces

These are strong next targets because the repo already has meaningful support in
many of these areas. They still must stay off private-framework execution paths
to count as Tier B progress.

### Tier C — Private-surface Apple applications and tools

Third priority.

This tier includes binaries that pull in any Apple-private frameworks,
including `DVTFoundation`, `IDEFoundation`, `CoreSimulator`, Instruments
internals, or similar private surfaces.

They are valid investigation corpora and long-term stretch goals, but they
should not redefine the MVP and should not be allowed to pull the architecture
back toward stub farming.

## Relationship To Xcode Compatibility Work

Xcode compatibility still matters, but it should be sized to serve application
execution rather than dominate it.

Implications:

- A Rust-native `xcodebuild` subset is valuable because it can build the kinds
  of app bundles burdettie should run.
- Build-time replacement for `ibtool` and `actool` is still post-MVP.
- Runtime loading of already-built nibs and app resources is valuable now and
  should be treated as application-runtime coverage, not blocked on tool
  replacement.
- Runtime loading now covers `NSMainNibFile` and a minimal `NSMainStoryboardFile`
  path for compiled `.storyboardc` scene nibs. Treat both as launch-enabling
  coverage until verified against real app bundles.
- Any Xcode work that only improves private-framework startup without improving
  real build or application behavior should rank behind Tier A or Tier B app
  enablement.

## Verification Strategy

Use three complementary lanes.

### Lane 1 — Repo-native baseline

Keep the existing GUI and runtime apps as a fast sanity check:

- `apps/hello-darwin`
- `apps/control-gallery`
- `apps/foundation-explorer`

These are not substitutes for Apple applications, but they prove that the
windowing, event, rendering, and Foundation stack still works while the corpus
expands.

### Lane 2 — Imported Apple application corpus

Build a small curated corpus of Mac-sourced app bundles chosen by dependency
shape, not by prestige.

The first corpus should contain 3-5 apps with increasing pressure:

1. one simple single-window app with minimal dependencies
2. one nib-backed or keyed-archive-backed app
3. one text-heavy or menu-heavy app
4. one image-heavy or layout-heavy app, if it stays on public surfaces

Current recommendation for the first real Apple-made GUI target: `Stickies.app`.
It is a better next milestone than `TextEdit.app` because the runtime already
has direct coverage for nib/storyboard loading, menus, scroll views, and text
views, while `NSDocument`/`NSDocumentController` coverage is still minimal and
should be validated before TextEdit is treated as a primary target.

For each imported app, record:

1. dependency map
2. framework classification: public-surface, mixed, or private-heavy
3. launch result under burdettie
4. observable behavior reached
5. divergence versus Mac-host behavior

Launchpad enforces the Docker-first manifest classification for imported app
bundles before they appear as scoreable desktop GUI apps. A manifest entry must
be Tier A or Tier B, `public-surface`, and `scoreable_in_docker`; the main
executable must also parse as a Mach-O whose linked install names stay on
system public surfaces. Validation-only, mixed, private-framework, missing, or
unparseable imports stay launch-visible as corpus targets instead of inflating
Desktop progress.

### Lane 3 — Mac oracle comparison

Use the Mac host as a comparison and diagnosis lane:

- capture bundle metadata and framework dependencies
- capture launch behavior and expected logs or visible milestones
- compare burdettie behavior against that external result

Mac-host runs do not count toward scored application progress if host-resolved
Apple frameworks or host-assisted symbol resolution are active. Only Linux or
Compose runs, or other runs that can prove those assists are absent, count for
Tier A or Tier B progress.

This keeps validation strong without letting host assistance masquerade as real
runtime coverage.

## Immediate Next Slice

1. Select the first 3-5 Apple app bundles by dependency shape and public-surface
  likelihood.
  Start with `Stickies.app` as the first text-heavy imported target, and treat
  `TextEdit.app` as the follow-on stretch target after the new document-controller
  coverage is verified against a real document-based bundle.
2. Keep extending the Launchpad dependency gate as new imported apps expose
   additional public framework patterns.
3. Keep Xcode build-driver work focused on building at least one simple app
   bundle that can join the same verification lane as the imported corpus.
4. Treat nib-backed app claims as code-backed verification work, not a docs-only
  assumption, until the wider documentation set is fully reconciled.
5. Wire the external Compose lane to mount the imported-artifact manifest and
  overlay read-only using the Docker-first contract.

## Related Repo Evidence

- `docs/COCOA_SUPPORT_PLAN.md`
- `docs/DOCKER_FIRST_DEVELOPMENT.md`
- `docs/APPLE_STACK_COMPARISON.md`
- `docs/XCODE_PUBLIC_COMPATIBILITY_PLAN.md`
- `docs/XCODE_CLI_INVESTIGATION.md`
- `scripts/extract_xcode_tools.sh`
