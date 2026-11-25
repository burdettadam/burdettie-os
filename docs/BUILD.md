# Build Guide

## Linux development is the primary workflow

Linux is Burdettie's shipping foundation under
[ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md). Build native Linux
ELF tools first; treat Darwin Mach-O inputs, cross-compilers, and reference
guests as separately qualified compatibility workflows. The
[developer-tool plan](XCODE_PUBLIC_COMPATIBILITY_PLAN.md) describes reuse of
Cargo/rustc, Clang, SwiftPM, language servers, and LLDB-DAP. The
[upstream strategy](UPSTREAM_STRATEGY.md) and
[research register](RESEARCH_QUESTIONS.md) track integration and packaging work
that is still needed.

This guide describes existing commands and their platform requirements. The
strategy change does not make every workspace member, fixture, or packaging
script platform-independent. The Docker images below build development and
runtime artifacts; they are not bootable Linux distribution images. The first
evaluation profile is Linux x86_64; existing ARM64 recipes remain useful for
their declared tests and do not establish another shipping architecture.

## Prerequisites

- **Linux** with a declared host architecture, or Docker capable of running the
  selected Linux image architecture.
- **Rust 1.93.0**, pinned in
  [`rust-toolchain.toml`](../rust-toolchain.toml), for builds outside Docker.
- **A native C toolchain** (`cc` or Clang and the corresponding linker/runtime)
  for native compilation and the `xcodebuild` adapter's compile/run regression.
- **Docker**, when using the reproducible container recipes below.

Linux GUI builds also need native development packages. The current Debian
Bookworm runtime build recipe in
[`Dockerfile.burdettie`](../docker/Dockerfile.burdettie) installs `pkg-config`,
`libfreetype-dev`, `libfontconfig1-dev`, `libwayland-dev`, `libxkbcommon-dev`,
`libx11-dev`, `libx11-xcb-dev`, `libxcb1-dev`, `libxrandr-dev`, `libxi-dev`,
`libxcursor-dev`, `libxinerama-dev`, `libxcb-shape0-dev`, and
`libxcb-xfixes0-dev`. Package names can differ on other distributions. See
[FONT_BACKENDS.md](FONT_BACKENDS.md) for the font integration and its coverage.

The current Mach-O fixture generator requires macOS and Xcode Command Line
Tools; native ARM64 Darwin execution uses the Apple Silicon reference
environment. Ordinary Linux tool development does not require that reference
environment. SwiftPM/LSP/DAP integrations in the developer plan are future
qualification work; installing them is not a prerequisite for building the
existing Rust command adapters.

## Quick start on Linux

```sh
git clone https://github.com/burdettadam/burdettie-os.git
cd burdettie-os

# Build the existing native command adapters.
cargo build --locked -p burdettie-xcode-tools --bins

# Test discovery, project parsing, settings, and the bounded host-compiler path.
cargo test --locked -p burdettie-xcode-tools

cargo fmt --all -- --check
```

These package commands do not require a Darwin SDK. The sample-project
compile/run test returns early if no compiler is available, so verify the host
compiler before treating a passing suite as evidence of a built executable.
The current adapter accepts command-line-tool products only and does not
implicitly choose a Darwin target or sysroot. On Linux, a native compiler's
output is a Linux target, even when the input project uses `.pbxproj`.

For the existing broader Linux workspace build, use the package exclusions
from [`Dockerfile.linux-e2e`](../docker/Dockerfile.linux-e2e), with the required
native dependencies installed:

```sh
cargo build --locked --workspace --exclude hello-darwin \
    --exclude control-gallery --exclude foundation-explorer \
    --exclude window-backend-darwin --exclude platform-darwin-host
```

A full `cargo test --workspace` is not the default Linux recipe: some members
are platform-specific, and some tests consume separately generated Mach-O
fixtures. Select the relevant test scope below and report missing inputs or
skipped checks.

## Linux Docker workflows

### Runtime and GUI image

The main image builds native Linux binaries and the current compatibility
runtime. It uses the Docker host's platform by default. Its normal smoke path
can run without imported Apple artifacts; discovery smoke tests do not
establish a complete SDK or application-build workflow.

```sh
docker build -f docker/Dockerfile.burdettie -t burdettie-os .
docker run --rm burdettie-os

# Interactive investigation of the same image.
docker run --rm -it burdettie-os bash

# Existing GUI session helper, using the image built above.
./scripts/docker-gui.sh
```

For an ARM64 image, choose that platform explicitly:

```sh
docker build --platform linux/arm64 -f docker/Dockerfile.burdettie -t burdettie-os .
docker run --rm --platform linux/arm64 burdettie-os
```

A different host CPU requires suitable container emulation. For GUI platform
selection, imported-artifact mounts, and the current environment variables,
see [DOCKER_FIRST_DEVELOPMENT.md](DOCKER_FIRST_DEVELOPMENT.md) and
[`docker-gui.sh`](../scripts/docker-gui.sh). The runtime image currently installs
`xcrun` and `xcode-select`; use the direct Cargo build above for the
`xcodebuild` adapter.

### Portable crate build and lint image

The dev image builds the parsing/analysis/trust-policy subset and runs focused
Linux pthread-shim regressions. Its default command runs Clippy with the
explicit allowances in the Dockerfile. This subset is useful for fast changes;
it is not the limit of runtime support on Linux.

```sh
docker build -f docker/Dockerfile.dev -t burdettie-os-dev .
docker run --rm burdettie-os-dev

# The current CI unit-test subset.
docker run --rm burdettie-os-dev cargo test --locked \
    -p platform-core -p macho-types -p macho-parse \
    -p macho-map -p macho-objc -p macho-resolve \
    -p macho-run-plan -p macho-diff -p trust-policy \
    -p macho-fixtures -p test-harness \
    --lib
```

CI restores the generated fixture archive before building this image. Tests
that read those Mach-O inputs still need them; a source-only checkout does not
contain the complete native fixture set.

### Linux ARM64 interpreted runtime tests

This image builds the portable workspace and runs
`macho-runtime`'s `interpreted_objc_tests`. Its default command selects the
interpreter, even on an ARM64 host. Provide the generated Mach-O fixtures in
the build context first, as the current CI fixture-download step does.

```sh
docker build --platform linux/arm64 -f docker/Dockerfile.linux-e2e -t burdettie-linux-e2e .
docker run --rm --platform linux/arm64 burdettie-linux-e2e
```

Execution results here qualify the tested interpreted paths. They do not
establish native Linux execution of all Darwin binaries or complete GUI/Apple
application compatibility.

### Existing ARM64 cross-build helper

[`linux-test.sh`](../scripts/linux-test.sh) is a narrower compatibility helper.
It cross-builds `macho-inspect` with `cargo-zigbuild` for
`aarch64-unknown-linux-gnu`/glibc 2.36 and mounts the binary into an existing
ARM64 `burdettie-os` image. It requires Zig, `cargo-zigbuild`, the Rust target,
and the matching image; its ordinary test mode expects the imported-tool
setup. It is not the general Linux-native development entry point.

```sh
./scripts/linux-test.sh
./scripts/linux-test.sh --shell
```

### ARM64 interpretation on an x86_64 host

The interpreter has an explicit typed adapter for the built-in Foundation
`rangeOfCharacterFromSet:` IMP. It calls the known three-argument shim with
its declared `NSRange` return type and writes both `location` and `length`
to the guest's X0/X1 registers. Normal, `objc_msgSendSuper`, and
`objc_msgSendSuper2` dispatch use that adapter only after excluding guest
code. A matching selector or guest-supplied type encoding does not authorize
casting another IMP to this signature.

The [adapter](../crates/cf-lite/src/foundation/interpreter_abi.rs) fixes this
specific two-word return through the
[runtime bridge](../crates/macho-runtime/src/interp_bridge.rs). The generic
x86_64 host-call fallback still captures only an integer result and ignores
the guest's variadic stack, FP arguments and indirect-result register. Other
struct-return methods and arbitrary host functions remain outside this
adapter's scope. A separate [bounded printf adapter](INTERPRETED_PRINTF.md)
handles literal bytes, `%%`, `%d`, `%lu` and `%s` on Unix x86_64 hosts, with
strict stack reads and explicit immutable string exports. Other printf formats,
variadic functions and `NSLog` remain unsupported by that adapter. Passing an
exit-code assertion alone does not qualify formatted output. Remaining ABI gaps are tracked in
[issue #9](https://github.com/burdettadam/burdettie-os/issues/9).

The [source-owned regression](../crates/macho-runtime/src/interp_bridge_nsrange_tests.rs)
runs real ARM64 BLR/RET instructions through Objective-C lookup and interpreter
return-register handling. Its child cases cover a matching range, no match,
an empty string, a different host IMP with the same selector, and a guest IMP
for all three dispatch routes. It needs no SDK or external fixture:

```sh
cargo test --locked -p macho-runtime --lib interp_bridge::nsrange_tests -- --nocapture
```

The existing `interp_nscharacterset_basics` fixture remains a separate
end-to-end check and requires its compiled Mach-O input. On Unix x86_64 hosts
it checks both the semantic exit and its 415-byte stdout golden. These host
adapters do not qualify native execution in a Darwin reference guest, general
variadic marshalling, worker isolation or security enforcement.

## Project structure

See [Cargo.toml](../Cargo.toml) for current members and
[CRATE_MAP.md](CRATE_MAP.md) for architectural context. Existing compatibility
crates remain useful while upstream alternatives and migration boundaries are
qualified.

| Layer | Key crates | Platform scope |
|---|---|---|
| Types and parsing | `macho-types`, `macho-parse`, `macho-shared-cache` | Portable parsing |
| Analysis | `macho-map`, `macho-objc`, `macho-resolve`, `macho-run-plan`, `macho-exec` | Portable analysis |
| Platform abstraction | `platform-core`, `platform-linux`, `platform-darwin-host` | Shared interfaces with OS-specific backends |
| Compatibility runtime | `macho-runtime`, `objc-runtime`, `cf-lite`, `libsystem-shims`, `icu-shims` | Linux and Darwin paths; support depends on architecture, execution mode, and API |
| Translation | `arm64-translate` | ARM64 interpretation/translation components |
| GUI | `app-kit-lite`, `ui-view`, `ui-controls`, `graphics-core`, `window-backend-*` | Shared components and platform-specific backends |
| Tooling | `burdettie-xcode-tools`, `macho-inspect`, `trust-policy`, `compat-runner`, `test-harness` | Native tools with separately scoped compatibility workflows |

## Darwin reference builds and fixture generation

The existing macOS reference workflow uses Apple Silicon, macOS 13 or later,
and Xcode Command Line Tools (`xcode-select --install`). It remains useful for
native Darwin behavior, reference outputs, and fixtures. It is not the
shipping foundation.

```sh
cargo build --workspace --locked
./scripts/build_fixtures.sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The strict Clippy command may expose existing warnings. The
[CI workflow](../.github/workflows/ci.yml) records its explicit allowances.
Report exact commands and outcomes instead of assuming a local invocation
matches CI.

The older convenience pipeline remains available on its reference platform:

```sh
./packaging/build.sh
./packaging/build.sh --release
```

It runs formatting, Clippy, workspace build, fixture generation, and tests.
The existing `--skip-tests` flag skips the main test step, but the script still
builds fixtures and invokes `cargo test` again while collecting its summary.
Use the direct Cargo build command when only a build is intended.

### Fixture inputs

Fixture sources (`.c`, `.m`, and assembly) live in `fixtures/`. The current
[`build_fixtures.sh`](../scripts/build_fixtures.sh) explicitly requires macOS and
uses Xcode's Clang/SDK to produce the Mach-O reference set. Generated outputs
stay local; small malformed parser inputs remain in version control. CI builds
an archive and restores it in the jobs that consume these inputs.

Source-owned tests that generate their inputs in Rust, including the bounded
ABI regression above, can avoid that SDK dependency. A Linux-hosted,
redistributable cross-build path for additional Mach-O fixtures is a research
and upstream-reuse task, not something enabled by changing this guide.

## Existing bundle and demo recipes

```sh
# Bundle already-built host binaries and available fixtures into dist/.
./packaging/bundle-runtime.sh
./packaging/bundle-runtime.sh --release

# Package that directory: DMG with hdiutil, otherwise a tar.gz archive.
./packaging/make-vm-image.sh
```

The bundle contains `bin/`, available `fixtures/`, and `docs/`. These scripts
collect existing artifacts; they do not cross-compile them or create a bootable
OS image. Their output inherits the binaries' architecture and operating
system requirements.

[`Dockerfile.demo`](../docker/Dockerfile.demo) is an older bundle-inspection
recipe. It copies `dist/` into Debian without recompiling the binaries. A
macOS-built bundle can be inspected there, but its Mach-O executables do not
become native Linux programs by being copied into the image. Use the Linux
runtime Dockerfile above for the current executable Linux environment.

```sh
# After preparing dist/ with the appropriate reference bundle:
docker build -f docker/Dockerfile.demo -t burdettie-os-demo dist/
docker run --rm burdettie-os-demo
docker run --rm burdettie-os-demo ls /opt/burdettie-os/fixtures/
```

## CI and reporting results

The current [CI workflow](../.github/workflows/ci.yml) includes macOS fixture,
quickstart, workspace, and C/Swift FFI jobs; a Linux Docker subset job; and a
Linux ARM64 interpreted-runtime job. Fixture consumers restore generated
inputs. Security/supply-chain checks are also defined there. Read the workflow
for exact commands and platform-specific exclusions; this guide is not a claim
that a particular revision has passed those jobs.

Record host OS/architecture, target format and architecture, compiler and SDK
where relevant, runtime backend, input provenance, and observed semantic
results. Keep startup probes, skipped tests, and native/interpreted execution
separate in result summaries.

## Troubleshooting

**Linux workspace failures:** Start with the affected packages or the relevant
Docker recipe. The broad Linux recipe excludes Darwin-specific members; GUI
packages need native libraries. Do not treat an unqualified all-workspace
command as a Linux platform gate.

**Missing Mach-O fixtures:** The current generator requires macOS/Xcode CLT.
Use the project's generated reference inputs when available, or choose a
source-owned test that does not need them. Missing inputs are not passing
compatibility results.

**Architecture mismatches:** Distinguish the Linux host, the target binary,
and the execution mode. The interpreter can exercise selected ARM64 inputs on
other host architectures; native execution and Docker emulation have different
requirements. Do not infer Apple Silicon requirements for all Linux tests.

**A build works but produces the wrong target:** Inspect compiler flags,
object format, target triple, and sysroot. The existing project adapter does
not make host `cc` a Darwin cross-compiler merely by reporting `SDKROOT`.
