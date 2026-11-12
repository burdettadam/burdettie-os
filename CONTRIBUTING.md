# Contributing to Burdettie-OS

Burdettie-OS is an experimental systems project. Useful contributions include
small compatibility fixes, focused tests, fixture sources, and documentation
that explains supported behavior and its limits.

## Reporting bugs

Open a GitHub issue with:

- The repository commit, host operating system, CPU architecture, and `rustc --version`.
- Commands to reproduce the problem and the expected and actual behavior.
- A minimal fixture source or a description of the affected binary's dependencies.

Use project-owned or redistributable test inputs. Keep imported application
bundles, extracted platform libraries, credentials, and private local logs out
of public issues. Report vulnerabilities through the [security policy](.github/SECURITY.md).

## Native development setup

The native fixture workflow uses Apple Silicon, macOS 13 or later, and Xcode
Command Line Tools. Rust is pinned to **1.93.0** in
[rust-toolchain.toml](rust-toolchain.toml); use that toolchain when reproducing
build results.

```sh
git clone https://github.com/burdettadam/burdettie-os.git
cd burdettie-os

# Build the workspace and compile local fixtures from their C/Objective-C sources.
cargo build --workspace --locked
./scripts/build_fixtures.sh

# Run checks after the fixtures are available.
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The strict Clippy command may expose existing warnings; include the exact
command and outcome in your change report. The repository's
[CI workflow](.github/workflows/ci.yml) is the reference for CI-specific lint
allowances and platform jobs. See the [build guide](docs/BUILD.md) for the Linux
and Docker workflows. Native execution, interpretation, GUI tests, and fixture
compilation have different platform requirements.

Fixture source, expectations, and small parser test inputs belong in version
control. Locally compiled applications, object files, Swift build caches, and
imported compatibility inputs are generated or external artifacts; see the
[source and artifact policy](docs/PUBLICATION.md).

## Pull requests

1. Create a focused branch from `main`.
2. Add or update a small reproducible test or fixture when changing runtime behavior.
3. Run the checks relevant to the affected crates and platform. Report failures,
   skipped checks, and any external inputs needed to reproduce the result.
4. Format Rust changes with `cargo fmt --all`.
5. Update documentation when behavior or setup changes.
6. Explain the problem, the implementation, and the resulting behavior in the PR.

## Code style

- Follow the workspace lint configuration in `Cargo.toml`.
- Explain any lint suppression instead of adding a broad allowance.
- Give each unsafe block a `// SAFETY:` comment describing the invariant that
  makes the operation valid.
- Keep shared runtime logic separate from platform-specific implementations.

The workspace is layered: lower-level crates should not depend on higher-level
ones. When adding a crate, update `Cargo.toml` and the relevant architecture
references in `docs/`. The [crate map](docs/CRATE_MAP.md) is a useful starting
point; `Cargo.toml` is the current workspace inventory.

## License

By submitting a pull request you agree that your contribution will be licensed
under the terms of both the [MIT](LICENSE-MIT) and
[Apache 2.0](LICENSE-APACHE) licenses, at the recipient's option. Preserve separate
license notices for third-party components.
