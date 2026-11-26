# Burdettie-OS

**A Linux-based OS with our own UI, developer experience and owner-controlled trust, built by reusing and improving open-source foundations.**

Burdettie-OS builds on Linux for the shipping kernel, drivers and platform services. Rust provides owned interfaces, compatibility bridges, supervision and product logic where its strengths help. Native Linux applications and selected macOS application workflows are distinct targets. We prefer maintained upstream implementations and contribute general fixes to the projects that own them.

The existing repository contains a user-space Mach-O loader/runtime, Rust shims, Objective-C and framework work, Linux and macOS platform backends, an ARM64 interpreter, source-owned fixtures, developer-tool adapters and GUI demonstrations. These are useful integration assets and regression baselines; they do not require us to maintain a separate implementation of every subsystem. Compatibility depends on the exact binary, CPU route, backend and APIs.

The product goal is a bootable Linux-based system with Burdettie's own UI and developer experience, named application workflows and owner-selectable permissive, audit and enforced policy. [ADR 0003](docs/adr/0003-linux-foundation-and-upstream-reuse.md) makes Linux the explicit product foundation. The former PureDarwin/XNU shipping decision and eventual owned Rust-kernel transition are withdrawn. Rust is an implementation choice for useful interfaces, not a requirement to replace the kernel or shared userland.

PureDarwin is a source, tooling and test partner. Reuse selected portable components and evaluate coherent upstream compatibility implementations for larger gaps. Darling is an evaluation candidate, not an adopted backend. The [research register](docs/RESEARCH_QUESTIONS.md), [upstream strategy](docs/UPSTREAM_STRATEGY.md) and [M1 Linux plan](docs/milestones/M1_PLATFORM_SECURITY.md) define the next experiments. A PureDarwin VM is optional reference infrastructure; its build/boot work is not on the shipping path.

[ADR 0004](docs/adr/0004-linux-integration-boundaries.md) now defines the integration boundaries: a complete runtime domain per application/profile, Rust owner authority outside it, thread-affine GUI ownership, separate shell clients over a reused compositor, and a reused developer workbench. The [executed research records](docs/research/README.md) explain the prototype choices and remaining gates. Their host/library/tool probes do not qualify an Apple application, release image or enforced launch.

[Quick start](#quick-start) · [Architecture](#architecture) · [Engineering highlights](#engineering-highlights) · [Validation](#validation) · [Documentation](#documentation)

## Start here

| Explore | What it demonstrates |
| --- | --- |
| **Inspect a Mach-O file** | Binary-format parsing, load commands, and structured JSON output. |
| **Run a source-built fixture** | Mapping, symbol binding, fixups, initialization, and execution through the runtime. |
| **Open the runtime shell** | Fixture discovery, compatibility reporting, and a launcher for the demo applications. |
| **Explore the desktop** | A view tree, controls, event dispatch, software rendering, and platform-specific window backends. |
| **Embed the runtime** | A C ABI with examples for C, Swift, and Python clients. |

## Engineering highlights

- **A staged loader.** Parsing, memory-mapping plans, dependency resolution, and execution live in separate crates. The runtime coordinates those stages through a typestate pipeline.
- **Runtime and ABI work.** The Objective-C implementation includes message dispatch, selectors, class registration, categories, protocols, blocks, and an ARM64 assembly trampoline. Foundation and system shims translate supported behavior into the host environment.
- **Two execution approaches.** Native execution and the ARM64 interpreter support different investigation paths. Instruction decoding, host FFI fallback, pointer authentication handling, and dynamic binding are substantial parts of the compatibility work.
- **Portable platform boundaries.** Shared runtime logic uses platform abstractions with Darwin and Linux implementations. The GUI similarly separates drawing and application behavior from window-system integration.
- **Observable behavior.** Inspection output, execution plans, host-observation comparisons, diagnostic event buffers, compatibility reports, and benchmarks make failures inspectable.
- **Experimental semantic memory.** Dedicated domain-map, memory-core, and memory-index crates explore memory as a runtime subsystem, with service and logging components around it.

For a code review, begin with [`macho-runtime`](crates/macho-runtime), [`macho-parse`](crates/macho-parse), [`objc-runtime`](crates/objc-runtime), and [`platform-core`](crates/platform-core). They provide a tour of the orchestration, binary format, runtime dispatch, and host boundaries.

## Quick start

### Native Linux development

Start with the existing native developer-tool adapters and a host C compiler:

```sh
git clone https://github.com/burdettadam/burdettie-os.git
cd burdettie-os
cargo build --locked -p burdettie-xcode-tools --bins
cargo test --locked -p burdettie-xcode-tools
```

Rust is pinned to **1.93.0**. The sample-project compile/run test returns early if no compiler is available, so record compiler availability before treating it as build evidence. The adapters are a bounded CLI/project subset, not a complete IDE. See the [Linux build guide](docs/BUILD.md) for broader runtime, Docker and GUI workflows and the [developer plan](docs/XCODE_PUBLIC_COMPATIBILITY_PLAN.md) for upstream compiler/LSP/debugger integration. Linux ELF output and Darwin Mach-O output require separate target configuration and qualification.

### Optional native macOS reference workflow

Use macOS 13 or later on Apple Silicon, Rust **1.93.0** as pinned in [`rust-toolchain.toml`](rust-toolchain.toml), and Xcode Command Line Tools. Parsing a file and executing it have different platform requirements; this walkthrough uses the native ARM64 execution path.

```bash
git clone https://github.com/burdettadam/burdettie-os.git
cd burdettie-os

# Build the Rust workspace.
cargo build --workspace --locked

# Compile one small fixture from this repository's own C source.
cc -o fixtures/simple_cli/app fixtures/simple_cli/main.c

# Inspect a fixture, then obtain machine-readable inspection output.
cargo run --locked -p macho-inspect -- fixtures/simple_cli/app
cargo run --locked -p macho-inspect -- --json fixtures/simple_cli/app

# Execute that fixture through the runtime.
cargo run --locked -p macho-inspect -- --exec fixtures/simple_cli/app
```

To build the wider fixture set on macOS, run `./scripts/build_fixtures.sh`. That script also exercises Objective-C/Foundation cases and generates malformed inputs for parser tests. Imported application bundles, extracted system libraries, and locally copied developer tools belong to separate compatibility experiments.

### Interactive showcase

```bash
cargo run --locked -p runtime-shell
```

Inside the shell:

```text
darwin> score
darwin> run hello
darwin> gui
darwin> gui hello-darwin
```

The scorecard describes the fixture set available in your checkout. The shell lists the hello-darwin, control-gallery, and foundation-explorer demos; the workspace also includes launcher and file-explorer applications. See the [demo guide](docs/DEMO.md) for setup and expected behavior.

### Docker and Linux

The repository provides Linux runtime and browser-viewer development workflows in [`docker/Dockerfile.burdettie`](docker/Dockerfile.burdettie) and [`scripts/docker-gui.sh`](scripts/docker-gui.sh). The image builds its Rust binaries and LibreSSL from source. The ARM64 lane also builds an alternate-layout libc++ from a pinned LLVM release; its [build recipe, ABI smoke test, and upstream notices](docker/apple-libcxx/README.md) are included. These are development/runtime images, not a qualified bootable Linux release.

Treat locally imported guest software as a separate development input. Imported tools are excluded from the default build context and can be mounted read-only for [local compatibility experiments](docker/xcode-tools/README.md). Source-owned Mach-O fixtures and exact application inputs have their own build/provenance requirements; guest compatibility still depends on the selected platform and interfaces.

## Architecture

The table inventories existing implementation assets. [ADR 0003](docs/adr/0003-linux-foundation-and-upstream-reuse.md) governs the product architecture: Linux underneath native applications and compatibility workers, Rust integration and owner policy, and reused desktop/developer components. An upstream worker may replace custom runtime components after qualification; the inventory is not a rewrite obligation.

| Area | Main components | Responsibility |
| --- | --- | --- |
| Binary model and planning | `macho-types`, `macho-parse`, `macho-map`, `macho-resolve`, `macho-run-plan` | Read Mach-O data and construct mapping, dependency, and execution plans. |
| Loading and execution | `macho-exec`, `macho-runtime`, `arm64-translate` | Apply fixups, bind symbols, initialize images, and coordinate native or interpreted execution. |
| Objective-C and framework compatibility | `macho-objc`, `objc-runtime`, `cf-lite`, `libsystem-shims`, `cg-shims`, `icu-shims` | Interpret metadata and implement supported runtime, Foundation, graphics, system, and internationalization behavior. |
| Host integration | `platform-core`, `platform-darwin-host`, `platform-linux` | Adapt memory, symbols, clocks, and other host services. |
| GUI and applications | `graphics-core`, `window-backend`, `app-kit-lite`, `ui-view`, `ui-controls`, `apps/` | Draw the interface, handle events, lay out views, and demonstrate application behavior. |
| Inspection and evaluation | `macho-inspect`, `macho-diff`, `runtime-diagnostics`, `compat-runner`, `test-harness`, `perf-bench` | Explain runtime decisions, compare observations, and measure supported behavior. |
| Embedding | `darwin-runtime-ffi`, `swift/`, `examples/` | Expose the runtime through a C ABI and language bindings. |
| Semantic memory and services | `burdettie-domain-map`, `burdettie-memory-core`, `burdettie-memory-index`, `burdettie-services`, `burdettie-log-pipeline` | Explore domain modeling, memory indexing, service integration, and logging. |

The [feature reference](docs/FEATURES.md) provides the detailed subsystem inventory. The [roadmap](docs/roadmap.md) records the intended direction and target matrix.

## Validation

The [CI workflow](.github/workflows/ci.yml) builds the project's own Mach-O fixtures. In [run 43343829535](https://github.com/burdettadam/burdettie-os/actions/runs/43343829535) for source commit `1bf44e2e552254c450d3a7a86cc55487e7f49b51`, native macOS ARM64 formatting, strict Clippy, debug/release builds, full workspace tests, GPU-feature tests, runtime packaging, README quick start, and C/Swift interop passed. Docker's 344 portable library tests and all 40 Linux ARM64 interpreter fixtures passed in their separate Linux environments.

That historical checkpoint also recorded an unsuppressed font-maintenance advisory and separate passing audit/hygiene checks. See the [revision-specific validation record](docs/PUBLICATION.md#validation-status), [font backend reference](docs/FONT_BACKENDS.md) and [recovery manifest](docs/RECOVERY_MANIFEST.json) for their exact sources and scope. Historical CI does not qualify the current Linux image, a newly selected compatibility backend or enforced policy. The strategy update adds no new runtime passing-result claim.

## Scope and limitations

- **Compatibility is incremental.** Framework coverage includes implemented behavior, partial shims, and stubs. A successful command-line smoke test does not establish complete application compatibility.
- **Execution is platform-sensitive.** CPU architecture, ABI details, native libraries, display availability, and the selected execution path affect what can run. Use the documented platform-specific workflows.
- **Guest execution uses the host process.** Treat binaries as executable code with the access available to that process. The runtime's trust-policy models and diagnostics are research facilities, not an isolation boundary.
- **Results need context.** Fixture totals, corpus pass rates, and performance measurements vary with the revision, test inputs, platform, and build profile. Consult the scripts and reports for the inputs and conditions behind each result.

## Development and contribution

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community expectations. Keep changes focused, add a small reproducible fixture when extending compatibility, and document unsupported behavior as carefully as supported behavior.

Source files, fixture generators, and documented expectations belong in version control. Build outputs, local observations, private memory state, credentials, imported application bundles, and extracted third-party binaries should remain outside the public repository. Existing committed copies require a separate history review; adding an ignore rule does not remove them. See the [source and artifact policy](docs/PUBLICATION.md) for the publication boundary.

## Documentation

| Guide | Contents |
| --- | --- |
| [Linux product decision](docs/adr/0003-linux-foundation-and-upstream-reuse.md) | Accepted Linux foundation, Rust interfaces and upstream reuse; supersedes the XNU/Rust-kernel transition. |
| [Integration boundaries](docs/adr/0004-linux-integration-boundaries.md) | Per-app runtime authority, GUI ownership, reused desktop/workbench and explicit prototype order. |
| [Executed research](docs/research/README.md) | Pinned sources, probe outcomes, contribution candidates and current architecture diagram. |
| [M1 platform and security contracts](docs/milestones/M1_PLATFORM_SECURITY.md) | Linux baseline, target/backend selection, shared launch contract and alpha acceptance. |
| [Research questions](docs/RESEARCH_QUESTIONS.md) | Sourced findings, remaining decisions, bounded experiments and exit criteria. |
| [Upstream strategy](docs/UPSTREAM_STRATEGY.md) | Reuse/contribution routing, first candidates and dependency/patch records. |
| [Demo guide](docs/DEMO.md) | Showcase workflows and packaging. |
| [Feature reference](docs/FEATURES.md) | Detailed runtime and tooling capabilities. |
| [Developer-tool plan](docs/XCODE_PUBLIC_COMPATIBILITY_PLAN.md) | Our developer experience over upstream build, language-service and debugger tools. |
| [Roadmap](docs/roadmap.md) | Linux product milestones, integration experiments and historical-reference boundaries. |
| [FFI guide](docs/FFI_GUIDE.md) | Embedding, API examples, memory ownership, and thread-safety notes. |
| [Technical manuscript](book/README.md) | Draft chapters explaining the loader, runtime, interpreter, and GUI. |
| [Changelog](CHANGELOG.md) | Recorded project changes. |

## License

Project-owned source is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. Preserve any separate license terms and notices supplied with third-party components. These licenses do not grant rights to redistribute locally imported Apple software or other external test inputs.

Apple, macOS, Darwin, Objective-C, and other product names identify the interfaces and formats studied by this independent project. Burdettie-OS is not affiliated with Apple.

