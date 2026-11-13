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

Validation is recorded by revision and execution environment. On 2026-11-12
(America/Denver), [CI run 43343829535](https://github.com/burdettadam/burdettie-os/actions/runs/43343829535) validated source commit
`1bf44e2e552254c450d3a7a86cc55487e7f49b51` with Rust 1.93.0. Native jobs used macOS 26.7.2 ARM64;
the Linux interpreter job used Ubuntu 24.04 ARM64, and the Docker development job
used an Ubuntu 26.04 x86_64 host. These are separate execution environments.

| Check | Observed result on this source revision |
| --- | --- |
| Native fixture generation | Passed; project fixtures and corpus rebuilt from source, including presence checks for all 40 interpreter fixtures. |
| Native macOS workspace | Formatting, strict Clippy with `--keep-going`, debug/all-targets and release builds, full workspace tests, GPU-feature graphics library tests, and runtime bundle generation/upload passed. |
| macOS README quick start | Passed: workspace build, fixture compilation, text/JSON inspection, guest output and successful execution. |
| Linux ARM64 interpreter | All 40 fixtures passed in the ARM64 Docker image, with no failures or ignored tests. Includes Objective-C superclass/bridging regressions. |
| Docker portable libraries | Strict container Clippy and 344 library tests passed. |
| C and Swift interop | 35 Rust FFI tests, 6 C client checks, and 15 Swift tests passed; installation, Swift package and client builds passed. |
| Security | `cargo audit` passed with maintenance warnings for `paste` (RUSTSEC-2024-0436) and `ttf-parser` (RUSTSEC-2026-0192). `cargo deny` passed bans, licenses and sources; its advisory check remains failed for `ttf-parser`. No advisory ignore or warning-policy change was introduced. |
| Source/artifact hygiene | Repository hygiene and full reachable-history Gitleaks passed on this head. |

The workspace and GPU-feature runs overlap; their test counts are not added to
other suites. The successful native job reported 2 ignored test entries across
its test invocations: the pre-existing unknown-ISA abort-path test and the
`register_shims` documentation example. Those cases are not claimed as executed.
Tests that return
early for unavailable external inputs are subject to their existing fixture and
platform prerequisites. See the job log for individual suites and exclusions.

### Recovery and preservation

The [recovery manifest](RECOVERY_MANIFEST.json) records all 35 original PR commits,
parents, trees, changed paths, and 34 verified reviewed-commit payloads recovered
from the maintenance workflow history. The recovered commit `852ce324` was
recreated as `12cb7d60c10080702a0b354d885f4997c36c6971` with the same parent
`dd08ad9248b46406f065c3a885798b90c45dc13d` and identical tree
`955636ce9ee8968ef5448d57cc52a93c20baa3a4`, using the public GitHub no-reply
identity for author and committer.

The manifest enumerates every intentional post-recovery difference. These include
native typed C-string lookup, single-close CoreGraphics ownership, Darwin host
range checks using Mach VM regions, and an isolated unmapped-page regression.
An ARM64 TLV trampoline now preserves live integer/vector registers across the
Rust getter, with a direct register-preservation regression; ordinary C calls
cannot satisfy that special Darwin lookup convention. Native stdio is registered
explicitly, and Darwin sysconf/rlimit constants are no longer translated to Linux
values on a macOS host. The host-fallback allowlist is unchanged.

Scorecard entries use the existing process-state reset API for independent Cocoa
application lifecycles. Native pthread attribute tests use complete aligned
storage while Linux retains its small opaque-handle virtualization regression.
Queue concurrency tests exercise the actual bounded worker count. Category
metadata is preserved during fixture linking; synthetic export terminal lengths,
code-signature oracle comparisons and Foundation count expectations match the
reproducible inputs and implementation. Scoped helper annotations retain the
strict warning policy. CI runs workspace tests with `--no-fail-fast` before release,
retains failure caches and captures diagnostic crash backtraces. All required
gates remain present, and other recovered source blobs are identical.

No regression test was removed without replacement: the invalid
fake-pointer descriptor-sorting test was renamed and repaired with real objects;
87 other named test functions were added relative to public `main`.

The abbreviated IDs `ab7abbd` and `15b33db` are unconfirmed: neither local checkout
nor the public GitHub commit API can resolve them. They are not described as
separately recovered commits. Public Git objects and verified payloads are the
source of truth. The old local checkout was not changed.

### Tracked font dependency migration

[Issue #2](https://github.com/burdettadam/burdettie-os/issues/2) tracks replacing
both `ttf-parser` paths: `cosmic-text -> fontdb -> ttf-parser`, and
`ab_glyph -> owned_ttf_parser -> ttf-parser`. Upstream cosmic-text 0.19's
[manifest](https://github.com/pop-os/cosmic-text/blob/be1e0bf8d104c61c0e094336407694d0466a2057/Cargo.toml)
uses skrifa, harfrust and fontdb 0.24; verify the published release dependency
graph before migration. Upgrading it alone leaves the ab_glyph path. The separate
migration plan covers skrifa-backed font metrics/outlines,
software/GPU rasterization, deterministic comparisons against embedded DejaVu
fonts, malformed font regressions, GUI checks, Cargo.lock and license updates,
and unsuppressed audit/deny verification. The runtime recovery does not claim that
this dependency migration or an all-green security workflow has been completed.

### Earlier local regression coverage

The following earlier recovery observations were recorded on Linux x86_64 with
Rust 1.93.0. They are retained from the reviewed recovery record and are not
reruns of `1bf44e2`. Rows describe individual suites; focused regressions may also
belong to a larger suite. Native results above supersede earlier pending CI notes.

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
| Foundation aggregate ABI follow-up | 248 affected-module tests passed, including seven new regressions for aggregate layouts, transform roundtrips, nil behavior, and XML normalization. These are host-side tests; native ARM64 calling-convention coverage is included in the workspace result above. |
| Complete Foundation library | All 3,020 tests passed in two consecutive runs with default parallel execution and no ignored tests. Coverage includes pointer-backed CFString decoding, timer callback reentrancy, repaired test objects, and process/global-state isolation. The focused timer, NSView, and responder runs passed 57, 66, and 37 tests respectively. |
| Shared-cache storage | The existing missing-cache error-path regression passed after restricting the owned-memory fallback to non-Unix hosts. |
| UI and service follow-ups | 69 `app-kit-lite`, 244 `ui-controls`, and 39 `burdettie-services` tests passed. |
| Darwin clock conversion arithmetic | Four unit tests extracted unchanged from the source passed on Linux: identity, non-unit scaling/truncation, intermediate overflow, and saturation. This does not compile the Darwin crate or execute Mach APIs. The native sample-bracketing test is covered by the native workspace result above. |

The exact portable Clippy command from
[`docker/Dockerfile.dev`](../docker/Dockerfile.dev) passed locally as well as in the
recorded Docker job. `arm64-translate --all-targets --target
aarch64-unknown-linux-gnu` passed with the CI warning policy; cross-compilation
does not execute ARM64 code.

An earlier workspace Clippy check passed on Linux with an explicit local
`-A dead_code` qualification for ARM-only JIT code on x86_64 and exclusion of the
Darwin-only host crate. Those qualifications belonged to that historical local
check. The native result above used the checked-in strict CI warning policy.

### Remaining validation and compatibility limits

- **Font dependency maintenance.** `cargo deny check advisories` reports
  [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html) for the
  unmaintained `ttf-parser` dependency. The advisory remains unsuppressed; the
  font stack migration is tracked in [issue #2](https://github.com/burdettadam/burdettie-os/issues/2).
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
