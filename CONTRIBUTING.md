# Contributing to Burdettie-OS

Burdettie-OS builds on Linux and existing open-source components, with Rust
interfaces and bridges where they serve the product. Useful contributions
include upstream fixes, focused compatibility tests, source-owned fixtures,
project integration, and documentation that explains supported behavior and
its limits. A broad Rust rewrite is not a contribution requirement.

The current direction is recorded in
[ADR 0003](docs/adr/0003-linux-foundation-and-upstream-reuse.md). Read the
[upstream strategy](docs/UPSTREAM_STRATEGY.md) before adding a replacement
component, and use [RESEARCH_QUESTIONS.md](docs/RESEARCH_QUESTIONS.md) for
unresolved architecture and component-selection work.

## Reporting bugs

Open a GitHub issue with:

- The repository commit, host operating system, CPU architecture, and `rustc --version`.
- Commands to reproduce the problem and the expected and actual behavior.
- The target object format/architecture and execution backend when reporting
  compatibility behavior; distinguish native Linux, interpreted Mach-O, native
  compatibility execution, and reference-guest results.
- A minimal fixture source or a description of the affected binary's dependencies.

Use project-owned or redistributable test inputs. Keep imported application
bundles, extracted platform libraries, credentials, and private local logs out
of public issues. Report vulnerabilities through the
[security policy](.github/SECURITY.md).

## Linux development setup

Linux is the primary runtime and development foundation. Rust is pinned to
**1.93.0** in [rust-toolchain.toml](rust-toolchain.toml); use that toolchain when
reproducing build results. A native C compiler is needed for the existing
command adapter's compile/run test.

```sh
git clone https://github.com/burdettadam/burdettie-os.git
cd burdettie-os

# A bounded native-tool workflow without a Darwin SDK.
cargo build --locked -p burdettie-xcode-tools --bins
cargo test --locked -p burdettie-xcode-tools
cargo fmt --all -- --check

# Reproducible Linux parsing/analysis build, focused shim tests, and lint.
docker build -f docker/Dockerfile.dev -t burdettie-os-dev .
docker run --rm burdettie-os-dev
```

The adapter's sample-project test returns early when no host compiler is
available. Verify compiler availability before using that test as evidence of
a successful build. A native Linux ELF output and a Darwin Mach-O output are
different targets, even when they share a project file or command name.

The [build guide](docs/BUILD.md) describes broader Linux builds, native GUI
dependencies, the runtime Docker image, and interpreted-runtime tests. Choose
the workflow relevant to the change. Some tests consume generated Mach-O
fixtures, and some workspace members remain Darwin-specific; the full
workspace test command is not a universal Linux setup check.

### Darwin reference workflow

The current native Darwin fixture workflow uses Apple Silicon, macOS 13 or
later, and Xcode Command Line Tools. It remains available for reference
behavior and generating the fixtures consumed by compatibility jobs:

```sh
cargo build --workspace --locked
./scripts/build_fixtures.sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The strict Clippy command may expose existing warnings. The
[CI workflow](.github/workflows/ci.yml) is the reference for its explicit lint
allowances, generated-input preparation, and platform jobs. Report exact
commands and outcomes; a documented recipe is not a claim of a passing result.

Fixture source, expectations, and small parser test inputs belong in version
control. Locally compiled applications, object files, Swift build caches, and
imported compatibility inputs are generated or external artifacts; see the
[source and artifact policy](docs/PUBLICATION.md).

## Reuse and upstream contributions

Before adding a framework, runtime service, compiler/build component, or
platform backend, identify the existing upstream option and the smallest
interface Burdettie needs. Evaluate the actual behavior, target ABI, dependency
closure, maintenance status, and license. Language preference alone does not
justify duplicating a maintained implementation.

Prefer a reusable fix in the project that owns the component. PureDarwin is
an upstream destination for changes within its Darwin system/build scope;
Darling, LLVM/Clang/LLDB, Swift, Rust tooling, and desktop projects have their
own scopes. PureDarwin is not the destination for every Linux compatibility or
Burdettie policy change. Keep product-specific trust rules, UX, and integration
in Burdettie unless an upstream explicitly wants a general interface.

For an upstream candidate, prepare a minimal reproducer, focused change,
relevant tests, source/license provenance, and an explanation of the broader
use case. Check that project's current contribution and AI-assistance rules
before submitting. Acceptance is a maintainer decision; this repository's
policy does not imply that another project has agreed to accept a change.
Record any necessary downstream patch, its owner, and the condition for
removing it after upstream integration.

Keep existing Burdettie implementations available until a replacement meets a
named behavioral contract. Preserve semantic regressions across that
transition. Rust remains appropriate for safe interfaces, adapters, and new
product logic, without requiring compilers, debuggers, libraries, or the Linux
kernel to be rewritten. See the
[developer-tool plan](docs/XCODE_PUBLIC_COMPATIBILITY_PLAN.md) for the existing
`.pbxproj` and Xcode command adapters and their supported subset.

## Pull requests

1. Create a focused branch from `main`.
2. Explain the user-visible problem, the component reused, and why any new
   implementation is necessary. Identify an upstream destination when relevant.
3. Add or update a small reproducible test or fixture when changing runtime
   behavior. Documentation-only changes do not require new runtime tests.
4. Run checks relevant to the affected crates and platform. Report failures,
   skipped checks, target/backend details, and external inputs needed to
   reproduce the result. Separate useful behavior from startup-only probes.
5. Format Rust changes with `cargo fmt --all` and update affected documentation.
6. Explain the implementation and resulting behavior, including any remaining
   compatibility limits or temporary downstream patches.

## Code style

- Follow the workspace lint configuration in `Cargo.toml`.
- Explain any lint suppression instead of adding a broad allowance.
- Give each unsafe block a `// SAFETY:` comment describing the invariant that
  makes the operation valid.
- Keep shared interfaces separate from platform-specific implementations and
  state the ownership, lifetime, and error contract of foreign-code bridges.

The workspace is layered: lower-level crates should not depend on higher-level
ones. When adding a crate, update `Cargo.toml` and the relevant architecture
references in `docs/`. The [crate map](docs/CRATE_MAP.md) is a useful starting
point; `Cargo.toml` is the current workspace inventory.

## License

By submitting a pull request you agree that your contribution will be licensed
under the terms of both the [MIT](LICENSE-MIT) and
[Apache 2.0](LICENSE-APACHE) licenses, at the recipient's option. Preserve
separate license notices and applicable obligations for third-party components;
reuse does not relicense upstream code under Burdettie's terms.
