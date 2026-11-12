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

## Outstanding validation gates

Source cleanup and a working quick start do not establish a fully passing CI
baseline. Two broader gates remain open:

- **Font dependency maintenance.** `cargo deny check advisories` reports
  [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html) for
  the unmaintained `ttf-parser` dependency. The advisory remains unsuppressed;
  the font stack needs a migration to maintained parsing dependencies.
- **Strict portability lints.** The Docker development job reaches existing
  `macho-parse` Clippy warnings under `-D warnings`, and the portable test targets
  also have existing `platform-core` lint debt. The macOS ARM64 job also reports
  existing `arm64-translate` Clippy warnings. These checks need to pass before
  claiming a clean strict-lint baseline; a successful image build does not mean
  its later test steps ran.

Use the [CI workflow](../.github/workflows/ci.yml), [dependency policy](../deny.toml),
and recorded job results to distinguish successful demonstrations from the
remaining repository-wide checks.
