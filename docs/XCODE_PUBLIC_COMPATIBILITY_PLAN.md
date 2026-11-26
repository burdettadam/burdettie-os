# Burdettie Developer Tools and Xcode Interoperability

## Goal and decision

Build a useful Xcode-like developer experience on Linux by integrating existing
workbench, compilers, build systems, language servers, and debuggers. Burdettie owns the
project experience, Rust interfaces and bridges, target selection, and trust
policy. Reimplementing those upstream tools in Rust is not a delivery goal.

Linux is the shipping foundation, as recorded in
[ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md). Native Linux programs
and libraries are the first developer targets. Public Xcode file formats and
command-line behavior remain interoperability surfaces for existing projects;
running Apple's Xcode IDE is a separate application-compatibility problem.

This document replaces the earlier plan that made a new Rust-native
`xcodebuild` stack and macOS ARM64 products the central milestone. Keep useful
existing code and tests while qualifying reusable backends. Follow the
[upstream strategy](UPSTREAM_STRATEGY.md) for component selection and the
[research register](RESEARCH_QUESTIONS.md) for unresolved decisions. The
integration milestones below are planned work, not claims of implemented IDE,
language-server, debugger, or cross-compilation support.

[ADR 0004](adr/0004-linux-integration-boundaries.md) selects workbench reuse
and an explicit trusted developer boundary. Evaluate Eclipse Theia with a
small Burdettie extension and Rust CLI/service; its implementation remains
unqualified. The [desktop/developer research](research/2026-11-25-linux-desktop-devtools.md)
records source findings, contribution constraints and the native C probe.

## Two explicit target profiles

| Profile | Build output and execution | Required evidence |
|---|---|---|
| Native Linux, first priority | ELF binaries and shared objects for a declared Linux architecture; execute directly on Linux | Compiler and linker identity, target triple, dependencies, useful program output, and relevant build/test/debug behavior |
| Darwin application compatibility | Mach-O binaries for a declared architecture and Darwin SDK/ABI; execute through a separately qualified compatibility backend or reference guest | SDK provenance and contents, compiler/linker target, object format, runtime dependency closure, and semantic execution results |

A `.pbxproj` input, an `SDKROOT` string, or an `xcrun`-shaped command does not
select an object format by itself. Building host-native C on Linux does not
qualify a Darwin cross-compiler. A PureDarwin guest result does not qualify
execution on Linux. Report the build host, target, and execution backend
separately in every compatibility result.

PureDarwin can supply reusable source, toolchain work, build knowledge, and a
Darwin reference environment. Its XNU kernel is not a Linux runtime layer.
Evaluate each library's dependencies and ABI before adopting it into the Linux
profile. Shared tool/SDK discovery fixes may be upstream candidates without
making a PureDarwin boot a prerequisite for ordinary Burdettie development.

## Reuse the developer stack

The following are integration choices to qualify, rather than requirements to
write equivalent tools:

| Capability | Reuse first | Burdettie responsibility |
|---|---|---|
| Editor, terminal, project tree and extension host | Eclipse Theia as the first workbench candidate; standard extensions where compatible | Own project/target experience through a narrow extension and Rust CLI/service, without a Rust-only frontend requirement |
| Rust build, test, and dependency resolution | Cargo and rustc | Project actions, target selection, diagnostics, and policy around tool execution |
| C/C++ compilation and linking | Clang/LLVM and an appropriate existing linker; retain a project's existing build system where possible | Translate supported project inputs and expose explicit compiler, sysroot, and target configuration |
| Swift source development on Linux | A supported Swift toolchain and Swift Package Manager | A bounded Linux package workflow and clear supported-platform reporting |
| Code navigation and completion | rust-analyzer, clangd, and SourceKit-LSP through the Language Server Protocol (LSP) | Editor integration, project configuration, and diagnostic presentation |
| Debugging | LLDB through its Debug Adapter Protocol adapter, `lldb-dap` | Launch/attach configuration, target-specific permissions, and the debugging interface |
| Xcode project interoperability | Existing Burdettie `.pbxproj` parser and command adapters, plus reusable format libraries where suitable | The documented subset, unsupported-input errors, and translation into the chosen build backend |

Cargo exposes machine-readable compiler messages for external tools. clangd
uses compilation commands to understand a C/C++ project. SourceKit-LSP
integrates with SwiftPM and compilation databases; use it with a compatible
Swift toolchain. These interfaces give Burdettie a smaller integration surface
than implementing compilers, indexing engines, or a debugger.

Swift on Linux is a source-development option. It does not establish support
for Apple's AppKit, UIKit, SwiftUI, Objective-C binary interfaces, or existing
Apple Swift application binaries. Those require their own compatibility
evidence. Native Linux debugging and debugging a Mach-O compatibility process
also need separate qualification.

Primary references:

- [Cargo: integration with external tools](https://doc.rust-lang.org/cargo/reference/external-tools.html)
- [clangd: compile commands](https://clangd.llvm.org/design/compile-commands)
- [Clang: cross-compilation](https://clang.llvm.org/docs/CrossCompilation.html)
- [rust-analyzer](https://github.com/rust-lang/rust-analyzer)
- [Swift platform support](https://www.swift.org/platform-support/) and [Swift overview](https://www.swift.org/about/)
- [SourceKit-LSP](https://github.com/swiftlang/sourcekit-lsp)
- [LLDB DAP](https://lldb.llvm.org/use/lldbdap.html)

## Existing adapters and their actual scope

The current implementation lives in
[`burdettie-xcode-tools`](../crates/burdettie-xcode-tools/Cargo.toml), with tool
discovery in [`lib.rs`](../crates/burdettie-xcode-tools/src/lib.rs) and project
parsing/build orchestration in
[`xcodebuild.rs`](../crates/burdettie-xcode-tools/src/xcodebuild.rs).

- The crate provides `xcode-select`, `xcrun`, and `xcodebuild` binaries.
- The project adapter parses a standalone `.xcodeproj/project.pbxproj`,
  resolves a bounded set of target/configuration settings, and implements
  `-list`, `-showBuildSettings`, `build`, and `clean` actions.
- Build planning currently accepts `com.apple.product-type.tool` only. Static
  libraries, application bundles, workspaces, schemes, and complete `.xcconfig`
  semantics must not be reported as delivered by this adapter.
- Compiler selection uses `BURDETTIE_XCODEBUILD_CC`, then `CC`, then a host
  `cc` or `clang` found outside the imported-tool overlay. The compiler receives
  source files, include paths, supported extra flags, and an output path. The
  adapter does not itself enforce a Darwin target triple, SDK, or deployment
  target.
- The `build_plan_compiles_and_runs_sample_project` regression compiles a
  source-owned C project and expects `hello xcodebuild 41`. It returns early
  when no compiler is available; a green suite alone therefore does not prove
  that compilation occurred. Record compiler availability and execution when
  using it as milestone evidence.

Retain these adapters as migration assets. Improve their errors, discovery
contract, supported-setting reports, and backend integration as needed. A new
build graph engine, compiler driver, or project format implementation needs an
explicit reuse evaluation before expansion. No complete IDE frontend is
established by these command-line implementations.

## Delivery sequence and acceptance gates

### Developer authority

Use a separately declared trusted developer route for the initial IDE
experiment. Workspace build scripts, extensions, terminals and debug adapters
can execute code; workbench workspace-trust settings do not implement Linux
confinement. Debugging and JIT-capable workbench processes must not silently
weaken the existing application contract's deny-only inspection or generated-
executable-memory controls. A confined developer route would need its own
reviewed capability contract and negative tests. See
[ADR 0004](adr/0004-linux-integration-boundaries.md).

### 1. Qualify a native Linux development loop

Use the Linux x86_64 evaluation profile selected in ADR 0003 and record its
distribution and toolchain versions. Build, test, run, and clean one Rust CLI
through Cargo and one C CLI through Clang or the selected host compiler.
Inspect the produced ELF target and compare expected stdout, exit status, and
build artifacts. Verify that a source edit causes the expected rebuild.

Reuse Cargo's and the selected C build system's own dependency/incremental
build behavior. The existing `.pbxproj` C fixture can exercise the adapter
path separately. Do not require a Darwin SDK or imported Apple tools for this
native milestone.

The research's native GCC C source-control probe passed build/run, edit/rebuild,
ELF target inspection and a structured invalid-source diagnostic. It did not
run Cargo, the project importer, an IDE, LSP or DAP; those acceptance steps
remain open.

### 2. Qualify navigation and debugging

Evaluate the existing Theia workbench and extension interfaces first.
Use a small project to prove diagnostics, completion, and go-to-definition
through the selected LSP server. Use `lldb-dap` to prove launch, breakpoint,
step, variable inspection, and clean termination for a native Linux program.
Record any ptrace/container restrictions required by the test environment.

Keep Burdettie project import, target selection and build/run results available
through a narrow CLI/service and suitable extension APIs. Theia's exact API
coverage, required extensions and distribution cost need qualification before
adoption. Small upstream-native frontend extensions are consistent with using
Rust for the interfaces and bridges.

### 3. Extend project import only for a named workflow

Start with the current standalone C command-line project. State every
supported product type and setting; reject or report unsupported inputs rather
than silently claiming Xcode equivalence. Decide whether the parser lowers to
a smaller internal manifest or directly configures an existing build backend.

Add a workspace, shared `.xcconfig`, library dependency, or resource step only
when a reference project needs it. Compare relevant public behavior with
Xcode on the same inputs when Darwin parity is claimed. Linux product layout
and Darwin bundle layout have separate expectations.

### 4. Evaluate a bounded Swift Linux package

Use upstream SwiftPM, SourceKit-LSP, and the matching toolchain on a supported
Linux platform. Prove build, test, diagnostics, and execution for one
source-owned package with Linux-supported dependencies. This experiment does
not promise imported Swift application or Apple framework compatibility.

### 5. Qualify a separate Mach-O build and execution workflow

Select one architecture, compiler/linker combination, redistributable sysroot,
and execution backend. PureDarwin toolchain recipes and reusable sources are
candidates under the upstream strategy. First build a minimal C Mach-O
program, inspect its imports, and prove useful execution. Expand to an
Objective-C or framework-using fixture only after its dependencies are
explicitly supported.

Keep this profile optional for native Linux development. An SDK discovery
success, a usage banner, or an executable reaching `main()` is insufficient
evidence of build, application, or debugger compatibility.

## Historical Xcode contract retained for reference

The earlier plan identified useful public interfaces: `xcode-select`, `xcrun`,
`xcodebuild`, `.xcodeproj`, `.pbxproj`, `.xcworkspace`, `.xcscheme`, `.xcconfig`,
`.plist`, bundle layout, and observable build settings. Their relevance now
depends on a chosen user workflow. They are not all requirements for the first
Linux developer release.

The previous reference product set was a macOS ARM64 CLI, a static library and
consumer, and a simple `.app` with `Info.plist` but no compiled interface or
asset resources. Preserve those as possible Darwin validation fixtures. The
build-setting comparison inventory was:

```text
SRCROOT PROJECT_DIR PROJECT_NAME TARGET_NAME CONFIGURATION SDKROOT ARCHS
OBJROOT SYMROOT TARGET_BUILD_DIR CONFIGURATION_BUILD_DIR PRODUCT_NAME
FULL_PRODUCT_NAME EXECUTABLE_PATH INFOPLIST_FILE PRODUCT_BUNDLE_IDENTIFIER
HEADER_SEARCH_PATHS FRAMEWORK_SEARCH_PATHS OTHER_CFLAGS OTHER_LDFLAGS
```

This is an inventory to scope, not a claim that all settings already have
correct semantics. Public compatibility work should still check supported
setting expansion, product paths, useful execution or linking, and clean
behavior against source-owned reference projects.

Earlier Xcode CLI startup counts in
[XCODE_CLI_INVESTIGATION.md](XCODE_CLI_INVESTIGATION.md) and private-framework
dependency findings in [XCODE_FRAMEWORK_CHAIN.md](XCODE_FRAMEWORK_CHAIN.md)
remain historical evidence. They are not release KPIs or proof of a working
IDE. Startup stubs and version/usage probes do not qualify semantic behavior.

## Scope boundaries

The native developer milestone does not require a new compiler, linker,
debugger, LSP server, general build system, or broad Rust reimplementation of
Apple frameworks. Resource compilers, Interface Builder parity, simulator and
device workflows, XCTest/result-bundle parity, archive/export, provisioning,
notarization, and App Store upload require separately justified work.

Apple-private `DVTFoundation`, `IDEFoundation`, PBX model classes,
`CoreSimulator`, Instruments internals, and Xcode-private Swift modules are not
dependencies of the native developer product. If locally available, imported
tools can inform reference comparisons under the
[source and artifact policy](PUBLICATION.md).

Burdettie's owner-controlled trust model governs its own tools and packages.
It does not imply that local signatures will be accepted by Apple distribution
services. Keep native package policy, compatibility metadata, and any external
distribution requirements explicit.

## Decisions still open

- Linux distribution, toolchain packaging and supported version combinations,
  plus qualification of shipping architectures beyond the x86_64 evaluation
  profile.
- Theia versus another maintained workbench, its extension API/version fit,
  and which LSP/DAP capabilities the first release must expose.
- Build backend and `.pbxproj` lowering strategy; the exact supported project,
  product, and setting subset.
- Which Swift Linux package and dependencies provide a useful first target.
- Darwin compiler/sysroot/runtime combination and provenance, plus whether
  debugger support can cross that compatibility boundary.
- Which fixes belong in upstream tool, runtime, or PureDarwin repositories and
  which interfaces are specific to Burdettie's product and trust model.

Resolve these through the small experiments in
[RESEARCH_QUESTIONS.md](RESEARCH_QUESTIONS.md), and record decisions before
turning them into implementation requirements.
