# Xcode Public Compatibility Plan

## Goal

Deliver Xcode interoperability by matching Xcode's public external behavior in a
Rust-native stack, without depending on Apple's private IDE frameworks.

For this project, the compatibility contract is:

- Documented command-line behavior: `xcode-select`, `xcrun`, `xcodebuild`, and
  adjacent developer-tool workflows.
- Stable external file formats and inputs: `.xcodeproj`, `.pbxproj`,
  `.xcworkspace`, `.xcscheme`, `.xcconfig`, `.plist`, bundle layouts, and build
  settings.
- Observable workflow semantics: project discovery, build setting expansion,
  target graph resolution, product assembly, test invocation, signing, and
  result reporting.

Apple-private frameworks such as `DVTFoundation`, `IDEFoundation`,
`CoreSimulator`, Instruments internals, and Xcode-private Swift modules are
validation oracles only. They are not the product surface and should not define
the MVP.

## Current Starting Point

- The runtime substrate is already strong: Mach-O loading, ObjC, Foundation,
  libSystem, and broad Xcode CLI execution are in place.
- The repo currently runs 67/72 Xcode CLI tools on Linux/ARM64.
- That 67/72 number is a runtime-compatibility baseline, not an MVP KPI. It
  includes shallow version and usage probes and must not be treated as proof of
  semantic build compatibility.
- Remaining failing tools include `xcodebuild`, `ibtool`, `actool`,
  `notarytool`, `altool`, `xctest`, and `xctrace`.
- Heavy Xcode tools currently fan into 80+ private framework images with 574
  unresolved symbols, including 196 Swift-mangled symbols.
- Some Swift bridge shims only unblock startup; they do not establish semantic
  compatibility for real tool behavior.
- The repo already has meaningful Cocoa runtime depth: simple AppKit
  applications, nib loading, Auto Layout, text, image, event-loop, and window
  behavior are not greenfield.

The practical conclusion is simple: shallow public-surface gaps should be
closed, but once a workflow crosses into deep private-framework or Swift-only
surfaces, the correct move is replacement with a Rust-native compatible layer,
not indefinite stub growth.

That same conclusion applies to application targeting: public Cocoa and AppKit
surfaces are a better near-term target than more private Xcode framework
emulation. See `docs/APPLE_APPLICATION_TARGETING_PLAN.md`.

## Decomposition And Priority Order

| Phase | Surface | External contract to match | Why it comes first | MVP |
|---|---|---|---|---|
| P0 | Environment fidelity | `xcode-select` paths, `xcrun --find`, `xcrun --show-sdk-path`, `DEVELOPER_DIR`, `SDKROOT`, toolchain lookup, semantic vs startup scoring | Every higher workflow depends on correct tool and SDK discovery. This is already partially working, so it is the cheapest place to harden behavior. | Yes |
| P1 | Project and workspace model | `.xcodeproj`, `.pbxproj`, `.xcworkspace`, `.xcscheme`, `.xcconfig`, build setting inheritance and expansion | Without a stable project graph and setting model, `xcodebuild` compatibility is just banner parity. | Yes |
| P2 | Build driver | `xcodebuild` command subset, action selection, scheme and target resolution, build setting output, clean/build orchestration | This is the highest-value missing public contract and avoids `DVTFoundation` and `IDEFoundation`. | Yes |
| P3 | Compilation and product assembly | Compiler and linker orchestration, Info.plist processing, bundle layout, product placement, derived data layout where needed | A build driver is only useful if it can produce compatible outputs for constrained project types. MVP must use a Rust-native driver over non-Apple toolchain components rather than depend on Apple's blocked `clang`/`ld`/`libtool` path. | Yes, narrow slice |
| P4 | Test execution | `xcodebuild test`, test bundle discovery, result reporting, minimal XCTest-compatible invocation | Valuable, but depends on successful bundle construction and adds another compatibility surface. | No |
| P5 | Resource pipeline | `actool`, `ibtool`, asset catalogs, storyboards, nib compilation, localization compilation | This is a private-framework-heavy area and should be replaced only after the build core works. Runtime loading of already-built app resources is a separate application-runtime concern. | No |
| P6 | Signing and distribution | Entitlements, bundle sealing, archive/export flows, local signing semantics, notarization-facing outputs | Packaging matters after builds are reproducible; notarization and upload add network and policy scope. | No |
| P7 | Simulator, devices, debugger, IDE | `simctl`, device install/run, LLDB integration, indexing, SourceKit, Interface Builder, Instruments, trace workflows | These surfaces are large, Swift-heavy, and often private. They are explicitly post-MVP. | No |

## Phase Rationale

### P0. Environment fidelity before everything else

This is the narrowest high-value closure step. The project already has working
`xcrun`-style behavior and strong runtime coverage. The remaining work here is
to tighten correctness around developer directory selection, SDK lookup,
toolchain lookup, and proof quality.

This phase must also introduce a hard distinction between:

- Startup success: a tool prints usage text or reaches `main()`.
- Semantic success: the workflow performs the expected job and produces the
  right output.

MVP progress should be measured against semantic success only.

### P1. Project model before build execution

The public contract most users care about is not the internal Xcode object
model. It is whether a real project, workspace, and scheme resolve the same
target graph and build settings that Xcode would resolve.

This phase should cover:

- `project.pbxproj` graph parsing
- Workspace membership resolution
- Scheme parsing for build actions
- `.xcconfig` inclusion and overriding
- Build setting expansion order and inheritance

Until this exists, any `xcodebuild` replacement is only a thin wrapper around
tool invocation.

### P2. Replace `xcodebuild`, do not emulate private IDE frameworks

The repo evidence already shows that Apple's `xcodebuild` binary falls into
exception-heavy startup behavior and deeper private framework chains.
Treating Apple's binary as the target creates the wrong optimization pressure.

The correct compatibility target is a Rust-native `xcodebuild`-compatible
driver that matches:

- CLI argument shape for a constrained subset
- build-setting reporting for chosen reference projects
- build and clean action behavior
- target and scheme selection semantics

This is the center of the MVP.

### P3. Narrow product assembly, not full Xcode products

The MVP needs to produce real outputs, but only for project types that do not
drag in the heaviest parts of Xcode.

The initial compatible product slice should be:

- macOS arm64 command-line tools
- Static libraries
- Simple `.app` bundles with `Info.plist` processing and deterministic bundle
  layout, but no storyboard, nib, or asset catalog compilation

The authoritative MVP compilation path is:

- Rust-native build orchestration over non-Apple toolchain components
- No dependency on Apple's `clang`, `ld`, `libtool`, or `xcodebuild` binaries
  for MVP builds

Apple-binary execution remains useful as a validation lane, but it is not on
the critical path for P2/P3 delivery.

Language scope should stay narrow:

- C and Objective-C first
- C++ only if it falls out naturally from the chosen compiler and linker path
- Swift explicitly deferred

### P4-P7 are real work, but not MVP work

Testing, resource compilation, signing, simulator flows, device management,
debugging, profiling, indexing, and IDE UX all matter eventually. They do not
belong in the first credible Xcode-compatible milestone because each widens the
 contract faster than the repo can verify.

## MVP Boundary

### Included in MVP

- Public-surface environment compatibility for `xcode-select` and `xcrun`
  behaviors needed by the chosen build flows
- Parsing and resolving:
  - one standalone `.xcodeproj`
  - one `.xcworkspace` containing at least one shared `.xcconfig` and one
    target-level override
- Scheme, target, configuration, and `.xcconfig` support for a constrained
  graph shape
- A Rust-native `xcodebuild`-compatible CLI with this minimum action set:
  - `-list`
  - `-showBuildSettings`
  - `build`
  - `clean`
- macOS arm64 outputs for:
  - one command-line tool target
  - one static library target
  - one simple app bundle target with `Info.plist` processing only and an
    observable launch proof for its main executable
- Build setting, product path, and bundle layout parity for the chosen
  reference projects
- Build-setting parity for a defined supported key set, at minimum:
  - `SRCROOT`
  - `PROJECT_DIR`
  - `PROJECT_NAME`
  - `TARGET_NAME`
  - `CONFIGURATION`
  - `SDKROOT`
  - `ARCHS`
  - `OBJROOT`
  - `SYMROOT`
  - `TARGET_BUILD_DIR`
  - `CONFIGURATION_BUILD_DIR`
  - `PRODUCT_NAME`
  - `FULL_PRODUCT_NAME`
  - `EXECUTABLE_PATH`
  - `INFOPLIST_FILE`
  - `PRODUCT_BUNDLE_IDENTIFIER`
  - `HEADER_SEARCH_PATHS`
  - `FRAMEWORK_SEARCH_PATHS`
  - `OTHER_CFLAGS`
  - `OTHER_LDFLAGS`
- Explicit proof that MVP flows do not depend on `DVTFoundation`,
  `IDEFoundation`, `PBX` private model classes, `CoreSimulator`, `XCTest`, or
  Instruments private surfaces, whether through Apple framework images or
  repo-local stubs

### Out of scope for MVP

- Apple's `xcodebuild`, `ibtool`, `actool`, `xctest`, `xctrace`, `notarytool`,
  and `altool` binaries as compatibility targets
- Build-time compilation of storyboards, nibs, and asset catalogs
- Swift compilation, Swift metadata compatibility, SourceKit, and Swift-only
  project graphs
- Simulator and physical-device workflows
- XCTest semantic compatibility and result bundles
- Archive/export workflows, notarization, App Store upload, and provisioning
- IDE/editor UX, indexing, code completion, Interface Builder, and Instruments
- Compatibility work that depends on growing private Xcode framework stubs just
  to make Apple binaries start

## Fastest Credible Verification Path

Use 2-3 reference projects and compare against Xcode's external behavior on the
same inputs.

Recommended reference set:

1. A macOS arm64 command-line tool project with one target and one scheme.
2. A workspace containing a static library plus consumer target, with a shared
  `.xcconfig` and at least one target-level setting override.
3. A minimal Foundation or AppKit app bundle with an `Info.plist`, but no nib,
  storyboard, or asset catalog.

For each project, prove:

1. `xcodebuild -list` reports the expected project or workspace structure.
2. `-showBuildSettings` resolves the same effective settings for the supported
   keys.
3. `build` produces the expected product in the expected layout and the output
  is usable:
  - executables launch under the runtime and produce the expected output
  - static libraries link successfully into the consumer target
  - app bundles launch their main executable under the runtime and reach a
    known observable behavior, such as deterministic log output, event-loop
    startup, or window creation
4. `clean` removes the expected derived outputs.
5. No disallowed private framework images or repo-local private-surface stubs
  are on the execution path.

Two policy gates should be added before calling the MVP complete:

1. Startup-only wins do not count. A command must perform useful work.
2. Any workflow that loads or depends on `DVTFoundation`, `IDEFoundation`,
  `PBX` private model classes, `CoreSimulator`, `XCTest`, or Instruments
  private surfaces fails the MVP gate.

## Recommended Next Design Inputs

Before implementation, two design decisions still need to be made explicitly:

1. Whether the build driver consumes `.pbxproj` directly or lowers it into a
   smaller internal manifest first.
2. Which exact supported build-setting key set is required for the first
  milestone beyond the minimum list above.

Those are design questions. They should be answered only after the scope above
is accepted.

## Related Repo Evidence

- `docs/XCODE_CLI_INVESTIGATION.md`
- `docs/XCODE_FRAMEWORK_CHAIN.md`
- `docs/roadmap.md`
- `docs/BURDETTIE_REPLACEMENT_TARGETS.md`
- `docs/APPLE_APPLICATION_TARGETING_PLAN.md`
- `README.md`