# Build Guide

## Prerequisites

- **macOS** (ARM64 / Apple Silicon) — required for full integration tests
- **Rust 1.93.0** — pinned in [`rust-toolchain.toml`](../rust-toolchain.toml)
- **Xcode Command Line Tools** — `xcode-select --install`
- **Docker** (optional) — for containerized builds of cross-platform crates

## Quick Start

```bash
# Clone and build
git clone https://github.com/burdettadam/burdettie-os.git
cd burdettie-os

# Full pipeline: format → clippy → build → fixtures → test
./packaging/build.sh

# Release build
./packaging/build.sh --release

# Build only (skip tests)
./packaging/build.sh --skip-tests
```

## Project Structure

The workspace is organized into layers. See [Cargo.toml](../Cargo.toml) for the
current members and [CRATE_MAP.md](CRATE_MAP.md) for architectural context.

| Layer | Key Crates | Platform |
|-------|--------|----------|
| **Types & Parsing** | `macho-types`, `macho-parse`, `macho-shared-cache` | Cross-platform |
| **Analysis** | `macho-map`, `macho-objc`, `macho-resolve`, `macho-run-plan`, `macho-exec` | Cross-platform |
| **Platform Abstraction** | `platform-core`, `platform-darwin-host`, `platform-linux` | Core: cross-platform; platform impls: OS-specific |
| **Runtime** | `macho-runtime`, `objc-runtime`, `cf-lite`, `libsystem-shims`, `icu-shims` | macOS primary; Linux via PAL |
| **Translation** | `arm64-translate` | Cross-platform |
| **GUI** | `app-kit-lite`, `ui-view`, `ui-controls`, `graphics-core`, `window-backend-*` | Cross-platform |
| **Tooling** | `macho-inspect`, `trust-policy`, `swift-stub-gen`, `compat-runner`, `test-harness` | Mixed |

## Build Commands

### Individual Steps

```bash
# Format check
cargo fmt --all -- --check

# Clippy (zero warnings policy)
cargo clippy --workspace --all-targets -- -D warnings

# Build all crates
cargo build --workspace --locked

# Build release
cargo build --workspace --release --locked

# Build test fixtures (C/ObjC sources → binaries) before running tests
./scripts/build_fixtures.sh

# Run all tests
cargo test --workspace --locked
```

### Packaging

```bash
# Bundle runtime + fixtures into dist/
./packaging/bundle-runtime.sh
./packaging/bundle-runtime.sh --release  # use release binaries

# Create distributable disk image
./packaging/make-vm-image.sh
```

The bundle layout:

```
dist/
├── bin/
│   └── macho-inspect
├── fixtures/
│   ├── simple_cli/app
│   ├── return42/app
│   └── ... (locally built fixture binaries)
└── docs/
    ├── README.md
    ├── architecture.md
    └── ...
```

## Docker

### Dev Image (build + lint)

The dev image compiles and checks the cross-platform crate subset on Linux.
macOS-only crates (runtime, ObjC bridge, system shims) are excluded since they
require ARM64 macOS.

```bash
# Build the dev image
docker build -f docker/Dockerfile.dev -t burdettie-os-dev .

# Run clippy in container
docker run --rm burdettie-os-dev

# Run unit tests in container
docker run --rm burdettie-os-dev cargo test \
    -p platform-core -p macho-types -p macho-parse \
    -p macho-map -p macho-objc -p macho-resolve \
    -p macho-run-plan -p macho-diff -p trust-policy \
    -p macho-fixtures -p test-harness \
    --lib
```

### Linux Runtime Images

Use the Linux runtime images when development must continue without a live Mac
tooling bridge.

```bash
# Full Linux ARM64 runtime build and interpreted runtime tests
docker build --platform linux/arm64 -f docker/Dockerfile.linux-e2e -t burdettie-linux-e2e .
docker run --rm --platform linux/arm64 burdettie-linux-e2e

# Runtime image with imported Apple tool support (host-native by default)
docker build -f docker/Dockerfile.burdettie -t burdettie-os .

# Force the ARM64 reference lane explicitly when needed
docker build --platform linux/arm64 -f docker/Dockerfile.burdettie -t burdettie-os .

# Fast runtime iteration loop
./scripts/linux-test.sh
./scripts/linux-test.sh --shell
```

For Docker-first runtime and imported-artifact workflows, see
`docs/DOCKER_FIRST_DEVELOPMENT.md`.

### Demo Image (runtime + fixtures)

The demo image packages pre-built binaries for inspection and exploration.
Build on macOS first, then create the image from the bundle.

```bash
# 1. Build and bundle on macOS
./packaging/build.sh --release
./packaging/bundle-runtime.sh --release

# 2. Create demo image
docker build -f docker/Dockerfile.demo -t burdettie-os-demo dist/

# 3. Explore
docker run --rm burdettie-os-demo
docker run --rm burdettie-os-demo macho-inspect fixtures/simple_cli/app
```

## CI Pipeline

GitHub Actions runs on every push/PR to `main`:

1. **Check formatting** — `cargo fmt --check`
2. **Clippy** — zero-warning policy with `-D warnings`
3. **Build** — debug and release profiles
4. **Fixture rebuild** — compiles C/ObjC test fixtures from source
5. **Test** — full workspace test suite with the generated fixtures available
6. **Bundle** — creates distributable package
7. **Artifact upload** — 14-day retention on bundle

A separate Docker job validates that the cross-platform crates build on Linux.

## Test Fixtures

Fixture sources (`.c`, `.m`, and assembly) live in `fixtures/`. Compile the
Mach-O executables and libraries locally on macOS; generated outputs are not
part of the public source checkout. Small malformed inputs used by parser tests
are retained. To build the native fixture set:

```bash
./scripts/build_fixtures.sh
```

Fixtures cover: basic executables, dylib chains, fat binaries, ObjC classes,
Foundation framework usage, weak linking, TLV, constructors, and more.

## Troubleshooting

**Build fails on Linux**: Use the Docker dev image or the documented per-crate
workflow. Runtime and GUI builds have additional native dependencies, and native
macOS fixture compilation requires Xcode Command Line Tools.

**Missing fixtures**: Run `./scripts/build_fixtures.sh`. Requires Xcode CLT.

**Test failures on non-ARM64**: Integration tests execute ARM64 Mach-O binaries
and require Apple Silicon hardware.
