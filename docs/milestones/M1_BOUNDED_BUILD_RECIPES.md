# Bounded selected-source build recipes

The native linker and downstream libraries have separate **plan-only** helpers.
They use the selected PureDarwin revision and immutable lock already checked by
the [fixture builder](../../scripts/build_puredarwin_fixtures.py). They do not
install tools, realize dependencies, link fixtures or run a guest. A successful
plan is not a capacity qualification or an M1 result.

## Native linker

[`plan_puredarwin_linker.py`](../../scripts/plan_puredarwin_linker.py) applies
[`puredarwin_native_linker.nix`](../../scripts/puredarwin_native_linker.nix) to the
pinned `native-ld` derivation. It changes the single raw Ninja invocation to
`ninja -j1`, asserts that the SDK tuple and unrelated source/input/configure/install
fields are unchanged, and reports the original and derived identities.

The pinned upstream
[`native-ld.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/native-ld.nix)
uses a custom build phase, so Nix's default Ninja phase cannot supply its job
limit. Its
[`libtapi.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/libtapi.nix)
dependency instead passes `NIX_BUILD_CORES` through `JOBS` to the
[pinned build script](https://github.com/tpoechtrager/apple-libtapi/blob/640b4623929c923c0468143ff2a363a48665fa54/build.sh).
The reported realization command uses one core and one Nix build job.

## Downstream stages

[`plan_puredarwin_libraries.py`](../../scripts/plan_puredarwin_libraries.py) applies
[`puredarwin_libraries.nix`](../../scripts/puredarwin_libraries.nix). Its required
stage selects one output:

| Stage | Changes from the pinned upstream derivation |
| --- | --- |
| `cross-toolchain` | Pass the bounded native linker into the existing toolchain constructor. Only the clang, clang++ and ld wrappers change. |
| `compiler-rt` | Pass that toolchain into compiler-rt, bound build/install Ninja to one job, and copy only compiler-rt into writable staging. |
| `libsystem` | Pass the same linker, toolchain and compiler-rt into libSystem; bound its raw Ninja invocation to one job. |

The SDK and bounded linker must already be realized. Their expected derivation
and output paths come from the retained, inspected receipts. The helper checks
both tuples before running the dry-run. It also recreates the linker override
and asserts that its identity matches the committed native linker recipe. This
prevents the downstream graph from silently restoring the original unbounded
linker.

All target, deployment, architecture and build-mode flags stay upstream's.
Compiler-rt retains its `Release` configuration and existing visibility patch;
libSystem retains `Debug`. No debug-information or optimization flags are
removed. The assertions compare phases after replacing only the intended
dependency paths and job count.

### Compiler target and archive inspection gate

Source inspection leaves a target-precedence question for the actual compiler
runtime build. The pinned
[`compiler-rt.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/compiler-rt.nix)
sets `CMAKE_C_COMPILER_TARGET` and `CMAKE_CXX_COMPILER_TARGET` to
`x86_64-apple-darwin20.4`. The
[`toolchain.nix` wrapper](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/toolchain.nix)
prepends `-target x86_64-apple-macosx26.5`, then forwards caller arguments, including
explicit target arguments. It also removes deployment-minimum arguments. A later
target argument from CMake can therefore affect the effective target. The bounded
recipe preserves this upstream behavior; the effective deployment target for
its compiler-rt objects has not yet been observed.

Retain the actual compile commands and use realized inspection tools with
checked exit statuses to enumerate archive members, verify their architecture,
inspect deployment-version load commands where present, and check required
builtin symbol definitions. Record and explain absent version metadata, missing
symbols and inspection failures before using the archive in a qualified fixture
link. Neither a wrapper filename nor a preserved CMake flag establishes the
resulting object ABI.

The upstream compiler-rt install phase makes its `otool -h` diagnostic optional
with `|| true`. A successful Nix build does not establish that this diagnostic
succeeded. Archive/object qualification therefore requires the separate checked
inspection above, with its output and failures retained. No target or deployment
flags are changed by this documentation requirement.

### LLVM staging and its remaining cost

The pinned
[`compiler-rt.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/compiler-rt.nix)
uses the complete LLVM source input but configures `compiler-rt/lib/builtins`.
The new unpack phase copies the `compiler-rt` directory, makes that copy writable
for the existing patch, and symlinks every other top-level source entry to the
unchanged store input. CMake can still reach the common modules, LLVM modules,
third-party headers and any other sibling source. The original store source is
never edited. An existing staging directory is refused.

This removes a duplicate writable copy of the full LLVM tree. **It does not
remove the complete LLVM source from the dependency closure**, establish a peak
disk requirement, or prove the derived compiler runtime builds successfully.
Sparse source fetching would change the input derivation and requires a separate
reviewed source hash, provenance record and bounded fetch assessment; these
recipes do not perform it.

The inspected source choices are grounded in the selected
[`flake.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.nix),
[`toolchain.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/toolchain.nix),
and [generic build recipe](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/build.nix).
The pinned Nixpkgs
[LLVM version table](https://github.com/NixOS/nixpkgs/blob/fd1462031fdee08f65fd0b4c6b64e22239a77870/pkgs/development/compilers/llvm/default.nix)
selects 21.1.8. Its
[builtins configuration](https://github.com/llvm/llvm-project/blob/llvmorg-21.1.8/compiler-rt/lib/builtins/CMakeLists.txt)
references source outside compiler-rt, which is why the staging recipe keeps
every sibling accessible.

## Plan and inspect before each realization

Set the source path and expected tuples privately from the retained receipts.
Choose a new output directory with an existing parent:

```sh
python3 scripts/plan_puredarwin_libraries.py \
  --stage cross-toolchain \
  --puredarwin-source "$PUREDARWIN_SOURCE" \
  --expected-sdk-derivation "$SDK_DRV" \
  --expected-sdk-output "$SDK_OUT" \
  --expected-linker-derivation "$LINKER_DRV" \
  --expected-linker-output "$LINKER_OUT" \
  --output-dir "$NEW_PLAN_DIRECTORY" \
  --query-substituters
```

Repeat with `--stage compiler-rt` and `--stage libsystem` only after inspecting
the preceding realized output and taking a fresh host census. A cross-toolchain
plan does not force evaluation of the library stages.

Evaluation is offline, disables import from derivation, and forbids lock writes
and updates. The default dry-run is also offline; it describes a local graph and
cannot establish the binary-cache download plan. `--query-substituters` permits
metadata queries during the dry-run. Neither mode realizes outputs. The receipt
distinguishes these modes, records the dry-run exit status and retained log hash,
and always leaves peak-space qualification false.

Receipts retain both recipe snapshots and hashes, source/lock identity,
Burdettie revision, expected and evaluated SDK/linker tuples, original/derived
stage paths, and exact evaluation/dry-run/realization argument arrays. Library
receipts also record the unchanged full LLVM input and unpack/build/install
phase hashes where applicable. Dry-run failure or timeout remains nonzero.

The realization array is recorded for a separately monitored invocation; the
helper never executes it. It sets one Nix job, one core, at most two substitution
jobs, no remote builders, no automatic garbage collection, retained failed build
directories, printed build logs, and a new output link that retains the result.
The owner's existing memory, CPU, disk-reserve, task and time limits must be
applied by the surrounding stage runner. Cache download/unpacked totals alone
do not account for temporary source trees, object files or all local outputs.

## Validation scope

Focused tests run the real Nix evaluator against synthetic package records to
check dependency propagation, stage isolation, unchanged fields and rejection of
missing/duplicated commands or unexpected source/flag changes. A shell test runs
the actual unpack phase and verifies that only compiler-rt is copied, other
entries remain symlinks, source bytes are preserved, and existing staging is
not overwritten. Python workflow tests check receipt binding, offline/default
behavior, explicit metadata queries, preservation of prior output and nonzero
failures. The existing M1 CI job requires its pinned Nix evaluator so these
assertions cannot silently skip there.

These checks establish recipe behavior. A real selected-source evaluation,
realization, strict fixture link, guest execution, Burdettie runtime execution
and security qualification require separate observed receipts.
