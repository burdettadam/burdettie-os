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

Validation is recorded by revision and execution environment. The latest completed
native run recorded here is [CI run 43311827892](https://github.com/burdettadam/burdettie-os/actions/runs/43311827892)
on 2026-11-12, for source commit
[`dd08ad9`](https://github.com/burdettadam/burdettie-os/commit/dd08ad9248b46406f065c3a885798b90c45dc13d).
Follow-up fixes have separate local coverage and need another native CI run.

### Recorded native CI

| Check | Result and scope |
| --- | --- |
| Source-built Mach-O fixtures | Passed. Fixtures and corpus programs were compiled from the repository's sources on macOS. |
| README quick start, macOS ARM64 | Passed. Built the workspace, compiled the C fixture, inspected its JSON metadata, and verified guest output and exit code 0. |
| Linux ARM64 interpreter | All 40 end-to-end fixtures passed, including the previously aborting Foundation bridging case. |
| Docker development checks | Passed the container's strict lint checks and all 344 tests across its 11 selected portable library packages. |
| C and Swift embedding | Passed 35 Rust FFI tests, 6 C client checks, and 15 Swift tests, plus the installed-library C client and Swift client build. |
| Full macOS build-and-test job | Stopped at strict Clippy with 8 diagnostics in native-only paths. Follow-up source corrections have been reviewed; the later debug/release build, workspace test, GPU-feature, and bundle steps still need a new native run. |
| Dependency checks | `cargo audit` passed with maintenance warnings. `cargo deny` passed bans, licenses, and sources, but failed the `ttf-parser` maintenance advisory described below. |

The [PR](https://github.com/burdettadam/burdettie-os/pull/1) and its checks record
subsequent revisions. A passing quick start or selected fixture set does not
establish that every workspace test or guest application passes.

The strict Clippy step uses `--keep-going` to collect independent failures without
changing the warning policy. Targets blocked by a failed dependency can still
reveal further diagnostics after that dependency is repaired.

### Local regression coverage

The following checks were run on Linux x86_64 with Rust 1.93.0. Rows describe
individual suites; focused regressions may also belong to a larger suite.

| Check | Result and scope |
| --- | --- |
| ARM64 interpreter library | 500 tests passed, including 22 atomic-instruction regressions and a C-string address-overflow regression. |
| GPU text-layout and glyph-cache feature | 253 library tests passed with `cargo test --locked -p graphics-core --lib --features gpu`; covers layout, rasterization, and atlas logic without a physical GPU. |
| Malformed Mach-O metadata | 20 integration tests passed, covering bounded export-trie traversal, invalid variable-length integers, address arithmetic, and architecture-specific thread entry points. |
| Foundation class storage | 7 focused tests passed, including pointer alignment, storage layout, and child/metaclass hierarchy reads. |
| Objective-C runtime | 219 library tests passed; one pre-existing abort-path test remained ignored. The focused superclass-repair cases cover mapped records, synthetic handles, replacement, removal, and overlays. |
| Mach-O execution, graphics shims, and internationalization | 154 library tests passed: 83 in `macho-exec`, 21 in `cg-shims`, and 50 in `icu-shims`. TLV registry tests now isolate their global resets through the full test lifetime. |
| Launchd | 170 library tests passed with default test threading. Error-string regressions verify NUL termination; global registry reset tests are isolated. |
| System shims | 603 library tests passed and 2 failed because this environment denies Unix-domain socket creation with `EPERM`; a separate direct socket probe reproduced that restriction. No tests were ignored. Three focused DNS service lifecycle/reentrant-callback tests passed. |
| Foundation aggregate ABI follow-up | 248 affected-module tests passed, including seven new regressions for aggregate layouts, transform roundtrips, nil behavior, and XML normalization. These are host-side tests; native ARM64 calling-convention execution still needs CI. |
| Complete Foundation library | All 3,020 tests passed in two consecutive runs with default parallel execution and no ignored tests. Coverage includes pointer-backed CFString decoding, timer callback reentrancy, repaired test objects, and process/global-state isolation. The focused timer, NSView, and responder runs passed 57, 66, and 37 tests respectively. |
| Shared-cache storage | The existing missing-cache error-path regression passed after restricting the owned-memory fallback to non-Unix hosts. |
| UI and service follow-ups | 69 `app-kit-lite`, 244 `ui-controls`, and 39 `burdettie-services` tests passed. |
| Darwin clock conversion arithmetic | Four unit tests extracted unchanged from the source passed on Linux: identity, non-unit scaling/truncation, intermediate overflow, and saturation. This does not compile the Darwin crate or execute Mach APIs. The added native sample-bracketing test still needs macOS CI. |

The exact portable Clippy command from
[`docker/Dockerfile.dev`](../docker/Dockerfile.dev) passed locally as well as in the
recorded Docker job. `arm64-translate --all-targets --target
aarch64-unknown-linux-gnu` passed with the CI warning policy; cross-compilation
does not execute ARM64 code.

The follow-up workspace Clippy check passed for all targets on Linux with an
explicit local `-A dead_code` qualification for ARM-only JIT code compiled on
x86_64 and exclusion of the Darwin-only host crate. The final NSView/responder
changes after that check only isolate existing test fixtures; their focused and
full-suite results are included above. Neither qualification changes the CI
workflow or establishes a native workspace pass.

### Remaining validation and compatibility limits

- **Full native workspace validation.** The follow-up source needs a fresh macOS
  ARM64 strict-lint, debug/release build, complete test, GPU-feature, and bundle
  run. Keep the actual job results visible when updating this record.
- **Font dependency maintenance.** `cargo deny check advisories` reports
  [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html) for the
  unmaintained `ttf-parser` dependency. The advisory remains unsuppressed; the
  font stack needs a migration to maintained parsing dependencies.
- **Interpreter synchronization.** Byte and halfword LSE/SWP operations retain a
  non-atomic fallback, and exclusive-monitor behavior is not a complete
  cross-thread synchronization model. Unsupported narrow CAS/pair-exclusive
  forms are rejected rather than executed at an incorrect width.
- **Host-pointer and ABI scope.** Guest execution remains inside the host process.
  A mapped-page check does not guarantee a host pointer's access permissions or
  lifetime. The x86_64 variadic FFI fallback still produces incorrect formatted
  values in the otherwise completing Foundation bridging fixture.
- **Stub coverage.** Framework and system shims include partial implementations.
  The DNS service ownership/reentrancy repairs do not establish full Bonjour
  callback or protocol compatibility.

Use the [CI workflow](../.github/workflows/ci.yml),
[dependency policy](../deny.toml), and revision-specific job results when assessing
which demonstrations are reproducible and which checks remain open.
