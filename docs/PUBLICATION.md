# Source and Artifact Policy

The public repository should let a reader inspect the implementation, build the
project's own examples, and understand the limits of the results. Keep source and
reproducible test definitions in Git; keep local builds and external validation
inputs outside the published tree and history.

## What belongs in Git

| Keep | Reason |
| --- | --- |
| Rust, C, Objective-C, Swift, and assembly source | The implementation and its source-built examples. |
| Fixture and corpus sources, build scripts, and expected output | Reproducible compatibility checks. |
| Small synthetic malformed inputs | Parser regression tests that need exact input bytes. |
| Dependency lockfiles and build definitions | A reviewable build configuration. |
| Architecture notes, limitations, and clearly scoped results | Evidence that helps readers assess the engineering. |
| Required redistributable assets with their notices | Runtime resources with explicit provenance, such as bundled fonts. |

## What stays local

- Cargo output, Swift `.build` and `.swiftpm` state, object files, debug symbols,
  compiler caches, and generated executable or shared-library files.
- Terminal transcripts, temporary analysis output, editor state, personal
  journals, credentials, and local memory databases.
- Imported application bundles, extracted system frameworks, SDKs, developer
  tools, and captures gathered from a particular host.
- Built packages, disk images, VM state, and container exports. Release artifacts
  need their own review before distribution.

Use `imports/` for local imported software and the documented
`docker/xcode-tools/` staging area for tool experiments. Build fixture binaries
with [`scripts/build_fixtures.sh`](../scripts/build_fixtures.sh). Do not replace
source inputs with an unexplained binary copied from a development machine.

The project's MIT/Apache licenses apply to project-owned source. They do not
grant redistribution rights for external software. Preserve third-party notices
and document the origin and build recipe for any dependency included in a
release.

## Before committing

Review the files actually tracked by Git, as well as ignore behavior:

```sh
git status --short --ignored
git ls-files --cached --ignored --exclude-standard
git diff --cached --stat
git diff --cached
```

The second command identifies files that are already tracked despite matching an
ignore rule. Remove generated or local-only files from the index while retaining
the local copy when it is still useful. Review any intentional exception rather
than broadly ignoring all binary files or all research documentation.

Keep local machine settings outside tracked build definitions. Prefer environment
variables such as `CARGO_TARGET_DIR` when a developer wants a different build
location.

## Before publishing a repository or release

An ignore rule only affects future tracking. Deleting a tracked file in a new
commit leaves earlier copies in Git history. A publication review therefore needs
to cover every branch and tag intended for publication, historical names of
removed files, commit metadata, and any separate release or Actions artifacts.

Prepare and verify a private backup before rewriting history. Inspect the cleaned
result, confirm that required sources and notices remain, and run the documented
build from a clean checkout. Refresh remote refs before applying a history
replacement so concurrent work is not overwritten. GitHub describes the
additional limitations around cached references and old clones in its
[history-removal guide](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository).

Record the commit, host architecture, toolchain, fixture build command, and test
command with compatibility results. Keep failures, skipped checks, and external
input requirements visible so a reviewer can reproduce the same scope.

## Validation status

The runtime, parser, and text-rendering fixes have local regression coverage.
The following checks passed on Linux x86_64 with Rust 1.93.0 on 2026-11-12:

| Check | Result and scope |
| --- | --- |
| Portable library suites | 344 tests passed across the 11 packages selected by the Docker development job. |
| Malformed Mach-O metadata | 20 integration tests passed, covering bounded export-trie traversal, invalid variable-length integers, address arithmetic, and architecture-specific thread entry points. |
| Objective-C superclass repair | 5 focused tests passed, covering mapped class records, synthetic handles, replacement, removal, and overlays. |
| Foundation class storage | 7 focused tests passed, including pointer alignment, storage layout, and child/metaclass hierarchy reads. |
| ARM64 interpreter library | 500 tests passed, including 22 atomic-instruction regressions and a string-read address-overflow regression. |
| GPU text-layout and glyph-cache feature | 253 library tests passed with `cargo test --locked -p graphics-core --lib --features gpu`. This exercises layout, rasterization, and atlas logic without a physical GPU. |
| Strict portable Clippy | The exact command from [`docker/Dockerfile.dev`](../docker/Dockerfile.dev) passed locally with `-D warnings` and the existing CI lint allowances. The Docker image itself was not executed in this check. |
| ARM64 interpreter Clippy | `arm64-translate --all-targets --target aarch64-unknown-linux-gnu` passed with the CI warning policy. This cross-compilation check does not execute ARM64 code. |

Foundation class storage now has the alignment required by its pointer fields,
and superclass repair reads only class records explicitly registered as mapped
storage. The previously crashing `interp_cf_bridging` fixture now completes
locally with exit code 0: one test passed, with the other 39 cases filtered out.
Its variadic `printf` output still contains incorrect values through the x86_64
FFI fallback. Formatted output and the complete native ARM64 interpreter suite
require further validation.

Byte and halfword LSE/SWP operations retain a non-atomic fallback, limiting their
use for guest-thread synchronization.

The interpreter's x86_64 test build still emits warnings for ARM64 JIT code.
The Clippy results above cover the listed packages and targets; workspace-wide
strict lint validation remains open.

Two broader gates remain open:

- **Font dependency maintenance.** `cargo deny check advisories` reports
  [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html) for
  the unmaintained `ttf-parser` dependency. The advisory remains unsuppressed;
  the font stack needs a migration to maintained parsing dependencies.
- **Native CI after the fixes.** The macOS ARM64 workspace build, strict lints,
  and tests, the Docker development job, and the complete Linux ARM64
  interpreter suite need fresh CI results. Passing local portable tests does
  not establish that these platform-specific jobs pass.

Use the [CI workflow](../.github/workflows/ci.yml), [dependency policy](../deny.toml),
and recorded job results to distinguish successful demonstrations from the
remaining repository-wide checks.
