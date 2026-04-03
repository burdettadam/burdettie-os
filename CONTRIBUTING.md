# Contributing to burdettieOS

Thank you for your interest in contributing!

## Reporting Bugs

Open a GitHub issue with:
- Operating system and Rust toolchain version (`rustc --version`)
- The Mach-O binary or fixture that triggers the issue (or a minimal reproducer)
- Steps to reproduce and expected vs. actual output

## Development Setup

```sh
# Requires Rust 1.82+ (see rust-toolchain.toml)
git clone https://github.com/burdettadam/burdettie-os
cd burdettieOS

# Build everything
cargo build --workspace

# Run the full test suite
cargo test --workspace

# Lint (must be zero warnings)
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt --all
```

Fixture binaries under `fixtures/*/app` are pre-built ARM64 Mach-O binaries
committed to the repo. To rebuild them you need an Apple Silicon Mac with Xcode
installed and can run `scripts/build_fixtures.sh`.

## Pull Requests

1. Fork the repo and create a branch from `main`.
2. Make your changes. Keep each PR focused — one logical change per PR.
3. Ensure `cargo test --workspace` and `cargo clippy --all-targets` pass with
   zero failures or warnings.
4. Run `cargo fmt --all` before pushing.
5. Update any relevant documentation in `docs/`.
6. Describe **what** changed and **why** in your PR description.

## Code Style

- Follow Rust idioms. The lints in the workspace `Cargo.toml` `[lints]` section
  are enforced by CI — fix all warnings before submitting.
- Do not add `#[allow(clippy::...)]` suppressions without a comment explaining
  why the lint does not apply.
- Unsafe code requires a `// SAFETY:` comment explaining the invariant.

## Crate Guidelines

The workspace is layered — lower-level crates must not depend on higher-level
ones. See [docs/CRATE_MAP.md](docs/CRATE_MAP.md) for the dependency graph.

Adding a new crate requires updating:
- `Cargo.toml` (workspace members)
- `docs/CRATE_MAP.md`
- `docs/CAPABILITIES.md` if it adds user-facing capability

## License

By submitting a pull request you agree that your contribution will be licensed
under the terms of both the [MIT](LICENSE-MIT) and
[Apache 2.0](LICENSE-APACHE) licenses, at the recipient's option.
